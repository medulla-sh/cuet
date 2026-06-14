package google

import (
	T "github.com/medulla-sh/cuet"
)

#IngressTrafficMap: {
	all:                  "INGRESS_TRAFFIC_ALL"
	internalOnly:         "INGRESS_TRAFFIC_INTERNAL_ONLY"
	internalLoadBalancer: "INGRESS_TRAFFIC_INTERNAL_LOAD_BALANCER"
}

#CloudRunService: {
	in: {
		#import?: string
		#ignoreChanges: [...string]

		name:     string
		location: #Region
		image:    string

		project?: string
		ingress: or([for k, _ in #IngressTrafficMap {k}])
		ingress:         _ | *"internalOnly"
		serviceAccount?: string

		containerPort: int

		command?: [...string]
		args?: [...string]

		env: {[string]: string}
		secretEnv: [...{
			name:    string
			secret:  string
			version: string
			version: _ | *"latest"
		}]

		secretFiles: [...{
			name:      string
			mountPath: string
			secret:    string
			version:   string
			version:   _ | *"latest"
			fileName:  string
		}]

		minInstances:  >=0
		minInstances:  _ | *0
		maxInstances?: int
	}

	ref: "google_cloud_run_v2_service.\(in.name)"
	out: T.#TerraformInput & {
		resource: "google_cloud_run_v2_service": (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			name:     in.name
			location: in.location

			if in.project != _|_ {
				project: in.project
			}

			ingress: #IngressTrafficMap[in.ingress]

			template: {
				if in.serviceAccount != _|_ {
					service_account: in.serviceAccount
				}

				scaling: {
					min_instance_count: in.minInstances

					if in.maxInstances != _|_ {
						max_instance_count: in.maxInstances
					}
				}

				containers: [
					{
						image: in.image

						if in.command != _|_ {
							command: in.command
						}

						if in.args != _|_ {
							args: in.args
						}

						ports: [{
							container_port: in.containerPort
						}]

						env: [
							for k, v in in.env {
								name:  k
								value: v
							},
							for s in in.secretEnv {
								name: s.name
								value_source: secret_key_ref: {
									secret:  s.secret
									version: s.version
								}
							},
						]

						volume_mounts: [
							for f in in.secretFiles {
								name:       f.name
								mount_path: f.mountPath
							},
						]
					},
				]

				volumes: [
					for f in in.secretFiles {
						name: f.name
						secret: {
							secret: f.secret
							items: [{
								version: f.version
								path:    f.fileName
							}]
						}
					},
				]
			}

			if len(in.#ignoreChanges) > 0 {
				lifecycle: ignore_changes: in.#ignoreChanges
			}
		}
	}
}
