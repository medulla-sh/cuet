package buildkite

import (
	"strings"

	T "github.com/medulla-sh/cuet"
)

#RetryAgentAffinity: "prefer-warmest" | "prefer-different"
#TerraformName:      =~"^[a-z0-9][a-z0-9_-]*$"

#Cluster: {
	in: {
		#import?: {
			cluster?: string
			queues?: [string]: string
			defaultQueue?: string
		}

		name: #TerraformName

		displayName: string & !=""
		displayName: _ | *name

		description?: string
		emoji?:       string
		color?:       =~"^#[0-9A-Fa-f]{6}$"

		queues: [#TerraformName]: {
			key?: string & !=""

			description?: string

			dispatchPaused: bool
			dispatchPaused: _ | *false

			retryAgentAffinity: #RetryAgentAffinity
			retryAgentAffinity: _ | *"prefer-warmest"
		}
		queues: _ | *{}

		defaultQueue?: or([for name, _ in queues {name}])
	}

	refs: {
		cluster: "buildkite_cluster.\(in.name)"
		queues: {
			for name, _ in in.queues {
				(name): "buildkite_cluster_queue.\(in.name)-\(name)"
			}
		}
		if in.defaultQueue != _|_ {
			defaultQueue: "buildkite_cluster_default_queue.\(in.name)"
		}
	}

	out: T.#TerraformInput & {
		resource: buildkite_cluster: (in.name): {
			if in.#import.cluster != _|_ {
				#import: in.#import.cluster
			}

			name: in.displayName

			if in.description != _|_ {
				description: in.description
			}
			if in.emoji != _|_ {
				emoji: in.emoji
			}
			if in.color != _|_ {
				color: in.color
			}
		}

		for name, queue in in.queues {
			let resourceName = "\(in.name)-\(name)"
			let queueKey = [
				if queue.key != _|_ {queue.key},
				name,
			][0]
			resource: buildkite_cluster_queue: (resourceName): {
				if in.#import.queues[name] != _|_ {
					#import: in.#import.queues[name]
				}

				cluster_id:           "${\(refs.cluster).id}"
				key:                  queueKey
				dispatch_paused:      queue.dispatchPaused
				retry_agent_affinity: queue.retryAgentAffinity

				if queue.description != _|_ {
					description: queue.description
				}
			}
		}

		if in.defaultQueue != _|_ {
			resource: buildkite_cluster_default_queue: (in.name): {
				if in.#import.defaultQueue != _|_ {
					#import: in.#import.defaultQueue
				}

				cluster_id: "${\(refs.cluster).id}"
				queue_id:   "${\(refs.queues[in.defaultQueue]).id}"
			}
		}
	}
}

#ClusterAgentToken: {
	in: {
		name:        #TerraformName
		clusterId:   string & !=""
		description: string & !=""

		allowedIpAddresses?: [...string]
	}

	ref: "buildkite_cluster_agent_token.\(in.name)"

	out: T.#TerraformInput & {
		resource: buildkite_cluster_agent_token: (in.name): {
			cluster_id:  in.clusterId
			description: in.description

			if in.allowedIpAddresses != _|_ {
				allowed_ip_addresses: in.allowedIpAddresses
			}
		}
	}
}

#ClusterSecret: {
	in: {
		#import?: string

		name:      #TerraformName
		clusterId: string & !=""
		key:       =~"^[A-Za-z][A-Za-z0-9_]*$"
		value:     string & !=""

		description?: string
		policy?:      string
	}

	if len(in.key) > 255 {
		_|_("Buildkite cluster secret keys must not exceed 255 characters")
	}
	if strings.HasPrefix(strings.ToLower(in.key), "bk") {
		_|_("Buildkite cluster secret keys must not use a reserved prefix")
	}
	if len(in.value) > 8192 {
		_|_("Buildkite cluster secret values must be smaller than 8 KiB")
	}

	ref: "buildkite_cluster_secret.\(in.name)"

	out: T.#TerraformInput & {
		resource: buildkite_cluster_secret: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			cluster_id: in.clusterId
			key:        in.key
			// Stateful values let Terraform detect and propagate rotations.
			value: in.value

			if in.description != _|_ {
				description: in.description
			}
			if in.policy != _|_ {
				policy: in.policy
			}
		}
	}
}
