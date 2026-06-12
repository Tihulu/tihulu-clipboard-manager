#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

REPO_URL="${REPO_URL:-https://github.com/Tihulu/tihulu-clipboard-manager.git}"
BRANCH="${BRANCH:-main}"
UUID="tihulu-clipboard-manager@tihulu.dev"
HELPER="tihulu-gnome-clipboard-helper"
TMP_DIR=""

cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT

need_cmd() {
    command -v "$1" >/dev/null 2>&1
}

missing=0
for cmd in git cargo gnome-extensions systemctl timeout; do
    if ! need_cmd "$cmd"; then
        echo "$cmd is required." >&2
        missing=1
    fi
done

case "${XDG_SESSION_TYPE:-unknown}" in
    wayland)
        for cmd in wl-copy wl-paste; do
            if ! need_cmd "$cmd"; then
                echo "$cmd is required on Wayland. Install wl-clipboard." >&2
                missing=1
            fi
        done
        ;;
    x11|xorg)
        if ! need_cmd xclip; then
            echo "xclip is required on GNOME X11/Xorg. Install it with: sudo apt install xclip" >&2
            missing=1
        fi
        ;;
    *)
        if ! need_cmd xclip && { ! need_cmd wl-copy || ! need_cmd wl-paste; }; then
            echo "No supported clipboard backend found. Install xclip for X11 or wl-clipboard for Wayland." >&2
            missing=1
        fi
        ;;
esac

if [ "$missing" -ne 0 ]; then
    cat >&2 <<'MSG'

Ubuntu / Pop!_OS dependency install:

  sudo apt update
  sudo apt install -y git cargo gnome-shell-extensions xclip wl-clipboard coreutils build-essential pkg-config libssl-dev

Then run this installer again.
MSG
    exit 1
fi

TMP_DIR="$(mktemp -d)"
git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$TMP_DIR/repo"

SOURCE_DIR="$TMP_DIR/repo/gnome/extension-native"
HELPER_DIR="$TMP_DIR/repo/gnome/native-helper"
SERVICE_SRC="$TMP_DIR/repo/gnome/systemd/tihulu-gnome-clipboard-daemon.service"
DEST_DIR="$HOME/.local/share/gnome-shell/extensions/$UUID"
BIN_DIR="$HOME/.local/bin"
SERVICE_DEST="$HOME/.config/systemd/user/tihulu-gnome-clipboard-daemon.service"
DATA_DIR="$HOME/.local/share/tihulu-clipboard-manager-gnome"

if [ ! -f "$SOURCE_DIR/metadata.json" ] || [ ! -f "$SOURCE_DIR/extension.js" ]; then
    echo "GNOME extension source files were not found in $SOURCE_DIR." >&2
    exit 1
fi

if [ ! -f "$SERVICE_SRC" ]; then
    echo "GNOME daemon service was not found in $SERVICE_SRC." >&2
    exit 1
fi

echo "Disabling old extension and daemon if present..."
gnome-extensions disable "$UUID" >/dev/null 2>&1 || true
systemctl --user stop tihulu-gnome-clipboard-daemon.service >/dev/null 2>&1 || true

if [ "${RESET_HISTORY:-0}" = "1" ] && [ -d "$DATA_DIR" ]; then
    backup="${DATA_DIR}.bak-$(date +%s)"
    echo "Backing up existing history to $backup"
    mv "$DATA_DIR" "$backup"
fi

rm -rf "$DATA_DIR/previews" 2>/dev/null || true
if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    rm -rf "$XDG_RUNTIME_DIR/tihulu-clipboard-manager-gnome/previews" 2>/dev/null || true
fi

echo "Building native helper..."
cargo build --release --manifest-path "$HELPER_DIR/Cargo.toml"
mkdir -p "$BIN_DIR"
cp "$HELPER_DIR/target/release/$HELPER" "$BIN_DIR/$HELPER"
chmod 0755 "$BIN_DIR/$HELPER"

echo "Installing GNOME extension..."
rm -rf "$DEST_DIR"
mkdir -p "$(dirname "$DEST_DIR")"
cp -R "$SOURCE_DIR" "$DEST_DIR"

echo "Installing systemd user daemon..."
mkdir -p "$(dirname "$SERVICE_DEST")"
cp "$SERVICE_SRC" "$SERVICE_DEST"
systemctl --user import-environment DISPLAY WAYLAND_DISPLAY XAUTHORITY XDG_SESSION_TYPE DBUS_SESSION_BUS_ADDRESS || true
systemctl --user daemon-reload
systemctl --user enable --now tihulu-gnome-clipboard-daemon.service

gnome-extensions enable "$UUID" || true

cat <<MSG

Tihulu Clipboard Manager GNOME/Ubuntu installed.

What was installed:
  Extension: $DEST_DIR
  Helper:    $BIN_DIR/$HELPER
  Service:   $SERVICE_DEST
  Data:      $DATA_DIR

Status:
  systemctl --user status tihulu-gnome-clipboard-daemon.service --no-pager

Logs:
  journalctl --user -u tihulu-gnome-clipboard-daemon.service -f

X11 shell refresh:
  Alt + F2 → r → Enter

MSG
