package google

import T "github.com/medulla-sh/cuet"

#LoggingBucket: {
	in: {
		#import?: string
		#dependsOn?: [...string]

		name:          =~"^[a-zA-Z0-9_-]{1,100}$" & !="_Default" & !="_Required"
		projectId:     string & !=""
		location:      string & !=""
		retentionDays: int & >=1 & <=3650

		locked: bool
		locked: _ | *false

		deletionPolicy: #DeletionPolicy
		deletionPolicy: _ | *"PREVENT"
	}

	ref:         "google_logging_project_bucket_config.\(in.name)"
	destination: "logging.googleapis.com/${\(ref).name}"

	out: T.#TerraformInput & {
		resource: google_logging_project_bucket_config: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			if in.#dependsOn != _|_ {
				#dependsOn: in.#dependsOn
			}
			bucket_id:       in.name
			project:         in.projectId
			location:        in.location
			retention_days:  in.retentionDays
			locked:          in.locked
			deletion_policy: in.deletionPolicy
		}
	}
}

// #OrgLoggingSink routes logs without intercepting child sinks.
#OrgLoggingSink: {
	in: {
		#import?: string

		name:        =~"^[a-zA-Z0-9_-]{1,100}$"
		orgId:       string & !=""
		destination: string & !=""
		filter:      string

		includeChildren: bool
		includeChildren: _ | *false
	}

	ref:            "google_logging_organization_sink.\(in.name)"
	writerIdentity: "${\(ref).writer_identity}"

	out: T.#TerraformInput & {
		resource: google_logging_organization_sink: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			name:               in.name
			org_id:             in.orgId
			destination:        in.destination
			filter:             in.filter
			include_children:   in.includeChildren
			intercept_children: false
		}
	}
}
