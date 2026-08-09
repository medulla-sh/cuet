@if(test)

package google

#ProjectTests: {
	"enabled-services": {
		input: #Project & {in: {
			name:  "internal"
			orgId: "123456789"
			enabledServices: [
				"cloudresourcemanager.googleapis.com",
				"connectgateway.googleapis.com",
				"gkeconnect.googleapis.com",
			]
		}}

		assert: input.out.resource.google_project_service["internal-connectgateway-googleapis-com"].service == "connectgateway.googleapis.com"
	}
}

projectResult: [for _, test in #ProjectTests {test.assert & true}]
