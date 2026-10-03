@if(test)

package cloudflare

#TunnelTests: {
	"remote-defaults": {
		input: #Tunnel & {in: {name: "example", accountId: "account-id"}}

		let tunnel = input.out.resource.cloudflare_zero_trust_tunnel_cloudflared.example

		assert: input.ref == "cloudflare_zero_trust_tunnel_cloudflared.example"
		assert: tunnel.name == input.in.name
		assert: tunnel.account_id == input.in.accountId
		assert: tunnel.config_src == "cloudflare"
		assert: tunnel.tunnel_secret == _|_
		assert: tunnel.#import == _|_
		assert: input.out.output == _|_
	}
	"local-with-import-and-distinct-name": {
		input: #Tunnel & {in: {
			name:         "example"
			accountId:    "account-id"
			tunnelName:   "example-dev"
			configSource: "local"
			tunnelSecret: "${random_bytes.tunnel.base64}"
			#import:      "account-id/tunnel-id"
		}}

		let tunnel = input.out.resource.cloudflare_zero_trust_tunnel_cloudflared.example

		assert: tunnel.name == input.in.tunnelName
		assert: tunnel.config_src == "local"
		assert: tunnel.tunnel_secret == input.in.tunnelSecret
		assert: tunnel.#import == input.in.#import
	}
	"token-composes-with-tunnel": {
		let tunnel = #Tunnel & {in: {name: "example", accountId: "account-id"}}
		input: #TunnelToken & {in: {
			name:      "connector"
			accountId: tunnel.in.accountId
			tunnelId:  "${\(tunnel.ref).id}"
		}}

		let token = input.out.data.cloudflare_zero_trust_tunnel_cloudflared_token.connector

		assert: input.ref == "data.cloudflare_zero_trust_tunnel_cloudflared_token.connector"
		assert: token.account_id == tunnel.in.accountId
		assert: token.tunnel_id == "${cloudflare_zero_trust_tunnel_cloudflared.example.id}"
		assert: input.out.resource == _|_
		assert: input.out.output == _|_
	}
	"invalid-config-source-rejected": {
		assert: (#Tunnel & {in: {name: "example", accountId: "account-id", configSource: "other"}}) == _|_
	}
	"empty-token-tunnel-id-rejected": {
		assert: (#TunnelToken & {in: {name: "example", accountId: "account-id", tunnelId: ""}}) == _|_
	}
}

tunnelResult: [for _, test in #TunnelTests {test.assert & true}]
