@if(test)

package google

#CloudIdentityGroupTests: {
	"defaults": {
		input: #CloudIdentityGroup & {in: {
			name:        "engineering"
			domain:      "example.com"
			displayName: "Engineering"
			memberships: {}
		}}

		assert: input.refs.group == "google_cloud_identity_group.engineering"
		assert: input.refs.memberships == {}
		assert: input.out.resource.google_cloud_identity_group.engineering == {
			parent: "customers/${data.google_organization.cloud-identity-example-com-default.directory_customer_id}"
			group_key: [{id: "engineering@example.com"}]
			labels: {
				"cloudidentity.googleapis.com/groups.discussion_forum": ""
			}
			lifecycle: ignore_changes: ["initial_group_config"]
			display_name:    "Engineering"
			deletion_policy: "PREVENT"
		}
		assert: input.out.data.google_organization["cloud-identity-example-com-default"] == {
			domain: "example.com"
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
			domain:      "example.com"
			displayName: "Security"
			description: "Security notifications"
			security:    true
			memberships: {
				user: _
				owner: role: "owner"
			}
		}}

		assert: input.refs.memberships.user == "google_cloud_identity_group_membership.security-user"
		assert: input.out.resource.google_cloud_identity_group.security == {
			#providerAlias: "directory"
			#import:        "groups/0123456789"
			parent:         "customers/${data.google_organization.cloud-identity-example-com-directory.directory_customer_id}"
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
		assert: input.out.data.google_organization["cloud-identity-example-com-directory"] == {
			#providerAlias: "directory"
			domain:         "example.com"
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

	"derives name and preserves external member domain": {
		input: #CloudIdentityGroup & {in: {
			email:       "engineering@example.com"
			displayName: "Engineering"
			memberships: {
				"contractor@vendor.example": role: "manager"
			}
		}}

		assert: input.refs.group == "google_cloud_identity_group.engineering"
		assert: input.refs.memberships["contractor@vendor.example"] == "google_cloud_identity_group_membership.engineering-contractor-vendor-example"
		assert: input.out.resource.google_cloud_identity_group_membership["engineering-contractor-vendor-example"] == {
			group: "${google_cloud_identity_group.engineering.id}"
			preferred_member_key: [{id: "contractor@vendor.example"}]
			roles: [{name: "MEMBER"}, {name: "MANAGER"}]
		}
	}

	"derives name and full email from local email": {
		input: #CloudIdentityGroup & {in: {
			email:       "operations"
			domain:      "example.com"
			displayName: "Operations"
			memberships: {}
		}}

		assert: input.refs.group == "google_cloud_identity_group.operations"
		assert: input.out.resource.google_cloud_identity_group.operations.group_key == [{id: "operations@example.com"}]
	}
}

cloudIdentityGroupResult: [for _, test in #CloudIdentityGroupTests {test.assert & true}]
