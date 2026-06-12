#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

REPO_URL="${REPO_URL:-https://github.com/Tihulu/tihulu-clipboard-manager.git}"
BRANCH="${BRANCH:-main}"
PREFIX="${PREFIX:-/usr}"
KEEP_BUILD_DIR="${KEEP_BUILD_DIR:-0}"

log() {
    printf '\n==> %s\n' "$*"
}

need_cmd() {
    command -v "$1" >/dev/null 2>&1
}

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
fi

if ! need_cmd curl; then
    echo "curl is required to run the updater." >&2
    exit 1
fi

log "Updating Tihulu Clipboard Manager from ${BRANCH}"

export REPO_URL
export BRANCH
export PREFIX
export KEEP_BUILD_DIR

curl -fsSL https://raw.githubusercontent.com/Tihulu/tihulu-clipboard-manager/main/scripts/quick-install.sh | bash

log "Update complete"
printf 'Restart COSMIC Shell or log out/in if the applet does not refresh immediately.\n'
