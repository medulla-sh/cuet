package cloudflare

import (
	"strings"
	T "github.com/medulla-sh/cuet"
)

// #AccountToken creates an account-owned Cloudflare API token.
// The provider credential must have Account API Tokens Read and Write.
#AccountToken: {
	in: {
		#import?: string

		name:      string
		accountId: string

		policies: [
			{
				effect: "allow" | "deny"
				permissionGroups: [string, ...string]
				resources: [string]: string
			},
			...{
				effect: "allow" | "deny"
				permissionGroups: [string, ...string]
				resources: [string]: string
			},
		]
	}

	ref: "cloudflare_account_token.\(in.name)"

	out: T.#TerraformInput & {
		resource: cloudflare_account_token: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			account_id: in.accountId
			name:       in.name
			policies: [for policy in in.policies {
				effect: policy.effect
				permission_groups: [for permissionGroup in policy.permissionGroups {
					id: permissionGroup
				}]
				let resourceEntries = [for resource, value in policy.resources {#""\#(resource)" = "\#(value)""#}]
				resources: #"${jsonencode({\#(strings.Join(resourceEntries, ", "))})}"#
			}]
		}
	}
}
