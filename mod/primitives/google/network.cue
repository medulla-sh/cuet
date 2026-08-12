package google

import (
	"net"
	"strings"
)

#RFC1035Name: =~"^[a-z]([-a-z0-9]*[a-z0-9])?$"

#Network: {
	in: {
		#import?: {
			network?: string
			subnets?: [string]: string
			privateServiceAccess?: [string]: {
				address?:    string
				connection?: string
			}
		}

		name: #RFC1035Name

		autoCreateSubnets: bool
		autoCreateSubnets: _ | *false

		routingMode: "regional" | "global"
		routingMode: _ | *"global"

		mtu?: int & >0

		privateServiceAccess: [#RFC1035Name]: {
			cidr:    net.IPCIDR
			service: string
			service: _ | *"servicenetworking.googleapis.com"
		}

		subnets: [#RFC1035Name]: {
			region: #Region

			primaryRange: #RFC1035Name
			primaryRange: or([for k, _ in cidrs {k}])

			cidrs: [#RFC1035Name]: net.IPCIDR

			egress: {
				privateIpGoogleAccess: bool
				privateIpGoogleAccess: _ | *true

				nat: bool
				nat: _ | *false
			}
		}
	}

	let subnetsByRegionsWithNat = {
		for subnetName, subnet in in.subnets if subnet.egress.nat {
			(subnet.region): {
				routerName: "\(in.name)-\(subnet.region)"
				natName:    "\(in.name)-\(subnet.region)"
				subnets: (subnetName): subnet
			}
		}
	}
	let subnetCidrs = [
		for _, subnet in in.subnets
		for _, cidr in subnet.cidrs {
			value:  cidr
			parsed: net.ParseCIDR(cidr)
		},
	]
	let privateServiceAccessCidrs = [
		for _, config in in.privateServiceAccess {
			value:  config.cidr
			parsed: net.ParseCIDR(config.cidr)
		},
	]
	for privateCidr in privateServiceAccessCidrs
	for subnetCidr in subnetCidrs {
		if net.InCIDR(privateCidr.parsed.prefix_addr, subnetCidr.value) ||
			net.InCIDR(subnetCidr.parsed.prefix_addr, privateCidr.value) {
			_|_("private service access CIDRs must not overlap subnet CIDRs")
		}
	}

	refs: {
		network: "google_compute_network.\(in.name)"
		subnets: {
			for name, _ in in.subnets {
				(name): "google_compute_subnetwork.\(name)"
			}
		}
		routers: {
			for region, data in subnetsByRegionsWithNat {
				(region): "google_compute_router.\(data.routerName)"
			}
		}
		nats: {
			for region, data in subnetsByRegionsWithNat {
				(region): "google_compute_router_nat.\(data.natName)"
			}
		}
		privateServiceAccess: {
			for name, _ in in.privateServiceAccess {
				(name): {
					address:    "google_compute_global_address.\(name)"
					connection: "google_service_networking_connection.\(name)"
				}
			}
		}
	}

	out: {
		resource: google_compute_network: (in.name): {
			if in.#import.network != _|_ {
				#import: in.#import.network
			}

			name:                    in.name
			auto_create_subnetworks: in.autoCreateSubnets

			routing_mode: strings.ToUpper(in.routingMode)

			if in.mtu != _|_ {
				mtu: in.mtu
			}
		}

		for subnetName, subnet in in.subnets {
			resource: google_compute_subnetwork: (subnetName): {
				if in.#import.subnets[subnetName] != _|_ {
					#import: in.#import.subnets[subnetName]
				}

				name: subnetName

				network: "${\(refs.network).id}"

				region: subnet.region

				private_ip_google_access: subnet.egress.privateIpGoogleAccess

				ip_cidr_range: subnet.cidrs[subnet.primaryRange]

				secondary_ip_range: [
					for name, cidr in subnet.cidrs if name != subnet.primaryRange {
						range_name:    name
						ip_cidr_range: cidr
					},
				]
			}
		}

		for name, config in in.privateServiceAccess {
			let cidr = net.ParseCIDR(config.cidr)
			resource: google_compute_global_address: (name): {
				if in.#import.privateServiceAccess[name].address != _|_ {
					#import: in.#import.privateServiceAccess[name].address
				}

				"name":        name
				purpose:       "VPC_PEERING"
				address_type:  "INTERNAL"
				address:       cidr.prefix_addr
				prefix_length: cidr.prefix_len
				network:       "${\(refs.network).id}"
			}

			resource: google_service_networking_connection: (name): {
				if in.#import.privateServiceAccess[name].connection != _|_ {
					#import: in.#import.privateServiceAccess[name].connection
				}

				network: "${\(refs.network).id}"
				service: config.service
				reserved_peering_ranges: ["${\(refs.privateServiceAccess[name].address).name}"]
			}
		}

		for region, data in subnetsByRegionsWithNat {
			resource: google_compute_router: (data.routerName): {
				name:     data.routerName
				network:  "${\(refs.network).id}"
				"region": region
			}

			resource: google_compute_router_nat: (data.natName): {
				name:     data.natName
				"region": region
				router:   "${\(refs.routers[region]).name}"

				nat_ip_allocate_option: "AUTO_ONLY"

				source_subnetwork_ip_ranges_to_nat: "LIST_OF_SUBNETWORKS"
				subnetwork: [
					for subnetName, _ in data.subnets {
						name: "${\(refs.subnets[subnetName]).name}"
						source_ip_ranges_to_nat: ["ALL_IP_RANGES"]
					},
				]
			}
		}
	}
}
