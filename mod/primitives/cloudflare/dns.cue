package cloudflare

import (
	"list"
	T "github.com/medulla-sh/cuet"
)

#Zone: {
	in: {
		#import?: string

		name: string

		zone: string

		accountId: string

		type: "full" | "partial"
		type: _ | *"full"
	}

	ref: "cloudflare_zone.\(in.name)"

	out: T.#TerraformInput & {
		resource: cloudflare_zone: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name: in.zone
			type: in.type

			account: id: in.accountId
		}
	}
}

#DnsRecord: {
	in: {
		#import?: string

		name: string

		zoneId: string

		dnsName: string

		type:  string
		value: string

		ttl: int & >=1
		ttl: _ | *300

		priority?: int

		proxied: bool
		proxied: _ | *false

		if !list.Contains(["A", "AAAA", "CNAME"], type) {
			proxied: false
		}

		if proxied {
			ttl: 1
		}
	}

	ref: "cloudflare_dns_record.\(in.name)"

	out: {
		resource: cloudflare_dns_record: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			zone_id: in.zoneId
			name:    in.dnsName
			type:    in.type
			content: in.value
			ttl:     in.ttl
			proxied: in.proxied

			if in.priority != _|_ {
				priority: in.priority
			}
		}
	}
}
