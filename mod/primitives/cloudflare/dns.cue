package cloudflare

import (
	"list"
	T "github.com/medulla-sh/cuet"
)

#ZoneCustomFirewallRule: {
	ref:         string
	description: string
	expression:  string
	action:      "block" | "challenge" | "js-challenge" | "managed-challenge" | "log"
	enabled:     bool
	enabled:     _ | *true
}

#Zone: {
	in: {
		#imports: {
			zone?:           string
			customFirewall?: string
		}

		name: string

		zone: string

		accountId: string

		type: "full" | "partial"
		type: _ | *"full"

		customFirewallRules: [...#ZoneCustomFirewallRule]

		let customFirewallRuleRefs = {
			for rule in customFirewallRules {
				(rule.ref): true
			}
		}
		if len(customFirewallRuleRefs) != len(customFirewallRules) {
			_|_("custom firewall rule refs must be unique")
		}
	}

	refs: {
		zone: "cloudflare_zone.\(in.name)"
		if len(in.customFirewallRules) > 0 {
			customFirewall: "cloudflare_ruleset.\(in.name)-custom-firewall"
		}
	}

	out: T.#TerraformInput & {
		resource: cloudflare_zone: (in.name): {
			if in.#imports.zone != _|_ {
				#import: in.#imports.zone
			}

			name: in.zone
			type: in.type

			account: id: in.accountId
		}

		if len(in.customFirewallRules) > 0 {
			resource: cloudflare_ruleset: "\(in.name)-custom-firewall": {
				if in.#imports.customFirewall != _|_ {
					#import: in.#imports.customFirewall
				}

				zone_id:     "${\(refs.zone).id}"
				name:        "\(in.name)-custom-firewall"
				description: "Custom firewall rules for \(in.zone)"
				kind:        "zone"
				phase:       "http_request_firewall_custom"
				rules: [for rule in in.customFirewallRules {
					ref:         rule.ref
					description: rule.description
					expression:  rule.expression
					action: [
						if rule.action == "js-challenge" {"js_challenge"},
						if rule.action == "managed-challenge" {"managed_challenge"},
						if rule.action != "js-challenge" && rule.action != "managed-challenge" {rule.action},
					][0]
					enabled: rule.enabled
				}]
			}
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
