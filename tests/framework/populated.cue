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
	in: dev: resource: terraform_data: current: {
		#history: ["original", "renamed"]
		input: "value"
	}
}

infra: {
	generated: close({dev: {
		moved: [{
			from: "terraform_data.original"
			to:   "terraform_data.renamed"
		}, {
			from: "terraform_data.renamed"
			to:   "terraform_data.current"
		}]
		...
	}})
	out: close({dev: _})
}
