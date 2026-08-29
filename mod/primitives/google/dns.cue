package google

#DnsRecordType: "A" | "AAAA" | "CAA" | "CNAME" | "MX" | "NAPTR" | "NS" | "PTR" | "SOA" | "SPF" | "SRV" | "TXT"

#PrivateDnsZone: {
	in: {
		#import?: {
			zone?: string
			records?: [string]: string
		}

		name:    #RFC1035Name
		dnsName: string & =~"\\.$"
		networks: [...string]
		networks: [_, ...]

		records: [#RFC1035Name]: {
			name: string & =~"\\.$"
			type: #DnsRecordType
			ttl:  int & >0
			rrdatas: [...string]
			rrdatas: [_, ...]
		}
	}

	refs: {
		zone: "google_dns_managed_zone.\(in.name)"
		records: {
			for name, _ in in.records {
				(name): "google_dns_record_set.\(name)"
			}
		}
	}

	out: {
		resource: google_dns_managed_zone: (in.name): {
			if in.#import.zone != _|_ {
				#import: in.#import.zone
			}

			name:       in.name
			dns_name:   in.dnsName
			visibility: "private"
			private_visibility_config: networks: [for network in in.networks {
				network_url: network
			}]
		}

		for name, record in in.records {
			resource: google_dns_record_set: (name): {
				if in.#import.records[name] != _|_ {
					#import: in.#import.records[name]
				}

				"name":       record.name
				managed_zone: "${\(refs.zone).name}"
				type:         record.type
				ttl:          record.ttl
				rrdatas:      record.rrdatas
			}
		}
	}
}
