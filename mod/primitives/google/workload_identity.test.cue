@if(test)

package google

#WorkloadIdentityTests: {
	"aws-provider": {
		input: #WorkloadIdentityProvider & {in: {
			name: "vanta-aws"
			project: {
				name: "vanta"
				id:   "vanta-scanner"
			}
			poolId: "${google_iam_workload_identity_pool.vanta-pool.workload_identity_pool_id}"
			attributeMapping: {
				"google.subject": "'vanta-scanner'"
				"attribute.arn":  "assertion.arn"
			}
			attributeCondition: "attribute.arn.extract('assumed-role/{role}/') == 'scanner'"
			aws: accountId: "123456789012"
		}}

		assert: input.ref == "google_iam_workload_identity_pool_provider.vanta-aws"
		assert: input.out.resource.google_iam_workload_identity_pool_provider["vanta-aws"].aws == {
			account_id: "123456789012"
		}
	}

	"oidc-provider": {
		input: #WorkloadIdentityProvider & {in: {
			name: "github-main"
			project: {
				name: "internal"
				id:   "oakmont-internal"
			}
			poolId: "${google_iam_workload_identity_pool.github.workload_identity_pool_id}"
			attributeMapping: {
				"google.subject":       "assertion.sub"
				"attribute.repository": "assertion.repository"
			}
			attributeCondition: "attribute.repository == 'oakmont-health/oakmont'"
			oidc: {
				issuerUri: "https://token.actions.githubusercontent.com"
				allowedAudiences: ["https://github.com/oakmont-health"]
			}
		}}

		assert: input.out.resource.google_iam_workload_identity_pool_provider["github-main"].oidc == {
			issuer_uri: "https://token.actions.githubusercontent.com"
			allowed_audiences: ["https://github.com/oakmont-health"]
		}
	}
}

workloadIdentityResult: [for _, test in #WorkloadIdentityTests {test.assert & true}]
