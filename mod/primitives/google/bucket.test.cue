@if(test)

package google

#BucketTests: {
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
