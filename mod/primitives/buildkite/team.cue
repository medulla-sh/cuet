package buildkite

import T "github.com/medulla-sh/cuet"

#DataTeam: {
	in: {
		name: #TerraformName

		{id: string & !=""} | {slug: string & !=""}
	}

	ref: "data.buildkite_team.\(in.name)"

	out: T.#TerraformInput & {
		data: buildkite_team: (in.name): {
			if in.id != _|_ {
				id: in.id
			}
			if in.slug != _|_ {
				slug: in.slug
			}
		}
	}
}
