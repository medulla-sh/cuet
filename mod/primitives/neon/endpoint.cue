package neon

import (
	T "github.com/medulla-sh/cuet"
)

#EndpointType: "read_write" | "read_only"

#Endpoint: {
	in: {
		#import?: string

		name:      string
		projectId: string
		branchId:  string

		type: #EndpointType
		type: _ | *"read_write"
	}

	ref: "neon_endpoint.\(in.name)"
	out: T.#TerraformInput & {
		resource: neon_endpoint: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			project_id: in.projectId
			branch_id:  in.branchId
			type:       in.type
		}
	}
}
