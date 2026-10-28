@if(test)

package google

#IdsTests: {
	"endpoint-defaults-and-collector-wiring": {
		endpoint: #CloudIdsEndpoint & {in: {
			name:     "production-ids"
			location: "us-west1-a"
			network:  "projects/example/global/networks/production"
		}}
		mirroring: #PacketMirroring & {in: {
			name:         "production-ids"
			region:       "us-west1"
			network:      endpoint.in.network
			collectorIlb: endpoint.forwardingRule
			mirroredResources: [{subnetwork: "projects/example/regions/us-west1/subnetworks/gke"}]
		}}

		let resource = mirroring.out.resource.google_compute_packet_mirroring["production-ids"]
		assert: endpoint.out.resource.google_cloud_ids_endpoint["production-ids"].severity == "INFORMATIONAL"
		assert: resource.collector_ilb.url == "${google_cloud_ids_endpoint.production-ids.endpoint_forwarding_rule}"
		assert: resource.mirrored_resources.subnetworks == [{url: "projects/example/regions/us-west1/subnetworks/gke"}]
		assert: resource.enable == "TRUE"
		assert: resource.filter == _|_
	}
	"severity-import-and-dependency": {
		endpoint: #CloudIdsEndpoint & {in: {
			name:     "ids"
			location: "us-west1-a"
			network:  "production"
			severity: "high"
			#import:  "projects/example/locations/us-west1-a/endpoints/ids"
			#dependsOn: ["google_service_networking_connection.services"]
		}}

		let resource = endpoint.out.resource.google_cloud_ids_endpoint.ids
		assert: resource.severity == "HIGH"
		assert: resource.#import == endpoint.in.#import
		assert: resource.#dependsOn == ["google_service_networking_connection.services"]
	}
	"filtered-disabled-mirroring": {
		mirroring: #PacketMirroring & {in: {
			name:         "ids"
			region:       "us-west1"
			network:      "production"
			collectorIlb: "collector"
			enabled:      false
			#import:      "projects/example/regions/us-west1/packetMirrorings/ids"
			mirroredResources: [{tag: "production"}, {instance: "instance-url"}]
			filter: {
				ipProtocols: ["tcp"]
				cidrRanges: ["10.0.0.0/8"]
				direction: "egress"
			}
		}}

		let resource = mirroring.out.resource.google_compute_packet_mirroring.ids
		assert: resource.enable == "FALSE"
		assert: resource.#import == mirroring.in.#import
		assert: resource.mirrored_resources.tags == ["production"]
		assert: resource.mirrored_resources.instances == [{url: "instance-url"}]
		assert: resource.filter == {ip_protocols: ["tcp"], cidr_ranges: ["10.0.0.0/8"], direction: "EGRESS"}
	}
	"empty-selectors-rejected": {
		assert: (#PacketMirroring & {in: mirroredResources: []}) == _|_
	}
	"ambiguous-selector-rejected": {
		assert: (#PacketMirroringResource & {subnetwork: "subnet", tag: "production"}) == _|_
	}
	"ipv6-filter-rejected": {
		assert: (#PacketMirroring & {in: {
			mirroredResources: [{tag: "production"}]
			filter: cidrRanges: ["2001:db8::/32"]
		}}) == _|_
	}
}

idsResult: [for _, test in #IdsTests {test.assert & true}]
