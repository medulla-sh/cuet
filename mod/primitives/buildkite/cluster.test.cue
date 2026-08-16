@if(test)

package buildkite

#ClusterTests: {
	"defaults": {
		input: #Cluster & {in: name: "validation"}

		assert: input.refs.cluster == "buildkite_cluster.validation"
		assert: input.refs.queues == {}
		assert: input.out.resource.buildkite_cluster.validation == {
			name: "validation"
		}
	}

	"queues": {
		input: #Cluster & {in: {
			#import: {
				cluster: "Q2x1c3Rlci0tLXZhbGlkYXRpb24="
				queues: validator: "Q2x1c3RlclF1ZXVlLS0tdmFsaWRhdG9y,35498aaf-ad05-4fa5-9a07-91bf6cacd2bd"
				defaultQueue: "Q2x1c3Rlci0tLXZhbGlkYXRpb24="
			}
			name:        "validation"
			displayName: "Oakmont Validation"
			description: "Runs pull request validation"
			emoji:       ":buildkite:"
			color:       "#14CC80"
			queues: {
				validator: {}
				large: {
					key:                "validator-large"
					description:        "Large validation jobs"
					dispatchPaused:     true
					retryAgentAffinity: "prefer-different"
				}
			}
			defaultQueue: "validator"
		}}

		assert: input.refs.cluster == "buildkite_cluster.validation"
		assert: input.refs.queues.validator == "buildkite_cluster_queue.validation-validator"
		assert: input.refs.queues.large == "buildkite_cluster_queue.validation-large"
		assert: input.refs.defaultQueue == "buildkite_cluster_default_queue.validation"
		assert: input.out.resource.buildkite_cluster.validation == {
			#import:     "Q2x1c3Rlci0tLXZhbGlkYXRpb24="
			name:        "Oakmont Validation"
			description: "Runs pull request validation"
			emoji:       ":buildkite:"
			color:       "#14CC80"
		}
		assert: input.out.resource.buildkite_cluster_queue["validation-validator"] == {
			#import:              "Q2x1c3RlclF1ZXVlLS0tdmFsaWRhdG9y,35498aaf-ad05-4fa5-9a07-91bf6cacd2bd"
			cluster_id:           "${buildkite_cluster.validation.id}"
			key:                  "validator"
			dispatch_paused:      false
			retry_agent_affinity: "prefer-warmest"
		}
		assert: input.out.resource.buildkite_cluster_queue["validation-large"] == {
			cluster_id:           "${buildkite_cluster.validation.id}"
			key:                  "validator-large"
			description:          "Large validation jobs"
			dispatch_paused:      true
			retry_agent_affinity: "prefer-different"
		}
		assert: input.out.resource.buildkite_cluster_default_queue.validation == {
			#import:    "Q2x1c3Rlci0tLXZhbGlkYXRpb24="
			cluster_id: "${buildkite_cluster.validation.id}"
			queue_id:   "${buildkite_cluster_queue.validation-validator.id}"
		}
	}
}

clusterResult: [for _, test in #ClusterTests {test.assert & true}]

#ClusterAgentTokenTests: {
	"defaults": {
		input: #ClusterAgentToken & {in: {
			name:        "validation"
			clusterId:   "${buildkite_cluster.validation.id}"
			description: "Internal validation agents"
		}}

		assert: input.ref == "buildkite_cluster_agent_token.validation"
		assert: input.out.resource.buildkite_cluster_agent_token.validation == {
			cluster_id:  "${buildkite_cluster.validation.id}"
			description: "Internal validation agents"
		}
	}

	"allowed IP addresses": {
		input: #ClusterAgentToken & {in: {
			name:        "validation"
			clusterId:   "${buildkite_cluster.validation.id}"
			description: "Internal validation agents"
			allowedIpAddresses: ["203.0.113.0/24"]
		}}

		assert: input.out.resource.buildkite_cluster_agent_token.validation.allowed_ip_addresses == ["203.0.113.0/24"]
	}
}

clusterAgentTokenResult: [for _, test in #ClusterAgentTokenTests {test.assert & true}]

#ClusterSecretTests: {
	"stateful value": {
		input: #ClusterSecret & {in: {
			name:        "builds-api-token"
			clusterId:   "${buildkite_cluster.publishers.uuid}"
			key:         "BUILDS_API_TOKEN"
			value:       "secret-token"
			description: "Reads successful delivery builds"
			policy:      "- pipeline_slug: oakmont-main-delivery\n  build_branch: main\n"
		}}

		assert: input.ref == "buildkite_cluster_secret.builds-api-token"
		assert: input.out.resource.buildkite_cluster_secret["builds-api-token"] == {
			cluster_id:  "${buildkite_cluster.publishers.uuid}"
			key:         "BUILDS_API_TOKEN"
			value:       "secret-token"
			description: "Reads successful delivery builds"
			policy:      "- pipeline_slug: oakmont-main-delivery\n  build_branch: main\n"
		}
	}

	"import": {
		input: #ClusterSecret & {in: {
			#import:   "35498aaf-ad05-4fa5-9a07-91bf6cacd2bd/fedcba98-7654-3210-fedc-ba9876543210"
			name:      "builds-api-token"
			clusterId: "35498aaf-ad05-4fa5-9a07-91bf6cacd2bd"
			key:       "BUILDS_API_TOKEN"
			value:     "secret-token"
		}}

		assert: input.out.resource.buildkite_cluster_secret["builds-api-token"].#import == "35498aaf-ad05-4fa5-9a07-91bf6cacd2bd/fedcba98-7654-3210-fedc-ba9876543210"
	}
}

clusterSecretResult: [for _, test in #ClusterSecretTests {test.assert & true}]
