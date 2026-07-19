package framework

import cuet "github.com/medulla-sh/cuet@v0"

cuet.#InfraModule

#Environments: {
	dev: {}
	prod: {}
}

#Terraform: {
	requiredVersion: ">= 1.0"
	backend: local: path: "state.tfstate"
	providers: {}
}

infra: {
	#metadata: {
		module:               "test/module"
		localBackendOverride: null
	}
	in: dev: {}
}

infra: {
	generated: close({dev: _})
	out: close({dev: _})
}
