package random

import T "github.com/medulla-sh/cuet"

#Password: {
	in: {
		name: string

		length: int & >=16
		length: _ | *32

		lower: bool
		lower: _ | *true

		upper: bool
		upper: _ | *true

		numeric: bool
		numeric: _ | *true

		special: bool
		special: _ | *true

		overrideSpecial?: string
		keepers: {[string]: string}
	}

	ref: "random_password.\(in.name)"

	out: T.#TerraformInput & {
		resource: random_password: (in.name): {
			length:  in.length
			lower:   in.lower
			upper:   in.upper
			numeric: in.numeric
			special: in.special

			if in.overrideSpecial != _|_ {
				override_special: in.overrideSpecial
			}

			if len(in.keepers) > 0 {
				keepers: in.keepers
			}
		}
	}
}
