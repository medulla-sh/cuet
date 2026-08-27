package google

import (
	"strings"

	T "github.com/medulla-sh/cuet"
)

#CloudIdentityEmail:          string & =~"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+$"
#CloudIdentityEmailLocalPart: string & =~"^[A-Za-z0-9._%+-]+$"

#CloudIdentityMembershipRole: "member" | "manager" | "owner"

#CloudIdentityGroup: {
	in: {
		#providerAlias?: string
		#import?: {
			group?: string
			memberships?: [string]: string
		}

		let emailParts = strings.SplitN(*email | "", "@", 2)
		name:       string & =~"^[A-Za-z_][A-Za-z0-9_-]*$"
		name:       _ | *emailParts[0]
		email:      #CloudIdentityEmail | #CloudIdentityEmailLocalPart
		email:      _ | *"\(name)@\(domain)"
		domain:     string & !="" & !~"@"
		domain:     _ | *emailParts[1]
		customerId: string & !=""

		displayName:  string & !=""
		description?: string

		security: bool
		security: _ | *false

		deletionPolicy: "PREVENT" | "ABANDON" | "DELETE"
		deletionPolicy: _ | *"PREVENT"

		memberships: [string]: {
			role: #CloudIdentityMembershipRole
			role: _ | *"member"
		}
	}
	let groupEmail = ([
		if strings.Contains(in.email, "@") {in.email},
		if !strings.Contains(in.email, "@") {"\(in.email)@\(in.domain)"},
	][0] & #CloudIdentityEmail)
	#domainMatchesEmail: in.domain & strings.SplitN(groupEmail, "@", 2)[1]
	let memberResourceNames = {
		for member, _ in in.memberships {
			(member): [
				if !strings.Contains(member, "@") {member},
				if strings.Contains(member, "@") {strings.Replace(strings.Replace(strings.Replace(strings.Replace(member, "@", "-", -1), ".", "-", -1), "+", "-", -1), "%", "-", -1)},
			][0]
		}
	}

	refs: {
		group: "google_cloud_identity_group.\(in.name)"
		memberships: {
			for member, _ in in.memberships {
				(member): "google_cloud_identity_group_membership.\(in.name)-\(memberResourceNames[member])"
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
			group_key: [{id: groupEmail}]
			labels: {
				"cloudidentity.googleapis.com/groups.discussion_forum": ""
				if in.security {
					"cloudidentity.googleapis.com/groups.security": ""
				}
			}
			// The provider injects its create-only default during import.
			lifecycle: ignore_changes: ["initial_group_config"]

			display_name:    in.displayName
			deletion_policy: in.deletionPolicy

			if in.description != _|_ {
				description: in.description
			}
		}

		for member, membership in in.memberships {
			let memberEmail = ([
				if !strings.Contains(member, "@") {"\(member)@\(in.domain)"},
				if strings.Contains(member, "@") {member},
			][0] & #CloudIdentityEmail)
			resource: google_cloud_identity_group_membership: ("\(in.name)-\(memberResourceNames[member])"): {
				if in.#providerAlias != _|_ {
					#providerAlias: in.#providerAlias
				}
				if in.#import.memberships[member] != _|_ {
					#import: in.#import.memberships[member]
				}

				group: [
					if in.#import.memberships[member] != _|_ {in.#import.group},
					"${\(refs.group).id}",
				][0]
				preferred_member_key: [{id: memberEmail}]
				roles: [
					{name: "MEMBER"},
					if membership.role != "member" {
						{name: strings.ToUpper(membership.role)}
					},
				]
			}
		}
	}
}
