@if(test)

package google

#OrgPolicyTests: {
	for scope in ["projects", "folders", "organizations"] {
		for setting in [{input: true, output: "TRUE"}, {input: false, output: "FALSE"}] {
			"\(scope)-\(setting.output)": {
				policy: #OrgPolicy & {in: {
					name:       "key-creation"
					parent:     "\(scope)/123"
					constraint: "iam.disableServiceAccountKeyCreation"
					enforce:    setting.input
				}}
				assert: policy.ref == "google_org_policy_policy.key-creation" && policy.out == {
					resource: google_org_policy_policy: "key-creation": {
						name:   "\(scope)/123/policies/iam.disableServiceAccountKeyCreation"
						parent: "\(scope)/123"
						spec: rules: enforce: setting.output
					}
				}
			}
		}
	}
	"framework controls": {
		policy: #OrgPolicy & {in: {
			name:       "key-creation"
			parent:     "projects/123"
			constraint: "iam.disableServiceAccountKeyCreation"
			enforce:    false
			#import:    "projects/123/policies/iam.disableServiceAccountKeyCreation"
			#dependsOn: ["google_project_service.orgpolicy"]
		}}
		assert: policy.out.resource.google_org_policy_policy["key-creation"].#import == policy.in.#import && policy.out.resource.google_org_policy_policy["key-creation"].#dependsOn == policy.in.#dependsOn
	}
}

orgPolicyResult: [for _, test in #OrgPolicyTests {test.assert & true}]
