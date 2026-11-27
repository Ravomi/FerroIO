# scripts

## Overview
Repository maintenance scripts for synchronizing upstream PlatformIO board definitions into `boards/`, regenerating the `probe-rs` target variant snapshot in `crates/ravom-core/data/probe-rs-targets.txt`, and automating Paseo isolated worktree lifecycle hooks.

## Interface
- `sync-pio-boards.sh [platform...]` — Clones upstream PlatformIO platform repositories, imports board definitions via `mio boards --import`, rebuilds `ravom-cli`, and validates with `mio boards --check`.
- `update-probe-rs-targets.sh` — Extracts `probe-rs chip list` variant names into `crates/ravom-core/data/probe-rs-targets.txt` and normalizes board chip names via `mio boards --normalize-chips boards`.
- `paseo-setup.sh` — Paseo worktree `setup` hook: loads `.env`, initializes git submodules (`docs/guide`, `examples`), warms Cargo crates from the shared registry cache, checks `rustfmt`/`clippy`/`thumbv7m-none-eabi` availability, and optionally writes `CARGO_TARGET_DIR` to `.env.local` when `RAVOM_SHARED_TARGET=1`.
- `paseo-teardown.sh` — Paseo worktree `teardown` hook: removes worktree-local `target/`, `examples/*/target`, `.env.local`, and `.ravom/` artifacts prior to worktree removal.

## Invariants
- All scripts use `set -euo pipefail`.
- `sync-pio-boards.sh` and `update-probe-rs-targets.sh` resolve paths relative to the repository root; `paseo-setup.sh` and `paseo-teardown.sh` operate strictly within the active worktree (`$PWD`) and never mutate `$PASEO_SOURCE_CHECKOUT_PATH`.
- `update-probe-rs-targets.sh` keeps only indented variant names (not ambiguous family names).

## Side Effects
- Network access (`git clone`, `git ls-remote` in `sync-pio-boards.sh`; `git submodule update`, `cargo fetch`, and optional `rustup target add` in `paseo-setup.sh`).
- Modifies `boards/` TOML files and `crates/ravom-core/data/probe-rs-targets.txt` (`sync-pio-boards.sh`, `update-probe-rs-targets.sh`).
- Runs `cargo build` / `cargo run` for `ravom-cli`, or cleans worktree-local `target/`, `examples/*/target`, `.env.local`, and `.ravom/` (`paseo-teardown.sh`).

## Verification
- Run: `cargo test -p ravom-cli`
