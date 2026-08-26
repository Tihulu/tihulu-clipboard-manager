#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

APP_NAME="${1:-tihulu-clipboard-manager}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-2}"

while true; do
    date '+%Y-%m-%d %H:%M:%S'

    mapfile -t app_pids < <(pgrep -x "$APP_NAME" || true)
    if [ "${#app_pids[@]}" -eq 0 ]; then
        echo "app: no running $APP_NAME process"
    fi

    for pid in "${app_pids[@]}"; do
        fd_count="n/a"
        if [ -d "/proc/$pid/fd" ]; then
            fd_count="$(find "/proc/$pid/fd" -maxdepth 1 -type l 2>/dev/null | wc -l)"
        fi

        rss_kib="$(ps -o rss= -p "$pid" 2>/dev/null | awk '{print $1}')"
        cpu_percent="$(ps -o %cpu= -p "$pid" 2>/dev/null | awk '{print $1}')"
        echo "app pid=$pid fd=$fd_count rss_kib=${rss_kib:-n/a} cpu=${cpu_percent:-n/a}%"

        children="$(pgrep -P "$pid" || true)"
        if [ -n "$children" ]; then
            echo "children:"
            # shellcheck disable=SC2086
            ps -o pid,ppid,stat,comm,args -p $children || true
        fi
    done

    echo "wl helpers:"
    pgrep -af 'wl-paste|wl-copy' || echo "none"
    echo "----"
    sleep "$INTERVAL_SECONDS"
done
