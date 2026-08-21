@if(test)

package github

#IssueLabelTests: {
	"renders-label": {
		input: #IssueLabel & {in: {
			#import:     "oakmont:merge-ready"
			repository:  "${github_repository.oakmont.name}"
			name:        "merge-ready"
			color:       "0e8A16"
			description: "Ready for the external merge queue"
		}}

		let label = input.out.resource.github_issue_label["merge-ready"]

		assert: input.ref == "github_issue_label.merge-ready"
		assert: label.#import == "oakmont:merge-ready"
		assert: label.repository == "${github_repository.oakmont.name}"
		assert: label.name == "merge-ready"
		assert: label.color == "0e8A16"
		assert: label.description == "Ready for the external merge queue"
	}

	"omits-optional-values": {
		input: #IssueLabel & {in: {
			repository: "oakmont"
			name:       "triage"
			color:      "FFFFFF"
		}}

		let label = input.out.resource.github_issue_label.triage

		assert: label.#import == _|_
		assert: label.description == _|_
	}

	"supports-distinct-resource-name": {
		input: #IssueLabel & {in: {
			repository:   "oakmont"
			name:         "priority: high"
			resourceName: "priority-high"
			color:        "B60205"
		}}

		let label = input.out.resource.github_issue_label["priority-high"]

		assert: input.ref == "github_issue_label.priority-high"
		assert: label.name == "priority: high"
	}

	"rejects-invalid-color": {
		input: {
			repository: "oakmont"
			name:       "triage"
			color:      "#ffffff"
		}

		assert: (#IssueLabel & {in: input}) == _|_
	}
}

issueLabelResult: [for _, test in #IssueLabelTests {test.assert & true}]
