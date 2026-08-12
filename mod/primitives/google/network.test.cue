@if(test)

package google

#NetworkTests: {
	"private-service-access": {
		input: #Network & {in: {
			#import: {
				network: "projects/example/global/networks/dev"
				privateServiceAccess: "google-services": {
					address: "projects/example/global/addresses/google-services"
				}
			}
			name: "dev"
			privateServiceAccess: "google-services": {
				cidr: "10.250.0.0/16"
			}
		}}

		assert: input.out.resource.google_compute_network.dev.#import == "projects/example/global/networks/dev"
		assert: input.out.resource.google_compute_global_address["google-services"] == {
			#import:       "projects/example/global/addresses/google-services"
			name:          "google-services"
			purpose:       "VPC_PEERING"
			address_type:  "INTERNAL"
			address:       "10.250.0.0"
			prefix_length: 16
			network:       "${google_compute_network.dev.id}"
		}
		assert: input.out.resource.google_service_networking_connection["google-services"] == {
			network: "${google_compute_network.dev.id}"
			service: "servicenetworking.googleapis.com"
			reserved_peering_ranges: ["${google_compute_global_address.google-services.name}"]
		}
		assert: input.refs.privateServiceAccess["google-services"] == {
			address:    "google_compute_global_address.google-services"
			connection: "google_service_networking_connection.google-services"
		}
	}

	"non-overlapping-ranges": {
		input: #Network & {in: {
			name: "dev"
			privateServiceAccess: "google-services": {
				cidr: "10.250.0.0/16"
			}
			subnets: k8s: {
				region:       "us-west1"
				primaryRange: "nodes"
				cidrs: nodes: "10.100.0.0/22"
			}
		}}

		assert: input.out.resource.google_compute_global_address["google-services"].address == "10.250.0.0"
	}
}

networkResult: [for _, test in #NetworkTests {test.assert & true}]
