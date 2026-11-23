package google

import (
	"strings"

	T "github.com/medulla-sh/cuet"
)

_OrgPolicyConfig: {
	#import?: string

	// Override the generated Terraform label to retain an existing address.
	name?: string

	({
		enforce: bool
		list?:   _|_
	} | {
		enforce?: _|_
		list: {
			({
				denyAll:        true
				allowedValues?: _|_
				deniedValues?:  _|_
			} | {
				denyAll?: _|_
				allowedValues: [...string & !=""]
				deniedValues: [...string & !=""]
				if len(allowedValues)+len(deniedValues) == 0 {
					_|_("a list policy must deny all or specify allowed or denied values")
				}
			})
		}
	})
}

#OrgPolicy: {
	in: {
		_OrgPolicyConfig
		#dependsOn?: [...string]

		name:       _ | *strings.Replace(constraint, ".", "-", -1)
		parent:     =~"^(projects|folders|organizations)/.+$"
		constraint: string & !=""
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
				if in.enforce != _|_ {
					enforce: strings.ToUpper("\(in.enforce)")
				}
				if in.list != _|_ {
					if in.list.denyAll != _|_ {
						deny_all: "TRUE"
					}
					if in.list.denyAll == _|_ {
						if len(in.list.allowedValues) > 0 {
							values: allowed_values: in.list.allowedValues
						}
						if len(in.list.deniedValues) > 0 {
							values: denied_values: in.list.deniedValues
						}
					}
				}
			}
		}
	}
}
