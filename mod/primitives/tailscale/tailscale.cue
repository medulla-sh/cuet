package tailscale

import (
	"encoding/json"

	T "github.com/medulla-sh/cuet"
)

#PolicyFile: {
	// Tailscale policy files support more top-level sections than we use today.
	// Keep this open so callers can adopt new sections without changing cuet.
	tagOwners?: [string]: [...string]
	grants?: [...{
		src: [...string]
		dst: [...string]
		ip?: [...string]
		...
	}]
	...
}

#OAuthClient: {
	in: {
		#import?: string

		name: string

		description: string
		description: _ | *name

		scopes: [string, ...string]

		tags: [...string]
		tags: _ | *[]
	}

	ref: "tailscale_oauth_client.\(in.name)"

	out: T.#TerraformInput & {
		resource: tailscale_oauth_client: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			description: in.description
			scopes:      in.scopes

			if len(in.tags) > 0 {
				tags: in.tags
			}
		}
	}
}

#Acl: {
	in: {
		#import?: string

		name: string

		policy: #PolicyFile

		overwriteExistingContent: bool
		overwriteExistingContent: _ | *false

		resetAclOnDestroy: bool
		resetAclOnDestroy: _ | *false
	}

	ref: "tailscale_acl.\(in.name)"

	out: T.#TerraformInput & {
		resource: tailscale_acl: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			acl: json.Marshal(in.policy)

			if in.overwriteExistingContent {
				overwrite_existing_content: in.overwriteExistingContent
			}

			if in.resetAclOnDestroy {
				reset_acl_on_destroy: in.resetAclOnDestroy
			}
		}
	}
}

#TailnetSettings: {
	in: {
		#import?: string

		name: string

		httpsEnabled: bool
	}

	ref: "tailscale_tailnet_settings.\(in.name)"

	out: T.#TerraformInput & {
		resource: tailscale_tailnet_settings: (in.name): {
			if in.#import != _|_ {
				#import: in.#import
			}

			https_enabled: in.httpsEnabled
		}
	}
}
