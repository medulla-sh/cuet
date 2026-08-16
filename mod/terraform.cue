package cuet

#TerraformResourceHistoryEntry: string | {
	module?: string
	env?:    string
	name?:   string
}

#TerraformResourceIdentity: {
	module: string
	env:    string
	name:   string
}

#HistoricalProvider: {
	source: string
	alias:  string
	alias:  _ | *""
}

#HistoricalResource: {
	#HistoricalProvider
	address: string
}

#TerraformInput: {
	#module: string
	#backendConfigs: [string]: [string]: {...}
	#backendConfigs: _ | *{}
	#envName: string
	#env:     _
	#historicalResources: [...#HistoricalResource]
	#historicalResources: _ | *[]
	#provider?:      string
	#providerAlias?: string
	#import?:        string
	#history?: [...string]

	resource?: [string]: [string]: {
		#history?: [...#TerraformResourceHistoryEntry]
		...
	}

	data?: [string]: [string]:      _
	ephemeral?: [string]: [string]: _

	variable?: [string]: {
		type?:        string
		default?:     _
		description?: string
		sensitive?:   bool
	}

	output?: [string]: {
		value:        _
		description?: string
		sensitive?:   bool
	}

	locals?: [string]: _
}

#TerraformOutput: {
	#crossStateTransitions: [...{
		resourceType: string
		from:         #TerraformResourceIdentity
		to:           #TerraformResourceIdentity
	}]
	#crossStateTransitions: _ | *[]

	terraform?: {
		required_version?: string
		required_providers?: [string]: {
			source?:  string
			version?: string
		}
		backend?: [string]: {...}
	}

	provider?: [string]: [...{...}]

	resource?: [string]: [string]: {...}

	data?: [string]: [string]: {...}
	ephemeral?: [string]: [string]: {...}

	variable?: [string]: {
		type?:        string
		default?:     _
		description?: string
		sensitive?:   bool
	}

	output?: [string]: {
		value:        _
		description?: string
		sensitive?:   bool
	}

	locals?: [string]: _

	import?: [...{
		to: string
		id: string
	}]

	moved?: [...{
		from: string
		to:   string
	}]
}
