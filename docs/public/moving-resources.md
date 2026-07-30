# Moving resources

Cuet records Terraform identity changes with hidden `#history` fields. It uses
native Terraform `moved` blocks for changes within one state, delegates partial
cross-state moves to [tfmigrate](https://github.com/minamijoyo/tfmigrate), and
uses native backend migration for complete state moves.

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

The most recent target is the source of the pending migration. Cuet exports the
complete current configuration, evaluates the historical backend separately,
and substitutes only the root `terraform.backend` value in its temporary copy.
This preserves current provider constraints and resource identities, and means
the old module directory does not need to remain in the workspace. OpenTofu's
native backend migration copies the complete state snapshot, including
resources and root outputs.

Module history and individual cross-state resource histories cannot be pending
in the same environment. Individual resource moves in one migration must all
originate from the same module environment.

## Plan and apply

After moving a complete module directory, check the repository layout:

```bash
cuet -t /infra/new:prod migrate check
```

For module history, the check requires a tracked provider lock to move from
`.cuet/<old-env>/.terraform.lock.hcl` under the historical module to the
equivalent destination path. Providerless modules may have no lock. It also
rejects stale tfmigrate artifacts and validates the current Terraform and
historical backend shapes. Only the root backend is replaced by construction.
The check is local-only and does not require OpenTofu or cloud credentials.

Inspect structured migration details for CI finalization:

```bash
cuet -t /infra/new:prod migrate inspect
```

The JSON output includes source and destination module identities,
environments, safe backend location fields, and lockfile paths. Each endpoint
includes `backendLocationComplete`; unknown or secret-bearing backend shapes
are intentionally incomplete and require backend-specific configuration.

Plan the migration against remote state:

```bash
cuet -t /infra/new:prod migrate plan
```

For module history, Cuet performs the following steps:

1. Evaluates the selected environment's history.
1. Initializes a clean generated directory against the historical backend.
1. Exports the destination configuration with a temporary local backend.
1. Uses `tofu init -migrate-state` to copy the complete source snapshot locally.
1. Requires the destination configuration to produce a no-change plan.

The plan locks and reads the source state but does not update either remote
backend.

For resource history, Cuet exports the source and destination configurations
and runs tfmigrate. The tfmigrate plan simulates the selected resource moves
against temporary states and requires both Terraform plans to have no changes.

Apply the migration explicitly after reviewing the plan:

```bash
cuet -t /infra/new:prod migrate apply
```

Module apply repeats the local preflight, then uses interactive
`tofu init -migrate-state` to copy the complete snapshot to the destination.
OpenTofu attempts to lock both states during the copy. Cuet verifies that the
destination contains the source lineage and serial before reporting success.
Complete module migration currently requires the backend to contain exactly the
default workspace because native backend migration would otherwise copy
unvalidated workspaces. The historical backend remains unchanged for recovery;
prevent concurrent state operations during the migration and applies through
the historical module afterward so the states cannot diverge.

Before applying, Cuet requires the destination state to be empty or to exactly
match the complete source snapshot. A matching snapshot is treated as already
migrated and only receives a no-change plan. Any other non-empty destination
fails closed. State-changing module migration remains interactive: OpenTofu has
no backend-agnostic atomic compare-and-swap operation that would make forced
noninteractive overwrite safe. A protected CI job can run the command with an
attached approval terminal, but Cuet does not bypass OpenTofu's confirmation.

Native backend migration copies state because OpenTofu has no backend-agnostic
source deletion command. Use `migrate inspect` to drive a backend-specific
post-copy cleanup step. For versioned object storage, remove only the old live
object and retain historical versions for recovery. Once cleanup removes the
source live state, further migration plan/apply commands fail closed because
Cuet can no longer independently prove the relationship between source and
destination; the backend-specific finalizer should treat source absence as its
own completion marker.

Additional tfmigrate flags remain available for resource migrations:

```bash
cuet -t /infra/new:prod migrate plan --out=migration.tfplan
```

Resource apply delegates the selected moves to tfmigrate.

## Installation

Install tfmigrate with Homebrew:

```bash
brew install tfmigrate
```

Partial cross-state resource moves require `tfmigrate` on `PATH`. Use
`--tfmigrate-path` to select another binary. Complete module moves do not
require tfmigrate. The configured `--tf-path` selects the OpenTofu or Terraform
binary used by either migration type.

tfmigrate temporarily switches each generated working directory to a local
backend. If an interrupted migration leaves `_tfmigrate_override.tf` behind,
cuet refuses further Terraform exports for that environment until the override
is removed and the remote backend is reinitialized.
