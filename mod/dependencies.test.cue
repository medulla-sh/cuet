@if(test)

package cuet

import "encoding/json"

#DependencyTests: {
	"absent": {
		block: {name: "example"}
		expected: {name: "example"}
	}
	"empty": {
		block: {name: "example", #dependsOn: []}
		expected: {name: "example"}
	}
	"multiple": {
		block: {#dependsOn: ["google_project_service.iam", "google_project.example"]}
		expected: {depends_on: ["google_project.example", "google_project_service.iam"]}
	}
	"combined": {
		block: {
			#dependsOn: ["google_project_service.iam", "google_project.example"]
			depends_on: ["google_project_service.sts", "google_project_service.iam"]
		}
		expected: {depends_on: ["google_project.example", "google_project_service.iam", "google_project_service.sts"]}
	}
	"raw-only": {
		block: {depends_on: ["google_project_service.iam", "google_project.example"]}
		expected: {depends_on: ["google_project_service.iam", "google_project.example"]}
	}
	"raw-empty": {
		block: {#dependsOn: [], depends_on: []}
		expected: {depends_on: []}
	}
	"provider-alias": {
		block: {#dependsOn: ["google_project.example"], #providerAlias: "directory"}
		expected: {depends_on: ["google_project.example"], provider: "google.directory"}
	}
}

dependencyResults: [
	for _, test in #DependencyTests
	for blockType in ["resource", "data", "ephemeral"] {
		let rendered = (_#RenderTerraformInput & {in: {
			(blockType): google_service_account: example: test.block
		}}).out
		(json.Unmarshal(json.Marshal(rendered[blockType].google_service_account.example)) == test.expected) & true
	},
]
