package google

import T "github.com/medulla-sh/cuet"

#CloudIdentityMembershipRole: "MEMBER" | "MANAGER" | "OWNER"

#CloudIdentityGroup: {
	in: {
		#providerAlias?: string
		#import?: {
			group?: string
			memberships?: [string]: string
		}

		name:       string & !=""
		customerId: string & !=""
		email:      string & !=""

		displayName:  string & !=""
		description?: string

		security: bool
		security: _ | *false

		initialGroupConfig: "WITH_INITIAL_OWNER" | "EMPTY"
		initialGroupConfig: _ | *"EMPTY"

		deletionPolicy: "PREVENT" | "ABANDON" | "DELETE"
		deletionPolicy: _ | *"PREVENT"

		memberships: [string]: {
			email: string & !=""
			role:  #CloudIdentityMembershipRole
			role:  _ | *"MEMBER"
		}
	}

	refs: {
		group: "google_cloud_identity_group.\(in.name)"
		memberships: {
			for name, _ in in.memberships {
				(name): "google_cloud_identity_group_membership.\(in.name)-\(name)"
			}
		}
	}

	out: T.#TerraformInput & {
		resource: google_cloud_identity_group: (in.name): {
			if in.#providerAlias != _|_ {
				#providerAlias: in.#providerAlias
			}
			if in.#import.group != _|_ {
				#import: in.#import.group
			}

			parent: "customers/\(in.customerId)"
			group_key: [{id: in.email}]
			labels: {
				"cloudidentity.googleapis.com/groups.discussion_forum": ""
				if in.security {
					"cloudidentity.googleapis.com/groups.security": ""
				}
			}

			display_name:         in.displayName
			initial_group_config: in.initialGroupConfig
			deletion_policy:      in.deletionPolicy

			if in.description != _|_ {
				description: in.description
			}
		}

		for name, membership in in.memberships {
			resource: google_cloud_identity_group_membership: ("\(in.name)-\(name)"): {
				if in.#providerAlias != _|_ {
					#providerAlias: in.#providerAlias
				}
				if in.#import.memberships[name] != _|_ {
					#import: in.#import.memberships[name]
				}

				group: "${\(refs.group).id}"
				preferred_member_key: [{id: membership.email}]
				roles: [
					{name: "MEMBER"},
					if membership.role != "MEMBER" {
						{name: membership.role}
					},
				]
			}
		}
	}
}
