package cloudflare

#Tunnel: {
	in: {
		#import?:      string
		name:          string & !=""
		accountId:     string & !=""
		tunnelName:    string & !=""
		tunnelName:    _ | *name
		configSource:  "local" | "cloudflare"
		configSource:  _ | *"cloudflare"
		tunnelSecret?: string & !=""
	}

	ref: "cloudflare_zero_trust_tunnel_cloudflared.\(in.name)"

	out: resource: cloudflare_zero_trust_tunnel_cloudflared: (in.name): {
		if in.#import != _|_ {
			#import: in.#import
		}
		account_id: in.accountId
		name:       in.tunnelName
		config_src: in.configSource
		if in.tunnelSecret != _|_ {
			tunnel_secret: in.tunnelSecret
		}
	}
}

#TunnelToken: {
	in: {
		name:      string & !=""
		accountId: string & !=""
		tunnelId:  string & !=""
	}

	ref: "data.cloudflare_zero_trust_tunnel_cloudflared_token.\(in.name)"

	out: data: cloudflare_zero_trust_tunnel_cloudflared_token: (in.name): {
		account_id: in.accountId
		tunnel_id:  in.tunnelId
	}
}
