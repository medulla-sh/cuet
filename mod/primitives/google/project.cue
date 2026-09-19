package google

import (
	"strings"
	T "github.com/medulla-sh/cuet"
)

#GcpServices:
	"accesscontextmanager.googleapis.com" |
	"alloydb.googleapis.com" |
	"apigateway.googleapis.com" |
	"apikeys.googleapis.com" |
	"artifactregistry.googleapis.com" |
	"bigquery.googleapis.com" |
	"bigtableadmin.googleapis.com" |
	"binaryauthorization.googleapis.com" |
	"certificatemanager.googleapis.com" |
	"cloudasset.googleapis.com" |
	"cloudbilling.googleapis.com" |
	"cloudbuild.googleapis.com" |
	"cloudfunctions.googleapis.com" |
	"cloudidentity.googleapis.com" |
	"cloudkms.googleapis.com" |
	"cloudresourcemanager.googleapis.com" |
	"cloudtrace.googleapis.com" |
	"composer.googleapis.com" |
	"compute.googleapis.com" |
	"connectgateway.googleapis.com" |
	"container.googleapis.com" |
	"containeranalysis.googleapis.com" |
	"dataflow.googleapis.com" |
	"dataproc.googleapis.com" |
	"datastore.googleapis.com" |
	"dns.googleapis.com" |
	"essentialcontacts.googleapis.com" |
	"firestore.googleapis.com" |
	"gkeconnect.googleapis.com" |
	"gkehub.googleapis.com" |
	"iam.googleapis.com" |
	"iamcredentials.googleapis.com" |
	"logging.googleapis.com" |
	"memcache.googleapis.com" |
	"meshca.googleapis.com" |
	"meshconfig.googleapis.com" |
	"monitoring.googleapis.com" |
	"networksecurity.googleapis.com" |
	"networkservices.googleapis.com" |
	"orgpolicy.googleapis.com" |
	"privateca.googleapis.com" |
	"pubsub.googleapis.com" |
	"redis.googleapis.com" |
	"run.googleapis.com" |
	"secretmanager.googleapis.com" |
	"servicedirectory.googleapis.com" |
	"servicenetworking.googleapis.com" |
	"serviceusage.googleapis.com" |
	"spanner.googleapis.com" |
	"sqladmin.googleapis.com" |
	"storage-api.googleapis.com" |
	"storage.googleapis.com" |
	"sts.googleapis.com" |
	"telemetry.googleapis.com" |
	"trafficdirector.googleapis.com" |
	"vpcaccess.googleapis.com"

#Project: {
	in: {
		#import?: string

		name:        string
		projectName: string
		projectName: _ | *name
		projectId:   string
		projectId:   _ | *name

		billingAccount?: string
		{orgId: string} | {folderId: string} | *{}

		deletionPolicy: #DeletionPolicy
		deletionPolicy: _ | *"PREVENT"

		enabledServices: [...#GcpServices]
		enabledServices: [_, ...]

		disableServicesOnDestroy: bool
		disableServicesOnDestroy: _ | *true
	}
	// Deprecated: use refs.project instead.
	ref: refs.project
	refs: {
		project: "google_project.\(in.name)"
		services: {
			for service in in.enabledServices {
				(service): "google_project_service.\(in.name)-\(strings.Replace(service, ".", "-", -1))"
			}
		}
	}
	out: T.#TerraformInput & {
		resource: google_project: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name:            in.projectName
			project_id:      in.projectId
			deletion_policy: in.deletionPolicy

			if in.billingAccount != _|_ {
				billing_account: in.billingAccount
			}

			if in.orgId != _|_ {
				org_id: in.orgId
			}

			if in.folderId != _|_ {
				folder_id: in.folderId
			}
			...
		}

		for service in in.enabledServices {
			let serviceName = "\(in.name)-\(strings.Replace(service, ".", "-", -1))"
			resource: google_project_service: (serviceName): {
				project:            "${\(refs.project).id}"
				"service":          service
				disable_on_destroy: in.disableServicesOnDestroy
			}
		}
	}
}

#DataProject: {
	in: {
		name: string
		name: _ | *projectId

		projectId: string
	}

	ref: "data.google_project.\(in.name)"

	out: T.#TerraformInput & {
		data: google_project: (in.name): {
			project_id: in.projectId
		}
	}
}

#ProjectServiceIdentity: {
	in: {
		name:    string
		service: #GcpServices
		project: {
			name: string
			id?:  string
		}
	}

	ref: "google_project_service_identity.\(in.name)"

	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}

		resource: google_project_service_identity: (in.name): {
			#provider: "google-beta"
			project:   "${data.google_project.\(in.project.name).project_id}"
			"service": in.service
		}
	}
}
