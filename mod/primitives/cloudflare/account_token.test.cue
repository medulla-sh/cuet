@if(test)

package cloudflare

#AccountTokenTests: {
	"resolves-scoped-permission-names": {
		input: #AccountToken & {in: {
			#import:   "token-id"
			name:      "example"
			accountId: "account-id"
			policies: [{
				permissions: account: ["Account API Tokens Read"]
				resources: {
					scope: "account"
					id:    "account-id"
				}
			}, {
				permissions: zone: ["DNS Write", "Zone Read"]
				resources: {
					scope: "zone"
					id:    #"${data.terraform_remote_state.dns.outputs["zone-id"]}"#
				}
			}]
		}}

		let token = input.out.resource.cloudflare_account_token.example

		assert: input.out.data.cloudflare_account_api_token_permission_groups_list.example.account_id == "account-id"
		assert: input.out.locals["example-account-token-permissions"] != _|_
		assert: token.#import == "token-id"
		assert: token.policies[0].effect == "allow"
		assert: token.policies[0].permission_groups[0].id == #"${local.example-account-token-permissions["Account API Tokens Read:com.cloudflare.api.account"].id}"#
		assert: token.policies[0].resources == #"{"com.cloudflare.api.account.account-id":"*"}"#
		assert: token.policies[1].permission_groups[0].id == #"${local.example-account-token-permissions["DNS Write:com.cloudflare.api.account.zone"].id}"#
		assert: token.policies[1].permission_groups[1].id == #"${local.example-account-token-permissions["Zone Read:com.cloudflare.api.account.zone"].id}"#
		assert: token.policies[1].resources == #"${jsonencode({(format("com.cloudflare.api.account.zone.%s", data.terraform_remote_state.dns.outputs["zone-id"])): "*"})}"#
	}

	"requires-permissions": {
		input: {
			name:      "example"
			accountId: "account-id"
			policies: [{
				resources: {
					scope: "account"
					id:    "account-id"
				}
			}]
		}

		assert: (#AccountToken & {in: input}) == _|_
	}
}

accountTokenResult: [for _, test in #AccountTokenTests {test.assert & true}]
