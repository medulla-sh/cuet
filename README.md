# cuet

[![CLI CI](https://github.com/medulla-sh/cuet/actions/workflows/cli-ci.yml/badge.svg)](https://github.com/medulla-sh/cuet/actions/workflows/cli-ci.yml)
[![Module CI](https://github.com/medulla-sh/cuet/actions/workflows/mod-ci.yml/badge.svg)](https://github.com/medulla-sh/cuet/actions/workflows/mod-ci.yml)
[![YAML CI](https://github.com/medulla-sh/cuet/actions/workflows/yaml-ci.yml/badge.svg)](https://github.com/medulla-sh/cuet/actions/workflows/yaml-ci.yml)
[![CLI Release](https://github.com/medulla-sh/cuet/actions/workflows/cli-release.yml/badge.svg)](https://github.com/medulla-sh/cuet/actions/workflows/cli-release.yml)
[![Module Release](https://github.com/medulla-sh/cuet/actions/workflows/mod-release.yml/badge.svg)](https://github.com/medulla-sh/cuet/actions/workflows/mod-release.yml)

`cuet` is a framework + CLI for modeling deployment configuration in CUE,
keeping application deployment concerns close to each app while centralizing
platform policy and standards.

Today, the primary execution backend is Terraform/OpenTofu.

At a high level:

- `cuet.#InfraModule` provides the schema and generation pipeline.
- Primitive building blocks under `cuet/primitives/` generate deployment input.
- The `cuet` CLI evaluates a target module and writes
  `.cuet/<env>/main.tf.json`.

## Install

Install the CLI with Homebrew:

```bash
brew tap medulla-sh/tap
brew install cuet
```

Install dependencies:

```bash
brew install cue-lang/tap/cue opentofu
```

<details>
<summary>Using your own CUE or Terraform/OpenTofu binaries</summary>

`cuet` expects `cue` and `tofu` to be available on `PATH`, but does not install
or vendor them. Manage those tools with Homebrew, Nix, mise, asdf, manual
downloads, or your team's existing toolchain.

Use `--cue-path` or `--tf-path` to point at alternate binaries, including
Terraform instead of OpenTofu.

</details>

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
