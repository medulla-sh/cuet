@if(test)

package github

#RepositoryTests: {
	"omits-allowed-actions-when-disabled": {
		input: #Repository & {in: {
			name:          "oakmont"
			visibility:    "private"
			defaultBranch: "main"
			features: {}
			merge: {}
			actions: {
				enabled: false
				allowed: "selected"
				selected: patterns: ["actions/checkout@*"]
			}
		}}

		let permissions = input.out.resource.github_actions_repository_permissions.oakmont

		assert: permissions.enabled == false
		assert: permissions.allowed_actions == _|_
		assert: permissions.allowed_actions_config == _|_
	}

	"renders-selected-actions-when-enabled": {
		input: #Repository & {in: {
			name:          "oakmont"
			visibility:    "private"
			defaultBranch: "main"
			features: {}
			merge: {}
			actions: {
				enabled: true
				allowed: "selected"
				selected: {
					githubOwned: false
					verified:    true
					patterns: ["actions/checkout@*"]
				}
			}
		}}

		let permissions = input.out.resource.github_actions_repository_permissions.oakmont

		assert: permissions.enabled == true
		assert: permissions.allowed_actions == "selected"
		assert: permissions.allowed_actions_config == {
			github_owned_allowed: false
			verified_allowed:     true
			patterns_allowed: ["actions/checkout@*"]
		}
	}

	"renders-ruleset-with-user-bypass": {
		input: #Repository & {in: {
			name:          "oakmont"
			visibility:    "private"
			defaultBranch: "main"
			features: {}
			merge: {}
			rulesets: "protect-main": {
				#import: "oakmont:1234"
				branches: {
					include: ["~DEFAULT_BRANCH"]
					exclude: []
				}
				bypassActors: [{user: "mezuzza"}]
				rules: preventUpdates: true
			}
		}}

		let ruleset = input.out.resource.github_repository_ruleset["protect-main"]

		assert: ruleset.#import == "oakmont:1234"
		assert: ruleset.name == "protect-main"
		assert: ruleset.repository == "${github_repository.oakmont.name}"
		assert: ruleset.target == "branch"
		assert: ruleset.enforcement == "active"
		assert: input.out.data.github_user["user-mezuzza"] == {
			username: "mezuzza"
		}
		assert: input.out.data.github_app == _|_
		assert: ruleset.bypass_actors == [{
			actor_type:  "User"
			actor_id:    "${data.github_user.user-mezuzza.id}"
			bypass_mode: "always"
		}]
		assert: ruleset.conditions.ref_name == {
			include: ["~DEFAULT_BRANCH"]
			exclude: []
		}
		assert: ruleset.rules.update == true
	}

	"resolves-github-app": {
		input: #Repository & {in: {
			name:          "oakmont"
			visibility:    "private"
			defaultBranch: "main"
			features: {}
			merge: {}
			rulesets: "protect-releases": {
				name: "Protect releases"
				tags: {
					include: ["refs/tags/v*"]
					exclude: []
				}
				bypassActors: [{app: "oakmont-mergequeue"}]
				rules: {}
			}
		}}

		let ruleset = input.out.resource.github_repository_ruleset["protect-releases"]

		assert: input.out.data.github_app["app-oakmont-mergequeue"] == {
			slug: "oakmont-mergequeue"
		}
		assert: input.out.data.github_user == _|_
		assert: ruleset.name == "Protect releases"
		assert: ruleset.target == "tag"
		assert: ruleset.conditions.ref_name == {
			include: ["refs/tags/v*"]
			exclude: []
		}
		assert: ruleset.bypass_actors == [{
			actor_type:  "Integration"
			actor_id:    "${data.github_app.app-oakmont-mergequeue.id}"
			bypass_mode: "always"
		}]
	}
}

repositoryResult: [for _, test in #RepositoryTests {test.assert & true}]
