package google

import T "github.com/medulla-sh/cuet"

#ServiceAccount: {
	in: {
		#import?: string

		accountId: string

		name: string
		name: _ | *accountId

		displayName: string
		displayName: _ | *accountId

		description?: string
		project: {
			name: string
			id?:  string
		}

		roles: [...string]
	}

	ref: "google_service_account.\(in.name)"
	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}

		resource: google_service_account: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			account_id: in.accountId

			display_name: in.displayName

			if in.description != _|_ {
				description: in.description
			}

			project: "${data.google_project.\(in.project.name).project_id}"
		}

		for role in in.roles {
			let iamMember = #IamMember & {"in": {
				"role": role
				member: "serviceAccount:${\(ref).email}"

				project: in.project
			}}
			iamMember.out
		}
	}
}
