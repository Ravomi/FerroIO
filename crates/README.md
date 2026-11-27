# crates

## Overview
Cargo workspace crates for RavomIO, separating the core engine, framework plugin protocol, CLI (`mio`), and the native **Ravomi** (`ravom-ui`) desktop application.

## Subdirectories & Boundaries
- `ravom-core`: [./ravom-core/README.md](./ravom-core/README.md) — Core engine: project parsing, board registry, chip mapping, package/toolchain management, VOFA+ serial protocol engine, build/flash/monitor/sim/deploy command plans.
- `ravom-plugin`: [./ravom-plugin/README.md](./ravom-plugin/README.md) — `FrameworkPlugin` trait and built-in framework/RTOS templates.
- `ravom-cli`: [./ravom-cli/README.md](./ravom-cli/README.md) — `mio` CLI binary dispatching subcommands to `ravom-core` and `ravom-plugin`, and launching `Ravomi` via `mio ui`.
- `ravom-ui`: [./ravom-ui/README.md](./ravom-ui/README.md) — Native monochrome **Ravomi** desktop workbench (`eframe` / `egui`).
