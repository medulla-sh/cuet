@if(test)

package google

#PrivateDnsZoneTests: {
	"zone-and-records": {
		input: #PrivateDnsZone & {in: {
			#import: {
				zone: "projects/example/managedZones/step-ca"
				records: ca: "projects/example/managedZones/step-ca/rrsets/ca.internal.example./A"
			}
			name:    "step-ca"
			dnsName: "ca.internal.example."
			networks: [
				"projects/dev/global/networks/dev",
				"projects/internal/global/networks/internal",
			]
			records: ca: {
				name: "ca.internal.example."
				type: "A"
				ttl:  300
				rrdatas: ["10.200.0.10"]
			}
		}}

		assert: input.out.resource.google_dns_managed_zone["step-ca"] == {
			#import:    "projects/example/managedZones/step-ca"
			name:       "step-ca"
			dns_name:   "ca.internal.example."
			visibility: "private"
			private_visibility_config: networks: [{
				network_url: "projects/dev/global/networks/dev"
			}, {
				network_url: "projects/internal/global/networks/internal"
			}]
		}
		assert: input.out.resource.google_dns_record_set.ca == {
			#import:      "projects/example/managedZones/step-ca/rrsets/ca.internal.example./A"
			name:         "ca.internal.example."
			managed_zone: "${google_dns_managed_zone.step-ca.name}"
			type:         "A"
			ttl:          300
			rrdatas: ["10.200.0.10"]
		}
		assert: input.refs.records.ca == "google_dns_record_set.ca"
	}
}

privateDnsZoneResult: [
	for _, test in #PrivateDnsZoneTests {test.assert & true},
]
