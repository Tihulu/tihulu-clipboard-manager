#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

UUID="tihulu-clipboard-manager@tihulu.dev"
HELPER="$HOME/.local/bin/tihulu-gnome-clipboard-helper"
EXT_DEST="$HOME/.local/share/gnome-shell/extensions/$UUID"
SERVICE_DEST="$HOME/.config/systemd/user/tihulu-gnome-clipboard-daemon.service"
DATA_DIR="$HOME/.local/share/tihulu-clipboard-manager-gnome"

gnome-extensions disable "$UUID" >/dev/null 2>&1 || true
systemctl --user disable --now tihulu-gnome-clipboard-daemon.service >/dev/null 2>&1 || true
rm -f "$SERVICE_DEST"
systemctl --user daemon-reload || true
rm -rf "$EXT_DEST"
rm -f "$HELPER"

if [ "${REMOVE_HISTORY:-0}" = "1" ]; then
    rm -rf "$DATA_DIR"
else
    echo "History kept at: $DATA_DIR"
    echo "To remove history too:"
    echo "  REMOVE_HISTORY=1 bash gnome/scripts/uninstall.sh"
fi

echo "Tihulu GNOME/Ubuntu version uninstalled."
