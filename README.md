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

Verify the installed version with:

```bash
cuet --version
```

Homebrew installs completions for Bash, Fish, and Zsh. For other installation
methods, generate completions for your shell with:

```
cuet completions <bash|elvish|fish|powershell|zsh>
```

For example, load completions for the current Bash session with:

```bash
source <(cuet completions bash)
```

Target completion discovers modules from the current cuet workspace. In Zsh,
selecting a module adds a temporary `:`: continue typing to complete its
populated environments, or type a space to remove the `:`. Other shells complete
the bare module; type `:` before requesting environment completion. Discovery
failures are ignored, so normal shell completion remains usable outside a
workspace.

Install the required development tools from the repository `Brewfile`:

```bash
brew bundle
```

Alternatively, install `just`, CUE, tfmigrate, and OpenTofu with another package
manager and ensure they are available on `PATH`.

<details>
<summary>Using your own CUE or Terraform/OpenTofu binaries</summary>

`cuet` expects `cue` and `tofu` to be available on `PATH`, and migration commands
also require `tfmigrate`. The CLI does not install or vendor these tools. Manage
them with Homebrew, Nix, mise, asdf, manual downloads, or your team's existing
toolchain.

Use `--cue-path` or `--tf-path` to point at alternate binaries, including
Terraform instead of OpenTofu. Use `--tfmigrate-path` to select an alternate
`tfmigrate` binary.

</details>

## Public docs

Start here for guides and examples:

- [`cuet/docs/public/README.md`](docs/public/README.md)
- [`cuet/docs/public/SUMMARY.md`](docs/public/SUMMARY.md)
- [`cuet/docs/public/module-model.md`](docs/public/module-model.md)
- [`cuet/docs/public/module-variables.md`](docs/public/module-variables.md)
- [`cuet/docs/public/moving-resources.md`](docs/public/moving-resources.md)

## Quick example

Each module directory has a marker file named `cuet.cue`. List all modules in
the current workspace with:

```bash
cuet modules list
cuet modules check
```

`modules check` validates every populated environment with CUE. It does not
require OpenTofu or cloud credentials.

```bash
cuet -t infra/neon:dev tf plan
```

This will:

1. Evaluate the CUE module at `infra/neon` for `dev`.
1. Generate `.cuet/dev/main.tf.json`.
1. Run `tofu init` in that generated folder.
1. Run `tofu plan` after initialization succeeds.

Every `cuet tf` command initializes the generated working directory when it
does not contain OpenTofu backend metadata. Explicit `cuet tf init` commands
run only the requested initialization, so backend migration and reconfiguration
options remain under your control.

To bootstrap a backend for a new environment, use local state first:

```bash
cuet -t infra/neon:dev --use-local-backend tf init
```
