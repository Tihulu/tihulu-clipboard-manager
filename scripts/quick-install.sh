#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

REPO_URL="${REPO_URL:-https://github.com/Tihulu/tihulu-clipboard-manager.git}"
BRANCH="${BRANCH:-main}"
PREFIX="${PREFIX:-/usr}"
KEEP_BUILD_DIR="${KEEP_BUILD_DIR:-0}"

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
fi

log() {
    printf '\n==> %s\n' "$*"
}

warn() {
    printf '\nWARN: %s\n' "$*" >&2
}

need_cmd() {
    command -v "$1" >/dev/null 2>&1
}

install_apt_deps() {
    if ! need_cmd apt-get; then
        warn "apt-get not found; skipping system dependency installation."
        return
    fi

    log "Installing build/runtime dependencies"
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        curl \
        git \
        libegl1-mesa-dev \
        libssl-dev \
        libwayland-dev \
        libxkbcommon-dev \
        pkg-config \
        wl-clipboard
}

ensure_rust() {
    if need_cmd cargo; then
        return
    fi

    log "Installing Rust with rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
}

ensure_just() {
    if need_cmd just; then
        return
    fi

    log "Installing just with cargo"
    cargo install just
}

main() {
    install_apt_deps
    ensure_rust
    ensure_just

    JUST_BIN="$(command -v just)"

    BUILD_DIR="$(mktemp -d -t tihulu-clipboard-manager.XXXXXX)"
    if [ "$KEEP_BUILD_DIR" != "1" ]; then
        trap 'rm -rf "$BUILD_DIR"' EXIT
    else
        log "Keeping build directory: $BUILD_DIR"
    fi

    log "Cloning $REPO_URL"
    git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$BUILD_DIR"
    cd "$BUILD_DIR"

    log "Checking Rust project"
    cargo check

    log "Running tests"
    cargo test --all-targets --all-features

    log "Building release binary"
    "$JUST_BIN" build-release

    log "Installing to $PREFIX"
    sudo env "prefix=$PREFIX" "$JUST_BIN" install

    log "Installed Tihulu Clipboard Manager"
    printf 'Restart COSMIC Shell or log out/in if the applet does not appear immediately.\n'
}

main "$@"
