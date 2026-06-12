#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

REPO_URL="${REPO_URL:-https://github.com/Tihulu/tihulu-clipboard-manager.git}"
BRANCH="${BRANCH:-main}"
CACHE_DIR="${CACHE_DIR:-$HOME/.cache/tihulu-clipboard-manager/update}"
TARGET="auto"
RESET_HISTORY="${RESET_HISTORY:-0}"
KEEP_BUILD_DIR="${KEEP_BUILD_DIR:-0}"
PREFIX="${PREFIX:-/usr}"

usage() {
    cat <<'USAGE'
Tihulu Clipboard Manager GitHub updater

Usage:
  update-from-github.sh [--auto|--gnome|--cosmic] [--branch main] [--reset-history]

Options:
  --auto            Detect installed desktop target automatically. Default.
  --gnome          Update GNOME/Ubuntu extension + helper + user service.
  --cosmic         Update COSMIC applet.
  --branch NAME    Git branch to install from. Default: main.
  --reset-history  Back up/reset GNOME history during install.
  -h, --help       Show this help.

Environment:
  REPO_URL          Git repository URL.
  BRANCH            Git branch. Overridden by --branch.
  CACHE_DIR         Clone/update directory.
  PREFIX            COSMIC install prefix. Default: /usr.
  KEEP_BUILD_DIR    Passed to COSMIC quick installer.
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --auto)
            TARGET="auto"
            shift
            ;;
        --gnome)
            TARGET="gnome"
            shift
            ;;
        --cosmic)
            TARGET="cosmic"
            shift
            ;;
        --branch)
            BRANCH="${2:-}"
            if [ -z "$BRANCH" ]; then
                echo "--branch requires a value" >&2
                exit 2
            fi
            shift 2
            ;;
        --reset-history)
            RESET_HISTORY=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

need_cmd() {
    command -v "$1" >/dev/null 2>&1
}

run() {
    printf '+ %s\n' "$*"
    "$@"
}

if ! need_cmd git; then
    echo "git is required. Install it first." >&2
    exit 1
fi

mkdir -p "$(dirname "$CACHE_DIR")"

if [ -d "$CACHE_DIR/.git" ]; then
    echo "Updating existing clone at $CACHE_DIR"
    run git -C "$CACHE_DIR" fetch --prune origin
    run git -C "$CACHE_DIR" checkout "$BRANCH"
    run git -C "$CACHE_DIR" pull --ff-only origin "$BRANCH"
else
    rm -rf "$CACHE_DIR"
    echo "Cloning $REPO_URL into $CACHE_DIR"
    run git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$CACHE_DIR"
fi

is_gnome_installed() {
    [ -d "$HOME/.local/share/gnome-shell/extensions/tihulu-clipboard-manager@tihulu.dev" ] \
        || systemctl --user list-unit-files 2>/dev/null | grep -q '^tihulu-gnome-clipboard-daemon\.service'
}

is_cosmic_session() {
    printf '%s %s\n' "${XDG_CURRENT_DESKTOP:-}" "${XDG_SESSION_DESKTOP:-}" \
        | grep -qi 'cosmic'
}

if [ "$TARGET" = "auto" ]; then
    if is_gnome_installed || printf '%s %s\n' "${XDG_CURRENT_DESKTOP:-}" "${DESKTOP_SESSION:-}" | grep -qi 'gnome'; then
        TARGET="gnome"
    elif is_cosmic_session || need_cmd tihulu-clipboard-manager; then
        TARGET="cosmic"
    else
        cat >&2 <<'MSG'
Could not auto-detect target.
Use one of:
  update-from-github.sh --gnome
  update-from-github.sh --cosmic
MSG
        exit 1
    fi
fi

case "$TARGET" in
    gnome)
        echo "Updating GNOME/Ubuntu version from $BRANCH..."
        export REPO_URL BRANCH RESET_HISTORY
        run bash "$CACHE_DIR/gnome/scripts/quick-install.sh"
        ;;
    cosmic)
        echo "Updating COSMIC applet from $BRANCH..."
        export REPO_URL BRANCH PREFIX KEEP_BUILD_DIR
        run bash "$CACHE_DIR/scripts/quick-install.sh"
        ;;
    *)
        echo "Unsupported target: $TARGET" >&2
        exit 2
        ;;
esac

cat <<MSG

Tihulu update finished.

GNOME status:
  systemctl --user status tihulu-gnome-clipboard-daemon.service --no-pager

GNOME X11 reload:
  Alt + F2 → r → Enter

COSMIC:
  Log out/in or restart the panel if the applet was already loaded.

MSG
