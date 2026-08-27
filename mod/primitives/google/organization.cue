package google

#Organization: {
	in: {
		#providerAlias?: string
		name:            string
		{domain: string} | {orgId: int} | *{}
	}
	ref: "data.google_organization.\(in.name)"
	out: {
		data: google_organization: (in.name): {
			if in.#providerAlias != _|_ {
				#providerAlias: in.#providerAlias
			}
			if in.orgId != _|_ {
				organization: "organizations/\(in.orgId)"
			}
			if in.domain != _|_ {
				domain: in.domain
			}
		}
	}
}
