# Getting Started

This guide walks through the minimum setup to evaluate a module and run the
current execution backend (OpenTofu/Terraform).

## Prerequisites

- `cue` available on your `PATH`
- `tofu` (or Terraform via `--tf-path`) available on your `PATH`
- A repository with a `.cuetroot.cue` marker at the workspace root
- A module CUE file that composes your base infra module

## Minimal module

```cue
package neon

import (
    "example.com/cuet/primitives/google"
    "example.com/cuet/primitives/neon"
    I "example.com/infra"
)

I.#InfraModule

infra: {
    dev: {
        (google.#Secret & {in: secretId: "neon-dev"}).out
        (neon.#ApiKey & {in: name: "neon-dev"}).out
    }
}
```

## Run the module

```bash
cuet -p infra/neon dev tf init
cuet -p infra/neon dev tf plan
```

`cuet` will:

1. Evaluate the selected module and environment.
1. Write `.cuet/dev/main.tf.json` under the module directory.
1. Execute `tofu` in that generated directory.

## Useful commands

```bash
# Inspect evaluated output for an environment
cuet -p infra/neon dev cue export

# Run with verbose logs
cuet -v -p infra/neon dev tf plan

# Override tool locations
cuet --cue-path /path/to/cue --tf-path /path/to/tofu -p infra/neon dev tf plan
```
