#!/usr/bin/env bash
# Generate the CUE Buildkite pipeline schema from the pinned upstream schema.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
schema_dir="${script_dir}/../schemas/buildkite"
schema_path="${schema_dir}/pipeline.schema.json"
upstream_path="${schema_dir}/upstream.cue"
output_path="${script_dir}/../mod/primitives/buildkite/pipeline_schema.gen.cue"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

generated_path="${temp_dir}/pipeline_schema.gen.cue"

schema_sha256="$(cue export "$upstream_path" --expression upstream.sha256 --out text)"
actual_sha256="$(shasum -a 256 "$schema_path")"
actual_sha256="${actual_sha256%% *}"
if [[ "$actual_sha256" != "$schema_sha256" ]]; then
	printf 'Buildkite pipeline schema checksum mismatch\n' >&2
	exit 1
fi

cue import jsonschema \
	--package buildkite \
	--path '#PipelineDefinition:' \
	--outfile "$generated_path" \
	"$schema_path"
cue fmt "$generated_path"
mv "$generated_path" "$output_path"
