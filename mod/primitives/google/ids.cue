package google

import (
	"strings"
	T "github.com/medulla-sh/cuet"
)

#CloudIdsEndpoint: {
	in: {
		#import?: string
		#dependsOn: [...string]
		name:         #RFC1035Name
		location:     =~"^[a-z]+-[a-z]+[0-9]+-[a-z]$"
		network:      string
		severity:     "informational" | "low" | "medium" | "high" | "critical"
		severity:     _ | *"informational"
		description?: string
	}

	ref:            "google_cloud_ids_endpoint.\(in.name)"
	forwardingRule: "${\(ref).endpoint_forwarding_rule}"
	out: T.#TerraformInput & {
		resource: google_cloud_ids_endpoint: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			#dependsOn: in.#dependsOn
			name:       in.name
			location:   in.location
			network:    in.network
			severity:   strings.ToUpper(in.severity)
			if in.description != _|_ {
				description: in.description
			}
		}
	}
}
