package github

import T "github.com/medulla-sh/cuet"

#IssueLabel: {
	in: {
		// Adopts an existing label using a <repository>:<label> identifier.
		#import?: string

		// Selects the repository by name or Terraform expression.
		repository: string & !=""

		// Sets the GitHub label name.
		name: string & !=""

		// Provides a Terraform-safe resource key when the label name is not one.
		resourceName: =~"^[A-Za-z_][A-Za-z0-9_-]*$"
		resourceName: _ | *name

		// Sets the label color as six hexadecimal digits without a leading #.
		color: =~"^[0-9A-Fa-f]{6}$"

		// Explains how the label is used.
		description?: string
	}

	ref: "github_issue_label.\(in.resourceName)"

	out: T.#TerraformInput & {
		resource: github_issue_label: (in.resourceName): {
			if in.#import != _|_ {
				#import: in.#import
			}

			"repository": in.repository
			name:         in.name
			color:        in.color

			if in.description != _|_ {
				description: in.description
			}
		}
	}
}
