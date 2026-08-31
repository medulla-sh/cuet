package google

import T "github.com/medulla-sh/cuet"

#DeletionPolicy: "ABANDON" | "DELETE" | "PREVENT"

#WorkloadIdentityPrincipal: {
	in: {
		// Full workload identity pool resource name, including project and location.
		poolName: string & !=""
		subject:  string & !=""
	}

	val: "principal://iam.googleapis.com/\(in.poolName)/subject/\(in.subject)"
}

#WorkloadIdentityPrincipalSet: {
	in: {
		// Full workload identity pool resource name, including project and location.
		poolName: string & !=""

		({
			attribute: {
				name:  =~"^[a-z0-9_]{1,100}$"
				value: string & !=""
			}
		} | {
			group: string & !=""
		} | {
			all: true
		})
	}

	if in.attribute != _|_ {
		val: "principalSet://iam.googleapis.com/\(in.poolName)/attribute.\(in.attribute.name)/\(in.attribute.value)"
	}
	if in.group != _|_ {
		val: "principalSet://iam.googleapis.com/\(in.poolName)/group/\(in.group)"
	}
	if in.all != _|_ {
		val: "principalSet://iam.googleapis.com/\(in.poolName)/*"
	}
}

#WorkloadIdentityPool: {
	in: {
		// Adopts an existing pool using a supported Google Cloud import identifier.
		#import?: string

		// Used as both the pool ID and Terraform resource key.
		name: =~"^[a-z0-9-]{4,32}$" & !~"^gcp-"

		// Displayed in the Google Cloud console.
		displayName: string
		displayName: _ | *name

		// Explains which external workloads the pool admits.
		description?: string

		project: {
			// Selects the Terraform data-source key.
			name: string

			// Selects a specific Google Cloud project when it differs from the key.
			id?: string
		}

		// Prevents new token exchanges while preserving the pool configuration.
		disabled: bool
		disabled: _ | *false

		// Controls what happens when Terraform removes the resource.
		deletionPolicy: #DeletionPolicy
		deletionPolicy: _ | *"PREVENT"
	}

	ref: "google_iam_workload_identity_pool.\(in.name)"

	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}

		resource: google_iam_workload_identity_pool: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			project:                   "${data.google_project.\(in.project.name).project_id}"
			workload_identity_pool_id: in.name
			display_name:              in.displayName
			disabled:                  in.disabled
			deletion_policy:           in.deletionPolicy

			if in.description != _|_ {
				description: in.description
			}
		}
	}
}

#WorkloadIdentityProvider: {
	in: {
		// Adopts an existing provider using a supported Google Cloud import identifier.
		#import?: string

		// Used as both the provider ID and Terraform resource key.
		name: =~"^[a-z0-9-]{4,32}$" & !~"^gcp-"

		// Displayed in the Google Cloud console.
		displayName: string
		displayName: _ | *name

		// Explains which external issuer and workload the provider trusts.
		description?: string

		project: {
			// Selects the Terraform data-source key.
			name: string

			// Selects a specific Google Cloud project when it differs from the key.
			id?: string
		}

		// References the containing pool ID or a Terraform expression that resolves to it.
		poolId: string & !=""

		// Maps external claims into Google Cloud subject and custom attributes.
		attributeMapping: {
			"google.subject": string & !=""
			[string]:         string & !=""
		}

		// Rejects tokens that do not satisfy the Common Expression Language predicate.
		attributeCondition: string & !=""

		{
			oidc: {
				// Identifies the OpenID Connect issuer.
				issuerUri: =~"^https://.+"

				// Restricts accepted token audiences when the issuer default is insufficient.
				allowedAudiences: [...string]
			}
		} | {
			// Identifies the trusted AWS account.
			aws: accountId: =~"^[0-9]{12}$"
		}

		// Prevents new token exchanges while preserving the provider configuration.
		disabled: bool
		disabled: _ | *false

		// Controls what happens when Terraform removes the resource.
		deletionPolicy: #DeletionPolicy
		deletionPolicy: _ | *"PREVENT"
	}

	ref: "google_iam_workload_identity_pool_provider.\(in.name)"

	out: T.#TerraformInput & {
		data: google_project: (in.project.name): {
			if in.project.id != _|_ {
				project_id: in.project.id
			}
		}

		resource: google_iam_workload_identity_pool_provider: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			project:                            "${data.google_project.\(in.project.name).project_id}"
			workload_identity_pool_id:          in.poolId
			workload_identity_pool_provider_id: in.name
			display_name:                       in.displayName
			attribute_mapping:                  in.attributeMapping
			attribute_condition:                in.attributeCondition
			disabled:                           in.disabled
			deletion_policy:                    in.deletionPolicy

			if in.description != _|_ {
				description: in.description
			}

			if in.oidc != _|_ {
				oidc: {
					issuer_uri: in.oidc.issuerUri
					if len(in.oidc.allowedAudiences) > 0 {
						allowed_audiences: in.oidc.allowedAudiences
					}
				}
			}

			if in.aws != _|_ {
				aws: account_id: in.aws.accountId
			}
		}
	}
}
