# ravom-ui

## Overview
Desktop GUI workbench (`eframe` / `egui`) for RavomIO, providing board browsing, project configuration, serial port discovery, background build/flash task execution, console logging, and signal plotting.

## Interface
- Binary entry point: `main() -> eframe::Result<()>` launching `RavomApp`
- `pub struct RavomApp` (`project_dir`, `board_query`, `boards: BoardIndex`, `selected_board`, `selected_env`, `release_mode`, `loaded_config`, `serial_ports`, `console`, `tasks: TaskRunner`, `plotter`, `brand_texture`)
- `RavomApp` methods: `new()`, `project_path()`, `try_load_project()`, `env_arg()`, `spawn_build()`, `spawn_flash()`
- `eframe::App::update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame)`

## Invariants
- `selected_board` stores a qualified `<platform>/<id>` string (defaults to `ststm32/bluepill_f103c8`).
- Background build/flash tasks are polled each frame via `drain_task_events()` and request 100ms repaints while running.

## Side Effects
- Reads `ravom.toml` from `project_dir` via `RavomConfig::load_from_dir`.
- Enumerates host serial ports via `ravom_core::monitor::list_serial_port_details()`.
- Spawns background build and flash tasks that invoke `ravom_core::build::build_project_with_log` and `ravom_core::flash::flash_project_with_log`.
- Creates a native OS window (`1200x760`) via `eframe::run_native`.

## Verification
- Run: `cargo test -p ravom-ui`
