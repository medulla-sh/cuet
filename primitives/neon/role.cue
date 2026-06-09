package neon

import (
	T "github.com/medulla-sh/cuet"
)

#Role: {
	in: {
		#import?: string

		name:      string
		projectId: string
		branchId:  string
	}

	ref: "neon_role.\(in.name)"
	out: T.#TerraformInput & {
		resource: neon_role: (in.name): {
			name:       in.name
			project_id: in.projectId
			branch_id:  in.branchId
		}
	}
}
