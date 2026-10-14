package google

import T "github.com/medulla-sh/cuet"

#OrgPolicy: {
	in: {
		#import?: string
		#dependsOn?: [...string]

		name:       string
		parent:     =~"^(projects|folders|organizations)/.+$"
		constraint: string & !=""
		enforce:    bool
	}

	ref: "google_org_policy_policy.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_org_policy_policy: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			if in.#dependsOn != _|_ {
				#dependsOn: in.#dependsOn
			}
			name:   "\(in.parent)/policies/\(in.constraint)"
			parent: in.parent
			spec: rules: {
				if in.enforce {
					enforce: "TRUE"
				}
				if !in.enforce {
					enforce: "FALSE"
				}
			}
		}
	}
}
