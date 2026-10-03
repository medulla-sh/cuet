package cloudflare

#ConnectivityDirectoryService: {
	in: {
		#import?:    string
		name:        string & !=""
		accountId:   string & !=""
		serviceName: string & !=""
		serviceName: _ | *name
		hostname:    string & !=""
		tunnelId:    string & !=""
		type:        "http" | "tcp"
		if type == "http" {
			httpPort?:            int & >0 & <=65535
			httpsPort?:           int & >0 & <=65535
			certVerificationMode: "verify_full" | "verify_ca" | "disabled"
			certVerificationMode: _ | *"verify_full"
		}
		if type == "tcp" {
			tcpPort:      int & >0 & <=65535
			appProtocol?: "postgresql" | "mysql"
		}
	}

	ref: "cloudflare_connectivity_directory_service.\(in.name)"

	out: resource: cloudflare_connectivity_directory_service: (in.name): {
		if in.#import != _|_ {
			#import: in.#import
		}
		account_id: in.accountId
		name:       in.serviceName
		type:       in.type
		host: {
			hostname: in.hostname
			resolver_network: tunnel_id: in.tunnelId
		}
		if in.type == "http" {
			if in.httpPort != _|_ {
				http_port: in.httpPort
			}
			if in.httpsPort != _|_ {
				https_port: in.httpsPort
				tls_settings: cert_verification_mode: in.certVerificationMode
			}
		}
		if in.type == "tcp" {
			tcp_port: in.tcpPort
			if in.appProtocol != _|_ {
				app_protocol: in.appProtocol
			}
		}
	}
}
