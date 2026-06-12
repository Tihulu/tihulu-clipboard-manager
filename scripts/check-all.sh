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

cargo fmt --manifest-path gnome/native-helper/Cargo.toml --all -- --check
cargo check --manifest-path gnome/native-helper/Cargo.toml
cargo test --manifest-path gnome/native-helper/Cargo.toml --all-targets --all-features
cargo clippy --manifest-path gnome/native-helper/Cargo.toml --all-targets --all-features -- -D warnings
(
    cd gnome/native-helper
    cargo generate-lockfile
    cargo audit
)
