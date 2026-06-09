package terraform

import (
	"strings"
	T "github.com/medulla-sh/cuet"
)

// #RemoteState adds a terraform_remote_state data source for a target module
// and environment. It defaults to the current module/environment when omitted.
#RemoteState: this=T.#TerraformInput & {
	in: {
		module?: string
		env?:    string
	}
	let sourceEnv = *in.env | this.#envName
	let sourceModule = *in.module | this.#module
	let remoteStateName = "\(strings.Replace(sourceModule, "/", "-", -1))-\(sourceEnv)"
	let backendConfig = *this.#backendConfigs[sourceEnv] | {
		gcs: {
			bucket: "medulla-tf-\(sourceEnv)"
			prefix: sourceModule
		}
	}
	let backendName = [for k, _ in backendConfig {k}][0]

	ref: "data.terraform_remote_state.\(remoteStateName)"
	out: T.#TerraformInput & {
		data: terraform_remote_state: (remoteStateName): {
			backend: backendName
			config:  backendConfig[backendName]

			if backendName == "gcs" {
				config: prefix: sourceModule
			}
		}
	}
}

// #RemoteVar reads a single output key from a remote state reference and
// exposes it as a Terraform interpolation string in `ref`.
#RemoteVar: this=T.#TerraformInput & {
	in: {
		// TODO(LUM-10): Make this optional
		module: string
		env?:   string
		key:    string
	}
	let sourceEnv = *in.env | this.#envName
	let sourceModule = in.module
	let remoteStateName = "\(strings.Replace(sourceModule, "/", "-", -1))-\(sourceEnv)"
	let backendConfig = *this.#backendConfigs[sourceEnv] | {
		gcs: {
			bucket: "medulla-tf-\(sourceEnv)"
			prefix: sourceModule
		}
	}
	let backendName = [for k, _ in backendConfig {k}][0]

	ref: #"${data.terraform_remote_state.\#(remoteStateName).outputs["\#(in.key)"]}"#
	out: T.#TerraformInput & {
		data: terraform_remote_state: (remoteStateName): {
			backend: backendName
			config:  backendConfig[backendName]

			if backendName == "gcs" {
				config: prefix: sourceModule
			}
		}
	}
}
