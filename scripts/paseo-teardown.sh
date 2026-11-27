#!/usr/bin/env bash
set -euo pipefail

log() {
  printf '[paseo-teardown] %s\n' "$*"
}

# Reclaim per-worktree Cargo build artifacts & local temp files
rm -rf target examples/*/target .env.local .ravom || true

log "Worktree teardown complete."
