package framework

import (
	cuet "github.com/medulla-sh/cuet@v0"
	G "github.com/medulla-sh/cuet/primitives/google"
	"github.com/medulla-sh/cuet/primitives/neon"
)

cuet.#InfraModule

#Environments: {
	dev: {}
	prod: {}
}

#Terraform: {
	requiredVersion: ">= 1.0"
	backend: local: path: "state.tfstate"
	providers: {
		google: {
			requiredProvider: {
				source:  "hashicorp/google"
				version: ">=6"
			}
			default: provider: project: "example"
		}
		neon: {
			requiredProvider: {
				source:  "kislerdm/neon"
				version: "~>0.13"
			}
			default: {
				let secret = G.#SecretVersion & {in: {
					name:     "neon"
					project:  "example"
					secretId: "neon"
				}}
				bootstrap: secret.out
				provider: api_key: "${data.google_secret_manager_secret_version.neon.secret_data}"
			}
		}
	}
}

infra: {
	#metadata: {
		module:               "test/module"
		localBackendOverride: null
	}
	in: dev: {
		let project = neon.#Project & {"in": {
			name:     "example"
			regionId: "aws-us-west-2"
		}}
		project.out
		let branch = neon.#Branch & {"in": {
			name:      "dev"
			projectId: project.ref
		}}
		branch.out

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
	out: close({
		dev: {
			terraform: {
				terraform: required_providers: {
					google: {
						source:  "hashicorp/google"
						version: ">=6"
					}
					neon: {
						source:  "kislerdm/neon"
						version: "~>0.13"
					}
				}
				provider: google: [{project: "example"}]
				data: google_secret_manager_secret_version: neon: {
					project: "example"
					secret:  "neon"
				}
				...
			}
			kubernetes: enabled: true
		}
		prod: terraform: _
	})
	out: dev: kubernetes: enabled: true
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
