@if(test)

package google

#GkeKubernetesServiceAccountPrincipalTests: {
	"current-environment": {
		input: #GkeKubernetesServiceAccountPrincipal & {"in": {
			namespace:      "flux-system"
			serviceAccount: "flux"
		}}
		input: out: #envName: "dev"

		assert: input.out.data.google_project.dev == {}
		assert: input.val == "principal://iam.googleapis.com/projects/${data.google_project.dev.number}/locations/global/workloadIdentityPools/${data.google_project.dev.project_id}.svc.id.goog/subject/ns/flux-system/sa/flux"
	}

	"id-only": {
		input: #GkeKubernetesServiceAccountPrincipal & {"in": {
			namespace:      "flux-system"
			serviceAccount: "flux"
			project: id: "oakmont-dev"
		}}

		assert: input.out.data.google_project["oakmont-dev"].project_id == "oakmont-dev"
		assert: input.val == "principal://iam.googleapis.com/projects/${data.google_project.oakmont-dev.number}/locations/global/workloadIdentityPools/oakmont-dev.svc.id.goog/subject/ns/flux-system/sa/flux"
	}
}

#GkeServiceAccountIamMemberTests: {
	"same-project": {
		input: #GkeServiceAccountIamMember & {"in": {
			name: "buildkite-trusted-validator"
			googleServiceAccount: {
				accountId: "ci-validator"
				projectId: "oakmont-internal"
			}
			kubernetesServiceAccount: {
				name:      "buildkite-trusted-validator"
				namespace: "buildkite"
				projectId: "oakmont-internal"
			}
		}}

		let resource = input.out.resource.google_service_account_iam_member["buildkite-trusted-validator"]
		assert: input.ref == "google_service_account_iam_member.buildkite-trusted-validator"
		assert: resource.service_account_id == "projects/oakmont-internal/serviceAccounts/ci-validator@oakmont-internal.iam.gserviceaccount.com"
		assert: resource.role == "roles/iam.workloadIdentityUser"
		assert: resource.member == "serviceAccount:oakmont-internal.svc.id.goog[buildkite/buildkite-trusted-validator]"
	}

	"cross-project": {
		input: #GkeServiceAccountIamMember & {"in": {
			name: "artifact-reader"
			googleServiceAccount: {
				accountId: "artifact-reader"
				projectId: "identity-project"
			}
			kubernetesServiceAccount: {
				name:      "reader"
				namespace: "workloads"
				projectId: "cluster-project"
			}
		}}

		let resource = input.out.resource.google_service_account_iam_member["artifact-reader"]
		assert: resource.service_account_id == "projects/identity-project/serviceAccounts/artifact-reader@identity-project.iam.gserviceaccount.com"
		assert: resource.member == "serviceAccount:cluster-project.svc.id.goog[workloads/reader]"
	}

	"import": {
		input: #GkeServiceAccountIamMember & {"in": {
			#import: "projects/identity-project/serviceAccounts/artifact-reader@identity-project.iam.gserviceaccount.com roles/iam.workloadIdentityUser serviceAccount:cluster-project.svc.id.goog[workloads/reader]"
			name:    "artifact-reader"
			googleServiceAccount: {
				accountId: "artifact-reader"
				projectId: "identity-project"
			}
			kubernetesServiceAccount: {
				name:      "reader"
				namespace: "workloads"
				projectId: "cluster-project"
			}
		}}

		assert: input.out.resource.google_service_account_iam_member["artifact-reader"].#import == input.in.#import
	}
}

#GkeClusterTests: {
	"standard-cluster": {
		input: #GkeCluster & {in: {
			#imports: {
				cluster: "projects/example/locations/us-west1/clusters/internal"
				nodePools: "app-connector": "projects/example/locations/us-west1/clusters/internal/nodePools/app-connector"
			}
			name:     "internal"
			location: "us-west1"
			mode:     "standard"
			networking: {
				vpc:    "${google_compute_network.internal.self_link}"
				subnet: "${google_compute_subnetwork.k8s.self_link}"
				ranges: {
					pods:     "pods"
					services: "services"
				}
				datapath: defaultSnat: false
			}
			access: nodeIPs: "private"
			deletionProtection: false
			nodePools: "gcs-init": customNodeInit: initScript: gcs: {
				uri:        "gs://example/init.sh"
				generation: 42
			}
			nodePools: "app-connector": {
				pod: range:                                              "app-connector-pods"
				labels: "node-restriction.kubernetes.io/oakmont-egress": "app-connector"
				customNodeInit: {
					dependencies: ["google_secret_manager_secret_iam_member.app-connector-init"]
					initScript: secretManagerSecretUri: "projects/example/secrets/app-connector-init/versions/3"
				}
				taints: [{
					key:    "oakmont.health/app-connector"
					value:  "true"
					effect: "no-execute"
				}]
				scaling: autoscaling: {
					min: 2
					max: 4
				}
			}
		}}
		input: out: #envName: "internal"

		assert: input.out.resource.google_container_cluster.internal.initial_node_count == 1
		assert: input.out.resource.google_container_cluster.internal.#import == "projects/example/locations/us-west1/clusters/internal"
		assert: input.out.resource.google_container_cluster.internal.remove_default_node_pool == true
		assert: input.out.resource.google_container_cluster.internal.datapath_provider == "ADVANCED_DATAPATH"
		assert: input.out.resource.google_container_cluster.internal.enable_intranode_visibility == true
		assert: input.out.resource.google_container_cluster.internal.default_snat_status.disabled == true
		assert: input.out.resource.google_container_node_pool["app-connector"].network_config == {
			enable_private_nodes: true
			create_pod_range:     false
			pod_range:            "app-connector-pods"
		}
		assert: input.out.resource.google_container_node_pool["app-connector"].#import == "projects/example/locations/us-west1/clusters/internal/nodePools/app-connector"
		assert: input.out.resource.google_container_node_pool["app-connector"].autoscaling == {
			total_min_node_count: 2
			total_max_node_count: 4
			location_policy:      "BALANCED"
		}
		assert: input.out.resource.google_container_node_pool["app-connector"].node_config.taint == [{
			key:    "oakmont.health/app-connector"
			value:  "true"
			effect: "NO_EXECUTE"
		}]
		assert: input.out.resource.google_container_node_pool["app-connector"].node_config.disk_type == "pd-balanced"
		assert: input.out.resource.google_container_node_pool["app-connector"].node_config.disk_size_gb == 100
		assert: input.out.resource.google_container_node_pool["app-connector"].depends_on == ["google_secret_manager_secret_iam_member.app-connector-init"]
		assert: input.out.resource.google_container_node_pool["app-connector"].node_config.linux_node_config.custom_node_init.init_script.gcp_secret_manager_secret_uri == "projects/example/secrets/app-connector-init/versions/3"
		assert: input.out.resource.google_container_node_pool["gcs-init"].node_config.linux_node_config.custom_node_init.init_script == {
			gcs_uri:        "gs://example/init.sh"
			gcs_generation: 42
		}
		assert: input.refs.cluster == "google_container_cluster.internal"
		assert: input.refs.nodePools["app-connector"] == "google_container_node_pool.app-connector"
	}

	"private-control-plane-requires-private-nodes": {
		input: {
			name:     "invalid"
			location: "us-west1"
			networking: {
				vpc:    "network"
				subnet: "subnet"
				ranges: {
					pods:     "pods"
					services: "services"
				}
			}
			access: {
				nodeIPs:              "public"
				controlPlaneEndpoint: "private"
			}
		}

		assert: (#GkeCluster & {in: input}) == _|_
	}
}

gkeResult: [
	for _, test in #GkeKubernetesServiceAccountPrincipalTests {test.assert & true},
	for _, test in #GkeServiceAccountIamMemberTests {test.assert & true},
	for _, test in #GkeClusterTests {test.assert & true},
]
