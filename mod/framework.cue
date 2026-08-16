package cuet

import (
	"list"
	"strings"
)

_#DefaultProviderAlias: ""

// A set of providers which are builtin and require no configuration
#BuiltinProviders: {
	"terraform": _
}

#ProviderInstance: {
	// Additional Terraform input required to bootstrap this provider instance.
	bootstrap: {...}
	bootstrap: _ | *{...}
	provider: {...}
}

#ProviderRegistration: {
	// Used to fill in the `terraform.required_providers` block.
	requiredProvider: {
		source:  string
		version: string
	}
	// Default provider instance. Used if no alias is specified.
	default?: #ProviderInstance
	// Alternate provider instances keyed by alias.
	aliases: [string]: #ProviderInstance
}

// Provider definitions supported by the framework, keyed by provider name.
#ProviderRegistry: [string]: #ProviderRegistration

// Metadata and configuration injected by the cuet cli
#Metadata: {
	// The name of the module
	module: string
	// If set, the backend should be overridden to a local backend. This is
	// useful when setting up a new backend
	localBackendOverride: string | null
	localBackendOverride: _ | *null
	reconciliation: null | {
		environment: string
		stateResources: [...#HistoricalResource]
	}
	reconciliation: _ | *null
}

#TerraformConfig: {
	#metadata: #Metadata
	#envName:  string
	#env:      _

	requiredVersion: string
	backend: [string]: {...}
	providers: #ProviderRegistry
}

// Base schema embedded by infrastructure modules.
//
// Infrastructure teams define shared environments, backend, and provider
// registry in this schema. Application teams compose it and provide
// per-environment Terraform input under `infra`.
//
//     // infra/config.cue
//     package infra
//
//     #InfraModule: cuet.#InfraModule & {
//         #Environments: {
//             dev: _
//             staging: _
//             prod: _
//         }
//         #Terraform: {
//             requiredVersion: "~>1.0.0"
//             backend: gcs: {
//                 bucket: "example-\(#env)"
//                 prefix: #module
//             }
//             providers: {
//                 google: {
//                     requiredProvider: {
//                         source: "hashicorp/google"
//                         version: "~>7.14.1"
//                     }
//                     default: provider: {
//                         project: "example-dev"
//                         region:  "us-west1"
//                     }
//                 }
//             }
//         }
//     }
//
// Using this configuration, you use this as follows:
//
//     // app/main.cue
//     import (
//         "github.com/medulla-sh/cuet/primitives/google"
//         "github.com/medulla-sh/cuet/primitives/neon"
//         I "myCompany.com/infra"
//     )
//
//     I.#InfraModule
//     infra: in: {
//         [string]: {
//             (google.#GcsBucket & {in: name: "my-bucket-#env"}).out
//         }
//         dev: {
//             (neon.#Project & {in: name: "my-dev-db"}).out
//         }
//         prod: {
//             (google.#CloudSqlInstance & {in: {
//                 name: "my-prod-db"
//                 region: "us-west1"
//                 engine: {
//                     type:    "postgres"
//                     version: 18
//                 }
//                 tier: "db-f1-micro"
//             }}).out
//         }
//     }
//
#EnvironmentOutput: [string]: _

#InfraModule: {
	// Configuration for environments. This configures what environments are
	// supported by the module.
	#Environments: [string]: _
	#Env: or([for k, _ in #Environments {k}])

	#OutputPolicy: {
		in:  #TerraformOutput
		out: #TerraformOutput
	} | *{
		in:  #TerraformOutput
		out: in
	}

	// Configuration for terraform. In theory, cuet will support multiple forms
	// of infra in a single file. We want to allow configuration globally to
	// avoid having to repeat it for each environment.
	#Terraform: #TerraformConfig & {backend: [string]: #envName: #Env}

	infra: infraThis={
		#metadata: #Metadata

		let backendConfigs = {
			for bEnv, _ in #Environments {
				(bEnv): (#Terraform & {
					#metadata: infraThis.#metadata
					#envName:  bEnv
					#env:      #Environments[bEnv]
				}).backend
			}
		}

		// The actual infrastructure configuration. This is where you place all
		// your configurations.
		let environmentInputs = {
			for e, _ in #Environments {
				(e): #TerraformInput & {
					#module:         infraThis.#metadata.module
					#backendConfigs: backendConfigs
					#envName:        e
					#env:            #Environments[e]
					resource?: [string]: [string]: {
						// Keep this separate from #TerraformResourceHistoryEntry so env can be
						// constrained to the configured environments.
						#history?: [...(string | {
							module?: string
							env?:    #Env
							name?:   string
						})]
					}
				}
			}
		}

		in: close({
			for e, _ in #Environments {
				(e)?: environmentInputs[e]
			}
			if infraThis.#metadata.reconciliation != null {
				(infraThis.#metadata.reconciliation.environment): environmentInputs[infraThis.#metadata.reconciliation.environment]
			}
		})

		#backends: {
			for e, _ in #Environments {
				(e): {
					terraform: required_version: #Terraform.requiredVersion
					terraform: backend: [
						if infraThis.#metadata.localBackendOverride != null {
							local: path: infraThis.#metadata.localBackendOverride
						},
						backendConfigs[e],
					][0]
				}
			}
		}

		generated: {
			let metadata = infraThis.#metadata

			for e, _ in infraThis["in"] {
				let historicalResources = [
					if metadata.reconciliation != null
					if metadata.reconciliation.environment == e {metadata.reconciliation.stateResources},
					[],
				][0]
				(e): (_#GenerateTf & {
					tfConfig: #Terraform & {
						#metadata: metadata
						#envName:  e
						#env:      #Environments[e]
					}
					tf: infraThis["in"][e] & {
						#module:              metadata.module
						#backendConfigs:      backendConfigs
						#historicalResources: historicalResources
					}
				}).out
			}
		}

		out: {
			for e, _ in infraThis["in"] {
				(e): #EnvironmentOutput & {
					terraform: (#OutputPolicy & {in: infraThis.generated[e]}).out
				}
			}
		}

		#migration: {
			for e, _ in infraThis["in"] {
				(e): {
					moduleHistory: *infraThis["in"][e].#history | []
					resourceTransitions: infraThis.generated[e].#crossStateTransitions
				}
			}
		}
	}

}

_#GenerateTf: {
	tfConfig: #TerraformConfig
	tf:       #TerraformInput
	let generatedMoves = _#GenerateMoves & {
		in: tf
	}

	out: #TerraformOutput & {
		terraform: required_version: tfConfig.requiredVersion
		terraform: backend: [
			if tfConfig.#metadata.localBackendOverride != null {
				local: path: tfConfig.#metadata.localBackendOverride
			},
			tfConfig.backend,
		][0]

		(_#GenerateProviders & {
			providerRegistry: tfConfig.providers
			in:               tf
			moves:            generatedMoves.#moves
		}).out

		(_#GenerateImports & {
			in: tf
		}).out

		generatedMoves.out
		#crossStateTransitions: generatedMoves.crossStateTransitions

		(_#RenderTerraformInput & {in: tf}).out
	}
}

_#RenderTerraformInput: {
	in: #TerraformInput

	out: {
		if len(*in.variable | {}) > 0 {variable: in.variable}
		if len(*in.locals | {}) > 0 {locals: in.locals}
		if len(*in.output | {}) > 0 {output: in.output}

		for blockType in ["resource", "data", "ephemeral"]
		if in[blockType] != _|_
		if len(in[blockType]) > 0 {
			(blockType): {
				for sourceName, blocks in in[blockType]
				for blockName, block in blocks {
					let providerName = [
						if block.#provider != _|_ {block.#provider},
						strings.SplitN(sourceName, "_", 2)[0],
					][0]
					(sourceName): (blockName): block & {
						if block.#provider != _|_ || block.#providerAlias != _|_ {
							"provider": [
								if block.#providerAlias != _|_ {"\(providerName).\(block.#providerAlias)"},
								providerName,
							][0]
						}
					}
				}
			}
		}
	}
}

_#NormalizeHistoryEntry: {
	raw: string
	out: {
		module: []
		env: []
		name: [raw]
	}
} | {
	raw: {
		module?: string
		env?:    string
		name?:   string
	}
	out: {
		module: *[raw.module] | []
		env: *[raw.env] | []
		name: *[raw.name] | []
	}
}

_#ResolveResourceHistory: {
	resourceType: _
	current: {
		module: _
		env:    _
		name:   _
	}
	history: [...#TerraformResourceHistoryEntry]

	let normalized = [
		for raw in history {
			(_#NormalizeHistoryEntry & {"raw": raw}).out
		},
	]

	let resolvedHistory = [
		for index, _ in normalized {
			let modules = list.Concat([[current.module],
				for previousIndex, entry in normalized
				if previousIndex <= index {
					entry.module
				},
			])
			let envs = list.Concat([[current.env],
				for previousIndex, entry in normalized
				if previousIndex <= index {
					entry.env
				},
			])
			let names = list.Concat([[current.name],
				for previousIndex, entry in normalized
				if previousIndex <= index {
					entry.name
				},
			])

			{
				module: modules[len(modules)-1]
				env:    envs[len(envs)-1]
				name:   names[len(names)-1]
			}
		},
	]

	transitions: [
		for index, identity in resolvedHistory
		if index+1 < len(resolvedHistory) {
			from: identity
			to:   resolvedHistory[index+1]
		},
	]

	if len(resolvedHistory) != 0 {
		last: resolvedHistory[len(resolvedHistory)-1]
	}

	sameCurrentStateMoves: [
		for transition in transitions
		if transition.from.module == current.module
		if transition.from.env == current.env
		if transition.to.module == current.module
		if transition.to.env == current.env {
			from: "\(resourceType).\(transition.from.name)"
			to:   "\(resourceType).\(transition.to.name)"
		},
	]

	crossStateTransitions: [
		for transition in transitions
		if transition.from.module != transition.to.module ||
			transition.from.env != transition.to.env {
			resourceType: resourceType
			from:         transition.from
			to:           transition.to
		},
	]
}

_#GenerateProviders: {
	providerRegistry: #ProviderRegistry
	in:               #TerraformInput
	moves: [...{
		from: string
		to:   string
	}]
	moves: _ | *[]

	out: #TerraformOutput & {
		let bootstrapContext = {bootstrap: {
			#module:         in.#module
			#backendConfigs: in.#backendConfigs
			#envName:        in.#envName
			#env:            in.#env
		}}
		let directProviders = {
			for source in [
				if in.resource != _|_ {in.resource},
				if in.data != _|_ {in.data},
				if in.ephemeral != _|_ {in.ephemeral},
			]
			for sourceName, blocks in source
			for _, block in blocks {
				let name = [
					if block.#provider != _|_ {block.#provider},
					strings.SplitN(sourceName, "_", 2)[0],
				][0]
				let alias = [
					if block.#providerAlias != _|_ {block.#providerAlias},
					_#DefaultProviderAlias,
				][0]
				(name): (alias): true
			}
		}
		let representedResources = list.Concat([[
			for resourceType, resources in (*in.resource | {})
			for resourceName, _ in resources {
				"\(resourceType).\(resourceName)"
			},
		], [
			for resourceType, resources in (*in.data | {})
			for resourceName, _ in resources {
				"data.\(resourceType).\(resourceName)"
			},
		], [
			for move in moves {
				move.from
			},
		]])
		let historicalProviders = {
			for resource in in.#historicalResources
			if !list.Contains(representedResources, resource.address) {
				let matches = list.Concat([[
					for name, registration in providerRegistry
					let registeredSource = strings.TrimPrefix(strings.TrimPrefix(registration.requiredProvider.source, "registry.opentofu.org/"), "registry.terraform.io/")
					let historicalSource = strings.TrimPrefix(strings.TrimPrefix(resource.source, "registry.opentofu.org/"), "registry.terraform.io/")
					if registeredSource == historicalSource
					if resource.alias == _#DefaultProviderAlias && registration.default != _|_ ||
						resource.alias != _#DefaultProviderAlias && registration.aliases[resource.alias] != _|_ {
						name
					},
				], [
					if resource.source == "terraform.io/builtin/terraform"
					if resource.alias == _#DefaultProviderAlias {"terraform"},
				]])
				if len(matches) == 1 {
					(matches[0]): (resource.alias): true
				}
				if len(matches) != 1 {
					(resource.source): (resource.alias): _|_("historical provider must match exactly one registered provider instance")
				}
			}
		}
		let requestedProviders = directProviders & historicalProviders
		let bootstrapProviders = {
			for providerName, aliases in requestedProviders if #BuiltinProviders[providerName] == _|_ {
				let registration = providerRegistry[providerName]
				let providerBlocks = [
					for alias, _ in aliases {[
						if alias == _#DefaultProviderAlias {registration.default & bootstrapContext},
						registration.aliases[alias] & {"alias": alias} & bootstrapContext,
					][0]},
				]

				for block in providerBlocks {
					for source in [
						if block.bootstrap.resource != _|_ {block.bootstrap.resource},
						if block.bootstrap.data != _|_ {block.bootstrap.data},
						if block.bootstrap.ephemeral != _|_ {block.bootstrap.ephemeral},
					]
					for sourceName, blocks in source
					for _, bootstrapBlock in blocks {
						let name = [
							if bootstrapBlock.#provider != _|_ {bootstrapBlock.#provider},
							strings.SplitN(sourceName, "_", 2)[0],
						][0]
						let alias = [
							if bootstrapBlock.#providerAlias != _|_ {bootstrapBlock.#providerAlias},
							_#DefaultProviderAlias,
						][0]
						(name): (alias): true
					}
				}
			}
		}
		let requestedProviderInstances = [
			for name, aliases in requestedProviders
			for alias, _ in aliases {
				"name":  name
				"alias": alias
			},
		]
		let bootstrapProviderInstances = [
			for name, aliases in bootstrapProviders
			for alias, _ in aliases {
				"name":  name
				"alias": alias
			},
		]
		let providerInstances = list.Concat([requestedProviderInstances, bootstrapProviderInstances])
		let selectedProviders = {
			for instance in providerInstances {
				(instance.name): (instance.alias): true
			}
		}
		let emittedBootstrapProviders = {
			for providerName, aliases in selectedProviders if #BuiltinProviders[providerName] == _|_ {
				let registration = providerRegistry[providerName]
				let providerBlocks = [
					for alias, _ in aliases {[
						if alias == _#DefaultProviderAlias {registration.default & bootstrapContext},
						registration.aliases[alias] & {provider: "alias": alias} & bootstrapContext,
					][0]},
				]

				for block in providerBlocks {
					for source in [
						if block.bootstrap.resource != _|_ {block.bootstrap.resource},
						if block.bootstrap.data != _|_ {block.bootstrap.data},
						if block.bootstrap.ephemeral != _|_ {block.bootstrap.ephemeral},
					]
					for sourceName, blocks in source
					for _, bootstrapBlock in blocks {
						let name = [
							if bootstrapBlock.#provider != _|_ {bootstrapBlock.#provider},
							strings.SplitN(sourceName, "_", 2)[0],
						][0]
						let alias = [
							if bootstrapBlock.#providerAlias != _|_ {bootstrapBlock.#providerAlias},
							_#DefaultProviderAlias,
						][0]
						(name): (alias): true
					}
				}
			}
		}
		#bootstrapDependencies: {
			for name, aliases in emittedBootstrapProviders
			for alias, _ in aliases
			if !list.Contains(providerInstances, {"name": name, "alias": alias}) {
				(name): (alias): _|_("provider bootstrap dependencies must be one level deep")
			}
		}

		for providerName, aliases in selectedProviders if #BuiltinProviders[providerName] == _|_ {
			let registration = providerRegistry[providerName]
			terraform: required_providers: (providerName): registration.requiredProvider

			let providerBlocks = [
				for alias, _ in aliases {[
					if alias == _#DefaultProviderAlias {registration.default & bootstrapContext},
					registration.aliases[alias] & {provider: "alias": alias} & bootstrapContext,
				][0]},
			]

			for block in providerBlocks {
				(_#RenderTerraformInput & {in: block.bootstrap}).out
			}
			"provider": (providerName): [for block in providerBlocks {block.provider}]
		}
	}
}

_#GenerateImports: {
	in: #TerraformInput

	out: {
		let imports = [
			for type, resources in (*in.resource | {})
			for name, block in resources if block.#import != _|_ {
				to: "\(type).\(name)"
				id: block.#import
			},
		]

		if len(imports) != 0 {
			import: imports
		}
	}
}

_#GenerateMoves: {
	in: #TerraformInput

	let histories = [
		for resourceType, resources in (*in.resource | {})
		for name, block in resources
		if block.#history != _|_ {
			let resolved = _#ResolveResourceHistory & {
				"resourceType": resourceType
				"current": {
					module: in.#module
					env:    in.#envName
					name:   name
				}
				"history": block.#history
			}

			sameCurrentStateMoves: list.Concat([resolved.sameCurrentStateMoves, [
				if len(block.#history) != 0
				if resolved.last.module == in.#module
				if resolved.last.env == in.#envName {
					from: "\(resourceType).\(resolved.last.name)"
					to:   "\(resourceType).\(name)"
				},
			]])

			crossStateTransitions: list.Concat([resolved.crossStateTransitions, [
				if len(block.#history) != 0
				if resolved.last.module != in.#module || resolved.last.env != in.#envName {
					resourceType: resourceType
					from:         resolved.last
					to: {
						module: in.#module
						env:    in.#envName
						name:   name
					}
				},
			]])
		},
	]

	let moves = list.Concat([
		for history in histories {
			history.sameCurrentStateMoves
		},
	])
	#moves: moves

	crossStateTransitions: list.Concat([
		for history in histories {
			history.crossStateTransitions
		},
	])

	out: {
		if len(moves) != 0 {
			moved: moves
		}
		...
	}
}
