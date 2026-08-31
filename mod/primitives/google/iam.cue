package google

import (
	"regexp"

	T "github.com/medulla-sh/cuet"
)

#OrgIamCustomRole: {
	in: {
		#import?: string

		name:   string
		orgId?: string & !=""

		roleId: string & !=""
		roleId: _ | *name

		title: string & !=""
		title: _ | *name

		description?: string

		permissions: [...string]
		permissions: [_, ...]
	}

	ref: "google_organization_iam_custom_role.\(in.name)"

	out: this=T.#TerraformInput & {
		let orgId = [
			if in.orgId != _|_ {in.orgId},
			if in.orgId == _|_ {"${data.google_project.\(this.#envName).org_id}"},
		][0]

		if in.orgId == _|_ {
			data: google_project: (this.#envName): {}
		}

		resource: google_organization_iam_custom_role: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			org_id:      orgId
			role_id:     in.roleId
			title:       in.title
			permissions: in.permissions

			if in.description != _|_ {
				description: in.description
			}
		}
	}
}

#OrgIamMember: {
	in: {
		#import?: string

		name: string
		name: _ | *regexp.ReplaceAll("[^[:alnum:]_-]", "\(in.role)-\(in.member)", "-")

		orgId?: string & !=""
		role:   string & !=""
		member: string & !=""
	}

	ref: "google_organization_iam_member.\(in.name)"

	out: this=T.#TerraformInput & {
		let orgId = [
			if in.orgId != _|_ {in.orgId},
			if in.orgId == _|_ {"${data.google_project.\(this.#envName).org_id}"},
		][0]

		if in.orgId == _|_ {
			data: google_project: (this.#envName): {}
		}

		resource: google_organization_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			org_id: orgId
			role:   in.role
			member: in.member
		}
	}
}

#IamMember: {
	in: {
		#import?: string

		name: string
		name: _ | *regexp.ReplaceAll("[^[:alnum:]_-]", "\(in.role)-\(in.member)", "-")
		project?: {
			name: string
			name: _ | *id
			id:   string
		}
		role:   string
		member: string
	}

	ref: "google_project_iam_member.\(in.name)"

	out: this=T.#TerraformInput & {
		let projectDataName = [
			if in.project != _|_ {in.project.name},
			this.#envName,
		][0]

		data: google_project: (projectDataName): {
			if in.project != _|_ {
				project_id: in.project.id
			}
		}

		resource: google_project_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			project: "${data.google_project.\(projectDataName).project_id}"

			role:   in.role
			member: in.member
			...
		}
	}
}

#BucketIamMember: {
	in: {
		#import?: string

		name:   string
		bucket: string
		role:   string
		member: string
	}

	ref: "google_storage_bucket_iam_member.\(in.name)"

	out: T.#TerraformInput & {
		resource: google_storage_bucket_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			bucket: in.bucket
			role:   in.role
			member: in.member
		}
	}
}

#ServiceAccountIamMember: {
	in: {
		#import?: string

		name: string

		serviceAccountId: string
		role:             string
		member:           string
	}

	ref: "google_service_account_iam_member.\(in.name)"

	out: T.#TerraformInput & {
		resource: google_service_account_iam_member: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			service_account_id: in.serviceAccountId
			role:               in.role
			member:             in.member
		}
	}
}
