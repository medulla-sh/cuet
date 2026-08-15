package cloudflare

import (
	"encoding/json"
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

	let sourceRef = "data.cloudflare_account_api_token_permission_groups_list.\(in.name)"
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
		data: cloudflare_account_api_token_permission_groups_list: (in.name): {
			account_id: in.accountId
		}
		locals: (localName): #"""
			${{
				for entry in flatten([
				for permission in \#(sourceRef).result : [
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
#AccountTokenPolicy: {
	effect: "allow" | "deny"
	effect: _ | *"allow"
	permissionGroups: [...string]
	permissionGroups: [_, ...]
	resources: [string]: string | {[string]: string}
}

#AccountToken: {
	in: {
		#import?: string

		name:      string
		accountId: string

		policies: [...#AccountTokenPolicy]
		policies: [_, ...]
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
				resources: json.Marshal(policy.resources)
			}]
		}
	}
}
