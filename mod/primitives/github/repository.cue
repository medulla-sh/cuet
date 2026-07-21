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
