# ravom-core

## Overview
Core engine of RavomIO: handles project manifest parsing (`ravom.toml`), `platformio.ini` migration, the embedded board registry and `probe-rs` chip mapping, library package search (`crates.io` + PlatformIO Registry v3), interactive toolchain overrides (`~/.ravom/tools.toml`) & Rust targets, build command planning, flashing, serial/RTT monitoring, VOFA+ protocol decoding (`FireWater` / `JustFloat` / `RawData`), simulation, security auditing, and remote deployment.

## Interface
- `pub mod build, chips, config, context, deploy, flash, migrate, monitor, pkg, process, project, registry, security, sim, toolchain, vofa, zephyr`
- `pub type Result<T> = std::result::Result<T, CoreError>`
- `pub enum CoreError`
- `pub fn version() -> &'static str`

## Invariants
- All public operations return `CoreError` on missing manifests, unknown boards or environments, missing tools, or failed subprocesses.

## Side Effects
- File I/O (`ravom.toml`, `platformio.ini`, `~/.ravom/tools.toml`, `~/.ravom/serial_controls.toml`, build artifacts, and board registry files).
- Subprocess execution (build, flash, monitor, toolchain installers, simulation, and deployment commands).
- Serial port access via `serialport`.

## Verification
- Run: `cargo test -p ravom-core`
