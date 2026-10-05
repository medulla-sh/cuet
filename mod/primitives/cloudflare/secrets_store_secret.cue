package cloudflare

import "list"

#SecretsStoreSecret: {
	in: {
		#import?:   string
		name:       string & !=""
		accountId:  string & !=""
		storeId:    string & !=""
		secretName: string & !=""
		secretName: _ | *name
		scopes: [..."workers" | "ai_gateway" | "dex" | "access"] & list.MinItems(1)
		value:    string
		comment?: string
	}

	ref: "cloudflare_secrets_store_secret.\(in.name)"

	out: resource: cloudflare_secrets_store_secret: (in.name): {
		if in.#import != _|_ {
			#import: in.#import
		}
		account_id: in.accountId
		store_id:   in.storeId
		name:       in.secretName
		// The provider requires scopes in alphabetical order.
		scopes: list.SortStrings(in.scopes)
		value:  in.value
		if in.comment != _|_ {
			comment: in.comment
		}
	}
}
