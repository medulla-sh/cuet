package google

import (
	"regexp"

	T "github.com/medulla-sh/cuet"
)

#IamMember: {
	in: {
		#import?: string

		name: string
		name: _ | *regexp.ReplaceAll("[^[:alnum:]_-]", "\(in.role)-\(in.member)", "-")
		project: {
			name: string
			id?:  string
		}
		role:   string
		member: string
	}

	ref: "google_project_iam_member.\(in.name)"

	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}

		resource: google_project_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			project: "${data.google_project.\(in.project.name).project_id}"

			role:   in.role
			member: in.member
			...
		}
	}
}

#ServiceAccountIamMember: {
	in: {
		#import?: string

		name: string

		serviceAccountId: string
		role:             string
		member:           string
	}

	ref: "google_service_account_iam_member.\(in.name)"

	out: T.#TerraformInput & {
		resource: google_service_account_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			service_account_id: in.serviceAccountId
			role:               in.role
			member:             in.member
		}
	}
}
