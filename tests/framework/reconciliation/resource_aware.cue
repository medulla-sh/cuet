package reconciliation

import cuet "github.com/medulla-sh/cuet@v0"

cuet.#InfraModule

#Environments: dev: {}

#Terraform: {
	requiredVersion: ">= 1.11"
	backend: local: path: "state.tfstate"
	providers: {
		google: {
			requiredProvider: {
				source:  "hashicorp/google"
				version: ">=6"
			}
			default: provider: project: "current"
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
			environment: "dev"
			stateResources: [{
				address: "google_example.current"
				source:  "hashicorp/google"
				alias:   "retired"
			}, {
				address: "data.google_example.lookup"
				source:  "hashicorp/google"
				alias:   "retired"
			}, {
				address: "google_example.original"
				source:  "hashicorp/google"
				alias:   "retired"
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
		resource: google_example: {
			current: {}
			moved: #history: ["original"]
		}
		data: google_example: lookup: {}
	}
	out: dev: terraform: {
		terraform: required_providers: {
			google: {
				source:  "hashicorp/google"
				version: ">=6"
			}
			archive: {
				source:  "example/archive"
				version: "1.0.0"
			}
			terraform?: _|_
		}
		provider: {
			google: [{project: "current"}]
			archive: [{
				alias:    "historical"
				endpoint: "https://archive.example.com"
			}]
			terraform?: _|_
		}
		moved: [{
			from: "google_example.original"
			to:   "google_example.moved"
		}]
		resource: google_example: {
			current: {}
			moved: {}
		}
		data: google_example: lookup: {}
	}
}
