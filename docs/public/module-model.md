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

## Explicit resource dependencies

Use `#dependsOn` when a prerequisite is not captured by an attribute reference,
such as enabling an API before creating resources through it. The framework
accepts this metadata on resource, data, and ephemeral blocks and renders it as
Terraform `depends_on`. Supporting primitives expose it through their `in`
fields:

```cue
let scannerAccount = google.#ServiceAccount & {in: {
    #dependsOn: [scannerProject.refs.services["iam.googleapis.com"]]
    accountId: "audit-scanner"
    project: {
        name: scannerProject.in.name
        id: "${\(scannerProject.refs.project).project_id}"
    }
}}
scannerAccount.out
```

`google.#Project.refs.project` names the project resource; `ref` remains a
deprecated compatibility alias. Its
`refs.services[apiName]` entries name the API-enablement resources for its
`enabledServices`. Depending on the project alone does not wait for API
enablement.

Dependencies are resource or data-source addresses in the same Terraform
configuration, without `${...}` interpolation. When `#dependsOn` is present,
the renderer combines it with any existing `depends_on`, removes duplicates,
and sorts the addresses. Empty metadata emits no dependency attribute unless
the block already explicitly declares one. Raw `depends_on` is preserved
unchanged when metadata is absent.

Primitive authors forward dependencies to the resource that requires them;
they do not automatically apply them to every generated resource or project
lookup. Normal attribute references continue to establish implicit dependencies.
Terraform/OpenTofu validates dependency targets and cycles. This mechanism does
not schedule separate modules or environments, and API propagation retries
remain the provider's responsibility.

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
