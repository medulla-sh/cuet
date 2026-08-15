@if(test)

package terraform

#RemoteStateTests: {
	"different-sources-compose": {
		#context: {
			#module:  "infra/ci"
			#envName: "internal"
			#env: {}
			#backendConfigs: {
				dev: gcs: {
					bucket: "oakmont-tf-dev"
					prefix: "infra/ci"
				}
				global: gcs: {
					bucket: "oakmont-tf-global"
					prefix: "infra/ci"
				}
			}
			...
		}
		first: #RemoteVar & {in: {
			module: "infra/backend"
			env:    "dev"
			key:    "bucket_name"
		}}
		first: out: #context
		second: #RemoteVar & {in: {
			module: "infra/github"
			env:    "global"
			key:    "organization_id"
		}}
		second: out: #context

		merged: {
			first.out
			second.out
		}

		assert: first.ref == #"${data.terraform_remote_state.infra-backend-dev.outputs["bucket_name"]}"#
		assert: second.ref == #"${data.terraform_remote_state.infra-github-global.outputs["organization_id"]}"#
		assert: merged.data.terraform_remote_state["infra-backend-dev"].config == {
			bucket: "oakmont-tf-dev"
			prefix: "infra/backend"
		}
		assert: merged.data.terraform_remote_state["infra-github-global"].config == {
			bucket: "oakmont-tf-global"
			prefix: "infra/github"
		}
	}
}

remoteStateResult: [for _, test in #RemoteStateTests {test.assert & true}]
