#!/usr/bin/env bash
# Re-import board definitions from upstream PlatformIO platform repositories
# (github.com/platformio/platform-<name>) into boards/, then validate.
#
#   scripts/sync-pio-boards.sh                 # every platform in boards/
#   scripts/sync-pio-boards.sh ststm32 gd32    # selected platforms
#
# Platforms without an upstream PlatformIO repo (FPGA, Linux SBC, ...) are
# curated by hand and skipped. Chip names are resolved against the probe-rs
# snapshot during import, so the result passes `mio boards --check`.
set -euo pipefail
cd "$(dirname "$0")/.."

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cargo build -q -p ravom-cli
mio=target/debug/mio

platforms=("$@")
if [ ${#platforms[@]} -eq 0 ]; then
  mapfile -t platforms < <(ls boards)
fi

for p in "${platforms[@]}"; do
  case "$p" in
    gd32) url="https://github.com/CommunityGD32Cores/platform-gd32.git" ;;
    ch32v) url="https://github.com/Community-PIO-CH32V/platform-ch32v.git" ;;
    raspberrypi-earlephilhower) url="https://github.com/maxgerhardt/platform-raspberrypi.git" ;;
    *) url="https://github.com/platformio/platform-${p}.git" ;;
  esac
  if ! git ls-remote --exit-code "$url" HEAD >/dev/null 2>&1; then
    echo "skip ${p}: no upstream PlatformIO platform"
    continue
  fi
  git clone -q --depth 1 "$url" "$work/$p"
  if [ -d "$work/$p/boards" ]; then
    "$mio" boards --import "$work/$p/boards" --platform "$p" --into boards
  fi
done

# Re-embed the updated boards and validate them.
cargo build -q -p ravom-cli
RAVOM_BOARDS_DIR= HOME="$work" "$mio" boards --check
