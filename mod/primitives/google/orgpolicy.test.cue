@if(test)

package google

import "encoding/json"

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
	"deny all": {
		policy: #OrgPolicy & {in: {
			parent:     "organizations/123"
			constraint: "compute.vmExternalIpAccess"
			list: denyAll: true
		}}

		assert: policy.ref == "google_org_policy_policy.compute-vmExternalIpAccess"
		assert: policy.out.resource.google_org_policy_policy["compute-vmExternalIpAccess"].spec.rules == {
			deny_all: "TRUE"
		}
	}
	for mode, settings in {
		allow: {allowedValues: ["is:sandbox"]}
		deny: {deniedValues: ["is:restricted"]}
		both: allow & deny
	} {
		"list values \(mode)": {
			policy: #OrgPolicy & {in: {
				parent:     "folders/123"
				constraint: "gcp.resourceLocations"
				list:       settings
			}}

			assert: policy.out.resource.google_org_policy_policy["gcp-resourceLocations"].spec.rules == {
				values: {
					if mode != "deny" {
						allowed_values: ["is:sandbox"]
					}
					if mode != "allow" {
						denied_values: ["is:restricted"]
					}
				}
			}
		}
	}
	for index, invalidConfig in [
		{},
		{enforce: false, list: denyAll: true},
		{list: {denyAll: true, allowedValues: ["test"]}},
		{list: allowedValues: []},
	] {
		"invalid \(index)": {
			let policy = #OrgPolicy & {in: {
				parent:     "organizations/123"
				constraint: "compute.vmExternalIpAccess"
			} & invalidConfig
			}

			assert: json.Marshal(policy.out) == _|_
		}
	}
}

orgPolicyResult: [for _, test in #OrgPolicyTests {test.assert & true}]
