#!/usr/bin/env bash
set -euo pipefail

action="${1:-install}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
app_bin="$HOME/.local/bin/aether-gtk"
app_id="io.github.evokelektrique.AetherGtk"
extension_id="aether-gtk@evokelektrique"
legacy_extension_ids=("aether-gtk@local" "aether-gtk@cluvexstudio.io")
extension_dir="$data_home/gnome-shell/extensions/$extension_id"
service_file="$data_home/dbus-1/services/$app_id.service"
desktop_file="$data_home/applications/$app_id.desktop"
icon_dir="$data_home/icons/hicolor"
icon_sizes=(64 128 256 512)
legacy_app_ids=("org.aethergtk.AetherGtk" "io.cluvex.AetherGtk")

remove_enabled_extension() {
    local escaped_id="${1//./\\.}"
    local enabled
    enabled="$(gsettings get org.gnome.shell enabled-extensions)"
    enabled="$(printf '%s\n' "$enabled" | sed -E "s/'$escaped_id', //g; s/, '$escaped_id'//g; s/'$escaped_id'//g")"
    gsettings set org.gnome.shell enabled-extensions "$enabled"
}

add_enabled_extension() {
    local extension="$1"
    local enabled
    enabled="$(gsettings get org.gnome.shell enabled-extensions)"
    if [[ "$enabled" != *"'$extension'"* ]]; then
        enabled="${enabled%]}"
        if [[ "$enabled" != "[" ]]; then
            enabled+=", "
        fi
        gsettings set org.gnome.shell enabled-extensions "${enabled}'$extension']"
    fi
}

if [[ "$action" == "uninstall" ]]; then
    gnome-extensions disable "$extension_id" >/dev/null 2>&1 || true
    gnome-extensions uninstall "$extension_id" >/dev/null 2>&1 || true
    remove_enabled_extension "$extension_id"
    rm -rf -- "$extension_dir"
    rm -f -- "$service_file" "$desktop_file" "$app_bin"
    for size in "${icon_sizes[@]}"; do
        rm -f -- "$icon_dir/${size}x${size}/apps/$app_id.png"
    done
    if command -v gtk-update-icon-cache >/dev/null 2>&1 && [[ -f "$icon_dir/index.theme" ]]; then
        gtk-update-icon-cache --force "$icon_dir" >/dev/null 2>&1 || true
    fi
    for legacy_id in "${legacy_extension_ids[@]}"; do
        gnome-extensions disable "$legacy_id" >/dev/null 2>&1 || true
        gnome-extensions uninstall "$legacy_id" >/dev/null 2>&1 || true
        remove_enabled_extension "$legacy_id"
        rm -rf -- "$data_home/gnome-shell/extensions/$legacy_id"
    done
    for legacy_id in "${legacy_app_ids[@]}"; do
        rm -f -- \
            "$data_home/dbus-1/services/$legacy_id.service" \
            "$data_home/applications/$legacy_id.desktop"
    done
    printf 'Removed Aether GNOME integration.\n'
    exit 0
fi

if [[ "$action" != "install" ]]; then
    printf 'Usage: %s [install|uninstall]\n' "$0" >&2
    exit 2
fi

if ! command -v gnome-extensions >/dev/null 2>&1; then
    printf 'GNOME Shell extension tools were not found. Install the GNOME Extensions app/tools and retry.\n' >&2
    exit 1
fi

cargo build --manifest-path "$repo_dir/Cargo.toml"
mkdir -p \
    "$(dirname -- "$app_bin")" \
    "$(dirname -- "$service_file")" \
    "$(dirname -- "$desktop_file")"
install -m 0755 "$repo_dir/target/debug/aether-gtk" "$app_bin"
for size in "${icon_sizes[@]}"; do
    icon="$repo_dir/data/icons/hicolor/${size}x${size}/apps/$app_id.png"
    if [[ ! -f "$icon" ]]; then
        printf 'Missing app icon: %s\n' "$icon" >&2
        exit 1
    fi
    install -D -m 0644 "$icon" "$icon_dir/${size}x${size}/apps/$app_id.png"
done
if command -v gtk-update-icon-cache >/dev/null 2>&1 && [[ -f "$icon_dir/index.theme" ]]; then
    gtk-update-icon-cache --force "$icon_dir" >/dev/null 2>&1 || true
fi
for legacy_id in "${legacy_extension_ids[@]}"; do
    remove_enabled_extension "$legacy_id"
    rm -rf -- "$data_home/gnome-shell/extensions/$legacy_id"
done
extension_package_dir="$(mktemp -d)"
trap 'rm -rf -- "$extension_package_dir"' EXIT
gnome-extensions pack --force --out-dir="$extension_package_dir" "$repo_dir/gnome-extension"
gnome-extensions install --force "$extension_package_dir/$extension_id.shell-extension.zip"

cat >"$service_file" <<EOF
[D-BUS Service]
Name=$app_id.Control
Exec=/usr/bin/env AETHER_GTK_BACKGROUND=1 "$app_bin"
EOF

cat >"$desktop_file" <<EOF
[Desktop Entry]
Name=Aether
Comment=Connect with Aether
Exec="$app_bin"
Icon=$app_id
Terminal=false
Type=Application
Categories=Network;Security;
StartupNotify=true
EOF
for legacy_id in "${legacy_app_ids[@]}"; do
    rm -f -- \
        "$data_home/dbus-1/services/$legacy_id.service" \
        "$data_home/applications/$legacy_id.desktop"
done

if [[ "$(gsettings get org.gnome.shell disable-user-extensions)" == "true" ]]; then
    printf 'Installed Aether and its extension files, but GNOME has user extensions disabled globally (org.gnome.shell disable-user-extensions=true). Enable user extensions, then run this installer again to activate the Quick Settings tile.\n'
    exit 0
fi

add_enabled_extension "$extension_id"

if ! gnome-extensions list | grep -Fxq "$extension_id"; then
    printf 'Enabled Aether in GNOME preferences. This Shell session has not discovered the newly installed extension yet; sign out and back in, then it should load in Quick Settings.\n'
    exit 0
fi

gnome-extensions disable "$extension_id"
gnome-extensions enable "$extension_id"
if ! gnome-extensions info "$extension_id" | grep -q '^  Enabled: Yes$'; then
    printf 'GNOME did not enable the Aether extension. Check: gnome-extensions info %s\n' "$extension_id" >&2
    exit 1
fi
printf 'Installed Aether and its GNOME Quick Settings extension for this user.\n'
printf 'Open Quick Settings and select Aether. Run %s uninstall to remove it.\n' "$0"
