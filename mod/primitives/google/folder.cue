package google

import T "github.com/medulla-sh/cuet"

#Folder: {
	in: {
		#import?: string
		#dependsOn?: [...string]

		name:        string
		displayName: string
		displayName: _ | *name
		// Accept resource names and Terraform expressions for nested folders.
		parent: string & !=""

		deletionProtection: bool
		deletionProtection: _ | *true
	}

	ref: "google_folder.\(in.name)"
	out: T.#TerraformInput & {
		resource: google_folder: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}
			if in.#dependsOn != _|_ {
				#dependsOn: in.#dependsOn
			}
			display_name:        in.displayName
			parent:              in.parent
			deletion_protection: in.deletionProtection
		}
	}
}
