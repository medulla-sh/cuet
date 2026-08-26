@if(test)

package google

#CloudIdentityGroupTests: {
	"defaults": {
		input: #CloudIdentityGroup & {in: {
			name:        "engineering"
			customerId:  "C01234567"
			email:       "engineering@example.com"
			displayName: "Engineering"
			memberships: {}
		}}

		assert: input.refs.group == "google_cloud_identity_group.engineering"
		assert: input.refs.memberships == {}
		assert: input.out.resource.google_cloud_identity_group.engineering == {
			parent: "customers/C01234567"
			group_key: [{id: "engineering@example.com"}]
			labels: {
				"cloudidentity.googleapis.com/groups.discussion_forum": ""
			}
			lifecycle: ignore_changes: ["initial_group_config"]
			display_name:    "Engineering"
			deletion_policy: "PREVENT"
		}
	}

	"imports security group and memberships": {
		input: #CloudIdentityGroup & {in: {
			#providerAlias: "directory"
			#import: {
				group: "groups/0123456789"
				memberships: {
					user:  "groups/0123456789/memberships/1111111111"
					owner: "groups/0123456789/memberships/2222222222"
				}
			}
			name:        "security"
			customerId:  "C01234567"
			email:       "security@example.com"
			displayName: "Security"
			description: "Security notifications"
			security:    true
			memberships: {
				user: email: "user@example.com"
				owner: {
					email: "owner@example.com"
					role:  "OWNER"
				}
			}
		}}

		assert: input.refs.memberships.user == "google_cloud_identity_group_membership.security-user"
		assert: input.out.resource.google_cloud_identity_group.security == {
			#providerAlias: "directory"
			#import:        "groups/0123456789"
			parent:         "customers/C01234567"
			group_key: [{id: "security@example.com"}]
			labels: {
				"cloudidentity.googleapis.com/groups.discussion_forum": ""
				"cloudidentity.googleapis.com/groups.security":         ""
			}
			lifecycle: ignore_changes: ["initial_group_config"]
			display_name:    "Security"
			description:     "Security notifications"
			deletion_policy: "PREVENT"
		}
		assert: input.out.resource.google_cloud_identity_group_membership["security-user"] == {
			#providerAlias: "directory"
			#import:        "groups/0123456789/memberships/1111111111"
			group:          "groups/0123456789"
			preferred_member_key: [{id: "user@example.com"}]
			roles: [{name: "MEMBER"}]
		}
		assert: input.out.resource.google_cloud_identity_group_membership["security-owner"] == {
			#providerAlias: "directory"
			#import:        "groups/0123456789/memberships/2222222222"
			group:          "groups/0123456789"
			preferred_member_key: [{id: "owner@example.com"}]
			roles: [{name: "MEMBER"}, {name: "OWNER"}]
		}
	}
}

cloudIdentityGroupResult: [for _, test in #CloudIdentityGroupTests {test.assert & true}]
