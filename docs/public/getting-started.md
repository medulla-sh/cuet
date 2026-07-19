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
cuet -t infra/neon:dev tf init
cuet -t infra/neon:dev tf plan
```

`cuet` will:

1. Evaluate the selected module and environment.
1. Write `.cuet/dev/main.tf.json` under the module directory.
1. Execute `tofu` in that generated directory.

The module target is relative to the current directory. Start it with `/` to
resolve it from the cuet workspace root, or set the exact workspace root with
`-w`:

```bash
cuet -t /infra/neon:dev tf plan
cuet -w /path/to/workspace -t /infra/neon:dev tf plan
```

## Useful commands

```bash
# Inspect evaluated output for an environment
cuet -t infra/neon:dev cue export

# Run with verbose logs
cuet -v -t infra/neon:dev tf plan

# Override tool locations
cuet --cue-path /path/to/cue --tf-path /path/to/tofu -t infra/neon:dev tf plan
```
