package google

import (
	T "github.com/medulla-sh/cuet"
)

#MultiRegion:    "us" | "eu" | "asia"
#BucketLocation: #Region | #MultiRegion

#Bucket: {
	in: {
		#import?: string

		name:     =~"^[a-z0-9-]{3,62}$"
		location: #BucketLocation

		enableVersioning: bool
		enableVersioning: _ | *false
	}
	ref: "google_storage_bucket.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_storage_bucket: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name:     in.name
			location: in.location

			versioning: enabled: in.enableVersioning
			...
		}
	}
}
