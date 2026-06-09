package google

import (
	T "github.com/medulla-sh/cuet"
)

#IamMember: {
	in: {
		#import?: string

		name: string

		project?: string
		role:     string
		member:   string
	}

	ref: "google_project_iam_member.\(in.name)"

	out: T.#TerraformInput & {
		resource: google_project_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			if in.project != _|_ {
				project: in.project
			}

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
