# Moving resources

Cuet records Terraform identity changes with hidden `#history` fields. It uses
native Terraform `moved` blocks for changes within one state and delegates
cross-state moves to [tfmigrate](https://github.com/minamijoyo/tfmigrate).

## Resource history

Resource history is chronological, from oldest to newest. The current resource
identity is implicitly appended to the history.

A string records a previous logical resource name:

```cue
infra: in: prod: resource: google_storage_bucket: assets: {
    #history: ["old_assets", "assets_bucket"]
    name: "assets"
}
```

Cuet infers the resource type and generates chained native `moved` blocks:

```hcl
moved {
  from = google_storage_bucket.old_assets
  to   = google_storage_bucket.assets_bucket
}

moved {
  from = google_storage_bucket.assets_bucket
  to   = google_storage_bucket.assets
}
```

A structured entry can also change the module or environment:

```cue
infra: in: prod: resource: google_storage_bucket: assets: {
    #history: [{
        module: "infra/old"
        name:   "old_assets"
    }]
    name: "assets"
}
```

Structured entries support `module`, `env`, and `name`. The first entry starts
with the current identity as its defaults. Each later entry inherits omitted
fields from the previous historical entry. A string is shorthand for an entry
that changes only `name`. Module values are workspace-relative module names;
unlike CLI targets, they do not start with `/`.

For example:

```cue
#history: [{
    module: "infra/first"
    name:   "original"
}, "renamed", {
    module: "infra/second"
}]
```

This describes the following identities:

```text
infra/first:<current-env>/original
infra/first:<current-env>/renamed
infra/second:<current-env>/renamed
<current-module>:<current-env>/<current-name>
```

Transitions entirely within the current module environment become native
`moved` blocks. The most recent transition into the current module environment
becomes a tfmigrate action.

Keep history after applying it so the identity changes remain documented and
same-state Terraform moves continue to support older state versions.

## Module history

Environment-level history records previous cuet targets for the complete state:

```cue
infra: in: prod: {
    #history: ["/infra/old:prod"]
}
```

The most recent target is the source of the pending migration. Cuet evaluates
its backend configuration from the current module, so the old module directory
does not need to remain in the workspace. The generated tfmigrate action moves
all state entries with `xmv * $1`.

Module history and individual cross-state resource histories cannot be pending
in the same environment. Individual resource moves in one migration must all
originate from the same module environment.

## Plan and apply

Select the destination module environment and plan the generated migration:

```bash
cuet -t /infra/new:prod tfmigrate plan
```

Cuet performs the following steps:

1. Evaluates the selected environment's history.
1. Exports the destination Terraform configuration.
1. Exports the source configuration or a historical backend-only configuration.
1. Writes `.cuet/<env>/tfmigrate.json` using tfmigrate's HCL JSON syntax.
1. Runs tfmigrate from the destination environment's generated directory.

The plan simulates the migration against temporary states and requires both
Terraform plans to have no changes. It does not update remote state.

Apply the migration explicitly after reviewing the plan:

```bash
cuet -t /infra/new:prod tfmigrate apply
```

Additional tfmigrate plan or apply flags are forwarded after the subcommand:

```bash
cuet -t /infra/new:prod tfmigrate plan --out=migration.tfplan
```

## Installation

Install tfmigrate with Homebrew:

```bash
brew install tfmigrate
```

Cuet uses `tfmigrate` from `PATH` by default. Use `--tfmigrate-path` to select
another binary. The configured `--tf-path` is passed through to tfmigrate, so
the migration uses the same OpenTofu or Terraform binary as other cuet commands.

tfmigrate temporarily switches each generated working directory to a local
backend. If an interrupted migration leaves `_tfmigrate_override.tf` behind,
cuet refuses further Terraform exports for that environment until the override
is removed and the remote backend is reinitialized.
