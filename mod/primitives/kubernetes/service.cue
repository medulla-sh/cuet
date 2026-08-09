package kubernetes

import T "github.com/medulla-sh/cuet"

#DataService: {
	in: {
		#providerAlias?: string

		serviceName: string

		namespace: string

		name: string
		name: _ | *"\(namespace)-\(serviceName)"
	}

	ref:              "data.kubernetes_service_v1.\(in.name)"
	loadBalancerIPv4: "${\(ref).status[0].load_balancer[0].ingress[0].ip}"

	out: T.#TerraformInput & {
		data: kubernetes_service_v1: (in.name): {
			if in.#providerAlias != _|_ {
				#providerAlias: in.#providerAlias
			}

			metadata: {
				name:      in.serviceName
				namespace: in.namespace
			}
		}
	}
}
