// JSON schema for Buildkite pipeline configuration files
package buildkite

import (
	"strings"
	"list"
	"struct"
	"regexp"
)

#PipelineDefinition: {
	@jsonschema(schema="http://json-schema.org/draft-07/schema#")
	env?:      #env
	agents?:   #agents
	notify?:   #buildNotify
	image?:    #image
	secrets?:  #secrets
	priority?: #priority

	// Configure git checkout behavior. Pipeline-level values are inherited by each
	// step unless that step overrides the same key. ssh_secret is step-level only
	// and is not valid here
	checkout?: #checkout & (null | bool | number | string | [...] | {
		ssh_secret?: error("disallowed")
		...
	})
	steps!: #pipelineSteps

	#agents: matchN(1, [#agentsObject, #agentsList])

	// Query rules to target specific agents in k=v format
	#agentsList: [...string]

	// Query rules to target specific agents
	#agentsObject: {
		...
	}

	// Whether to proceed with this step and further steps if a step named in the
	// depends_on attribute fails
	#allowDependencyFailure: true | false | "true" | "false"

	// A list of teams that are permitted to unblock this step, whose values are a
	// list of one or more team slugs or IDs
	#allowedTeams: matchN(>=1, [string, [...string]])

	#automaticRetry: close({
		// The exit status number that will cause this job to retry
		exit_status?: matchN(>=1, ["*", int, [...int]])

		// The number of times this job can be retried
		limit?: int & >=0 & <=10

		// The exit signal, if any, that may be retried
		signal?: string

		// The exit signal reason, if any, that may be retried
		signal_reason?: "*" | "none" | "agent_incompatible" | "agent_refused" | "agent_stop" | "cancel" | "process_run_error" | "signature_rejected" | "stack_error"
	})

	#automaticRetryList: [...#automaticRetry]

	#blockStep: close({
		allow_dependency_failure?: #allowDependencyFailure

		// The label of the block step
		block?: _#defs."/definitions/blockStep/properties/block"

		// The state that the build is set to when the build is blocked by this block step
		blocked_state?: "passed" | "failed" | "running"
		branches?:      #branches
		depends_on?:    #dependsOn
		fields?:        #fields
		if?:            #if
		key?:           _#defs."/definitions/blockStep/properties/key"
		identifier?:    _#defs."/definitions/blockStep/properties/key"
		id?:            _#defs."/definitions/blockStep/properties/key"
		label?:         _#defs."/definitions/blockStep/properties/block"
		name?:          _#defs."/definitions/blockStep/properties/block"
		prompt?:        #prompt
		allowed_teams?: #allowedTeams
		type?:          "block"
	})

	// Which branches will include this step in their builds
	#branches: matchN(>=1, [string, [...string]])

	// Array of notification options for this step
	#buildNotify: [...matchN(1, [#notifySimple, #notifyEmail, #notifyBasecamp, #notifySlack, #notifyWebhook, #notifyPagerduty, #notifyGithubCommitStatus, #notifyGithubCheck])]

	// The paths for the caches to be used in the step
	#cache: matchN(>=1, [string, [...string], {
		paths!: [...string]
		size?: =~"^\\d+g$"
		name?: string
		...
	}])

	// Whether to cancel the job as soon as the build is marked as failing
	#cancelOnBuildFailing: true | false | "true" | "false"

	// Configure git checkout behavior. Pipeline-level values are inherited by each
	// step unless that step overrides the same key
	#checkout: close({
		// Number of commits to fetch when performing a shallow clone; omit for full history
		depth?: int & >=1 | =~"^[1-9][0-9]*$"

		// Skip the git checkout phase. An explicit null is treated as unset
		skip?: true | false | "true" | "false" | null

		// Initialize, sync, update, and clean submodules recursively as part of
		// checkout. An explicit null is treated as unset
		submodules?: true | false | "true" | "false" | null

		// Verify the checked-out commit is on the expected branch; strict fails the job
		// on a definitive mismatch, warn only logs
		commit_verification?: "strict" | "warn"

		// Custom flags passed to git commands during checkout. Flag strings are passed
		// to git verbatim and are not sanitized; do not interpolate untrusted input.
		// See the agent documentation for precedence and defaults:
		// https://buildkite.com/docs/agent/v3/configuration
		flags?: close({
			// Flags for the git clone command
			clone?: string

			// Flags for the git fetch command
			fetch?: string

			// Flags for the git checkout command
			checkout?: string

			// Flags for the git clean command
			clean?: string
		})

		// Name of an SSH private key secret from Buildkite Secrets to use for git
		// clone/fetch operations. The name must start with a letter, contain only
		// letters, numbers, and underscores, and must not start with `buildkite` or
		// `bk` (case-insensitive).
		ssh_secret?: matchN(0, [null | bool | number | =~"^([Bb][Uu][Ii][Ll][Dd][Kk][Ii][Tt][Ee]|[Bb][Kk])" | [...] | {
			...
		}]) & (strings.MinRunes(1) & strings.MaxRunes(255) & =~"^[A-Za-z][A-Za-z0-9_]*$")

		// Whether to install Git LFS and fetch LFS objects after checkout. An explicit
		// null is treated as unset
		lfs?: true | false | "true" | "false" | null

		// Check out only the specified paths in git cone mode; requires git 2.27+ on the agent
		sparse?: close({
			// Repository-relative directory paths within the worktree, as one path or a
			// list of paths; cone mode includes each directory recursively, not file globs
			paths!: matchN(>=1, [#checkoutSparsePath, list.UniqueItems() & [...#checkoutSparsePath] & [_, ...]])
		})
	})

	#checkoutSparsePath: matchN(0, [null | bool | number | =~"^[-\\t\\n\\v\\f\\r \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]|[\\t\\n\\v\\f\\r \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]$" | [...] | {
		...
	}]) & =~"^[^,\\t\\n\\v\\f\\r]+$"

	#commandStep: close({
		agents?:                   #agents
		allow_dependency_failure?: #allowDependencyFailure

		// The glob path/s of artifacts to upload once this step has finished running
		artifact_paths?: matchN(>=1, [string, [...string]])
		branches?:                #branches
		cache?:                   #cache
		cancel_on_build_failing?: #cancelOnBuildFailing
		checkout?:                #checkout
		command?:                 #commandStepCommand
		commands?:                #commandStepCommand

		// The maximum number of jobs created from this step that are allowed to run at
		// the same time. If you use this attribute, you must also define
		// concurrency_group.
		concurrency?: int

		// A unique name for the concurrency group that you are creating with the concurrency attribute
		concurrency_group?: string

		// Control command order, allowed values are 'ordered' (default) and 'eager'. If
		// you use this attribute, you must also define concurrency_group and
		// concurrency.
		concurrency_method?: "ordered" | "eager"
		depends_on?:         #dependsOn
		env?:                #env
		if?:                 #if
		if_changed?:         #ifChanged
		key?:                _#defs."/definitions/commandStep/properties/key"
		identifier?:         _#defs."/definitions/commandStep/properties/key"
		id?:                 _#defs."/definitions/commandStep/properties/key"
		image?:              #image
		label?:              _#defs."/definitions/commandStep/properties/label"

		// The signature of the command step, generally injected by agents at pipeline upload
		signature?: {
			// The algorithm used to generate the signature
			algorithm?: string

			// The signature value, a JWS compact signature with a detached body
			value?: string

			// The fields that were signed to form the signature value
			signed_fields?: [...string]
			...
		}
		matrix?: #matrix
		name?:   _#defs."/definitions/commandStep/properties/label"
		notify?: #commandStepNotify

		// The number of parallel jobs that will be created based on this step
		parallelism?: int
		plugins?:     #plugins
		soft_fail?:   #softFail

		// The conditions for retrying this step.
		retry?: close({
			automatic?: #commandStepAutomaticRetry
			manual?:    #commandStepManualRetry
		})
		skip?: #skip

		// The number of minutes to time out a job
		timeout_in_minutes?: int & >=1
		type?:               "script" | "command" | "commands"
		priority?:           #priority
		secrets?:            #secrets
	})

	// Whether to allow a job to retry automatically. If set to true, the retry
	// conditions are set to the default value.
	#commandStepAutomaticRetry: matchN(>=1, [true | false | "true" | "false", #automaticRetry, #automaticRetryList])

	// The commands to run on the agent
	#commandStepCommand: matchN(>=1, [[...string], string])

	// Whether to allow a job to be retried manually
	#commandStepManualRetry: matchN(>=1, [true | false | "true" | "false", #commandStepManualRetryObject])

	#commandStepManualRetryObject: close({
		// Whether or not this job can be retried manually
		allowed?: true | false | "true" | "false"

		// Whether or not this job can be retried after it has passed
		permit_on_passed?: true | false | "true" | "false"

		// A string that will be displayed in a tooltip on the Retry button in
		// Buildkite. This will only be displayed if the allowed attribute is set to
		// false.
		reason?: string
	})

	// Array of notification options for this step
	#commandStepNotify: [...matchN(1, [#notifySimple, #notifyBasecamp, #notifySlack, #notifyGithubCommitStatus, #notifyGithubCheck])]

	// The step keys for a step to depend on
	#dependsOn: matchN(>=1, [null, string, #dependsOnList])

	#dependsOnList: [...matchN(>=1, [string, close({
		step?:          string
		allow_failure?: true | false | "true" | "false"
	})])]

	// Environment variables for this step
	#env: {
		...
	}

	// A list of input fields required to be filled out before unblocking the step
	#fields: [...matchN(1, [#textField, #selectField])]

	#groupStep: close({
		depends_on?: #dependsOn

		// The name to give to this group of steps
		group!:                    _#defs."/definitions/groupStep/properties/group"
		if?:                       #if
		if_changed?:               #ifChanged
		key?:                      _#defs."/definitions/groupStep/properties/key"
		identifier?:               _#defs."/definitions/groupStep/properties/key"
		id?:                       _#defs."/definitions/groupStep/properties/key"
		label?:                    _#defs."/definitions/groupStep/properties/group"
		name?:                     _#defs."/definitions/groupStep/properties/group"
		allow_dependency_failure?: #allowDependencyFailure
		notify?:                   #buildNotify
		skip?:                     #skip
		steps!:                    #groupSteps
	})

	// A list of steps
	#groupSteps: [...matchN(>=1, [#blockStep, #nestedBlockStep, #stringBlockStep, #inputStep, #nestedInputStep, #stringInputStep, #commandStep, #nestedCommandStep, #waitStep, #nestedWaitStep, #stringWaitStep, #triggerStep, #nestedTriggerStep])] & [_, ...]

	// A boolean expression that omits the step when false
	#if: string

	// Agent-applied attribute: A glob pattern that omits the step from a build if
	// it does not match any files changed in the build. Can be a single pattern,
	// list of patterns, or an object with include/exclude attributes.
	#ifChanged: matchN(1, [string, [...string], close({
		// Pattern or list of patterns to include
		include!: matchN(1, [string, [...string]])

		// Pattern or list of patterns to exclude
		exclude?: matchN(1, [string, [...string]])
	})])

	// (Kubernetes stack only) The container image to use for this pipeline or step
	#image: string

	#inputStep: close({
		allow_dependency_failure?: #allowDependencyFailure

		// The label of the input step
		input?:      _#defs."/definitions/inputStep/properties/input"
		branches?:   #branches
		depends_on?: #dependsOn
		fields?:     #fields

		// The state that the build is set to when the build is blocked by this input step
		blocked_state?: "passed" | "failed" | "running"
		if?:            #if
		key?:           _#defs."/definitions/inputStep/properties/key"
		identifier?:    _#defs."/definitions/inputStep/properties/key"
		id?:            _#defs."/definitions/inputStep/properties/key"
		label?:         _#defs."/definitions/inputStep/properties/input"
		name?:          _#defs."/definitions/inputStep/properties/input"
		prompt?:        #prompt
		allowed_teams?: #allowedTeams
		type?:          "input"
	})

	// A unique identifier for a step, must not resemble a UUID
	#key: matchN(0, [null | bool | number | =~"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$" | [...] | {
		...
	}]) & (strings.MaxRunes(100) & =~"^[a-zA-Z0-9_\\-:${}.,]+$")

	// The label that will be displayed in the pipeline visualisation in Buildkite. Supports emoji.
	#label: string

	#matrix: matchN(1, [#matrixElementList, #matrixObject])

	// An adjustment to a Build Matrix
	#matrixAdjustments: {
		with!: matchN(1, [#matrixElementList, #matrixAdjustmentsWithObject])
		skip?:      #skip
		soft_fail?: #softFail
		...
	}

	// Specification of a new or existing Build Matrix combination
	#matrixAdjustmentsWithObject: {
		...
	} & {
		[string]: string
	}

	#matrixElement: matchN(1, [string, int, bool])

	// List of elements for single-dimension Build Matrix
	#matrixElementList: [...#matrixElement]

	// Configuration for multi-dimension Build Matrix
	#matrixObject: {
		setup!: #matrixSetup

		// List of Build Matrix adjustments
		adjustments?: [...#matrixAdjustments]
		...
	}

	#matrixSetup: matchN(1, [#matrixElementList, {
		[=~"^[a-zA-Z0-9_]+$"]: _
	} & {
		[string]: [...#matrixElement]
	}])

	#nestedBlockStep: close({
		block?: #blockStep
	})

	#nestedCommandStep: close({
		command?:  #commandStep
		commands?: #commandStep
		script?:   #commandStep
	})

	#nestedInputStep: close({
		input?: #inputStep
	})

	#nestedTriggerStep: close({
		trigger?: #triggerStep
	})

	#nestedWaitStep: close({
		wait?:   #waitStep
		waiter?: #waitStep
	})

	#notifyBasecamp: close({
		basecamp_campfire?: string
		if?:                #if
	})

	#notifyEmail: close({
		email?: string
		if?:    #if
	})

	#notifyGithubCheck: close({
		github_check?: close({
			// The name of the GitHub check
			name?: string
			output?: close({
				// The title of the GitHub check's output
				title?: string

				// The summary of the GitHub check's output
				summary?: string

				// The details of the GitHub check's output. Supports Markdown
				text?: string
				annotations?: [...close({
					// The path of the file to add an annotation to, relative to the repository root
					path!: string

					// The start line of the annotation
					start_line!: int

					// The end line of the annotation
					end_line!: int

					// The start column of the annotation. Only valid when start_line and end_line are equal
					start_column?: int

					// The end column of the annotation. Only valid when start_line and end_line are equal
					end_column?: int

					// The level of the annotation
					annotation_level!: "notice" | "warning" | "failure"

					// The message for the annotation
					message!: string

					// The title for the annotation
					title?: string

					// Additional details for the annotation, displayed alongside the message
					raw_details?: string
				})]
			})
		})
		if?: #if
	})

	#notifyGithubCommitStatus: close({
		github_commit_status?: close({
			// GitHub commit status name
			context?: string
		})
		if?: #if
	})

	#notifyPagerduty: close({
		pagerduty_change_event?: string
		if?:                     #if
	})

	#notifySimple: "github_check" | "github_commit_status"

	#notifySlack: close({
		slack?: matchN(1, [=~"^[^ \\t\\n\\v\\f\\r]+$", #notifySlackObject])
		if?: #if
	})

	#notifySlackObject: {
		channels!: [_, ...] & [...=~"^[^ \\t\\n\\v\\f\\r]+$"]
		message?: string
		...
	}

	#notifyWebhook: close({
		webhook?: string
		if?:      #if
	})

	// A list of steps
	#pipelineSteps: [...matchN(>=1, [#blockStep, #nestedBlockStep, #stringBlockStep, #inputStep, #nestedInputStep, #stringInputStep, #commandStep, #nestedCommandStep, #waitStep, #nestedWaitStep, #stringWaitStep, #triggerStep, #nestedTriggerStep, #groupStep])]

	#plugins: matchN(>=1, [#pluginsList, #pluginsObject])

	// Array of plugins for this step
	#pluginsList: [...matchN(1, [string, struct.MaxFields(1)])]

	// A map of plugins for this step. Deprecated: please use the array syntax.
	#pluginsObject: {
		...
	}

	// Priority of all jobs in the pipeline, higher priorities are assigned to
	// agents. When set pipeline-wide, it applies to all steps that do not have
	// their own priority key set.
	#priority: int

	// The instructional message displayed in the dialog box when the unblock step is activated
	#prompt: string

	// A list of secret names or a mapping of environment variable names to secret
	// names to be made available to the build or step
	#secrets: matchN(>=1, [[...string], {
		[string]: string
	}])

	#selectField: close({
		// The text input name
		select?: string

		// The meta-data key that stores the field's input
		key!: =~"^[a-zA-Z0-9-_]+$"

		// The value of the option(s) that will be pre-selected in the dropdown
		default?: matchN(1, [string, [...string]])

		// The explanatory text that is shown after the label
		hint?: string

		// Whether more than one option may be selected
		multiple?: true | false | "true" | "false"
		options!: [_, ...] & [...#selectFieldOption]

		// Whether the field is required for form submission
		required?: true | false | "true" | "false"
	})

	#selectFieldOption: close({
		// The text displayed on the select list item
		label!: string

		// The value to be stored as meta-data
		value!: string

		// The text displayed directly under the select field’s label
		hint?: string

		// Whether the field is required for form submission
		required?: true | false | "true" | "false"
	})

	// Whether this step should be skipped. Passing a string provides a reason for skipping this command
	#skip: matchN(>=1, [bool, strings.MaxRunes(70)])

	// The conditions for marking the step as a soft-fail.
	#softFail: matchN(>=1, [true | false | "true" | "false", #softFailList])

	#softFailList: [...#softFailObject]

	#softFailObject: {
		// The exit status number that will cause this job to soft-fail
		exit_status?: matchN(>=1, ["*", int])
		...
	}

	// Pauses the execution of a build and waits on a user to unblock it
	#stringBlockStep: "block"

	// Pauses the execution of a build and waits on a user to unblock it
	#stringInputStep: "input"

	// Waits for previous steps to pass before continuing
	#stringWaitStep: "wait" | "waiter"

	#textField: close({
		// The text input name
		text?: string

		// The meta-data key that stores the field's input
		key!: =~"^[a-zA-Z0-9-_]+$"

		// The explanatory text that is shown after the label
		hint?: string

		// The format must be a regular expression implicitly anchored to the beginning
		// and end of the input and is functionally equivalent to the HTML5 pattern
		// attribute.
		format?: regexp.Valid

		// Whether the field is required for form submission
		required?: true | false | "true" | "false"

		// The value that is pre-filled in the text field
		default?: string
	})

	#triggerStep: close({
		allow_dependency_failure?: #allowDependencyFailure

		// Whether to continue the build without waiting for the triggered step to complete
		async?:    true | false | "true" | "false"
		branches?: #branches

		// Properties of the build that will be created when the step is triggered
		build?: close({
			// The branch for the build
			branch?: string

			// The commit hash for the build
			commit?: string
			env?:    #env

			// The message for the build (supports emoji)
			message?: string

			// Meta-data for the build
			meta_data?: {
				...
			}
		})
		depends_on?: #dependsOn
		if?:         #if
		if_changed?: #ifChanged
		key?:        _#defs."/definitions/triggerStep/properties/key"
		identifier?: _#defs."/definitions/triggerStep/properties/key"
		id?:         _#defs."/definitions/triggerStep/properties/key"
		label?:      _#defs."/definitions/triggerStep/properties/label"
		name?:       _#defs."/definitions/triggerStep/properties/label"
		type?:       "trigger"

		// The slug of the pipeline to create a build
		trigger!:   string
		skip?:      #skip
		soft_fail?: #softFail
	})

	#waitStep: close({
		allow_dependency_failure?: #allowDependencyFailure
		branches?:                 #branches

		// Continue to the next steps, even if the previous group of steps fail
		continue_on_failure?: true | false | "true" | "false"
		depends_on?:          #dependsOn
		if?:                  #if
		key?:                 _#defs."/definitions/waitStep/properties/key"
		label?:               _#defs."/definitions/waitStep/properties/wait"
		name?:                _#defs."/definitions/waitStep/properties/wait"
		identifier?:          _#defs."/definitions/waitStep/properties/key"
		id?:                  _#defs."/definitions/waitStep/properties/key"
		type?:                "wait" | "waiter"

		// Waits for previous steps to pass before continuing
		wait?: _#defs."/definitions/waitStep/properties/wait"
	})

	// The label of the block step
	_#defs: "/definitions/blockStep/properties/block": string

	_#defs: "/definitions/blockStep/properties/key": #key

	_#defs: "/definitions/commandStep/properties/key": #key

	_#defs: "/definitions/commandStep/properties/label": #label

	// The name to give to this group of steps
	_#defs: "/definitions/groupStep/properties/group": null | string

	_#defs: "/definitions/groupStep/properties/key": #key

	// The label of the input step
	_#defs: "/definitions/inputStep/properties/input": string

	_#defs: "/definitions/inputStep/properties/key": #key

	_#defs: "/definitions/triggerStep/properties/key": #key

	_#defs: "/definitions/triggerStep/properties/label": #label

	_#defs: "/definitions/waitStep/properties/key": #key

	// Waits for previous steps to pass before continuing
	_#defs: "/definitions/waitStep/properties/wait": null | string
	...
}
