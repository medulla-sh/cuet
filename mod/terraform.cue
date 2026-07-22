package cuet

#TerraformInput: {
	#module: string
	#backendConfigs: [string]: [string]: {...}
	#backendConfigs: _ | *{}
	#envName:        string
	#env:            _
	#provider?:      string
	#providerAlias?: string
	#import?:        string

	resource?: [string]: [string]: {
		#history?: [...string]
		...
	}

	data?: [string]: [string]: _

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
