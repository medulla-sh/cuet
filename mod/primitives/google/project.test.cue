@if(test)

package google

import "encoding/json"

#ProjectTests: {
	for name, services in {omitted: {}, empty: {enabledServices: []}} {
		"\(name)-services": {
			input: #Project & {in: {name: "sso"} & services}

			assert: input.out.resource.google_project.sso.project_id == "sso"
			assert: [for resourceType, _ in input.out.resource {resourceType}] == ["google_project"]
			assert: len(input.refs.services) == 0
		}
	}

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
		assert: input.refs.services["connectgateway.googleapis.com"] == "google_project_service.internal-connectgateway-googleapis-com"
		assert: len(input.refs.services) == len(input.in.enabledServices)
	}

	"project policies": {
		let dev = #Project & {in: {
			name: "dev"
			enabledServices: ["orgpolicy.googleapis.com"]
			policies: "iam.disableServiceAccountKeyCreation": {
				name:    "dev-service-account-key-creation"
				enforce: false
			}
		}}
		let sandbox = #Project & {in: {
			name: "sandbox"
			enabledServices: ["orgpolicy.googleapis.com"]
			policies: "compute.vmExternalIpAccess": list: denyAll: true
		}}

		policies: (dev.out & sandbox.out).resource.google_org_policy_policy
		assert: policies["dev-service-account-key-creation"] == {
			name:   "projects/${google_project.dev.number}/policies/iam.disableServiceAccountKeyCreation"
			parent: "projects/${google_project.dev.number}"
			#dependsOn: ["google_project_service.dev-orgpolicy-googleapis-com"]
			spec: rules: enforce: "FALSE"
		}
		assert: policies["sandbox-compute-vmExternalIpAccess"] == {
			name:   "projects/${google_project.sandbox.number}/policies/compute.vmExternalIpAccess"
			parent: "projects/${google_project.sandbox.number}"
			#dependsOn: ["google_project_service.sandbox-orgpolicy-googleapis-com"]
			spec: rules: deny_all: "TRUE"
		}
	}

	"missing policy service": {
		let project = #Project & {in: {
			name: "sandbox"
			policies: "compute.skipDefaultNetworkCreation": enforce: true
		}}

		assert: json.Marshal(project.out) == _|_
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
