@if(test)

package buildkite

#PipelineTests: {
	"defaults": {
		input: #Pipeline & {in: {
			name:       "oakmont-ci"
			repository: "git@github.com:oakmont-health/oakmont.git"
			clusterId:  "${buildkite_cluster.validation.id}"
		}}

		assert: input.ref == "buildkite_pipeline.oakmont-ci"
		assert: input.out.resource.buildkite_pipeline["oakmont-ci"] == {
			name:                       "oakmont-ci"
			repository:                 "git@github.com:oakmont-health/oakmont.git"
			cluster_id:                 "${buildkite_cluster.validation.id}"
			allow_rebuilds:             true
			archived:                   false
			cancel_intermediate_builds: false
			skip_intermediate_builds:   false
			tags: []
			visibility: "PRIVATE"
		}
	}

	"pull requests": {
		input: #Pipeline & {in: {
			#import:                              "UGlwZWxpbmUtLS1vYWttb250LWNp"
			name:                                 "oakmont-ci"
			displayName:                          "Oakmont CI"
			repository:                           "git@github.com:oakmont-health/oakmont.git"
			clusterId:                            "${buildkite_cluster.validation.id}"
			defaultTeamId:                        "${data.buildkite_team.owners.id}"
			defaultBranch:                        "main"
			cancelIntermediateBuilds:             true
			cancelIntermediateBuildsBranchFilter: "!main"
			steps:                                "steps:\n  - command: just check\n"
			providerSettings: {
				triggerMode:                             "code"
				buildBranches:                           false
				buildTags:                               false
				buildPullRequests:                       true
				buildPullRequestForks:                   false
				buildPullRequestMergeCommits:            true
				buildPullRequestReadyForReview:          true
				skipPullRequestBuildsForExistingCommits: true
				publishCommitStatus:                     true
				publishCommitStatusPerStep:              false
				separatePullRequestStatuses:             true
			}
		}}

		assert: input.out.resource.buildkite_pipeline["oakmont-ci"].#import == "UGlwZWxpbmUtLS1vYWttb250LWNp"
		assert: input.out.resource.buildkite_pipeline["oakmont-ci"].name == "Oakmont CI"
		assert: input.out.resource.buildkite_pipeline["oakmont-ci"].default_team_id == "${data.buildkite_team.owners.id}"
		assert: input.out.resource.buildkite_pipeline["oakmont-ci"].default_branch == "main"
		assert: input.out.resource.buildkite_pipeline["oakmont-ci"].cancel_intermediate_builds == true
		assert: input.out.resource.buildkite_pipeline["oakmont-ci"].cancel_intermediate_builds_branch_filter == "!main"
		assert: input.out.resource.buildkite_pipeline["oakmont-ci"].provider_settings == {
			trigger_mode:                                  "code"
			build_branches:                                false
			build_tags:                                    false
			build_pull_requests:                           true
			build_pull_request_forks:                      false
			build_pull_request_merge_commits:              true
			build_pull_request_ready_for_review:           true
			skip_pull_request_builds_for_existing_commits: true
			publish_commit_status:                         true
			publish_commit_status_per_step:                false
			separate_pull_request_statuses:                true
		}
	}

	"tag ingress": {
		input: #Pipeline & {in: {
			name:       "oakmont-release-tags"
			repository: "git@github.com:oakmont-health/oakmont.git"
			clusterId:  "${buildkite_cluster.publishers.id}"
			providerSettings: {
				triggerMode:       "code"
				buildBranches:     false
				buildPullRequests: false
				buildTags:         true
			}
		}}

		assert: input.out.resource.buildkite_pipeline["oakmont-release-tags"].provider_settings == {
			trigger_mode:        "code"
			build_branches:      false
			build_pull_requests: false
			build_tags:          true
		}
	}
}

pipelineResult: [for _, test in #PipelineTests {test.assert & true}]

#PipelineWebhookTests: {
	"composition": {
		pipeline: #Pipeline & {in: {
			name:       "oakmont-ci"
			repository: "git@github.com:oakmont-health/oakmont.git"
			clusterId:  "${buildkite_cluster.validation.id}"
		}}
		input: #PipelineWebhook & {in: {
			name:       "oakmont-ci"
			pipelineId: "${\(pipeline.ref).id}"
			repository: "${\(pipeline.ref).repository}"
		}}

		assert: input.ref == "buildkite_pipeline_webhook.oakmont-ci"
		assert: input.out.resource.buildkite_pipeline_webhook["oakmont-ci"] == {
			pipeline_id: "${buildkite_pipeline.oakmont-ci.id}"
			repository:  "${buildkite_pipeline.oakmont-ci.repository}"
		}
	}

	"import": {
		input: #PipelineWebhook & {in: {
			#import:    "UGlwZWxpbmUtLS1vYWttb250LWNp"
			name:       "oakmont-ci"
			pipelineId: "${buildkite_pipeline.oakmont-ci.id}"
			repository: "${buildkite_pipeline.oakmont-ci.repository}"
		}}

		assert: input.out.resource.buildkite_pipeline_webhook["oakmont-ci"].#import == "UGlwZWxpbmUtLS1vYWttb250LWNp"
	}
}

pipelineWebhookResult: [for _, test in #PipelineWebhookTests {test.assert & true}]
