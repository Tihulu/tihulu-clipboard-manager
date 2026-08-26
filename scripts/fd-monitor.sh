#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

INTERVAL="${INTERVAL:-5}"
NAMES=("$@")

if [ "${#NAMES[@]}" -eq 0 ]; then
  NAMES=("tihulu-clipboard-manager" "cosmic-panel" "cosmic-comp")
fi

while true; do
  printf '\n%s\n' "$(date --iso-8601=seconds)"

  for name in "${NAMES[@]}"; do
    while IFS= read -r pid; do
      [ -n "$pid" ] || continue
      fd_count="$(ls "/proc/$pid/fd" 2>/dev/null | wc -l || true)"
      printf '%s pid=%s fd=%s\n' "$name" "$pid" "$fd_count"
      grep -E 'VmRSS|VmSwap|Threads' "/proc/$pid/status" 2>/dev/null || true
    done < <(pgrep -x "$name" 2>/dev/null || true)
  done

  sleep "$INTERVAL"
done
