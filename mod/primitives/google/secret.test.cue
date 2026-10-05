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

#SecretVersionTests: {
	let base = {project: "example-project", secretId: "example-secret"}
	"ephemeral-by-default": {
		input: #SecretVersion & {in: base}

		assert: input.ref == "ephemeral.google_secret_manager_secret_version.example-secret"
		assert: input.out.ephemeral.google_secret_manager_secret_version[base.secretId] == {
			project: base.project
			secret:  base.secretId
			version: "latest"
		}
		assert: input.out.data == _|_
	}
	"persisted-read-with-explicit-name-and-version": {
		input: #SecretVersion & {in: base & {
			name:      "replicated"
			version:   "3"
			ephemeral: false
		}}

		assert: input.ref == "data.google_secret_manager_secret_version.replicated"
		assert: input.out.data.google_secret_manager_secret_version.replicated == {
			project: base.project
			secret:  base.secretId
			version: "3"
		}
		assert: input.out.ephemeral == _|_
	}
}

secretVersionResult: [for _, test in #SecretVersionTests {test.assert & true}]
