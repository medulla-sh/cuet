package google

import (
	T "github.com/medulla-sh/cuet"
)

#Secret: {
	in: {
		#import?: string

		name: string
		name: _ | *secretId

		secretId:               string
		value?:                 string
		versionDeletionPolicy?: "DELETE" | "DISABLE" | "ABANDON" | "PREVENT"
		accessors: {[string]: string}
		annotations: {[string]: string}
	}
	ref: "google_secret_manager_secret.\(in.name)"
	out: T.#TerraformInput & {
		let secretName = in.name
		let secretRef = ref

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

		for name, accessor in in.accessors {
			resource: google_secret_manager_secret_iam_member: ("\(secretName)-\(name)-accessor"): {
				secret_id: "${\(secretRef).id}"
				role:      "roles/secretmanager.secretAccessor"
				"member":  accessor
				...
			}
		}

		if in.value != _|_ {
			resource: google_secret_manager_secret_version: (secretName): {
				secret:      "${\(secretRef).id}"
				secret_data: in.value
				if in.versionDeletionPolicy != _|_ {
					deletion_policy: in.versionDeletionPolicy
				}
			}
		}
	}
}

#SecretIamMember: {
	in: {
		#import?: string

		name:     string
		secretId: string
		role:     string
		role:     _ | *"roles/secretmanager.secretAccessor"
		member:   string
	}

	ref: "google_secret_manager_secret_iam_member.\(in.name)"

	out: T.#TerraformInput & {
		resource: google_secret_manager_secret_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			secret_id: in.secretId
			role:      in.role
			member:    in.member
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
		version:  string
		version:  _ | *"latest"
	}
	ref: "ephemeral.google_secret_manager_secret_version.\(in.name)"
	out: T.#TerraformInput & {
		ephemeral: "google_secret_manager_secret_version": (in.name): {
			project: in.project
			secret:  in.secretId
			version: in.version
		}
	}
}
