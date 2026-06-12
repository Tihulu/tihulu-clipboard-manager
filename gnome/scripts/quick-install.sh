#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

REPO_URL="${REPO_URL:-https://github.com/Tihulu/tihulu-clipboard-manager.git}"
BRANCH="${BRANCH:-main}"
UUID="tihulu-clipboard-manager@tihulu.dev"
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

if ! need_cmd git; then
    echo "git is required." >&2
    exit 1
fi

if ! need_cmd gnome-extensions; then
    echo "gnome-extensions is required. Install GNOME Shell extension tooling first." >&2
    exit 1
fi

TMP_DIR="$(mktemp -d)"
git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$TMP_DIR/repo"

SOURCE_DIR="$TMP_DIR/repo/gnome/extension"
DEST_DIR="$HOME/.local/share/gnome-shell/extensions/$UUID"

if [ ! -f "$SOURCE_DIR/metadata.json" ] || [ ! -f "$SOURCE_DIR/extension.js" ]; then
    echo "GNOME extension source files were not found in $SOURCE_DIR." >&2
    exit 1
fi

rm -rf "$DEST_DIR"
mkdir -p "$(dirname "$DEST_DIR")"
cp -R "$SOURCE_DIR" "$DEST_DIR"

gnome-extensions enable "$UUID" || true

printf '\nTihulu Clipboard Manager GNOME extension installed.\n'
printf 'If it does not appear immediately, log out and log back in. On Xorg you can also press Alt+F2, type r, and press Enter.\n'
