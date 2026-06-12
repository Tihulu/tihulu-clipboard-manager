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

for cmd in git cargo gnome-extensions wl-copy wl-paste; do
    if ! need_cmd "$cmd"; then
        echo "$cmd is required." >&2
        exit 1
    fi
done

TMP_DIR="$(mktemp -d)"
git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$TMP_DIR/repo"

SOURCE_DIR="$TMP_DIR/repo/gnome/extension-native"
DEST_DIR="$HOME/.local/share/gnome-shell/extensions/$UUID"
BIN_DIR="$HOME/.local/bin"

if [ ! -f "$SOURCE_DIR/metadata.json" ] || [ ! -f "$SOURCE_DIR/extension.js" ]; then
    echo "GNOME extension source files were not found in $SOURCE_DIR." >&2
    exit 1
fi

cargo build --release --manifest-path "$TMP_DIR/repo/gnome/native-helper/Cargo.toml"
mkdir -p "$BIN_DIR"
cp "$TMP_DIR/repo/gnome/native-helper/target/release/$HELPER" "$BIN_DIR/$HELPER"
chmod 0755 "$BIN_DIR/$HELPER"

rm -rf "$DEST_DIR"
mkdir -p "$(dirname "$DEST_DIR")"
cp -R "$SOURCE_DIR" "$DEST_DIR"

gnome-extensions enable "$UUID" || true

printf '\nTihulu Clipboard Manager GNOME extension installed.\n'
printf 'Native helper installed to %s/%s.\n' "$BIN_DIR" "$HELPER"
printf 'If it does not appear immediately, log out and log back in. On Xorg you can also press Alt+F2, type r, and press Enter.\n'
