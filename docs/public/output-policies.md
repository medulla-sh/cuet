# Output policies

Output policies are post-generation transforms for Terraform/OpenTofu output.
They let platform teams apply fleet-wide constraints after module primitives have
finished composing their raw input.

Use output policies for backend-specific rules such as:

- Adding required fields to generated resources.
- Adding supporting data sources when matching resources exist.
- Enforcing provider-specific safety defaults.

## Why output policies exist

Primitive composition happens in `infra.in`. Policies that inspect `infra.in` can
accidentally participate in field-set construction while primitives are still
being unified. That is hard to reason about and can create cyclic or order-
sensitive behavior.

Output policies run after `cuet` has generated concrete backend output for an
environment. At that point, resources are already shaped as Terraform/OpenTofu
JSON, so policies can inspect and transform concrete resource maps.

## Contract

`#OutputPolicy` is a transform with explicit input and output fields:

```cue
#OutputPolicy: {
    in:  cuet.#TerraformOutput
    out: cuet.#TerraformOutput
}
```

If a module does not configure an output policy, `cuet` uses a no-op policy:

```cue
#OutputPolicy: {
    in:  cuet.#TerraformOutput
    out: in
}
```

`cuet.#InfraModule` evaluates output in two stages:

1. `infra.generated[env]` is the raw Terraform/OpenTofu output generated from
   `infra.in[env]`.
1. `infra.out[env].terraform` is the result of applying `#OutputPolicy` to
   `infra.generated[env]`.

Conceptually:

```cue
infra: out: {
    for env, _ in #Environments {
        (env): terraform: (#OutputPolicy & {in: infra.generated[env]}).out
    }
}
```

## Configuring Policies

Base infrastructure configuration can provide a policy implementation by
setting `#OutputPolicy`:

```cue
package infra

import "github.com/medulla-sh/cuet"

#InfraModule: cuet.#InfraModule & {
    #Environments: {
        dev: _
        prod: _
    }

    #Terraform: {
        // backend and providers
    }

    #OutputPolicy: #Policies
}
```

## Example

This policy sets uniform bucket-level access on every generated Google Cloud
Storage bucket:

```cue
package infra

import "github.com/medulla-sh/cuet"

#Policies: {
    in: cuet.#TerraformOutput

    out: in & #GoogleCloudBucketsMustHaveUniformAccess
}

#GoogleCloudBucketsMustHaveUniformAccess: {
    ["resource"]: {
        ["google_storage_bucket"]: [string]: {
            uniform_bucket_level_access: true
            ...
        }
        ...
    }
    ...
}
```

## Conditional Supporting Data

Policies can also add supporting data sources only when a matching resource type
exists. Iterate over `in.resource` rather than directly probing a field during
input composition.

```cue
#AllProjectsMustUseBillingAccount: this={
    let billingAccountId = "000000-000000-000000"

    for resourceType, _ in (*this["resource"] | {}) if resourceType == "google_project" {
        data: google_billing_account: default: {
            billing_account: billingAccountId
        }
    }

    ["resource"]: {
        ["google_project"]: [string]: {
            billing_account: "${data.google_billing_account.default.id}"
            ...
        }
        ...
    }
    ...
}
```

## Guidelines

- Keep policies backend-specific. Terraform resource-shape policies should run
  as output policies.
- Prefer output policies for rules that inspect `resource` or `data` maps.
- Keep each policy as a pure transform from `in` to `out`.
- Use bracket notation and `...` when constraining resource maps so policies do
  not accidentally close unrelated resource types.
- Avoid reading or mutating `infra.in` from output policies. Use the generated
  `in` value passed to the policy.
