@if(test)

package google

#ServiceAccountKeyTests: {
	"default key": {
		key: #ServiceAccountKey & {in: {
			name:             "signer"
			serviceAccountId: "${google_service_account.signer.name}"
		}}
		assert: key.ref == "google_service_account_key.signer" && key.out == {
			resource: google_service_account_key: signer: {
				service_account_id: "${google_service_account.signer.name}"
			}
		}
	}
	"rotation and framework controls": {
		key: #ServiceAccountKey & {in: {
			name:             "signer"
			serviceAccountId: "projects/example/serviceAccounts/signer@example.iam.gserviceaccount.com"
			keepers: rotation: "2026-10"
			#import: "projects/example/serviceAccounts/signer@example.iam.gserviceaccount.com/keys/key-id"
			#dependsOn: ["google_service_account.signer"]
		}}
		assert: key.out == {
			resource: google_service_account_key: signer: {
				service_account_id: "projects/example/serviceAccounts/signer@example.iam.gserviceaccount.com"
				keepers: rotation: "2026-10"
				#import: "projects/example/serviceAccounts/signer@example.iam.gserviceaccount.com/keys/key-id"
				#dependsOn: ["google_service_account.signer"]
			}
		}
	}
}

serviceAccountKeyResult: [for _, test in #ServiceAccountKeyTests {test.assert & true}]
