package google

import (
	"list"
	"math/bits"
	"net"
	"strings"
)

#RFC1035Name: =~"^[a-z]([-a-z0-9]*[a-z0-9])?$"

#PublicNatSourceRange:
	"all-ip-ranges" |
	"primary-ip-range" |
	"list-of-secondary-ip-ranges"

#PublicNatSourceRangeMap: {
	"all-ip-ranges":               "ALL_IP_RANGES"
	"primary-ip-range":            "PRIMARY_IP_RANGE"
	"list-of-secondary-ip-ranges": "LIST_OF_SECONDARY_IP_RANGES"
}

#NetworkTier: "premium" | "standard"

#NetworkTierMap: {
	premium:  "PREMIUM"
	standard: "STANDARD"
}

#PublicNatLogFilter: "all" | "errors-only" | "translations-only"

#PublicNatLogFilterMap: {
	all:                 "ALL"
	"errors-only":       "ERRORS_ONLY"
	"translations-only": "TRANSLATIONS_ONLY"
}

#PublicNatPortCount: int & >=32 & <=65536

#NatConfig: {
	#import?: {
		router?: string
		nat?:    string
		addresses?: [string]: string
	}
	#history?: {
		router?: [...string]
		nat?: [...string]
		addresses?: [string]: [...string]
	}

	subnets: [#RFC1035Name]: {
		sourceIpRanges: [#PublicNatSourceRange, ...#PublicNatSourceRange]
		sourceIpRanges: _ | *["all-ip-ranges"]

		secondaryRangeNames: [...#RFC1035Name]
		secondaryRangeNames: _ | *[]

		if list.Contains(sourceIpRanges, "all-ip-ranges") && len(sourceIpRanges) > 1 {
			_|_("all-ip-ranges cannot be combined with other NAT source ranges")
		}
		if list.Contains(sourceIpRanges, "list-of-secondary-ip-ranges") && len(secondaryRangeNames) == 0 {
			_|_("secondary NAT source ranges require at least one secondary range name")
		}
		if !list.Contains(sourceIpRanges, "list-of-secondary-ip-ranges") && len(secondaryRangeNames) > 0 {
			_|_("secondary range names require list-of-secondary-ip-ranges")
		}
	}
	addresses: [#RFC1035Name]: {
		networkTier: #NetworkTier
		networkTier: _ | *"premium"
	}
	addresses: _ | *{}

	defaultAddresses: [...#RFC1035Name]
	defaultAddresses: _ | *[]

	rules: [...{
		ruleNumber: int & >=0 & <=65000
		match:      string

		activeAddresses: [#RFC1035Name, ...#RFC1035Name]

		drainAddresses: [...#RFC1035Name]
		drainAddresses: _ | *[]
	}]
	rules: _ | *[]

	enableEndpointIndependentMapping: bool
	enableEndpointIndependentMapping: _ | *false

	minPortsPerVm: #PublicNatPortCount
	minPortsPerVm: _ | *64
	if bits.OnesCount(minPortsPerVm) != 1 {
		_|_("minimum ports per VM must be a power of two")
	}

	enableDynamicPortAllocation: bool
	enableDynamicPortAllocation: _ | *false

	if !enableDynamicPortAllocation {
		minPortsPerVm: >=64
	}
	if enableDynamicPortAllocation {
		maxPortsPerVm: #PublicNatPortCount & >minPortsPerVm
		maxPortsPerVm: _ | *65536
		if bits.OnesCount(maxPortsPerVm) != 1 {
			_|_("maximum ports per VM must be a power of two")
		}
	}
	if enableDynamicPortAllocation && enableEndpointIndependentMapping {
		_|_("dynamic port allocation and endpoint-independent mapping are mutually exclusive")
	}
	if len(rules) > 0 && enableEndpointIndependentMapping {
		_|_("NAT rules cannot use endpoint-independent mapping")
	}
	if len(addresses) == 0 && (len(defaultAddresses) > 0 || len(rules) > 0) {
		_|_("automatic NAT cannot configure addresses or rules")
	}
	if len(addresses) > 0 && len(defaultAddresses) == 0 {
		_|_("manual NAT requires at least one default address")
	}

	logging: {
		enabled: bool
		enabled: _ | *true

		filter: #PublicNatLogFilter
		filter: _ | *"errors-only"
	}
	logging: _ | *{}

	let configuredAddressNames = [for name, _ in addresses {name}]
	let usedAddressNames = list.Concat([
		defaultAddresses,
		[
			for rule in rules
			for name in rule.activeAddresses {name}
		],
		[
			for rule in rules
			for name in rule.drainAddresses {name}
		],
	])

	for name in usedAddressNames {
		if !list.Contains(configuredAddressNames, name) {
			_|_("NAT address \(name) is not configured")
		}
	}
	for i, name in usedAddressNames
	for j, otherName in usedAddressNames
	if i < j && name == otherName {
		_|_("NAT address \(name) is used more than once")
	}
	for i, rule in rules
	for j, otherRule in rules
	if i < j && rule.ruleNumber == otherRule.ruleNumber {
		_|_("NAT rule number \(rule.ruleNumber) is used more than once")
	}
}

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

		publicNats: [#Region]: [#RFC1035Name]: #NatConfig
		publicNats: _ | *{}

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
	for region, nats in in.publicNats
	for natName, nat in nats {
		if len(nat.subnets) == 0 {
			_|_("NAT requires at least one subnet")
		}
		for shorthandRegion, shorthandNat in subnetsByRegionsWithNat
		if shorthandRegion == region && natName == shorthandNat.natName {
			_|_("NAT \(natName) conflicts with the automatic NAT shorthand")
		}
		for otherNatName, otherNat in nats
		if natName != otherNatName {
			for subnetName, _ in nat.subnets
			if otherNat.subnets[subnetName] != _|_ {
				_|_("NAT subnet \(subnetName) is configured more than once in \(region)")
			}
			for addressName, _ in nat.addresses
			if otherNat.addresses[addressName] != _|_ {
				_|_("NAT address \(addressName) is configured more than once in \(region)")
			}
		}
	}
	for region, nats in in.publicNats
	for _, nat in nats
	for subnetName, _ in nat.subnets {
		if in.subnets[subnetName] == _|_ {
			_|_("NAT subnet \(subnetName) is not configured")
		}
		if in.subnets[subnetName].region != region {
			_|_("NAT subnet \(subnetName) is not in region \(region)")
		}
		if in.subnets[subnetName].egress.nat {
			_|_("NAT subnet \(subnetName) also enables the automatic NAT shorthand")
		}
		for secondaryRangeName in nat.subnets[subnetName].secondaryRangeNames {
			if in.subnets[subnetName].cidrs[secondaryRangeName] == _|_ ||
				secondaryRangeName == in.subnets[subnetName].primaryRange {
				_|_("NAT secondary range \(secondaryRangeName) is not configured on subnet \(subnetName)")
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
		publicNats: {
			for region, nats in in.publicNats {
				(region): {
					for name, nat in nats {
						let resourceName = "\(region)__\(name)"
						(name): {
							router: "google_compute_router.\(resourceName)"
							"nat":  "google_compute_router_nat.\(resourceName)"
							addresses: {
								for addressName, _ in nat.addresses {
									(addressName): "google_compute_address.\(resourceName)__\(addressName)"
								}
							}
						}
					}
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

		for region, nats in in.publicNats
		for name, nat in nats {
			let resourceName = "\(region)__\(name)"
			let natRefs = refs.publicNats[region][name]

			for addressName, address in nat.addresses {
				resource: google_compute_address: "\(resourceName)__\(addressName)": {
					if nat.#import.addresses[addressName] != _|_ {
						#import: nat.#import.addresses[addressName]
					}
					if nat.#history.addresses[addressName] != _|_ {
						#history: nat.#history.addresses[addressName]
					}

					"name":       addressName
					"region":     region
					address_type: "EXTERNAL"
					network_tier: #NetworkTierMap[address.networkTier]
					lifecycle: create_before_destroy: true
				}
			}

			resource: google_compute_router: (resourceName): {
				if nat.#import.router != _|_ {
					#import: nat.#import.router
				}
				if nat.#history.router != _|_ {
					#history: nat.#history.router
				}

				"name":    name
				"region":  region
				"network": "${\(refs.network).id}"
			}

			resource: google_compute_router_nat: (resourceName): {
				if nat.#import.nat != _|_ {
					#import: nat.#import.nat
				}
				if nat.#history.nat != _|_ {
					#history: nat.#history.nat
				}

				"name":   name
				"region": region
				router:   "${\(natRefs.router).name}"

				if len(nat.addresses) == 0 {
					nat_ip_allocate_option: "AUTO_ONLY"
				}
				if len(nat.addresses) > 0 {
					nat_ip_allocate_option: "MANUAL_ONLY"
					nat_ips: [for addressName in nat.defaultAddresses {
						"${\(natRefs.addresses[addressName]).self_link}"
					}]
				}

				source_subnetwork_ip_ranges_to_nat: "LIST_OF_SUBNETWORKS"
				subnetwork: [for subnetName, subnet in nat.subnets {
					name: "${\(refs.subnets[subnetName]).id}"
					source_ip_ranges_to_nat: [for sourceRange in subnet.sourceIpRanges {
						#PublicNatSourceRangeMap[sourceRange]
					}]
					if len(subnet.secondaryRangeNames) > 0 {
						secondary_ip_range_names: subnet.secondaryRangeNames
					}
				}]

				rules: [for rule in nat.rules {
					rule_number: rule.ruleNumber
					match:       rule.match
					action: {
						source_nat_active_ips: [for addressName in rule.activeAddresses {
							"${\(natRefs.addresses[addressName]).self_link}"
						}]
						if len(rule.drainAddresses) > 0 {
							source_nat_drain_ips: [for addressName in rule.drainAddresses {
								"${\(natRefs.addresses[addressName]).self_link}"
							}]
						}
					}
				}]

				enable_endpoint_independent_mapping: nat.enableEndpointIndependentMapping
				min_ports_per_vm:                    nat.minPortsPerVm
				enable_dynamic_port_allocation:      nat.enableDynamicPortAllocation
				if nat.enableDynamicPortAllocation {
					max_ports_per_vm: nat.maxPortsPerVm
				}

				log_config: {
					enable: nat.logging.enabled
					filter: #PublicNatLogFilterMap[nat.logging.filter]
				}
			}
		}
	}
}
