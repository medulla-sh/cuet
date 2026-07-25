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
		project?: {
			name: string
			name: _ | *id
			id:   string
		}
		role:   string
		member: string
	}

	ref: "google_project_iam_member.\(in.name)"

	out: this=T.#TerraformInput & {
		let projectDataName = [
			if in.project != _|_ {in.project.name},
			this.#envName,
		][0]

		data: google_project: (projectDataName): {
			if in.project != _|_ {
				project_id: in.project.id
			}
		}

		resource: google_project_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			project: "${data.google_project.\(projectDataName).project_id}"

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
