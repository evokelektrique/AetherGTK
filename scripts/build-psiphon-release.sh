#!/usr/bin/env bash
set -euo pipefail

output_dir="${1:?Usage: build-psiphon-release.sh OUTPUT_DIR}"
repo="https://github.com/CluvexStudio/psiphon-tunnel-core.git"
branch="shirokhorshid"
commit="83aa73b9b982e7421e00117f5b0c5aceb5dda452"

for tool in go git tar sha256sum install; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'Missing required command: %s\n' "$tool" >&2
        exit 1
    fi
done

mkdir -p "$output_dir/pt"
output_dir="$(cd -- "$output_dir" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
source_dir="$work/psiphon-tunnel-core"
git -c core.longpaths=true clone --quiet --depth 50 --branch "$branch" "$repo" "$source_dir"
git -C "$source_dir" checkout --quiet --detach "$commit"

(
    cd "$source_dir"
    GOOS=linux GOARCH=amd64 CGO_ENABLED=0 GOFLAGS=-mod=vendor \
        go build -trimpath -ldflags '-s -w' -o "$output_dir/pt/psiphon-tunnel-core" ./ConsoleClient
)
install -m 0644 "$source_dir/LICENSE" "$output_dir/PSIPHON-LICENSE"
source_archive="$output_dir/psiphon-tunnel-core-source-$commit.tar.gz"
git -C "$source_dir" archive --format=tar.gz --prefix="psiphon-tunnel-core-$commit/" "$commit" >"$source_archive"
(
    cd "$output_dir"
    sha256sum "$(basename -- "$source_archive")" >"$(basename -- "$source_archive").sha256"
)

printf 'Built Psiphon client from %s\n' "$commit"
