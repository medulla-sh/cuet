# Adding your First Provider

In `cuet`, providers are declared in the base infra config under
`#Terraform.providers`. Each provider entry has:

- `requiredProvider`: source/version metadata for `terraform.required_providers`
- `default`: the default provider instance
- `aliases`: optional aliased provider instances

Each provider instance has:

- `bootstrap`: Terraform input needed to make provider configuration work
- `provider`: fields rendered into the Terraform `provider` block

This page uses `google` as the first-provider example because it is standalone.
For a provider that depends on another provider (for example `neon` depending on
`google` for secrets), see
[Adding a dependent provider](adding-a-dependent-provider.md).

## Step 1: register the provider

```cue
package infra

import (
    "example.com/cuet"
)

#InfraModule: cuet.#InfraModule & {
    #Terraform: this={
        providers: {
            google: {
                requiredProvider: {
                    source:  "hashicorp/google"
                    version: "~>7.26.0"
                }
                default: provider: {
                    project: "example-\(this.#envName)"
                    region:  "us-west1"
                }
            }
        }
    }
}
```

## Step 2: reference provider resources in modules

```cue
package app

import (
    "example.com/cuet/primitives/google"
    I "example.com/infra"
)

I.#InfraModule

infra: dev: {
    (google.#Bucket & {in: {
        name:     "example-app-dev"
        location: "us-west1"
    }}).out
}
```

When a resource/data block uses the `google` provider, `cuet` emits:

- `terraform.required_providers.google`
- `provider.google` block(s)
- any `bootstrap` input required by that provider instance

## Aliases

Use aliases when one provider needs multiple credentials/configurations:

```cue
providers: google: {
    requiredProvider: {
        source:  "hashicorp/google"
        version: "~>7.26.0"
    }
    default: provider: {
        project: "example-dev"
        region:  "us-west1"
    }
    aliases: prod: provider: {
        project: "example-prod"
        region:  "us-west1"
    }
}
```

Resources can set `#providerAlias` to target a non-default instance.
