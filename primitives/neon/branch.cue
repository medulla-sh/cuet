package neon

import (
	T "github.com/medulla-sh/cuet"
)

#Branch: {
	in: {
		#import?: string

		name:      string
		projectId: string

		protected: bool
		protected: _ | *false
	}

	ref: "neon_branch.\(in.name)"
	out: T.#TerraformInput & {
		resource: neon_branch: (in.name): {
			project_id: in.projectId
			name:       in.name
			protected: [if in.protected {"yes"}, "no"][0]
		}
	}
}
