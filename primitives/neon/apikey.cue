package neon

import (
	T "github.com/medulla-sh/cuet"
)

#ApiKey: {
	in: {
		name: string
	}
	ref: "neon_api_key.\(in.name)"
	out: T.#TerraformInput & {
		resource: neon_api_key: (in.name): {
			name: in.name
			...
		}
	}
}
