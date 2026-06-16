package terraform

import (
	"strings"
	T "github.com/medulla-sh/cuet"
)

// #RemoteState adds a terraform_remote_state data source for a target module
// and environment. It defaults to the current module/environment when omitted.
#RemoteState: {
	in: {
		module?: string
		env?:    string
	}

	ref: "data.terraform_remote_state.\(out._remoteStateName)"
	out: this=T.#TerraformInput & {
		let sourceEnv = *in.env | this.#envName
		let sourceModule = *in.module | this.#module
		_remoteStateName: "\(strings.Replace(sourceModule, "/", "-", -1))-\(sourceEnv)"
		let backendConfig = this.#backendConfigs[sourceEnv]
		let backendName = [for k, _ in backendConfig {k}][0]

		data: terraform_remote_state: (_remoteStateName): {
			backend: backendName
			config: {
				for k, v in backendConfig[backendName] if k != "prefix" {
					(k): v
				}
				prefix: sourceModule
			}
		}
	}
}

// #RemoteVar reads a single output key from a remote state reference and
// exposes it as a Terraform interpolation string in `ref`.
#RemoteVar: {
	in: {
		module?: string
		env?:    string
		key:     string
	}

	ref: #"${data.terraform_remote_state.\#(out._remoteStateName).outputs["\#(in.key)"]}"#
	out: this=T.#TerraformInput & {
		let sourceEnv = *in.env | this.#envName
		let sourceModule = *in.module | this.#module
		_remoteStateName: "\(strings.Replace(sourceModule, "/", "-", -1))-\(sourceEnv)"
		let backendConfig = this.#backendConfigs[sourceEnv]
		let backendName = [for k, _ in backendConfig {k}][0]
		data: terraform_remote_state: (_remoteStateName): {
			backend: backendName
			config: {
				for k, v in backendConfig[backendName] if k != "prefix" {
					(k): v
				}
				prefix: sourceModule
			}
		}
	}
}
