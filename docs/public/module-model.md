# Module model

This page describes the main objects in `cuet` and how they relate.

## Core objects

- **Module**: a directory with a required `cuet.cue` marker whose CUE package
  embeds `I.#InfraModule` and defines environment-specific infra.
- **Environment**: a named deployment target (for example `dev`, `internal`,
  `global`) declared by the base infra config.
- **Input graph** (`infra.in`): per-environment deployment input assembled from
  primitives.
- **Generated graph** (`infra.generated`): backend-specific deployment object
  before output policies are applied.
- **Output graph** (`infra.out`): final deployment envelope per environment.
  Terraform/OpenTofu output is stored under its `terraform` backend key after
  output policies are applied.
- **Platform policy**: fleet-wide constraints applied centrally (for example in
  `infra/policy.cue`).

## Data flow

`cuet` evaluates a module by injecting metadata (`module`,
`localBackendOverride`) into the module's `infra` object, then reading
`infra.out[env].terraform`.

At a high level:

1. Base config defines environments and backend defaults.
1. Module writes environment input under `infra.in`.
1. Framework transforms `infra.in[env]` into `infra.generated[env]`.
1. Framework applies `#OutputPolicy` to produce
   `infra.out[env].terraform`.
1. CLI exports that output to `.cuet/<env>/main.tf.json`.

## Removing environments

Generated `.cuet/<env>` directories also identify environments that may retain
managed objects after their input is removed from `infra.in`. The CLI includes
initialized directory names in environment selection so remaining objects can
be destroyed. Keep provider configurations used by removed objects available
until Terraform/OpenTofu has destroyed those objects.

After a historical environment has no resources, data sources, or outputs, the
CLI removes its local `.cuet/<env>` directory. It does not delete the empty
state snapshot from a remote backend; backend-specific cleanup belongs outside
Cuet.

Selecting a historical environment whose state is already empty removes its
stale local directory and exits without running the requested Terraform command.

## Shape of a module

```cue
package example

import I "example.com/infra"

I.#InfraModule

infra: in: {
    dev: {
        // primitives and terraform input here
    }
    global: {
        // primitives and terraform input here
    }
}
```

## Notes

- Other frameworks may compose their backend output beside `terraform` in the
  environment envelope.
- `infra.generated` is useful for debugging raw generated output before output
  policies run.
- `infra.#metadata` is CLI-injected framework context and should generally be
  treated as internal plumbing.
- Terraform/OpenTofu is the current backend in this repository.
- Cross-environment dependencies (for example remote state reads) should be
  explicit in module input.
- Cross-module dependencies should flow through producer `output` values and
  consumer `terraform.#RemoteVar` reads (often described as `module:env` in
  prose, for example `auth/anubis:dev`).
