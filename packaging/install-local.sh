#!/bin/sh
# Install Cube Escape's menu entry and icon for the current user, so the
# desktop panel, application menu and Alt+Tab show the icon. Nothing outside
# your home directory is touched, and nothing needs sudo.
#
#     cargo build --release
#     sh packaging/install-local.sh              # uses target/release/cube-escape
#     sh packaging/install-local.sh /path/to/cube-escape
#     sh packaging/install-local.sh --uninstall
#
# The window class in the desktop file (StartupWMClass) must match
# `cube_escape::APP_ID`, which a unit test checks.
set -eu

APP_ID=cube-escape
repo=$(cd "$(dirname "$0")/.." && pwd)
data=${XDG_DATA_HOME:-$HOME/.local/share}
apps="$data/applications"
icons="$data/icons/hicolor"

refresh() {
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$apps" 2>/dev/null || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$icons" 2>/dev/null || true
}

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$apps/$APP_ID.desktop"
    for size in 16 32 64 256; do
        rm -f "$icons/${size}x${size}/apps/$APP_ID.png"
    done
    refresh
    echo "Removed the Cube Escape menu entry and icons."
    exit 0
fi

bin=${1:-$repo/target/release/$APP_ID}
if [ ! -x "$bin" ]; then
    echo "Cannot find the game at: $bin" >&2
    echo "Build it first with: cargo build --release" >&2
    exit 1
fi
bin=$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")

mkdir -p "$apps"
for size in 16 32 64 256; do
    mkdir -p "$icons/${size}x${size}/apps"
    cp "$repo/assets/icons/cube_escape_$size.png" "$icons/${size}x${size}/apps/$APP_ID.png"
done
sed "s|@EXEC@|$bin|" "$repo/packaging/$APP_ID.desktop.in" > "$apps/$APP_ID.desktop"
refresh

echo "Installed. Launch 'Cube Escape' from the menu, or pin it to the panel."
echo "It runs: $bin"
echo "If the icon looks stale, close the game and open it again."
