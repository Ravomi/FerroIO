# crates

## Overview
Cargo workspace crates for RavomIO, separating the core engine, framework plugin protocol, CLI (`mio`), and `egui` desktop workbench.

## Subdirectories & Boundaries
- `ravom-core`: [./ravom-core/README.md](./ravom-core/README.md) — Core engine: project parsing, board registry, chip mapping, build/flash/monitor/sim/deploy command plans.
- `ravom-plugin`: [./ravom-plugin/README.md](./ravom-plugin/README.md) — `FrameworkPlugin` trait and built-in framework/RTOS/HDL templates.
- `ravom-cli`: [./ravom-cli/README.md](./ravom-cli/README.md) — `mio` CLI binary dispatching subcommands to `ravom-core` and `ravom-plugin`.
- `ravom-ui`: [./ravom-ui/README.md](./ravom-ui/README.md) — `eframe` / `egui` desktop workbench.
