package google

import (
	T "github.com/medulla-sh/cuet"
)

#BillingAccount: {
	in: {
		name: string
		id:   string
	}
	ref: "data.google_billing_account.\(in.name)"
	out: T.#TerraformInput & {
		data: google_billing_account: (in.name): {
			billing_account: in.id
			...
		}
	}
}
