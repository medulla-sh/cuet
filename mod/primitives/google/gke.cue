package google

import (
	"net"

	T "github.com/medulla-sh/cuet"
)

#GkeReleaseChannelMap: {
	unspecified: "UNSPECIFIED"
	rapid:       "RAPID"
	regular:     "REGULAR"
	stable:      "STABLE"
	extended:    "EXTENDED"
}

#GkeReleaseChannel: or([for channel, _ in #GkeReleaseChannelMap {channel}])

#GkeMeshManagementMap: {
	automatic: "MANAGEMENT_AUTOMATIC"
	manual:    "MANAGEMENT_MANUAL"
}

#GkeMeshManagement: or([for management, _ in #GkeMeshManagementMap {management}])

#GkeHubLocation: "global" | #Region

#GkeDatapathProviderMap: {
	advanced: "ADVANCED_DATAPATH"
	legacy:   "LEGACY_DATAPATH"
}

#GkeDatapathProvider: or([for provider, _ in #GkeDatapathProviderMap {provider}])

#GkeNodePoolTaintEffectMap: {
	"no-schedule":        "NO_SCHEDULE"
	"prefer-no-schedule": "PREFER_NO_SCHEDULE"
	"no-execute":         "NO_EXECUTE"
}

#GkeNodePoolTaintEffect: or([for effect, _ in #GkeNodePoolTaintEffectMap {effect}])

#GkeNodePoolLocationPolicyMap: {
	balanced: "BALANCED"
	any:      "ANY"
}

#GkeNodePoolLocationPolicy: or([for policy, _ in #GkeNodePoolLocationPolicyMap {policy}])

#GkeNodePoolConfig: {
	zones: [...string]

	machineType: string
	machineType: _ | *"e2-standard-2"

	imageType: string
	imageType: _ | *"COS_CONTAINERD"

	disk: {
		type: string
		type: _ | *"pd-balanced"

		size: int & >0
		size: _ | *100
	}

	pod: {
		range?:      #RFC1035Name
		maxPerNode?: int & >0
	}

	scaling: ({
		autoscaling: {
			min: int & >=0
			max: int & >=min

			locationPolicy: #GkeNodePoolLocationPolicy
			locationPolicy: _ | *"balanced"
		}
	} | *{
		fixed: {
			count: int & >0
			count: _ | *1
		}
	})

	identity: {
		serviceAccount?: string

		accessScopes: [...string]
		accessScopes: _ | *["https://www.googleapis.com/auth/cloud-platform"]
	}

	labels: [string]: string
	labels: _ | *{}

	taints: [...{
		key:    string
		value:  string
		effect: #GkeNodePoolTaintEffect
	}]
	taints: _ | *[]

	management: {
		autoRepair: bool
		autoRepair: _ | *true
	}

	upgrade: {
		automatic: bool
		automatic: _ | *true

		maxSurge: int & >=0
		maxSurge: _ | *1

		maxUnavailable: int & >=0
		maxUnavailable: _ | *0
		if maxSurge == 0 && maxUnavailable == 0 {
			_|_("maxSurge and maxUnavailable cannot both be zero")
		}
	}

	security: {
		secureBoot: bool
		secureBoot: _ | *true

		integrityMonitoring: bool
		integrityMonitoring: _ | *true
	}
}

#DefaultComputeServiceAccount: {
	in: {
		project: {
			name: string
			id?:  string
		}
	}

	val: "serviceAccount:${data.google_project.\(in.project.name).number}-compute@developer.gserviceaccount.com"

	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}
	}
}

#GkeKubernetesServiceAccountPrincipal: {
	in: {
		namespace:      string
		serviceAccount: string

		name?: string
		project: {
			name?: string
			id?:   string
		}
		project: _ | *{}
	}

	val: out.#val

	out: this=T.#TerraformInput & {
		let projectDataName = [
			if in.name != _|_ {in.name},
			if in.project.name != _|_ {in.project.name},
			if in.project.id != _|_ {in.project.id},
			this.#envName,
		][0]

		let projectId = [
			if in.project.id != _|_ {in.project.id},
			"${data.google_project.\(projectDataName).project_id}",
		][0]

		#val: "principal://iam.googleapis.com/projects/${data.google_project.\(projectDataName).number}/locations/global/workloadIdentityPools/\(projectId).svc.id.goog/subject/ns/\(in.namespace)/sa/\(in.serviceAccount)"

		data: google_project: (projectDataName): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}
	}
}

#GkeCluster: {
	in: {
		#imports: {
			cluster?: string
			nodePools?: [#RFC1035Name]: string
		}

		name:     #RFC1035Name
		location: #Region

		project: {
			name?: string
			id?:   string
		}
		project: _ | *{}

		mode: "autopilot" | "standard"
		mode: _ | *"autopilot"

		networking: {
			vpc:    string
			subnet: string
			ranges: {
				pods:     #RFC1035Name
				services: #RFC1035Name
			}

			if mode == "standard" {
				datapath: {
					provider: #GkeDatapathProvider
					provider: _ | *"advanced"

					intranodeVisibility: bool
					intranodeVisibility: _ | *true

					defaultSnat: bool
					defaultSnat: _ | *true
				}
			}
		}

		access: {
			nodeIPs: "public" | "private"
			nodeIPs: _ | *"public"

			controlPlaneEndpoint: "public" | "private"
			controlPlaneEndpoint: _ | *"public"

			// GKE requires a non-overlapping /28 when explicitly assigning this range.
			controlPlaneIpv4Cidr?: net.IPCIDR & =~"^[0-9.]+/28$"

			if controlPlaneEndpoint == "private" {
				nodeIPs: "private"
			}
			if controlPlaneIpv4Cidr != _|_ {
				nodeIPs: "private"
			}
		}

		workloadIdentity: {
			pool?: string
		}

		serviceMesh: {
			certificates: bool
			certificates: _ | *false
		}

		observability: {
			managedOpenTelemetry: bool
			managedOpenTelemetry: _ | *false
		}

		secrets: {
			manager: bool
			manager: _ | *false

			sync?: {
				rotationInterval?: =~"^([6-9][0-9]|[1-9][0-9]{2,})s$"
			}
		}

		deletionProtection: bool
		deletionProtection: _ | *true

		releaseChannel: #GkeReleaseChannel
		releaseChannel: _ | *"regular"

		if mode == "standard" {
			nodePools: [#RFC1035Name]: #GkeNodePoolConfig
			if len(nodePools) == 0 {
				_|_("Standard clusters require at least one node pool")
			}
		}
	}

	refs: {
		cluster: "google_container_cluster.\(in.name)"
		if in.mode == "standard" {
			nodePools: {
				for name, _ in in.nodePools {
					(name): "google_container_node_pool.\(name)"
				}
			}
		}
	}

	out: this=T.#TerraformInput & {
		let projectDataName = [
			if in.project.name != _|_ {in.project.name},
			this.#envName,
		][0]

		if in.workloadIdentity.pool == _|_ {
			data: google_project: (projectDataName): {
				if in.project.id != _|_ {
					project_id: in.project.id
				}
			}
		}

		let workloadPool = [
			if in.workloadIdentity.pool != _|_ {in.workloadIdentity.pool},
			if in.workloadIdentity.pool == _|_ {"${data.google_project.\(projectDataName).project_id}.svc.id.goog"},
		][0]

		resource: google_container_cluster: (in.name): {
			if in.#imports.cluster != _|_ {
				#import: in.#imports.cluster
			}

			name:     in.name
			location: in.location

			if in.project.id != _|_ {
				project: in.project.id
			}

			deletion_protection: in.deletionProtection
			if in.mode == "autopilot" {
				enable_autopilot: true
			}
			if in.mode == "standard" {
				initial_node_count:          1
				remove_default_node_pool:    true
				datapath_provider:           #GkeDatapathProviderMap[in.networking.datapath.provider]
				enable_intranode_visibility: in.networking.datapath.intranodeVisibility
				default_snat_status: disabled: !in.networking.datapath.defaultSnat
			}

			network:    in.networking.vpc
			subnetwork: in.networking.subnet

			networking_mode: "VPC_NATIVE"
			ip_allocation_policy: {
				cluster_secondary_range_name:  in.networking.ranges.pods
				services_secondary_range_name: in.networking.ranges.services
			}

			if in.access.nodeIPs == "private" {
				private_cluster_config: {
					enable_private_nodes: true
					if in.access.controlPlaneEndpoint == "private" {
						enable_private_endpoint: true
					}
					if in.access.controlPlaneIpv4Cidr != _|_ {
						master_ipv4_cidr_block: in.access.controlPlaneIpv4Cidr
					}
				}

				if in.access.controlPlaneEndpoint == "private" {
					master_authorized_networks_config: {
						gcp_public_cidrs_access_enabled: false
					}
				}
			}

			workload_identity_config: {
				workload_pool: workloadPool
			}

			if in.serviceMesh.certificates {
				mesh_certificates: {
					enable_certificates: true
				}
			}

			if in.secrets.manager {
				secret_manager_config: {
					enabled: true
				}
			}

			if in.observability.managedOpenTelemetry {
				managed_opentelemetry_config: {
					scope: "COLLECTION_AND_INSTRUMENTATION_COMPONENTS"
				}
			}

			if in.secrets.sync != _|_ {
				secret_sync_config: {
					enabled: true
					if in.secrets.sync.rotationInterval != _|_ {
						rotation_config: {
							enabled:           true
							rotation_interval: in.secrets.sync.rotationInterval
						}
					}
				}
			}

			release_channel: {
				channel: #GkeReleaseChannelMap[in.releaseChannel]
			}
		}

		if in.mode == "standard" {
			for nodePoolName, nodePool in in.nodePools {
				resource: google_container_node_pool: (nodePoolName): {
					if in.#imports.nodePools[nodePoolName] != _|_ {
						#import: in.#imports.nodePools[nodePoolName]
					}

					name:     nodePoolName
					cluster:  "${\(refs.cluster).id}"
					location: in.location

					if in.project.id != _|_ {
						project: in.project.id
					}

					if nodePool.scaling.fixed != _|_ {
						node_count: nodePool.scaling.fixed.count
					}

					if len(nodePool.zones) > 0 {
						node_locations: nodePool.zones
					}

					if nodePool.pod.maxPerNode != _|_ {
						max_pods_per_node: nodePool.pod.maxPerNode
					}

					if nodePool.pod.range != _|_ {
						network_config: {
							enable_private_nodes: in.access.nodeIPs == "private"
							create_pod_range:     false
							pod_range:            nodePool.pod.range
						}
					}

					if nodePool.scaling.autoscaling != _|_ {
						autoscaling: {
							total_min_node_count: nodePool.scaling.autoscaling.min
							total_max_node_count: nodePool.scaling.autoscaling.max
							location_policy:      #GkeNodePoolLocationPolicyMap[nodePool.scaling.autoscaling.locationPolicy]
						}
					}

					management: {
						auto_repair:  nodePool.management.autoRepair
						auto_upgrade: nodePool.upgrade.automatic
					}

					upgrade_settings: {
						max_surge:       nodePool.upgrade.maxSurge
						max_unavailable: nodePool.upgrade.maxUnavailable
					}

					node_config: {
						machine_type: nodePool.machineType
						disk_type:    nodePool.disk.type
						disk_size_gb: nodePool.disk.size
						image_type:   nodePool.imageType

						if nodePool.identity.serviceAccount != _|_ {
							service_account: nodePool.identity.serviceAccount
						}

						oauth_scopes: nodePool.identity.accessScopes
						labels:       nodePool.labels

						taint: [for taint in nodePool.taints {
							key:    taint.key
							value:  taint.value
							effect: #GkeNodePoolTaintEffectMap[taint.effect]
						}]

						shielded_instance_config: {
							enable_secure_boot:          nodePool.security.secureBoot
							enable_integrity_monitoring: nodePool.security.integrityMonitoring
						}
					}
				}
			}
		}
	}
}

#GkeManagedServiceMesh: {
	in: {
		#importMembership?:        string
		#importFeature?:           string
		#importFeatureMembership?: string

		name: #RFC1035Name

		project?: string

		clusterId: string

		membershipId: #RFC1035Name
		membershipId: _ | *name

		membershipLocation: #GkeHubLocation
		membershipLocation: _ | *"global"

		featureName: #RFC1035Name
		featureName: _ | *"servicemesh"

		featureLocation: #GkeHubLocation
		featureLocation: _ | *"global"

		management: #GkeMeshManagement
		management: _ | *"automatic"
	}

	let featureMembershipName = "\(in.membershipId)-\(in.featureName)"

	refs: {
		membership:        "google_gke_hub_membership.\(in.membershipId)"
		feature:           "google_gke_hub_feature.\(in.featureName)"
		featureMembership: "google_gke_hub_feature_membership.\(featureMembershipName)"
	}

	out: T.#TerraformInput & {
		resource: google_gke_hub_membership: (in.membershipId): {
			if in.#importMembership != _|_ {
				#import: in.#importMembership
			}

			membership_id: in.membershipId
			location:      in.membershipLocation

			if in.project != _|_ {
				project: in.project
			}

			endpoint: {
				gke_cluster: {
					resource_link: "//container.googleapis.com/\(in.clusterId)"
				}
			}
		}

		resource: google_gke_hub_feature: (in.featureName): {
			if in.#importFeature != _|_ {
				#import: in.#importFeature
			}

			name:     in.featureName
			location: in.featureLocation

			if in.project != _|_ {
				project: in.project
			}
		}

		resource: google_gke_hub_feature_membership: (featureMembershipName): {
			if in.#importFeatureMembership != _|_ {
				#import: in.#importFeatureMembership
			}

			location:            in.featureLocation
			feature:             "${\(refs.feature).name}"
			membership:          "${\(refs.membership).membership_id}"
			membership_location: in.membershipLocation

			if in.project != _|_ {
				project: in.project
			}

			mesh: {
				management: #GkeMeshManagementMap[in.management]
			}
		}
	}
}
