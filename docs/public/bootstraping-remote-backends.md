# Bootstraping remote backends

When you create a brand new remote backend (for example, a new GCS bucket),
`tofu init` cannot use that backend until the backend resource exists.

Use `--use-local-backend` to bootstrap safely.

## Workflow

1. Define backend resources in your module/base config.
1. Run `cuet` with local backend override.
1. Apply once to create backend infrastructure.
1. Re-run `init` without override and migrate state.

## Commands

```bash
# 1) Local state bootstrap
cuet -t infra/backends:dev --use-local-backend tf init
cuet -t infra/backends:dev --use-local-backend tf apply

# 2) Switch to remote backend
cuet -t infra/backends:dev tf init -migrate-state
```

`--use-local-backend` makes `cuet` inject a local backend path (`local.tfstate`)
for that run, instead of your configured remote backend.

## Why this exists

It solves the chicken-and-egg problem where the Terraform state backend is
managed by Terraform itself.
