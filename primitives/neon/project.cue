package neon

import (
	T "github.com/medulla-sh/cuet"
)

#NeonRegion: "aws-us-east-1" | "aws-us-east-2" | "aws-us-west-2"

#Project: {
	in: {
		#import?: string

		name: string

		pgVersion: int
		pgVersion: _ | *18

		historyRetentionSeconds: int & >=0
		historyRetentionSeconds: _ | *0

		regionId: #NeonRegion
	}

	ref: "neon_project.\(in.name)"
	out: T.#TerraformInput & {
		resource: neon_project: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			name: in.name

			history_retention_seconds: in.historyRetentionSeconds
			region_id:                 in.regionId
		}
	}
}
