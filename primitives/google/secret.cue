package google

import (
	T "github.com/medulla-sh/cuet"
)

#Secret: {
	in: {
		#import?: string

		name: string
		name: _ | *secretId

		secretId: string
		annotations: {[string]: string}
	}
	ref: "google_secret_manager_secret.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_secret_manager_secret: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			secret_id: in.secretId

			if len(in.annotations) > 0 {
				annotations: in.annotations
			}

			// TODO(mez): make this configurable
			replication: auto: {}
			...
		}
	}
}

#SecretVersion: {
	in: {
		name: string
		name: _ | *secretId

		project:  string
		secretId: string
	}
	ref: "data.google_secret_manager_secret_version.\(in.name)"
	out: T.#TerraformInput & {
		data: "google_secret_manager_secret_version": (in.name): {
			project: in.project
			secret:  in.secretId
		}
	}
}
