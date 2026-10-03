@if(test)

package cloudflare

#ConnectivityDirectoryServiceTests: {
	let base = {
		name:      "private-api"
		accountId: "account-id"
		hostname:  "gateway.example.internal"
		tunnelId:  "${cloudflare_zero_trust_tunnel_cloudflared.api.id}"
	}
	let http = base & {type: "http"}

	"https-default-verification-and-import": {
		input: #ConnectivityDirectoryService & {in: http & {
			serviceName: "private-api-dev"
			#import:     "account-id/service-id"
			httpsPort:   8443
		}}

		let service = input.out.resource.cloudflare_connectivity_directory_service["private-api"]

		assert: input.ref == "cloudflare_connectivity_directory_service.private-api"
		assert: service.#import == "account-id/service-id"
		assert: service.name == "private-api-dev"
		assert: service.type == "http"
		assert: service.host.resolver_network.tunnel_id == base.tunnelId
		assert: service.https_port == 8443
		assert: service.http_port == _|_
		assert: service.tls_settings.cert_verification_mode == "verify_full"
		assert: input.out.output == _|_
	}
	"http-without-tls": {
		input: #ConnectivityDirectoryService & {in: http & {httpPort: 8080}}

		let service = input.out.resource.cloudflare_connectivity_directory_service["private-api"]

		assert: service.http_port == 8080
		assert: service.name == base.name
		assert: service.https_port == _|_
		assert: service.tls_settings == _|_
		assert: service.#import == _|_
	}
	"both-ports-and-explicit-verification": {
		input: #ConnectivityDirectoryService & {in: http & {
			httpPort:             8080
			httpsPort:            8443
			certVerificationMode: "verify_ca"
		}}

		let service = input.out.resource.cloudflare_connectivity_directory_service["private-api"]

		assert: service.http_port == 8080
		assert: service.https_port == 8443
		assert: service.tls_settings.cert_verification_mode == "verify_ca"
	}
	"invalid-port-rejected": {
		assert: (#ConnectivityDirectoryService & {in: http & {httpsPort: 65536}}) == _|_
	}
	"tcp-with-application-protocol": {
		input: #ConnectivityDirectoryService & {in: base & {
			type:        "tcp"
			tcpPort:     5432
			appProtocol: "postgresql"
		}}

		let service = input.out.resource.cloudflare_connectivity_directory_service["private-api"]

		assert: service.name == base.name
		assert: service.type == "tcp"
		assert: service.tcp_port == 5432
		assert: service.app_protocol == "postgresql"
		assert: service.http_port == _|_
		assert: service.https_port == _|_
		assert: service.tls_settings == _|_
	}
	"tcp-without-application-protocol": {
		input: #ConnectivityDirectoryService & {in: base & {type: "tcp", tcpPort: 9000}}

		assert: input.out.resource.cloudflare_connectivity_directory_service["private-api"].app_protocol == _|_
	}
	"mixed-protocol-fields-rejected": {
		assert: (#ConnectivityDirectoryService & {in: http & {tcpPort: 5432}}) == _|_
	}
}

connectivityDirectoryServiceResult: [for _, test in #ConnectivityDirectoryServiceTests {test.assert & true}]
