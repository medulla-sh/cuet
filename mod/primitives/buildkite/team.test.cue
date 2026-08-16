@if(test)

package buildkite

#DataTeamTests: {
	"slug": {
		input: #DataTeam & {in: {
			name: "owners"
			slug: "owners"
		}}

		assert: input.ref == "data.buildkite_team.owners"
		assert: input.out.data.buildkite_team.owners == {
			slug: "owners"
		}
	}

	"id": {
		input: #DataTeam & {in: {
			name: "owners"
			id:   "VGVhbS0tLW93bmVycw=="
		}}

		assert: input.out.data.buildkite_team.owners == {
			id: "VGVhbS0tLW93bmVycw=="
		}
	}
}

dataTeamResult: [for _, test in #DataTeamTests {test.assert & true}]
