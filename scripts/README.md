# scripts

## Overview
Repository maintenance scripts for synchronizing upstream PlatformIO board definitions into `boards/` and regenerating the `probe-rs` target variant snapshot in `crates/ravom-core/data/probe-rs-targets.txt`.

## Interface
- `sync-pio-boards.sh [platform...]` — Clones upstream PlatformIO platform repositories, imports board definitions via `mio boards --import`, rebuilds `ravom-cli`, and validates with `mio boards --check`.
- `update-probe-rs-targets.sh` — Extracts `probe-rs chip list` variant names into `crates/ravom-core/data/probe-rs-targets.txt` and normalizes board chip names via `mio boards --normalize-chips boards`.

## Invariants
- Both scripts use `set -euo pipefail` and resolve paths relative to the repository root.
- `update-probe-rs-targets.sh` keeps only indented variant names (not ambiguous family names).

## Side Effects
- Network access (`git clone`, `git ls-remote` in `sync-pio-boards.sh`).
- Modifies `boards/` TOML files and `crates/ravom-core/data/probe-rs-targets.txt`.
- Runs `cargo build` / `cargo run` for `ravom-cli`.

## Verification
- Run: `cargo test -p ravom-cli`
