package framework

import (
	cuet "github.com/medulla-sh/cuet@v0"
	G "github.com/medulla-sh/cuet/primitives/google"
	"github.com/medulla-sh/cuet/primitives/neon"
)

cuet.#InfraModule

#Environments: {
	dev: {}
	old: {}
	prod: {}
}

#Terraform: {
	requiredVersion: ">= 1.11"
	backend: local: path: "state.tfstate"
	providers: {
		google: {
			requiredProvider: {
				source:  "hashicorp/google"
				version: ">=6"
			}
			default: provider: project: "example"
		}
		random: {
			requiredProvider: {
				source:  "hashicorp/random"
				version: "~>3.7"
			}
			default: provider: {}
		}
		neon: {
			requiredProvider: {
				source:  "registry.opentofu.org/kislerdm/neon"
				version: "~>0.13"
			}
			default: {
				let secret = G.#SecretVersion & {in: {
					name:     "neon"
					project:  "example"
					secretId: "neon"
				}}
				bootstrap: secret.out
				provider: api_key: "${ephemeral.google_secret_manager_secret_version.neon.secret_data}"
			}
			aliases: readonly: provider: api_key: "readonly"
		}
		kubernetes: {
			requiredProvider: {
				source:  "hashicorp/kubernetes"
				version: "~>2.38"
			}
			aliases: {
				dev: provider: host:      "https://dev.example.com"
				internal: provider: host: "https://internal.example.com"
			}
		}
		archive: {
			requiredProvider: {
				source:  "example/archive"
				version: "1.0.0"
			}
			aliases: historical: provider: endpoint: "https://archive.example.com"
		}
	}
}

infra: {
	#metadata: {
		module:               "test/module"
		localBackendOverride: null
		reconciliation: {
			environment: "old"
			stateResources: [{
				address: "neon_project.deleted"
				source:  "kislerdm/neon"
				alias:   "readonly"
			}, {
				address: "archive_file.deleted"
				source:  "example/archive"
				alias:   "historical"
			}, {
				address: "terraform_data.deleted"
				source:  "terraform.io/builtin/terraform"
			}]
		}
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
		data: kubernetes_service_v1: example: {
			#providerAlias: "dev"
		}
		data: google_project: framework: {}
		ephemeral: random_password: framework: length: 32
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
		old:  _
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
						source:  "registry.opentofu.org/kislerdm/neon"
						version: "~>0.13"
					}
					kubernetes: {
						source:  "hashicorp/kubernetes"
						version: "~>2.38"
					}
					random: {
						source:  "hashicorp/random"
						version: "~>3.7"
					}
				}
				provider: {
					google: [{project: "example"}]
					neon: [{api_key: "${ephemeral.google_secret_manager_secret_version.neon.secret_data}"}]
					kubernetes: [{
						alias: "dev"
						host:  "https://dev.example.com"
					}]
					random: [{}]
				}
				data: kubernetes_service_v1: example: provider: "kubernetes.dev"
				data: google_project: framework: {}
				ephemeral: google_secret_manager_secret_version: neon: {
					project: "example"
					secret:  "neon"
					version: "latest"
				}
				ephemeral: random_password: framework: length: 32
				...
			}
			kubernetes: enabled: true
		}
		old: terraform: {
			terraform: required_providers: {
				google?:     _|_
				kubernetes?: _|_
				archive: {
					source:  "example/archive"
					version: "1.0.0"
				}
				neon: {
					source:  "registry.opentofu.org/kislerdm/neon"
					version: "~>0.13"
				}
			}
			provider: {
				google?:     _|_
				kubernetes?: _|_
				archive: [{
					alias:    "historical"
					endpoint: "https://archive.example.com"
				}]
				neon: [{
					alias:   "readonly"
					api_key: "readonly"
				}]
			}
			data?:      _|_
			ephemeral?: _|_
			...
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
