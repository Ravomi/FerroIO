# ravom-core

## Overview
Core engine of RavomIO: handles project manifest parsing (`ravom.toml`), `platformio.ini` migration, the embedded board registry and `probe-rs` chip mapping, build command planning, flashing, serial/RTT monitoring, simulation, and remote deployment.

## Interface
- `pub mod build, chips, config, context, deploy, flash, migrate, monitor, process, project, registry, sim, toolchain, zephyr`
- `pub type Result<T> = std::result::Result<T, CoreError>`
- `pub enum CoreError`
- `pub fn version() -> &'static str`

## Invariants
- All public operations return `CoreError` on missing manifests, unknown boards or environments, missing tools, or failed subprocesses.

## Side Effects
- File I/O (`ravom.toml`, `platformio.ini`, build artifacts, and board registry files).
- Subprocess execution (build, flash, monitor, simulation, and deployment commands).
- Serial port access via `serialport`.

## Verification
- Run: `cargo test -p ravom-core`
