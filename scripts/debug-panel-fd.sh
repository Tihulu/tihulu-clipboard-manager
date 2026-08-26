#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

APP_NAME="${1:-tihulu-clipboard-manager}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-2}"
DETAIL="${DETAIL:-0}"

find_pids() {
    local pattern="$1"
    pgrep -f "$pattern" 2>/dev/null | while read -r pid; do
        [ -n "$pid" ] || continue
        [ "$pid" != "$$" ] || continue
        case "$(ps -o args= -p "$pid" 2>/dev/null || true)" in
            *debug-panel-fd.sh*) continue ;;
            *pgrep\ -f*) continue ;;
        esac
        echo "$pid"
    done
}

print_process_group() {
    local label="$1"
    local pattern="$2"
    mapfile -t pids < <(find_pids "$pattern" || true)

    if [ "${#pids[@]}" -eq 0 ]; then
        echo "$label: none"
        return
    fi

    if [ "$label" = "app" ] && [ "${#pids[@]}" -gt 1 ]; then
        echo "warning: ${#pids[@]} app instances matched '$pattern'"
    fi

    for pid in "${pids[@]}"; do
        fd_count="n/a"
        if [ -d "/proc/$pid/fd" ]; then
            fd_count="$(find "/proc/$pid/fd" -maxdepth 1 -type l 2>/dev/null | wc -l)"
        fi

        rss_kib="$(ps -o rss= -p "$pid" 2>/dev/null | awk '{print $1}')"
        cpu_percent="$(ps -o %cpu= -p "$pid" 2>/dev/null | awk '{print $1}')"
        stat="$(ps -o stat= -p "$pid" 2>/dev/null | awk '{print $1}')"
        args="$(ps -o args= -p "$pid" 2>/dev/null || true)"
        echo "$label pid=$pid stat=${stat:-n/a} fd=$fd_count rss_kib=${rss_kib:-n/a} cpu=${cpu_percent:-n/a}% args=$args"

        children="$(pgrep -P "$pid" || true)"
        child_count=0
        if [ -n "$children" ]; then
            child_count="$(printf '%s\n' "$children" | wc -l)"
        fi
        echo "$label children=$child_count"

        if [ "$DETAIL" = "1" ] && [ -n "$children" ]; then
            # shellcheck disable=SC2086
            ps -o pid,ppid,stat,comm,args -p $children || true
        fi
    done
}

while true; do
    date '+%Y-%m-%d %H:%M:%S'
    print_process_group "app" "$APP_NAME"
    print_process_group "panel" "^cosmic-panel$|cosmic-panel$"
    print_process_group "comp" "^cosmic-comp$|cosmic-comp$"

    echo "wl helpers:"
    pgrep -af 'wl-paste|wl-copy' || echo "none"
    echo "----"
    sleep "$INTERVAL_SECONDS"
done
