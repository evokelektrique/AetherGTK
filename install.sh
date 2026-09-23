#!/usr/bin/env bash
set -euo pipefail

repo="evokelektrique/AetherGTK"
app_id="io.github.evokelektrique.AetherGtk"
extension_id="aether-gtk@evokelektrique"
archive_name="aether-gtk-linux-x86_64.tar.gz"
version="${1:-latest}"
interactive=false
reset= bold= muted= accent= green= red=

if [[ -t 2 && "${TERM:-}" != dumb ]]; then
    interactive=true
    if [[ -z "${NO_COLOR:-}" ]]; then
        reset="$(printf '\033[0m')"
        bold="$(printf '\033[1m')"
        muted="$(printf '\033[2m')"
        accent="$(printf '\033[94m')"
        green="$(printf '\033[32m')"
        red="$(printf '\033[31m')"
    fi
fi

step() {
    if [[ "$interactive" == true ]]; then
        printf '\r\033[2K  %s%s%s' "$muted" "$1" "$reset" >&2
    else
        printf '  %s\n' "$1" >&2
    fi
}

done_step() {
    if [[ "$interactive" == true ]]; then
        printf '\r\033[2K  %s%s%s\n' "$green" "$1" "$reset" >&2
    else
        printf '  %s\n' "$1" >&2
    fi
}

fail() {
    if [[ "$interactive" == true ]]; then
        printf '\r\033[2K  %sError: %s%s\n' "$red" "$1" "$reset" >&2
    else
        printf 'Error: %s\n' "$1" >&2
    fi
    exit 1
}

uninstall() {
    local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    local config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
    local app_id="io.github.evokelektrique.AetherGtk"
    local extension_id="aether-gtk@evokelektrique"

    if command -v gdbus >/dev/null 2>&1; then
        gdbus call --session \
            --dest "$app_id.Control" \
            --object-path "/io/github/evokelektrique/AetherGtk/Control" \
            --method "$app_id.Control.Quit" >/dev/null 2>&1 || true
    fi
    if command -v gnome-extensions >/dev/null 2>&1; then
        gnome-extensions disable "$extension_id" >/dev/null 2>&1 || true
        gnome-extensions uninstall "$extension_id" >/dev/null 2>&1 || true
    fi

    rm -f -- \
        "$HOME/.local/bin/aether-gtk" \
        "$data_home/applications/$app_id.desktop" \
        "$data_home/dbus-1/services/$app_id.service" \
        "$data_home/dbus-1/interfaces/$app_id.Control.xml" \
        "$config_home/autostart/$app_id.desktop"
    rm -rf -- \
        "$data_home/aether-gtk" \
        "$data_home/gnome-shell/extensions/$extension_id" \
        "$config_home/aether-gtk"
    for size in 64 128 256 512; do
        rm -f -- "$data_home/icons/hicolor/${size}x${size}/apps/$app_id.png"
    done
    if command -v gtk-update-icon-cache >/dev/null 2>&1 \
        && [[ -f "$data_home/icons/hicolor/index.theme" ]]; then
        gtk-update-icon-cache --force "$data_home/icons/hicolor" >/dev/null 2>&1 || true
    fi
    done_step 'Aether GTK and its saved settings were removed'
}

if [[ "$interactive" == true ]]; then
    printf '\n  %sAETHER GTK%s\n  %sInstaller%s\n\n' "$bold$accent" "$reset" "$muted" "$reset" >&2
else
    printf 'Aether GTK installer\n' >&2
fi

if (( EUID == 0 )); then
    fail 'Run this installer as your regular user. It does not need sudo.'
fi
if [[ "$version" == uninstall ]]; then
    if (( $# != 1 )); then
        printf 'Usage: %s uninstall\n' "$0" >&2
        exit 2
    fi
    uninstall
    exit 0
fi
if (( $# > 1 )) || [[ "$version" != latest && ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][[:alnum:].-]+)?$ ]]; then
    printf 'Usage: %s [latest|vMAJOR.MINOR.PATCH|uninstall]\n' "$0" >&2
    exit 2
fi
if [[ "$(uname -m)" != x86_64 ]]; then
    fail 'A prebuilt release is not available for this processor. Build from source instead.'
fi
for tool in curl tar sha256sum install; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        fail "Missing required command: $tool"
    fi
done

step 'Finding the release'
release_url="https://github.com/$repo/releases"
if [[ "$version" == latest ]]; then
    download_url="$release_url/latest/download"
else
    download_url="$release_url/download/$version"
fi
tmp="$(mktemp -d "${TMPDIR:-/tmp}/aether-gtk.XXXXXX")"
trap 'rm -rf -- "$tmp"' EXIT
trap 'printf "\n" >&2; exit 130' INT
download() {
    if [[ "$interactive" == true ]]; then
        curl --fail --location --show-error --progress-bar "$1" -o "$2"
    else
        curl --fail --location --silent --show-error "$1" -o "$2"
    fi
}

step 'Downloading Aether GTK'
download "$download_url/$archive_name" "$tmp/$archive_name" \
    || fail 'Could not download the release archive.'
download "$download_url/$archive_name.sha256" "$tmp/$archive_name.sha256" \
    || fail 'Could not download the release checksum.'
done_step 'Download complete'

step 'Verifying the release'
(cd "$tmp" && sha256sum --check --strict "$archive_name.sha256" >/dev/null) \
    || fail 'The release checksum does not match.'
done_step 'Checksum verified'

step 'Preparing the app'
mkdir "$tmp/unpacked"
tar -xzf "$tmp/$archive_name" --no-same-owner --strip-components=1 -C "$tmp/unpacked" \
    || fail 'Could not unpack the release archive.'
bundle="$tmp/unpacked"
if [[ ! -x "$bundle/aether-gtk" || ! -f "$bundle/extension.zip" ]]; then
    fail 'The release archive is missing required Aether GTK files.'
fi
if [[ -f "$bundle/pt/psiphon-tunnel-core" && ! -x "$bundle/pt/psiphon-tunnel-core" ]]; then
    fail 'The bundled Psiphon client is not executable.'
fi
if command -v ldd >/dev/null 2>&1; then
    missing_libraries="$(ldd "$bundle/aether-gtk" | grep 'not found' || true)"
    if [[ -n "$missing_libraries" ]]; then
        fail "$(printf 'Missing runtime libraries:\n%s\nInstall GTK 4 and libadwaita, then retry.' "$missing_libraries")"
    fi
fi
done_step 'App files ready'

data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
bin_dir="$HOME/.local/bin"
icon_root="$data_home/icons/hicolor"
app_bin="$bin_dir/aether-gtk"
step 'Installing Aether GTK'
install -D -m 0755 "$bundle/aether-gtk" "$app_bin"
psiphon_message='Psiphon helper was not included in this release.'
if [[ -x "$bundle/pt/psiphon-tunnel-core" ]]; then
    install -D -m 0755 "$bundle/pt/psiphon-tunnel-core" \
        "$data_home/aether-gtk/pt/psiphon-tunnel-core"
    psiphon_message='Psiphon support installed.'
fi
for size in 64 128 256 512; do
    install -D -m 0644 \
        "$bundle/icons/hicolor/${size}x${size}/apps/$app_id.png" \
        "$icon_root/${size}x${size}/apps/$app_id.png"
done
install -D -m 0644 \
    "$bundle/aether-gtk-control.xml" \
    "$data_home/dbus-1/interfaces/$app_id.Control.xml"

mkdir -p "$data_home/dbus-1/services" "$data_home/applications"
cat >"$data_home/dbus-1/services/$app_id.service" <<EOF
[D-BUS Service]
Name=$app_id.Control
Exec=/usr/bin/env AETHER_GTK_BACKGROUND=1 "$app_bin"
EOF
cat >"$data_home/applications/$app_id.desktop" <<EOF
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

if command -v gtk-update-icon-cache >/dev/null 2>&1 && [[ -f "$icon_root/index.theme" ]]; then
    gtk-update-icon-cache --force "$icon_root" >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data_home/applications" >/dev/null 2>&1 || true
fi
done_step 'App and launcher installed'

extension_message='GNOME extension tools were not found. The Quick Settings control was skipped.'
if command -v gnome-extensions >/dev/null 2>&1 && command -v gsettings >/dev/null 2>&1 \
    && gsettings list-schemas | grep -Fxq org.gnome.shell; then
    if gnome-extensions install --force "$bundle/extension.zip"; then
        if [[ "$(gsettings get org.gnome.shell disable-user-extensions 2>/dev/null || true)" == true ]]; then
            extension_message='The Quick Settings extension is installed, but GNOME user extensions are disabled.'
        else
            enabled="$(gsettings get org.gnome.shell enabled-extensions)"
            if [[ "$enabled" != *"'$extension_id'"* ]]; then
                enabled="${enabled%]}"
                [[ "$enabled" == '[' ]] || enabled+=", "
                gsettings set org.gnome.shell enabled-extensions "${enabled}'$extension_id']"
            fi
            gnome-extensions enable "$extension_id" >/dev/null 2>&1 || true
            if ! gnome-extensions list | grep -Fxq "$extension_id"; then
                extension_message='Sign out and back in to load the Quick Settings extension.'
            else
                extension_message='GNOME Quick Settings extension installed.'
            fi
        fi
    else
        extension_message='GNOME could not install the Quick Settings extension in this session.'
    fi
fi
done_step 'Installation complete'
printf '  - %s\n' "$extension_message" >&2
printf '  - %s\n' "$psiphon_message" >&2
printf '  - Launch Aether from your app menu or run %s\n' "$app_bin" >&2
