package google

import (
	"regexp"
	T "github.com/medulla-sh/cuet"
)

#ServiceAccount: {
	in: {
		#import?: string

		accountId: string

		name: string
		name: _ | *accountId

		displayName: string
		displayName: _ | *accountId

		description?: string
		project?:     string

		roles: [...string]
	}

	ref: "google_service_account.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_service_account: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			account_id: in.accountId

			display_name: in.displayName

			if in.description != _|_ {
				description: in.description
			}

			if in.project != _|_ {
				project: in.project
			}
		}

		for role in in.roles {
			let roleName = regexp.ReplaceAll("[:/.]", role, "-")
			let iamMember = #IamMember & {"in": {
				name:   "\(in.name)-\(roleName)"
				"role": role
				member: "serviceAccount:${\(ref).email}"

				if in.project != _|_ {
					project: in.project
				}
			}}
			iamMember.out
		}
	}
}
