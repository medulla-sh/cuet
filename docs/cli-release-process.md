# CLI Release Process

Releasing the `cuet` CLI is equivalent to pushing an annotated release tag.

CLI release tags must use the `cli/` prefix:

```bash
git tag -a cli/0.1.0 -m "Release cuet CLI 0.1.0"
git push origin cli/0.1.0
```

The version prefix after `cli/` must match the version in `cli/Cargo.toml`.
For example, if `cli/Cargo.toml` has `version = "0.1.0"`, the release tag
must start with `cli/0.1.0`.

Examples:

- `cli/0.1.0`
- `cli/0.1.0-rc.1`
- `cli/0.1.0-test`

The release workflow builds the CLI artifacts, publishes the GitHub Release, and
updates the Homebrew tap formula automatically.
