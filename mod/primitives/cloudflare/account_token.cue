package cloudflare

import (
	"encoding/json"
	"strings"
	T "github.com/medulla-sh/cuet"
)

#accountTokenPermissionScopes: {
	account: "com.cloudflare.api.account"
	zone:    "com.cloudflare.api.account.zone"
}

// #AccountToken creates an account-owned Cloudflare API token.
// The provider credential must have Account API Tokens Read and Write.
#AccountTokenPolicy: {
	effect: "allow" | "deny"
	effect: _ | *"allow"
	permissions: {
		account: [...string]
		zone: [...string]
	}
	resources: {
		scope: "account" | "zone" | "accountZones"
		id:    string
	}
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
	let permissionSourceRef = "data.cloudflare_account_api_token_permission_groups_list.\(in.name)"
	let permissionsLocalName = "\(in.name)-account-token-permissions"

	out: T.#TerraformInput & {
		data: cloudflare_account_api_token_permission_groups_list: (in.name): {
			account_id: in.accountId
		}
		locals: (permissionsLocalName): #"""
			${{
				for entry in flatten([
				for permission in \#(permissionSourceRef).result : [
						for scope in permission.scopes : {
							key        = join(":", [permission.name, scope])
							permission = permission
						}
					]
				]) : entry.key => entry.permission
			}}
			"""#
		resource: cloudflare_account_token: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			account_id: in.accountId
			name:       in.name
			policies: [for policy in in.policies {
				let resourceIdExpression = [
					if strings.HasPrefix(policy.resources.id, "${") && strings.HasSuffix(policy.resources.id, "}") {
						strings.TrimSuffix(strings.TrimPrefix(policy.resources.id, "${"), "}")
					},
					json.Marshal(policy.resources.id),
				][0]
				effect: policy.effect
				permission_groups: [_, ...]
				permission_groups: [
					for scope, permissions in policy.permissions
					for permission in permissions {
						let localKey = "\(permission):\(#accountTokenPermissionScopes[scope])"
						id: #"${local.\#(permissionsLocalName)["\#(localKey)"].id}"#
					},
				]
				resources: [
					if policy.resources.scope == "account" {
						json.Marshal({("com.cloudflare.api.account.\(policy.resources.id)"): "*"})
					},
					if policy.resources.scope == "accountZones" {
						json.Marshal({("com.cloudflare.api.account.\(policy.resources.id)"): {
							"com.cloudflare.api.account.zone.*": "*"
						}})
					},
					if policy.resources.scope == "zone" {
						#"${jsonencode({(format("com.cloudflare.api.account.zone.%s", \#(resourceIdExpression))): "*"})}"#
					},
				][0]
			}]
		}
	}
}
