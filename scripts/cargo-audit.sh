#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

# Temporary transitive dependency ignores.
# quick-xml 0.39.4 is pulled in by the current COSMIC/libcosmic dependency graph.
# Remove these ignores once the dependency tree can resolve quick-xml >= 0.41.0.
#
# RUSTSEC-2026-0194: quick-xml quadratic duplicate-attribute check
# RUSTSEC-2026-0195: quick-xml namespace declaration allocation DoS
cargo audit \
  --ignore RUSTSEC-2026-0194 \
  --ignore RUSTSEC-2026-0195 \
  "$@"
