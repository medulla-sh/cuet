# Why Cuet?

`cuet` is intended to be a deployment control plane, not just a Terraform
wrapper. It keeps app-level deployment concerns close to application modules,
while platform-level policy stays centralized.

Terraform/OpenTofu is the current execution backend, and `cuet` layers CUE
composability and constraints on top.

## What you get

- **Typed module contract**: `cuet.#InfraModule` defines the shape for
  environments, provider registry, policy application, and module input/output.
- **Composable primitives**: reusable CUE definitions emit backend-specific
  deployment fragments.
- **Provider ergonomics**: provider setup can include both provider block values
  and bootstrap Terraform input.
- **Environment-aware generation**: each environment is generated independently
  from one source module.
- **No custom runtime lock-in**: backend output remains native to the executor.
  Today that means Terraform JSON consumed by `tofu`/`terraform`.

## Mental model

1. Define shared policy in a base infra config (environments, backend,
   providers).
1. Compose primitives in app/module CUE files.
1. Run `cuet ... tf <command>`.
1. `cuet` exports `main.tf.json` and forwards to `tofu`.

You keep standard Terraform operations (`init`, `plan`, `apply`) today, while
building toward one place to model all deployment concerns.
