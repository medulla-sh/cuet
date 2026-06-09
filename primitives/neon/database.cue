package neon

import (
	T "github.com/medulla-sh/cuet"
)

#Database: {
	in: {
		#import?: string

		name:      string
		ownerName: string
		projectId: string
		branchId:  string
	}
	ref: "neon_database.\(in.name)"
	out: T.#TerraformInput & {
		resource: neon_database: (in.name): {
			name:       in.name
			project_id: in.projectId
			branch_id:  in.branchId
			owner_name: in.ownerName
		}
	}
}
