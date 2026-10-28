package google

import (
	"net"
	"strings"
	T "github.com/medulla-sh/cuet"
)

#PacketMirroringResource: {subnetwork: string} | {instance: string} | {tag: string}

#PacketMirroring: {
	in: {
		#import?:     string
		name:         #RFC1035Name
		region:       #Region
		network:      string
		collectorIlb: string
		description?: string
		enabled:      bool
		enabled:      _ | *true
		mirroredResources: [#PacketMirroringResource, ...#PacketMirroringResource]
		filter?: {
			ipProtocols: [...string]
			cidrRanges: [...(net.IPCIDR & =~"^[0-9.]+/")]
			direction: "ingress" | "egress" | "both"
			direction: _ | *"both"
		}
	}

	ref: "google_compute_packet_mirroring.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_compute_packet_mirroring: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			name:   in.name
			region: in.region
			network: url:       in.network
			collector_ilb: url: in.collectorIlb
			enable: [if in.enabled {"TRUE"}, if !in.enabled {"FALSE"}][0]
			if in.description != _|_ {
				description: in.description
			}
			mirrored_resources: {
				subnetworks: [for selector in in.mirroredResources if selector.subnetwork != _|_ {{url: selector.subnetwork}}]
				instances: [for selector in in.mirroredResources if selector.instance != _|_ {{url: selector.instance}}]
				tags: [for selector in in.mirroredResources if selector.tag != _|_ {selector.tag}]
			}
			if in.filter != _|_ {
				filter: {
					ip_protocols: in.filter.ipProtocols
					cidr_ranges:  in.filter.cidrRanges
					direction:    strings.ToUpper(in.filter.direction)
				}
			}
		}
	}
}
