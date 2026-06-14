package google

import (
	"net"
	"strings"
)

#RFC1035Name: =~"^[a-z]([-a-z0-9]*[a-z0-9])?$"

#Network: {
	in: {
		name: #RFC1035Name

		autoCreateSubnets: bool
		autoCreateSubnets: _ | *false

		routingMode: "regional" | "global"
		routingMode: _ | *"global"

		mtu?: int & >0

		subnets: [#RFC1035Name]: {
			region: #Region

			privateIpGoogleAccess: bool
			privateIpGoogleAccess: _ | *true

			primaryRange: #RFC1035Name
			primaryRange: or([for k, _ in cidrs {k}])

			cidrs: [#RFC1035Name]: net.IPCIDR
		}
	}

	refs: {
		network: "google_compute_network.\(in.name)"
		subnets: {
			for name, _ in in.subnets {
				(name): "google_compute_subnetwork.\(name)"
			}
		}
	}

	out: {
		resource: google_compute_network: (in.name): {
			name:                    in.name
			auto_create_subnetworks: in.autoCreateSubnets

			routing_mode: strings.ToUpper(in.routingMode)

			if in.mtu != _|_ {
				mtu: in.mtu
			}
		}

		for subnetName, subnet in in.subnets {
			resource: google_compute_subnetwork: (subnetName): {
				name: subnetName

				network: "${\(refs.network).id}"

				region: subnet.region

				private_ip_google_access: subnet.privateIpGoogleAccess

				ip_cidr_range: subnet.cidrs[subnet.primaryRange]

				secondary_ip_range: [
					for name, cidr in subnet.cidrs if name != subnet.primaryRange {
						range_name:    name
						ip_cidr_range: cidr
					},
				]
			}
		}
	}
}
