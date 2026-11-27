# ravom-ui (`Ravomi`)

## Overview
Native monochrome Black / White / Zinc (`shadcn/ui` New York Dark) desktop GUI application **Ravomi** (`eframe` / `egui`, built from crate `ravom-ui` as binary `Ravomi`). Features a unified integrated titlebar (`with_fullsize_content_view` + transparent titlebar on macOS, frameless window with integrated controls elsewhere) merging into a single-hairline 42px top command bar, far-left activity rail, workflow-guided Workspace dashboard (with PlatformIO migration & onboarding), PlatformIO-style Libraries & Community Package Manager hub (`crates.io` + PlatformIO Registry v3 via `ravom_core::pkg`), Master-Detail Board Catalog with monochrome chip filters, VOFA+-inspired Serial Studio (`FireWater` / `JustFloat` / `RawData`, multi-channel interactive oscilloscope, telemetry gauges, parameter sliders, and persistent command macros), interactive Toolchain Doctor & Rust Targets manager (with one-click installers and `~/.ravom/tools.toml` path overrides), collapsible Output Drawer, and bottom Status Bar.

## Interface
- Crate: `ravom-ui`; binary target: `Ravomi` (`main() -> eframe::Result<()>` launching `RavomApp`)
- `pub enum NavTab` (`Workspace`, `Libraries`, `Boards`, `Monitor`, `Doctor`), `pub enum LibrarySubTab` (`Registry`, `Installed`), `pub enum DoctorSubTab` (`Toolchains`, `RustTargets`), `pub enum MonitorViewMode` (`Split`, `Oscilloscope`, `Terminal`, `Widgets`), `pub enum DrawerTab` (`Console`, `Serial`, `Plotter`)
- `pub struct RavomApp` (`project_dir`, `board_query`, `board_filter`, `boards: BoardIndex`, `selected_board`, `selected_env`, `init_framework`, `release_mode`, `loaded_config`, `serial_ports`, `selected_port`, `serial_baud`, `serial_session`, `serial_log`, `monitor_view`, `monitor_controls_open`, `serial_protocol`, `serial_decoder`, `serial_controls`, `console`, `tasks: TaskRunner`, `plotter`, `brand_texture`, `active_tab`, `drawer_open`, `drawer_tab`, `doctor_tools`, `doctor_warnings`, `doctor_subtab`, `doctor_category`, `selected_doctor_tool`, `doctor_path_draft`, `doctor_targets`, `custom_rust_target`, `last_build_ok`, `last_flash_ok`, `lib_subtab`, `pkg_query`, `pkg_source`, `pkg_category`, `pkg_results`, `selected_pkg`, `pkg_remote_total`, `pkg_warnings`, `pkg_searching`, `installed_packages`, `custom_dep_spec`)
- `RavomApp` methods: `new()`, `project_path()`, `try_load_project()`, `env_arg()`, `refresh_ports()`, `connect_serial()`, `disconnect_serial()`, `set_serial_protocol()`, `apply_serial_lines()`, `reset_serial_target()`, `send_serial()`, `send_serial_payload()`, `save_serial_controls_state()`, `export_plotter_csv()`, `run_init_here()`, `run_migrate()`, `run_dry_run_plan()`, `run_doctor()`, `select_doctor_tool()`, `save_doctor_tool_path()`, `clear_doctor_tool_path()`, `open_tool_file_dialog()`, `spawn_doctor_install_tool()`, `spawn_doctor_install_target()`, `spawn_doctor_remove_target()`, `spawn_build()`, `spawn_flash()`, `refresh_installed_packages()`, `run_pkg_search()`, `spawn_pkg_install()`, `spawn_pkg_install_spec()`, `spawn_pkg_remove()`, `spawn_pkg_sync()`, `workflow_steps()`
- `eframe::App::update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame)`

## Invariants
- `selected_board` stores a qualified `<platform>/<id>` string (defaults to `espressif32/esp32-c3-devkitm-1` and syncs automatically when loading a project).
- `spawn_flash()` automatically disconnects any open `SerialSession` prior to invoking `probe-rs` / `espflash` / `west` so the USB port is not locked.
- Community package searches, tool installations, Rust target updates, and `install`/`remove`/`sync` runs execute off the UI thread and stream updates back cleanly over channels.
- Background build/flash/package/doctor tasks and live serial streams poll on every frame and request 30ms repaints while active.

## Side Effects
- Reads/writes `ravom.toml` (including `lib_deps`), `~/.ravom/tools.toml` (custom toolchain executable overrides), `~/.ravom/serial_controls.toml` (custom Serial Studio command macros and parameter sliders), `ravom_waveform.csv` on waveform export, `Cargo.toml`, `.ravom/libdeps/<env>/`, `app.overlay`, and migrates `platformio.ini` via `ravom_core`.
- Enumerates host serial ports and opens non-blocking `SerialSession` streams via `ravom_core::monitor`.
- Spawns background build and flash tasks that invoke `ravom_core::build::build_project_with_log` and `ravom_core::flash::flash_project_with_log`.
- Creates a native OS window titled **Ravomi** (`1320x820`, min `960x620`) with unified titlebar integration (`with_fullsize_content_view` / transparent titlebar on macOS, frameless chrome on non-macOS) via `eframe::run_native`.

## Verification
- Run: `cargo test -p ravom-ui`
