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
	"source-based-rule": {
		input: #Network & {in: {
			name: "internal"
			subnets: k8s: {
				region:       "us-west1"
				primaryRange: "nodes"
				cidrs: nodes: "10.200.0.0/22"
			}
			publicNats: "us-west1": "internal-us-west1": {
				#history: {
					router: ["internal-us-west1"]
					nat: ["internal-us-west1"]
				}
				subnets: k8s: {}
				addresses: {
					"internal-default": {}
					"internal-connector": {}
				}
				defaultAddresses: ["internal-default"]
				rules: [{
					ruleNumber: 100
					match:      "inIpRange(source.ip, '10.200.128.0/24')"
					activeAddresses: ["internal-connector"]
				}]
			}
		}}

		let resourceName = "us-west1__internal-us-west1"
		let natResource = input.out.resource.google_compute_router_nat[resourceName]

		assert: natResource.#history == ["internal-us-west1"]
		assert: input.out.resource.google_compute_router[resourceName].#history == ["internal-us-west1"]
		assert: natResource.nat_ip_allocate_option == "MANUAL_ONLY"
		assert: natResource.nat_ips == [
			"${google_compute_address.us-west1__internal-us-west1__internal-default.self_link}",
		]
		assert: natResource.subnetwork[0].source_ip_ranges_to_nat == ["ALL_IP_RANGES"]
		assert: natResource.rules == [{
			rule_number: 100
			match:       "inIpRange(source.ip, '10.200.128.0/24')"
			action: source_nat_active_ips: [
				"${google_compute_address.us-west1__internal-us-west1__internal-connector.self_link}",
			]
		}]
		assert: natResource.enable_endpoint_independent_mapping == false
		assert: input.out.resource.google_compute_address["\(resourceName)__internal-connector"].lifecycle.create_before_destroy == true
		assert: natResource.log_config == {
			enable: true
			filter: "ERRORS_ONLY"
		}
		assert: input.refs.publicNats["us-west1"]["internal-us-west1"].nat == "google_compute_router_nat.\(resourceName)"
	}

	"region-and-name-identity": {
		input: #Network & {in: {
			name: "global"
			subnets: {
				west: {
					region:       "us-west1"
					primaryRange: "primary"
					cidrs: primary: "10.0.0.0/24"
				}
				east: {
					region:       "us-east1"
					primaryRange: "primary"
					cidrs: primary: "10.0.1.0/24"
				}
			}
			publicNats: {
				"us-west1": default: subnets: west: {}
				"us-east1": default: subnets: east: {}
			}
		}}

		assert: input.refs.publicNats["us-west1"].default.nat == "google_compute_router_nat.us-west1__default"
		assert: input.refs.publicNats["us-east1"].default.nat == "google_compute_router_nat.us-east1__default"
	}

	"duplicate-subnet-rejected": {
		input: {
			name: "internal"
			subnets: k8s: {
				region:       "us-west1"
				primaryRange: "primary"
				cidrs: primary: "10.0.0.0/24"
			}
			publicNats: "us-west1": {
				first: subnets: k8s: {}
				second: subnets: k8s: {}
			}
		}

		assert: (#Network & {in: input}) == _|_
	}

	"invalid-secondary-range-rejected": {
		input: {
			name: "internal"
			subnets: k8s: {
				region:       "us-west1"
				primaryRange: "primary"
				cidrs: primary: "10.0.0.0/24"
			}
			publicNats: "us-west1": default: subnets: k8s: {
				sourceIpRanges: ["list-of-secondary-ip-ranges"]
				secondaryRangeNames: ["missing"]
			}
		}

		assert: (#Network & {in: input}) == _|_
	}

	"dynamic-port-allocation-boundaries": {
		input: #Network & {in: {
			name: "internal"
			subnets: k8s: {
				region:       "us-west1"
				primaryRange: "primary"
				cidrs: primary: "10.0.0.0/24"
			}
			publicNats: "us-west1": default: {
				subnets: k8s: {}
				minPortsPerVm:               32
				enableDynamicPortAllocation: true
				maxPortsPerVm:               65536
			}
		}}

		let nat = input.out.resource.google_compute_router_nat["us-west1__default"]

		assert: nat.min_ports_per_vm == 32
		assert: nat.max_ports_per_vm == 65536
	}

	"non-power-port-counts-rejected": {
		input: {
			name: "internal"
			subnets: k8s: {
				region:       "us-west1"
				primaryRange: "primary"
				cidrs: primary: "10.0.0.0/24"
			}
			publicNats: "us-west1": default: {
				subnets: k8s: {}
				minPortsPerVm:               64
				enableDynamicPortAllocation: true
				maxPortsPerVm:               96
			}
		}

		assert: (#Network & {in: input}) == _|_
	}
}

networkResult: [
	for _, test in #NetworkTests {test.assert & true},
]
