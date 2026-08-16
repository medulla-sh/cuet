@if(test)

package cloudflare

#ZoneTests: {
	"defaults-and-action-normalization": {
		input: #Zone & {in: {
			#imports: {
				zone:           "zone-id"
				customFirewall: "ruleset-id"
			}
			name:      "oakmont-health"
			zone:      "oakmont.health"
			accountId: "account-id"
			customFirewallRules: [{
				ref:         "restrict-console"
				description: "Restrict console ingress"
				expression:  #"http.host eq "app.dev.oakmont.health" and ip.src ne 192.0.2.1"#
				action:      "managed-challenge"
			}]
		}}

		let ruleset = input.out.resource.cloudflare_ruleset["oakmont-health-custom-firewall"]

		assert: input.out.resource.cloudflare_zone["oakmont-health"].#import == "zone-id"
		assert: input.refs.zone == "cloudflare_zone.oakmont-health"
		assert: ruleset.#import == "ruleset-id"
		assert: ruleset.zone_id == "${cloudflare_zone.oakmont-health.id}"
		assert: ruleset.description == "Custom firewall rules for oakmont.health"
		assert: ruleset.kind == "zone"
		assert: ruleset.phase == "http_request_firewall_custom"
		assert: ruleset.rules[0].action == "managed_challenge"
		assert: ruleset.rules[0].enabled == true
		assert: input.refs.customFirewall == "cloudflare_ruleset.oakmont-health-custom-firewall"
	}

	"zone-without-ruleset": {
		input: #Zone & {in: {
			name:      "oakmont-health"
			zone:      "oakmont.health"
			accountId: "account-id"
		}}

		assert: input.out.resource.cloudflare_ruleset == _|_
		assert: input.refs.customFirewall == _|_
	}

	"duplicate-rule-ref-rejected": {
		input: {
			name:      "oakmont-health"
			zone:      "oakmont.health"
			accountId: "account-id"
			customFirewallRules: [{
				ref:         "restrict-console"
				description: "First rule"
				expression:  "true"
				action:      "block"
			}, {
				ref:         "restrict-console"
				description: "Second rule"
				expression:  "true"
				action:      "log"
			}]
		}

		assert: (#Zone & {in: input}) == _|_
	}
}

zoneResult: [for _, test in #ZoneTests {test.assert & true}]
