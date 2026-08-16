package buildkite

import T "github.com/medulla-sh/cuet"

#PipelineVisibility: "PRIVATE" | "PUBLIC"

#PipelineProviderSettings: {
	triggerMode?: "code" | "deployment" | "fork" | "none"

	buildBranches?:                  bool
	buildTags?:                      bool
	buildPullRequests?:              bool
	buildPullRequestForks?:          bool
	buildPullRequestMergeCommits?:   bool
	buildPullRequestReadyForReview?: bool

	pullRequestBranchFilterEnabled?:       bool
	pullRequestBranchFilterConfiguration?: string

	skipBuildsForExistingCommits?:            bool
	skipPullRequestBuildsForExistingCommits?: bool

	publishCommitStatus?:         bool
	publishCommitStatusPerStep?:  bool
	publishBlockedAsPending?:     bool
	separatePullRequestStatuses?: bool

	filterEnabled?:   bool
	filterCondition?: string
}

#Pipeline: {
	in: {
		#import?: string

		name: #TerraformName

		displayName: string & !=""
		displayName: _ | *name

		repository: string & !=""
		clusterId:  string & !=""

		defaultTeamId?: string & !=""

		description?: string
		emoji?:       string
		color?:       =~"^#[0-9A-Fa-f]{6}$"
		slug?:        =~"^[a-z0-9-]{1,100}$"

		branchConfiguration?: string
		defaultBranch?:       string & !=""

		allowRebuilds: bool
		allowRebuilds: _ | *true

		archived: bool
		archived: _ | *false

		cancelIntermediateBuilds:              bool
		cancelIntermediateBuilds:              _ | *false
		cancelIntermediateBuildsBranchFilter?: string

		skipIntermediateBuilds:              bool
		skipIntermediateBuilds:              _ | *false
		skipIntermediateBuildsBranchFilter?: string

		defaultTimeoutMinutes?: int & >=0
		maximumTimeoutMinutes?: int & >=0

		steps?: string

		tags: [...string]
		tags: _ | *[]

		visibility: #PipelineVisibility
		visibility: _ | *"PRIVATE"

		providerSettings?: #PipelineProviderSettings
	}

	ref: "buildkite_pipeline.\(in.name)"

	out: T.#TerraformInput & {
		resource: buildkite_pipeline: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name:       in.displayName
			repository: in.repository
			cluster_id: in.clusterId

			allow_rebuilds: in.allowRebuilds
			archived:       in.archived

			cancel_intermediate_builds: in.cancelIntermediateBuilds
			skip_intermediate_builds:   in.skipIntermediateBuilds

			tags:       in.tags
			visibility: in.visibility

			if in.defaultTeamId != _|_ {
				default_team_id: in.defaultTeamId
			}
			if in.description != _|_ {
				description: in.description
			}
			if in.emoji != _|_ {
				emoji: in.emoji
			}
			if in.color != _|_ {
				color: in.color
			}
			if in.slug != _|_ {
				slug: in.slug
			}
			if in.branchConfiguration != _|_ {
				branch_configuration: in.branchConfiguration
			}
			if in.defaultBranch != _|_ {
				default_branch: in.defaultBranch
			}
			if in.cancelIntermediateBuildsBranchFilter != _|_ {
				cancel_intermediate_builds_branch_filter: in.cancelIntermediateBuildsBranchFilter
			}
			if in.skipIntermediateBuildsBranchFilter != _|_ {
				skip_intermediate_builds_branch_filter: in.skipIntermediateBuildsBranchFilter
			}
			if in.defaultTimeoutMinutes != _|_ {
				default_timeout_in_minutes: in.defaultTimeoutMinutes
			}
			if in.maximumTimeoutMinutes != _|_ {
				maximum_timeout_in_minutes: in.maximumTimeoutMinutes
			}
			if in.steps != _|_ {
				steps: in.steps
			}

			if in.providerSettings != _|_ {
				provider_settings: {
					if in.providerSettings.triggerMode != _|_ {
						trigger_mode: in.providerSettings.triggerMode
					}
					if in.providerSettings.buildBranches != _|_ {
						build_branches: in.providerSettings.buildBranches
					}
					if in.providerSettings.buildTags != _|_ {
						build_tags: in.providerSettings.buildTags
					}
					if in.providerSettings.buildPullRequests != _|_ {
						build_pull_requests: in.providerSettings.buildPullRequests
					}
					if in.providerSettings.buildPullRequestForks != _|_ {
						build_pull_request_forks: in.providerSettings.buildPullRequestForks
					}
					if in.providerSettings.buildPullRequestMergeCommits != _|_ {
						build_pull_request_merge_commits: in.providerSettings.buildPullRequestMergeCommits
					}
					if in.providerSettings.buildPullRequestReadyForReview != _|_ {
						build_pull_request_ready_for_review: in.providerSettings.buildPullRequestReadyForReview
					}
					if in.providerSettings.pullRequestBranchFilterEnabled != _|_ {
						pull_request_branch_filter_enabled: in.providerSettings.pullRequestBranchFilterEnabled
					}
					if in.providerSettings.pullRequestBranchFilterConfiguration != _|_ {
						pull_request_branch_filter_configuration: in.providerSettings.pullRequestBranchFilterConfiguration
					}
					if in.providerSettings.skipBuildsForExistingCommits != _|_ {
						skip_builds_for_existing_commits: in.providerSettings.skipBuildsForExistingCommits
					}
					if in.providerSettings.skipPullRequestBuildsForExistingCommits != _|_ {
						skip_pull_request_builds_for_existing_commits: in.providerSettings.skipPullRequestBuildsForExistingCommits
					}
					if in.providerSettings.publishCommitStatus != _|_ {
						publish_commit_status: in.providerSettings.publishCommitStatus
					}
					if in.providerSettings.publishCommitStatusPerStep != _|_ {
						publish_commit_status_per_step: in.providerSettings.publishCommitStatusPerStep
					}
					if in.providerSettings.publishBlockedAsPending != _|_ {
						publish_blocked_as_pending: in.providerSettings.publishBlockedAsPending
					}
					if in.providerSettings.separatePullRequestStatuses != _|_ {
						separate_pull_request_statuses: in.providerSettings.separatePullRequestStatuses
					}
					if in.providerSettings.filterEnabled != _|_ {
						filter_enabled: in.providerSettings.filterEnabled
					}
					if in.providerSettings.filterCondition != _|_ {
						filter_condition: in.providerSettings.filterCondition
					}
				}
			}
		}
	}
}

#PipelineWebhook: {
	in: {
		#import?: string

		name:       #TerraformName
		pipelineId: string & !=""
		repository: string & !=""
	}

	ref: "buildkite_pipeline_webhook.\(in.name)"

	out: T.#TerraformInput & {
		resource: buildkite_pipeline_webhook: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			pipeline_id: in.pipelineId
			repository:  in.repository
		}
	}
}
