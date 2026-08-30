package github

import (
	T "github.com/medulla-sh/cuet"
)

#RepositoryVisibility: "public" | "private" | "internal"

#Repository: {
	in: {
		// Adopts an existing repository by its GitHub name.
		#import?: string

		// Used as both the GitHub repository name and Terraform resource key.
		name: =~"^[A-Za-z0-9_.-]{1,100}$"

		// Displayed below the repository name on GitHub.
		description?: string

		// Links to the project website displayed by GitHub.
		homepageUrl?: string

		// Must be explicit because the provider defaults new repositories to public.
		visibility: #RepositoryVisibility

		// Selects an existing branch as the repository default.
		defaultBranch: string & !=""

		// Controls which GitHub Actions workflows may run in the repository.
		actions?: {
			// Enables workflow execution.
			enabled: bool

			// Allows all, local-only, or explicitly selected actions and workflows.
			allowed: "all" | "local_only" | "selected"
			allowed: _ | *"all"

			// Requires external actions and reusable workflows to use commit SHAs.
			requireShaPinning: bool
			requireShaPinning: _ | *false

			if allowed == "selected" {
				selected: {
					// Allows actions maintained by GitHub.
					githubOwned: bool
					githubOwned: _ | *true

					// Allows actions from verified Marketplace creators.
					verified: bool
					verified: _ | *false

					// Allows additional action and reusable workflow patterns.
					patterns: [...string]
				}
			}
		}

		// Applies named policies to matching branches or tags.
		rulesets?: [string]: {
			// Adopts an existing ruleset using a <repository>:<ruleset ID> identifier.
			#import?: string

			// Overrides the map key as the ruleset name displayed by GitHub.
			name?: string & !=""

			// Controls whether rules are active, evaluated, or disabled.
			enforcement: "disabled" | "active" | "evaluate"
			enforcement: _ | *"active"

			({
				// Selects the branch refs governed by this ruleset.
				branches: {
					include: [string, ...string]
					exclude: [...string]
				}
			} | {
				// Selects the tag refs governed by this ruleset.
				tags: {
					include: [string, ...string]
					exclude: [...string]
				}
			})

			// Allows selected GitHub users or Apps to bypass this ruleset.
			bypassActors: [...{
				bypassMode: "always" | "pull_request" | "exempt"
				bypassMode: _ | *"always"

				({
					// Resolves a GitHub user by login.
					user: =~"^[A-Za-z0-9][A-Za-z0-9-]*$" & !~"-$"
				} | {
					// Resolves a GitHub App by slug.
					app: =~"^[A-Za-z0-9][A-Za-z0-9-]*$" & !~"-$"
				})
			}]

			rules: {
				// Restricts matching ref updates to bypass actors.
				preventUpdates: bool
				preventUpdates: _ | *false

				// Prevents matching refs from being deleted.
				preventDeletion: bool
				preventDeletion: _ | *false

				// Prevents force pushes to matching refs.
				preventForcePushes: bool
				preventForcePushes: _ | *false

				// Rejects merge commits on matching refs.
				requireLinearHistory: bool
				requireLinearHistory: _ | *false

				// Accepts only commits with verified signatures.
				requireSignatures: bool
				requireSignatures: _ | *false

				statusChecks?: {
					// Requires the tested commit to include the latest target branch state.
					strict: bool
					strict: _ | *false

					// Allows a matching ref to be created before checks can run.
					allowCreationBeforeChecks: bool
					allowCreationBeforeChecks: _ | *false

					required: [{
						// Matches the status check context reported to GitHub.
						context: string & !=""

						// Restricts the result to a specific GitHub App when set.
						integrationId?: int & >0
					}, ...]
				}
			}
		}

		// Controls the optional collaboration surfaces available in the repository.
		features: {
			// Enables GitHub Issues.
			issues: bool
			issues: _ | *true

			// Enables GitHub Discussions.
			discussions: bool
			discussions: _ | *false

			// Enables classic repository projects when allowed by the organization.
			projects: bool
			projects: _ | *true

			// Enables the repository wiki.
			wiki: bool
			wiki: _ | *true
		}

		// Controls pull request merge methods and post-merge behavior.
		merge: {
			// Allows merge commits.
			commit: bool
			commit: _ | *true

			// Allows squash merging.
			squash: bool
			squash: _ | *true

			// Allows rebase merging.
			rebase: bool
			rebase: _ | *true

			// Allows pull requests to merge automatically after requirements pass.
			auto: bool
			auto: _ | *false

			// Lets authors update an out-of-date pull request branch from GitHub.
			updateBranch: bool
			updateBranch: _ | *false

			// Removes a pull request branch after merge.
			deleteBranch: bool
			deleteBranch: _ | *false
		}

		// Searchable labels attached to the repository.
		topics: [...string]

		// Configures private or internal repository forking when set.
		allowForking?: bool

		// Allows other repositories to be generated from this repository.
		isTemplate: bool
		isTemplate: _ | *false

		// Creates an initial commit when GitHub creates an empty repository.
		autoInit: bool
		autoInit: _ | *false

		// Makes the repository read-only; GitHub does not support unarchiving via this API.
		archived: bool
		archived: _ | *false

		// Archives the repository instead of deleting it from GitHub.
		archiveOnDestroy: bool
		archiveOnDestroy: _ | *true

		// Requires DCO signoff for commits authored in GitHub's web UI.
		webCommitSignoffRequired: bool
		webCommitSignoffRequired: _ | *false
	}

	ref: "github_repository.\(in.name)"

	out: T.#TerraformInput & {
		resource: github_branch_default: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			"repository": "${\(ref).name}"
			branch:       in.defaultBranch
		}

		if in.actions != _|_ {
			resource: github_actions_repository_permissions: (in.name): {
				if in.#import != _|_ {
					#import: in.#import
				}

				"repository":         "${\(ref).name}"
				enabled:              in.actions.enabled
				sha_pinning_required: in.actions.requireShaPinning

				if in.actions.enabled {
					allowed_actions: in.actions.allowed

					if in.actions.allowed == "selected" {
						allowed_actions_config: {
							github_owned_allowed: in.actions.selected.githubOwned
							verified_allowed:     in.actions.selected.verified
							patterns_allowed:     in.actions.selected.patterns
						}
					}
				}
			}
		}

		if in.rulesets != _|_ {
			for _, ruleset in in.rulesets {
				for actor in ruleset.bypassActors if actor.user != _|_ {
					data: github_user: ("user-\(actor.user)"): username: actor.user
				}
				for actor in ruleset.bypassActors if actor.app != _|_ {
					data: github_app: ("app-\(actor.app)"): slug: actor.app
				}
			}

			resource: github_repository_ruleset: {
				for name, ruleset in in.rulesets {
					(name): {
						if ruleset.#import != _|_ {
							#import: ruleset.#import
						}

						"name":       *ruleset.name | name
						"repository": "${\(ref).name}"
						enforcement:  ruleset.enforcement

						if ruleset.branches != _|_ {
							target: "branch"
							conditions: ref_name: {
								include: ruleset.branches.include
								exclude: ruleset.branches.exclude
							}
						}
						if ruleset.tags != _|_ {
							target: "tag"
							conditions: ref_name: {
								include: ruleset.tags.include
								exclude: ruleset.tags.exclude
							}
						}

						bypass_actors: [for actor in ruleset.bypassActors {
							if actor.user != _|_ {
								actor_type:  "User"
								actor_id:    "${data.github_user.user-\(actor.user).id}"
								bypass_mode: actor.bypassMode
							}
							if actor.app != _|_ {
								actor_type:  "Integration"
								actor_id:    "${data.github_app.app-\(actor.app).id}"
								bypass_mode: actor.bypassMode
							}
						}]

						rules: {
							update:                  ruleset.rules.preventUpdates
							deletion:                ruleset.rules.preventDeletion
							non_fast_forward:        ruleset.rules.preventForcePushes
							required_linear_history: ruleset.rules.requireLinearHistory
							required_signatures:     ruleset.rules.requireSignatures

							if ruleset.rules.statusChecks != _|_ {
								required_status_checks: {
									strict_required_status_checks_policy: ruleset.rules.statusChecks.strict
									do_not_enforce_on_create:             ruleset.rules.statusChecks.allowCreationBeforeChecks
									required_check: [for check in ruleset.rules.statusChecks.required {
										context: check.context
										if check.integrationId != _|_ {
											integration_id: check.integrationId
										}
									}]
								}
							}
						}
					}
				}
			}
		}

		resource: github_repository: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name:       in.name
			visibility: in.visibility

			if in.description != _|_ {
				description: in.description
			}
			if in.homepageUrl != _|_ {
				homepage_url: in.homepageUrl
			}

			has_issues:      in.features.issues
			has_discussions: in.features.discussions
			has_projects:    in.features.projects
			has_wiki:        in.features.wiki

			allow_merge_commit:     in.merge.commit
			allow_squash_merge:     in.merge.squash
			allow_rebase_merge:     in.merge.rebase
			allow_auto_merge:       in.merge.auto
			allow_update_branch:    in.merge.updateBranch
			delete_branch_on_merge: in.merge.deleteBranch

			topics: in.topics

			if in.allowForking != _|_ {
				allow_forking: in.allowForking
			}
			is_template: in.isTemplate
			auto_init:   in.autoInit

			archived:           in.archived
			archive_on_destroy: in.archiveOnDestroy

			web_commit_signoff_required: in.webCommitSignoffRequired
		}
	}
}
