#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output_dir="${1:-$repo_dir/dist}"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_dir/Cargo.toml" | head -n 1)"
app_id="io.github.evokelektrique.AetherGtk"
bundle_name="aether-gtk-linux-x86_64"

if [[ -z "$version" || ! -x "$repo_dir/target/release/aether-gtk" ]]; then
    printf 'Build the release binary first with cargo build --release --features aether/tor --locked\n' >&2
    exit 1
fi
if [[ ! -x "$output_dir/pt/psiphon-tunnel-core" || ! -x "$output_dir/pt/lyrebird" || ! -f "$output_dir/PSIPHON-LICENSE" ]]; then
    printf 'Build Psiphon and Tor transports first with the release build scripts\n' >&2
    exit 1
fi
if ! command -v zip >/dev/null 2>&1; then
    printf 'Packaging requires zip\n' >&2
    exit 1
fi

mkdir -p "$output_dir"
stage="$(mktemp -d)"
trap 'rm -rf -- "$stage"' EXIT
bundle="$stage/$bundle_name"
mkdir -p "$bundle/icons/hicolor" "$bundle/pt"
install -m 0755 "$repo_dir/target/release/aether-gtk" "$bundle/aether-gtk"
cp -a "$output_dir/pt/." "$bundle/pt/"
cp -R "$repo_dir/data/icons/hicolor/64x64" "$repo_dir/data/icons/hicolor/128x128" \
    "$repo_dir/data/icons/hicolor/256x256" "$repo_dir/data/icons/hicolor/512x512" \
    "$bundle/icons/hicolor/"
(
    cd "$repo_dir/gnome-extension"
    zip -q -r "$bundle/extension.zip" extension.js metadata.json
)
install -m 0644 "$repo_dir/LICENSE" "$bundle/AETHERGTK-LICENSE"
install -m 0644 "$repo_dir/vendor/Aether/LICENSE" "$bundle/AETHER-LICENSE"
install -m 0644 "$output_dir/PSIPHON-LICENSE" "$bundle/PSIPHON-LICENSE"
install -m 0644 "$repo_dir/data/io.github.evokelektrique.AetherGtk.Control.xml" \
    "$bundle/aether-gtk-control.xml"
cat >"$bundle/UPSTREAM.txt" <<EOF
Aether GTK version: $version
Aether source: https://github.com/CluvexStudio/Aether
Aether commit: $(git -C "$repo_dir/vendor/Aether" rev-parse HEAD)
Source archive: AetherGTK-v$version-source.tar.gz
Psiphon source: https://github.com/CluvexStudio/psiphon-tunnel-core/tree/83aa73b9b982e7421e00117f5b0c5aceb5dda452
Psiphon source archive: psiphon-tunnel-core-source-83aa73b9b982e7421e00117f5b0c5aceb5dda452.tar.gz
EOF

archive="$output_dir/$bundle_name.tar.gz"
tar -czf "$archive" -C "$stage" "$bundle_name"
(
    cd "$output_dir"
    sha256sum "$bundle_name.tar.gz" >"$bundle_name.tar.gz.sha256"
)
printf 'Created %s and checksum\n' "$archive"
