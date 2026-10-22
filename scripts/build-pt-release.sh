#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output_dir="${1:?Usage: build-pt-release.sh OUTPUT_DIR}"

mkdir -p "$output_dir/pt"
bash "$repo_dir/vendor/Aether/pt-build.sh" linux amd64 "$output_dir/pt"
