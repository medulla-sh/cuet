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
		iam: [string]: {
			#import?: string
			role:     string
			member:   string
		}

		roles: [...string]
	}

	ref: "google_service_account.\(in.name)"
	let serviceAccountName = in.name
	let serviceAccountRef = ref
	let serviceAccountProject = in.project
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

		for name, iam in in.iam {
			resource: google_service_account_iam_member: ("\(serviceAccountName)-\(name)"): {
				if iam.#import != _|_ {
					#import: iam.#import
				}

				service_account_id: "${\(serviceAccountRef).name}"
				role:               iam.role
				member:             iam.member
			}
		}

		for role in in.roles {
			let iamMember = #IamMember & {"in": {
				"role": role
				member: "serviceAccount:${\(serviceAccountRef).email}"

				project: serviceAccountProject
			}}
			iamMember.out
		}
	}
}
