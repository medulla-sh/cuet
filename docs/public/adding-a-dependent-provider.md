# Adding a dependent provider

Some providers need values that come from other providers. A common pattern is
`neon` using a Google Secret Manager value for its API key.

In `cuet`, this is what `bootstrap` is for: you can emit extra Terraform input
that must exist before rendering the provider block.

## Example: `neon` depends on `google`

```cue
package infra

import (
    "example.com/cuet"
    G "example.com/cuet/primitives/google"
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
            neon: {
                requiredProvider: {
                    source:  "kislerdm/neon"
                    version: "~>0.13"
                }
                default: {
                    let neonSecret = G.#SecretVersion & {
                        in: secretId: "neon-\(this.#envName)"
                    }
                    bootstrap: {
                        neonSecret.out
                    }
                    provider: {
                        api_key: "\(neonSecret.ref).secretData"
                    }
                }
            }
        }
    }
}
```

## Why this works

- `bootstrap` contributes Terraform input (in this case a Google data source).
- The generated provider block can then reference the bootstrapped value.
- `cuet` still emits normal Terraform JSON, so execution stays in
  `tofu`/`terraform`.

## Tips

- Keep dependency chains shallow and explicit.
- Put provider wiring in your shared base infra module, not app modules.
- Use clear naming for secret IDs across environments (for example, `neon-dev`,
  `neon-staging`, `neon-prod`).
