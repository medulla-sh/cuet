@if(test)

package google

#BucketTests: {
	for mode in ["inherited", "enforced"] {
		"public access prevention \(mode)": {
			bucket: #Bucket & {in: {
				name:                   "private-documents"
				location:               "us-west1"
				publicAccessPrevention: mode
			}}
			assert: bucket.out.resource.google_storage_bucket["private-documents"].public_access_prevention == mode
		}
	}
	"lifecycle rules and principals": {
		bucket: #Bucket & {in: {
			name:     "cache"
			location: "us"
			lifecycleRules: [{
				action: type: "Delete"
				condition: {
					age:       30
					withState: "ANY"
					matchesPrefix: ["tmp/"]
				}
			}]
			principals: writer: {
				role:   "roles/storage.objectUser"
				member: "serviceAccount:writer@example.com"
			}
		}}

		assert: bucket.out == {
			resource: {
				google_storage_bucket: cache: {
					name:     "cache"
					location: "us"
					versioning: enabled: false
					lifecycle_rule: [{
						action: type: "Delete"
						condition: {
							age:        30
							with_state: "ANY"
							matches_prefix: ["tmp/"]
						}
					}]
				}
				google_storage_bucket_iam_member: "cache-writer": {
					bucket: "${google_storage_bucket.cache.name}"
					role:   "roles/storage.objectUser"
					member: "serviceAccount:writer@example.com"
				}
			}
		}
	}
}

bucketResult: [for _, test in #BucketTests {test.assert & true}]
