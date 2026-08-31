@if(test)

package google

#ProjectTests: {
	"enabled-services": {
		input: #Project & {in: {
			name:                     "internal"
			orgId:                    "123456789"
			deletionPolicy:           "ABANDON"
			disableServicesOnDestroy: false
			enabledServices: [
				"cloudresourcemanager.googleapis.com",
				"connectgateway.googleapis.com",
				"gkeconnect.googleapis.com",
			]
		}}

		assert: input.out.resource.google_project.internal.deletion_policy == "ABANDON"
		assert: input.out.resource.google_project_service["internal-connectgateway-googleapis-com"].service == "connectgateway.googleapis.com"
		assert: input.out.resource.google_project_service["internal-connectgateway-googleapis-com"].disable_on_destroy == false
	}

	"service-identity": {
		input: #ProjectServiceIdentity & {in: {
			name:    "internal_pubsub"
			service: "pubsub.googleapis.com"
			project: {
				name: "internal"
				id:   "oakmont-internal"
			}
		}}

		assert: input.out.resource.google_project_service_identity.internal_pubsub.#provider == "google-beta"
		assert: input.out.resource.google_project_service_identity.internal_pubsub == {
			#provider: "google-beta"
			project:   "${data.google_project.internal.project_id}"
			service:   "pubsub.googleapis.com"
		}
	}
}

projectResult: [for _, test in #ProjectTests {test.assert & true}]
