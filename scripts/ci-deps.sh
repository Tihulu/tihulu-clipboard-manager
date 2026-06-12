#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later

set -Eeuo pipefail

sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev libwayland-dev libxkbcommon-dev libegl1-mesa-dev
