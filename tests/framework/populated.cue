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
	in: dev: {
		resource: terraform_data: {
			current: {
				#history: ["original", "renamed"]
				input: "value"
			}
			moved: {
				#history: [{
					module: "old/module"
					name:   "old_name"
				}, "renamed", {
					module: "test/module"
				}, "local_name"]
				input: "moved"
			}
		}
	}
	in: prod: {
		#history: ["old/module:prod"]
	}
}

infra: {
	generated: close({
		dev: {
			moved: [{
				from: "terraform_data.original"
				to:   "terraform_data.renamed"
			}, {
				from: "terraform_data.renamed"
				to:   "terraform_data.current"
			}, {
				from: "terraform_data.renamed"
				to:   "terraform_data.local_name"
			}, {
				from: "terraform_data.local_name"
				to:   "terraform_data.moved"
			}]
			...
		}
		prod: _
	})
	out: close({dev: _, prod: _})
	#migration: dev: {
		moduleHistory: []
		resourceTransitions: [{
			resourceType: "terraform_data"
			from: {
				module: "old/module"
				env:    "dev"
				name:   "renamed"
			}
			to: {
				module: "test/module"
				env:    "dev"
				name:   "renamed"
			}
		}]
	}
	#migration: prod: {
		moduleHistory: ["old/module:prod"]
		resourceTransitions: []
	}
}
