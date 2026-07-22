package github

import T "github.com/medulla-sh/cuet"

#DataOrganization: {
	in: {
		name: string & !=""

		summaryOnly: bool
		summaryOnly: _ | *true
	}

	ref: "data.github_organization.\(in.name)"

	out: T.#TerraformInput & {
		data: github_organization: (in.name): {
			name:         in.name
			summary_only: in.summaryOnly
		}
	}
}
