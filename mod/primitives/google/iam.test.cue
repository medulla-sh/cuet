@if(test)

package google

#IamMemberTests: {
	"current-environment": {
		input: #IamMember & {"in": {
			name:   "reader"
			role:   "roles/artifactregistry.reader"
			member: "user:test@example.com"
		}}
		input: out: #envName: "internal"

		assert: input.out.data.google_project.internal == {}
		assert: input.out.resource.google_project_iam_member.reader.project == "${data.google_project.internal.project_id}"
	}

	"id-only": {
		input: #IamMember & {"in": {
			name: "reader"
			project: id: "oakmont-internal"
			role:   "roles/artifactregistry.reader"
			member: "user:test@example.com"
		}}

		assert: input.out.data.google_project["oakmont-internal"].project_id == "oakmont-internal"
		assert: input.out.resource.google_project_iam_member.reader.project == "${data.google_project.oakmont-internal.project_id}"
	}

	"name-and-id": {
		input: #IamMember & {"in": {
			name: "reader"
			project: {
				name: "internal"
				id:   "oakmont-internal"
			}
			role:   "roles/artifactregistry.reader"
			member: "user:test@example.com"
		}}

		assert: input.out.data.google_project.internal.project_id == "oakmont-internal"
		assert: input.out.resource.google_project_iam_member.reader.project == "${data.google_project.internal.project_id}"
	}
}

result: [for _, test in #IamMemberTests {test.assert & true}]

#BucketIamMemberTests: {
	"import": {
		input: #BucketIamMember & {"in": {
			#import: "b/oakmont-tf-dev roles/storage.objectViewer serviceAccount:deployment-publisher@example.iam.gserviceaccount.com"
			name:    "state-reader"
			bucket:  "oakmont-tf-dev"
			role:    "roles/storage.objectViewer"
			member:  "serviceAccount:deployment-publisher@example.iam.gserviceaccount.com"
		}}

		assert: input.out.resource.google_storage_bucket_iam_member["state-reader"].#import == "b/oakmont-tf-dev roles/storage.objectViewer serviceAccount:deployment-publisher@example.iam.gserviceaccount.com"
	}
}

bucketIamMemberResult: [for _, test in #BucketIamMemberTests {test.assert & true}]
