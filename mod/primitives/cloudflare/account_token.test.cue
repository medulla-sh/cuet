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
				resources: "com.cloudflare.api.account.account-id": "*"
			}, {
				permissions: zone: ["DNS Write", "Zone Read"]
				resources: "com.cloudflare.api.account.zone.zone-id": "*"
			}]
		}}

		let token = input.out.resource.cloudflare_account_token.example

		assert: input.out.data.cloudflare_account_api_token_permission_groups_list.example.account_id == "account-id"
		assert: input.out.locals["example-account-token-permissions"] != _|_
		assert: token.#import == "token-id"
		assert: token.policies[0].effect == "allow"
		assert: token.policies[0].permission_groups[0].id == #"${local.example-account-token-permissions["Account API Tokens Read:com.cloudflare.api.account"].id}"#
		assert: token.policies[1].permission_groups[0].id == #"${local.example-account-token-permissions["DNS Write:com.cloudflare.api.account.zone"].id}"#
		assert: token.policies[1].permission_groups[1].id == #"${local.example-account-token-permissions["Zone Read:com.cloudflare.api.account.zone"].id}"#
	}

	"requires-permissions": {
		input: {
			name:      "example"
			accountId: "account-id"
			policies: [{
				resources: "com.cloudflare.api.account.account-id": "*"
			}]
		}

		assert: (#AccountToken & {in: input}) == _|_
	}
}

accountTokenResult: [for _, test in #AccountTokenTests {test.assert & true}]
