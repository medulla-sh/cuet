# cuet

`cuet` is a framework + CLI for modeling deployment configuration in CUE,
keeping application deployment concerns close to each app while centralizing
platform policy and standards.

Today, the primary execution backend is Terraform/OpenTofu.

At a high level:

- `cuet.#InfraModule` provides the schema and generation pipeline.
- Primitive building blocks under `cuet/primitives/` generate deployment input.
- The `cuet` CLI evaluates a target module and writes
  `.cuet/<env>/main.tf.json`.

## Public docs

Start here for guides and examples:

- [`cuet/docs/public/README.md`](docs/public/README.md)
- [`cuet/docs/public/SUMMARY.md`](docs/public/SUMMARY.md)
- [`cuet/docs/public/module-model.md`](docs/public/module-model.md)
- [`cuet/docs/public/module-variables.md`](docs/public/module-variables.md)

## Quick example

```bash
cuet -p infra/neon dev tf init
cuet -p infra/neon dev tf plan
```

This will:

1. Evaluate the CUE module at `infra/neon` for `dev`.
1. Generate `.cuet/dev/main.tf.json`.
1. Run `tofu init` or `tofu plan` in that generated folder.

To bootstrap a backend for a new environment, use local state first:

```bash
cuet -p infra/neon dev --use-local-backend tf init
```
