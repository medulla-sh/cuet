package google

import T "github.com/medulla-sh/cuet"

#ServiceAccountKey: {
	in: {
		#import?: string
		#dependsOn?: [...string]

		name:             string
		serviceAccountId: string
		keepers: [string]: string
	}

	ref: "google_service_account_key.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_service_account_key: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			if in.#dependsOn != _|_ {
				#dependsOn: in.#dependsOn
			}
			service_account_id: in.serviceAccountId
			if len(in.keepers) > 0 {
				keepers: in.keepers
			}
		}
	}
}
