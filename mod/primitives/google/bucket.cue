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

		lifecycleRules: [...{
			action: {
				type:          "Delete" | "SetStorageClass" | "AbortIncompleteMultipartUpload"
				storageClass?: string
			}
			condition: {
				age?:                     int & >=0
				createdBefore?:           string
				customTimeBefore?:        string
				daysSinceCustomTime?:     int & >=0
				daysSinceNoncurrentTime?: int & >=0
				matchesPrefix?: [...string]
				matchesStorageClass?: [...string]
				matchesSuffix?: [...string]
				noncurrentTimeBefore?: string
				numNewerVersions?:     int & >=0
				withState?:            "ANY" | "LIVE" | "ARCHIVED"
			}
		}]
		lifecycleRules: _ | *[]

		principals: [string]: {
			#import?: string

			name?:  string
			role:   string
			member: string
		}
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

			if len(in.lifecycleRules) > 0 {
				lifecycle_rule: [for rule in in.lifecycleRules {
					action: {
						type: rule.action.type
						if rule.action.storageClass != _|_ {
							storage_class: rule.action.storageClass
						}
					}
					condition: {
						if rule.condition.age != _|_ {
							age: rule.condition.age
						}
						if rule.condition.createdBefore != _|_ {
							created_before: rule.condition.createdBefore
						}
						if rule.condition.customTimeBefore != _|_ {
							custom_time_before: rule.condition.customTimeBefore
						}
						if rule.condition.daysSinceCustomTime != _|_ {
							days_since_custom_time: rule.condition.daysSinceCustomTime
						}
						if rule.condition.daysSinceNoncurrentTime != _|_ {
							days_since_noncurrent_time: rule.condition.daysSinceNoncurrentTime
						}
						if rule.condition.matchesPrefix != _|_ {
							matches_prefix: rule.condition.matchesPrefix
						}
						if rule.condition.matchesStorageClass != _|_ {
							matches_storage_class: rule.condition.matchesStorageClass
						}
						if rule.condition.matchesSuffix != _|_ {
							matches_suffix: rule.condition.matchesSuffix
						}
						if rule.condition.noncurrentTimeBefore != _|_ {
							noncurrent_time_before: rule.condition.noncurrentTimeBefore
						}
						if rule.condition.numNewerVersions != _|_ {
							num_newer_versions: rule.condition.numNewerVersions
						}
						if rule.condition.withState != _|_ {
							with_state: rule.condition.withState
						}
					}
				}]
			}
			...
		}

		for principalName, principal in in.principals {
			let iamMember = #BucketIamMember & {"in": {
				if principal.name != _|_ {
					name: principal.name
				}
				if principal.name == _|_ {
					name: "\(in.name)-\(principalName)"
				}
				bucket: "${\(ref).name}"
				role:   principal.role
				member: principal.member
				if principal.#import != _|_ {
					#import: principal.#import
				}
			}}
			iamMember.out
		}
	}
}
