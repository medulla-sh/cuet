@if(test)

package cloudflare

#SecretsStoreSecretTests: {
	let base = {
		name:      "api-key"
		accountId: "account-id"
		storeId:   "${cloudflare_secrets_store.shared.id}"
		value:     "${data.google_secret_manager_secret_version.api.secret_data}"
	}

	"default-name-and-no-optional-fields": {
		input: #SecretsStoreSecret & {in: base & {scopes: ["workers"]}}
		let secret = input.out.resource.cloudflare_secrets_store_secret[base.name]

		assert: input.ref == "cloudflare_secrets_store_secret.api-key"
		assert: secret.name == base.name
		assert: secret.account_id == base.accountId
		assert: secret.store_id == base.storeId
		assert: secret.value == base.value
		assert: secret.scopes == ["workers"]
		assert: secret.#import == _|_
		assert: secret.comment == _|_
		assert: input.out.output == _|_
	}
	"explicit-name-import-comment-and-sorted-scopes": {
		input: #SecretsStoreSecret & {in: base & {
			secretName: "shared-api-key"
			#import:    "account-id/store-id/secret-id"
			comment:    "Owned by the API integration"
			scopes: ["workers", "dex", "ai_gateway", "access"]
		}}
		let secret = input.out.resource.cloudflare_secrets_store_secret[base.name]

		assert: secret.name == input.in.secretName
		assert: secret.#import == input.in.#import
		assert: secret.comment == input.in.comment
		assert: secret.scopes == ["access", "ai_gateway", "dex", "workers"]
	}
	"empty-scopes-rejected": {
		assert: (#SecretsStoreSecret & {in: base & {scopes: []}}) == _|_
	}
	"unsupported-scope-rejected": {
		assert: (#SecretsStoreSecret & {in: base & {scopes: ["unknown"]}}) == _|_
	}
}

secretsStoreSecretResult: [for _, test in #SecretsStoreSecretTests {test.assert & true}]
