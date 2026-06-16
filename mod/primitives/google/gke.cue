package google

import (
	T "github.com/medulla-sh/cuet"
)

#GkeReleaseChannel:
	"UNSPECIFIED" |
	"RAPID" |
	"REGULAR" |
	"STABLE" |
	"EXTENDED"

#GkeMeshManagement: "automatic" | "manual"

#GkeMeshManagementMap: {
	automatic: "MANAGEMENT_AUTOMATIC"
	manual:    "MANAGEMENT_MANUAL"
}

#GkeHubLocation: "global" | #Region

#GkeCluster: {
	in: {
		#import?: string

		name:     string
		location: #Region

		project?: string

		network:    string
		subnetwork: string

		podRangeName:     #RFC1035Name
		serviceRangeName: #RFC1035Name

		workloadPool?: string

		meshCertificates: bool
		meshCertificates: _ | *false

		deletionProtection: bool
		deletionProtection: _ | *true

		releaseChannel: #GkeReleaseChannel
		releaseChannel: _ | *"REGULAR"

		// TODO(LUM-15): Allow for non-autopilot GKE clusters
		enableAutopilot: true
	}

	ref: "google_container_cluster.\(in.name)"

	out: T.#TerraformInput & {
		let projectDataName = in.name

		if in.workloadPool == _|_ {
			data: google_project: (projectDataName): {
				if in.project != _|_ {
					project_id: in.project
				}
			}
		}

		let workloadPool = [
			if in.workloadPool != _|_ {in.workloadPool},
			if in.workloadPool == _|_ {"${data.google_project.\(projectDataName).project_id}.svc.id.goog"},
		][0]

		resource: google_container_cluster: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name:     in.name
			location: in.location

			if in.project != _|_ {
				project: in.project
			}

			enable_autopilot:    in.enableAutopilot
			deletion_protection: in.deletionProtection

			network:    in.network
			subnetwork: in.subnetwork

			networking_mode: "VPC_NATIVE"
			ip_allocation_policy: {
				cluster_secondary_range_name:  in.podRangeName
				services_secondary_range_name: in.serviceRangeName
			}

			workload_identity_config: {
				workload_pool: workloadPool
			}

			if in.meshCertificates {
				mesh_certificates: {
					enable_certificates: true
				}
			}

			release_channel: {
				channel: in.releaseChannel
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
