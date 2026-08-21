@if(test)

package google

#SecretTests: {
	"renders-accessors": {
		input: #Secret & {in: {
			secretId: "example"
			accessors: deployment: "serviceAccount:deployment@example.iam.gserviceaccount.com"
		}}

		let member = input.out.resource.google_secret_manager_secret_iam_member["example-deployment-accessor"]

		assert: member.secret_id == "${google_secret_manager_secret.example.id}"
		assert: member.role == "roles/secretmanager.secretAccessor"
		assert: member.member == "serviceAccount:deployment@example.iam.gserviceaccount.com"
	}

	"omits-empty-accessors": {
		input: #Secret & {in: secretId: "example"}

		assert: input.out.resource.google_secret_manager_secret_iam_member == _|_
	}

	"retains-version": {
		input: #Secret & {in: {
			secretId:              "node-init"
			value:                 "#!/bin/sh"
			versionDeletionPolicy: "ABANDON"
		}}

		assert: input.out.resource.google_secret_manager_secret_version["node-init"].deletion_policy == "ABANDON"
	}
}

secretResult: [for _, test in #SecretTests {test.assert & true}]
