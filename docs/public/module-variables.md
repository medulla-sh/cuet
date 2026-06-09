# Module variables

This page documents framework-provided variables available while authoring a
module.

## In `infra.in.<env>` blocks

Inside an environment block (for example `dev: this={ ... }`), these hidden
fields are available via `this`:

- `#envName` (`string`): current environment name.
- `#env` (`_`): environment payload from `#Environments[envName]`.

Example:

```cue
infra: in: {
    dev: this={
        resource: my_resource: example: {
            name: "app-\(this.#envName)"
        }
    }
}
```

## In primitive/generator context (`T.#TerraformInput`)

When a primitive embeds `T.#TerraformInput`, these hidden fields are available
during framework generation:

- `#envName`: current environment name.
- `#env`: current environment payload.
- `#module`: current module path (for example `auth/anubis`).
- `#backendConfigs`: backend configs indexed by environment.

These are primarily for framework-aware primitives (for example remote-state
helpers), not typical module resource definitions.

## Practical guidance

- Use `#envName` for environment-specific naming.
- Keep dependence on hidden fields minimal in app modules.
- Prefer explicit values for cross-environment references when readability is
  better than implicit defaults.

## Cross-module outputs and `RemoteVar`

When one module needs a value from another module, use Terraform outputs from
the source module and consume them with `terraform.#RemoteVar`.

- Source module should define an explicit `output` key for the shared value.
- Consumer module should read that output with explicit `module` and `env`
  fields.
- Do not recreate source-owned resources in consumer modules just to reference
  their IDs.

In prose, docs may refer to a source as `module:env` (for example
`auth/anubis:dev`). Current CUE examples keep `module` and `env` separate; a
direct `module:env` encoding in CUE is also acceptable if/when adopted:

```cue
let jwtSecretId = terraform.#RemoteVar & {in: {
    module: "auth/anubis"
    env:    "dev"
    key:    "jwt_secret_id"
}}
```
