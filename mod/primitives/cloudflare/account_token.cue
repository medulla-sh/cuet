package cloudflare

import (
	"strings"
	T "github.com/medulla-sh/cuet"
)

#accountTokenPermissionScopes: {
	account: "com.cloudflare.api.account"
	zone:    "com.cloudflare.api.account.zone"
}

// #AccountTokenPermissions resolves account-owned token permissions from one listing.
#AccountTokenPermissions: {
	in: {
		name:      string
		accountId: string
		permissions: {
			account?: [string]: string
			zone?: [string]:    string
		}
	}

	let sourceRef = "data.cloudflare_account_api_token_permission_groups.\(in.name)"
	let localName = "\(in.name)-account-token-permissions"

	refs: {
		for scope, permissions in in.permissions {
			(scope): {
				for key, permission in permissions {
					let localKey = "\(permission):\(#accountTokenPermissionScopes[scope])"
					(key): id: #"local.\#(localName)["\#(localKey)"].id"#
				}
			}
		}
	}

	out: T.#TerraformInput & {
		data: cloudflare_account_api_token_permission_groups: (in.name): {
			account_id: in.accountId
		}
		locals: (localName): #"""
			${{
				for entry in flatten([
					for permission in \#(sourceRef).permission_groups : [
						for scope in permission.scopes : {
							key        = join(":", [permission.name, scope])
							permission = permission
						}
					]
				]) : entry.key => entry.permission
			}}
			"""#
	}
}

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
