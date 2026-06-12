#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
fi

need_cmd() {
    command -v "$1" >/dev/null 2>&1
}

if ! need_cmd cargo; then
    echo "cargo is required." >&2
    exit 1
fi

if ! cargo audit --version >/dev/null 2>&1; then
    echo "cargo-audit is not installed; installing it now."
    cargo install cargo-audit --locked
fi

cargo fmt --all -- --check
cargo check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
