package google

import (
	"strings"
	T "github.com/medulla-sh/cuet"
)

#GcpServices:
	"artifactregistry.googleapis.com" |
	"cloudresourcemanager.googleapis.com" |
	"cloudidentity.googleapis.com" |
	"cloudtrace.googleapis.com" |
	"compute.googleapis.com" |
	"connectgateway.googleapis.com" |
	"container.googleapis.com" |
	"gkeconnect.googleapis.com" |
	"gkehub.googleapis.com" |
	"iam.googleapis.com" |
	"iamcredentials.googleapis.com" |
	"logging.googleapis.com" |
	"meshca.googleapis.com" |
	"meshconfig.googleapis.com" |
	"monitoring.googleapis.com" |
	"networksecurity.googleapis.com" |
	"networkservices.googleapis.com" |
	"pubsub.googleapis.com" |
	"run.googleapis.com" |
	"secretmanager.googleapis.com" |
	"servicenetworking.googleapis.com" |
	"sqladmin.googleapis.com" |
	"sts.googleapis.com" |
	"telemetry.googleapis.com" |
	"trafficdirector.googleapis.com"

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

		enabledServices: [...#GcpServices]
		enabledServices: [_, ...]
	}
	ref: "google_project.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_project: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name:       in.projectName
			project_id: in.projectId

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
				project:   "${\(ref).id}"
				"service": service
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
