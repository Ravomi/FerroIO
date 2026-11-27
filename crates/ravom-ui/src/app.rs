//! Application state, serial/task orchestration, and IDE pane layout.

use crate::board_index::{BoardFilter, BoardIndex};
use crate::console::Console;
use crate::panels;
use crate::plotter::Plotter;
use crate::tasks::{TaskEvent, TaskRunner};
use crate::theme::{self, StepState};
use eframe::egui;
use ravom_core::config::RavomConfig;
use ravom_core::monitor::{list_serial_port_details, SerialEvent, SerialPortEntry, SerialSession};
use ravom_core::pkg::{
    curated_catalog, install_package, install_spec, list_installed, remove_package,
    search_packages, sync_lib_deps, InstalledPackage, PackageRecord, PkgCategory, SearchOutcome,
    SearchQuery, SearchSourceFilter,
};
use ravom_core::registry::registry;
use ravom_core::toolchain::{
    check_host_setup, check_toolchains, install_rust_target_streaming, install_tool_streaming,
    list_rust_targets, remove_rust_target_streaming, save_tool_override, RustTargetStatus,
    ToolCategory, ToolStatus,
};
use ravom_core::vofa::{
    format_hex_dump, load_serial_controls, parse_hex_bytes, save_serial_controls, LineEnding,
    SerialControlsConfig, VofaDecoder, VofaProtocol,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

/// Primary workspace view selected in the left activity rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Workspace,
    Libraries,
    Boards,
    Monitor,
    Doctor,
}

/// Sub-tab inside the embedded Library & Package Manager hub.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySubTab {
    Registry,
    Installed,
}

/// Sub-tab inside the Toolchain Doctor & Rust Targets Configuration Center.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorSubTab {
    Toolchains,
    RustTargets,
}

/// Layout view mode inside the VOFA+-inspired Serial Studio panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorViewMode {
    Split,
    Oscilloscope,
    Terminal,
    Widgets,
}

/// Active tab inside the collapsible bottom output drawer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerTab {
    Console,
    Serial,
    Plotter,
}

/// Root application state for the RavomIO desktop workbench.
pub struct RavomApp {
    pub project_dir: String,
    pub board_query: String,
    pub board_filter: BoardFilter,
    pub boards: BoardIndex,
    /// Qualified `<platform>/<id>` of the selected board.
    pub selected_board: String,
    pub selected_env: String,
    /// Framework override used when clicking `Initialize` (`""` = board default).
    pub init_framework: String,
    pub release_mode: bool,
    pub loaded_config: Option<RavomConfig>,
    pub overlay_draft: String,
    pub overlay_dirty: bool,
    pub serial_ports: Vec<SerialPortEntry>,
    pub selected_port: String,
    pub serial_baud: u32,
    pub serial_tx_input: String,
    pub serial_auto_scroll: bool,
    pub show_all_ports: bool,
    pub serial_session: Option<SerialSession>,
    pub serial_log: Console,
    pub monitor_view: MonitorViewMode,
    pub monitor_controls_open: bool,
    pub serial_protocol: VofaProtocol,
    pub serial_decoder: VofaDecoder,
    pub serial_hex_rx: bool,
    pub serial_hex_tx: bool,
    pub serial_timestamps: bool,
    pub serial_pause_rx: bool,
    pub serial_line_ending: LineEnding,
    pub serial_filter: String,
    pub serial_dtr: bool,
    pub serial_rts: bool,
    pub serial_rx_bytes: u64,
    pub serial_tx_bytes: u64,
    pub serial_fps: f32,
    pub serial_tx_history: Vec<String>,
    pub serial_tx_history_idx: Option<usize>,
    pub serial_auto_send: bool,
    pub serial_auto_send_ms: u32,
    pub serial_controls: SerialControlsConfig,
    pub new_macro_label: String,
    pub new_macro_payload: String,
    pub new_macro_hex: bool,
    pub new_slider_name: String,
    pub new_slider_template: String,
    pub new_slider_min: String,
    pub new_slider_max: String,
    pub console: Console,
    pub tasks: TaskRunner,
    pub plotter: Plotter,
    pub brand_texture: Option<egui::TextureHandle>,
    pub active_tab: NavTab,
    pub drawer_open: bool,
    pub drawer_tab: DrawerTab,
    pub doctor_tools: Vec<ToolStatus>,
    pub doctor_warnings: Vec<String>,
    pub doctor_subtab: DoctorSubTab,
    pub doctor_category: ToolCategory,
    pub selected_doctor_tool: String,
    pub doctor_path_draft: String,
    pub doctor_targets: Vec<RustTargetStatus>,
    pub custom_rust_target: String,
    pub last_build_ok: bool,
    pub last_flash_ok: bool,
    pub lib_subtab: LibrarySubTab,
    pub pkg_query: String,
    pub pkg_source: SearchSourceFilter,
    pub pkg_category: PkgCategory,
    pub pkg_results: Vec<PackageRecord>,
    pub selected_pkg: Option<String>,
    pub pkg_remote_total: usize,
    pub pkg_warnings: Vec<String>,
    pub pkg_searching: bool,
    pub installed_packages: Vec<InstalledPackage>,
    pub custom_dep_spec: String,
    pkg_search_rx: Option<Receiver<SearchOutcome>>,
    folder_picker_rx: Option<Receiver<Option<String>>>,
    tool_picker_rx: Option<Receiver<Option<String>>>,
    serial_started_at: Option<Instant>,
    serial_fps_window_start: Instant,
    serial_fps_frame_count: u32,
    serial_last_auto_send: Instant,
}

impl RavomApp {
    /// Construct initial workbench state from the current directory and embedded board catalog.
    pub fn new() -> Self {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();
        let boards = BoardIndex::new(registry().boards().cloned().collect());
        let initial_pkgs = curated_catalog();
        let first_pkg = initial_pkgs.first().map(PackageRecord::qualified_id);
        let mut console = Console::default();
        console.push(format!(
            "Ravomi v{} ready ({} boards loaded).",
            ravom_core::version(),
            boards.len()
        ));
        let now = Instant::now();
        let mut app = Self {
            project_dir: cwd,
            board_query: String::new(),
            board_filter: BoardFilter::All,
            boards,
            selected_board: "espressif32/esp32-c3-devkitm-1".to_string(),
            selected_env: String::new(),
            init_framework: String::new(),
            release_mode: false,
            loaded_config: None,
            overlay_draft: String::new(),
            overlay_dirty: false,
            serial_ports: Vec::new(),
            selected_port: String::new(),
            serial_baud: 115_200,
            serial_tx_input: String::new(),
            serial_auto_scroll: true,
            show_all_ports: false,
            serial_session: None,
            serial_log: Console::default(),
            monitor_view: MonitorViewMode::Split,
            monitor_controls_open: false,
            serial_protocol: VofaProtocol::FireWater,
            serial_decoder: VofaDecoder::new(VofaProtocol::FireWater),
            serial_hex_rx: false,
            serial_hex_tx: false,
            serial_timestamps: false,
            serial_pause_rx: false,
            serial_line_ending: LineEnding::CrLf,
            serial_filter: String::new(),
            serial_dtr: true,
            serial_rts: false,
            serial_rx_bytes: 0,
            serial_tx_bytes: 0,
            serial_fps: 0.0,
            serial_tx_history: Vec::new(),
            serial_tx_history_idx: None,
            serial_auto_send: false,
            serial_auto_send_ms: 500,
            serial_controls: load_serial_controls(),
            new_macro_label: String::new(),
            new_macro_payload: String::new(),
            new_macro_hex: false,
            new_slider_name: String::new(),
            new_slider_template: String::new(),
            new_slider_min: String::from("0"),
            new_slider_max: String::from("100"),
            console,
            tasks: TaskRunner::default(),
            plotter: Plotter::default(),
            brand_texture: None,
            active_tab: NavTab::Workspace,
            drawer_open: false,
            drawer_tab: DrawerTab::Console,
            doctor_tools: Vec::new(),
            doctor_warnings: Vec::new(),
            doctor_subtab: DoctorSubTab::Toolchains,
            doctor_category: ToolCategory::All,
            selected_doctor_tool: String::from("cargo"),
            doctor_path_draft: String::new(),
            doctor_targets: Vec::new(),
            custom_rust_target: String::new(),
            last_build_ok: false,
            last_flash_ok: false,
            lib_subtab: LibrarySubTab::Registry,
            pkg_query: String::new(),
            pkg_source: SearchSourceFilter::All,
            pkg_category: PkgCategory::All,
            pkg_results: initial_pkgs,
            selected_pkg: first_pkg,
            pkg_remote_total: 0,
            pkg_warnings: Vec::new(),
            pkg_searching: false,
            installed_packages: Vec::new(),
            custom_dep_spec: String::new(),
            pkg_search_rx: None,
            folder_picker_rx: None,
            tool_picker_rx: None,
            serial_started_at: None,
            serial_fps_window_start: now,
            serial_fps_frame_count: 0,
            serial_last_auto_send: now,
        };
        app.refresh_ports();
        app.try_load_project();
        app
    }

    /// Current workspace root path.
    pub fn project_path(&self) -> PathBuf {
        PathBuf::from(self.project_dir.trim())
    }

    /// Optional active environment override trimmed of whitespace.
    pub fn env_arg(&self) -> Option<String> {
        let trimmed = self.selected_env.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }

    /// Reload `ravom.toml` and `app.overlay` from `project_dir`, syncing `selected_board`.
    pub fn try_load_project(&mut self) {
        let dir = self.project_path();
        self.last_build_ok = false;
        self.last_flash_ok = false;
        match RavomConfig::load_from_dir(&dir) {
            Ok(cfg) => {
                if let Ok((env_name, env)) = cfg.resolve_env(None) {
                    self.selected_env = env_name.to_string();
                    if let Ok(board) = registry().resolve(&env.board, Some(&env.platform)) {
                        self.selected_board = board.qualified_id();
                    }
                }
                self.loaded_config = Some(cfg);
                let overlay_path = dir.join("app.overlay");
                self.overlay_draft = std::fs::read_to_string(&overlay_path).unwrap_or_default();
                self.overlay_dirty = false;
                self.console
                    .push(format!("Loaded ravom.toml from {}", dir.display()));
            }
            Err(err) => {
                self.loaded_config = None;
                self.console.push(format!(
                    "Workspace '{}' has no ravom.toml ({err})",
                    dir.display()
                ));
            }
        }
        self.refresh_installed_packages();
    }

    /// Return serial ports filtered for embedded USB devices unless `show_all_ports` is set.
    pub fn visible_serial_ports(&self) -> Vec<&SerialPortEntry> {
        let filtered: Vec<&SerialPortEntry> = self
            .serial_ports
            .iter()
            .filter(|p| {
                self.show_all_ports
                    || (!p.description.eq_ignore_ascii_case("Bluetooth")
                        && !p.path.contains("Bluetooth")
                        && !p.path.contains("debug-console"))
            })
            .collect();
        if filtered.is_empty() && !self.show_all_ports {
            Vec::new()
        } else {
            filtered
        }
    }

    /// Refresh host serial ports and keep `selected_port` pointed at a valid USB device.
    pub fn refresh_ports(&mut self) {
        self.serial_ports = list_serial_port_details();
        let visible_paths: Vec<String> = self
            .visible_serial_ports()
            .iter()
            .map(|p| p.path.clone())
            .collect();
        if !visible_paths.iter().any(|p| p == &self.selected_port) {
            self.selected_port = visible_paths
                .iter()
                .find(|p| p.contains("usbmodem") || p.contains("ttyUSB") || p.contains("ttyACM"))
                .cloned()
                .or_else(|| visible_paths.into_iter().next())
                .unwrap_or_default();
        }
    }

    /// Open a non-blocking serial monitor session on `selected_port` at `serial_baud`.
    pub fn connect_serial(&mut self) {
        if self.selected_port.trim().is_empty() {
            self.serial_log
                .push("No serial port selected. Click Refresh after plugging in your board.");
            return;
        }
        self.disconnect_serial();
        self.serial_rx_bytes = 0;
        self.serial_tx_bytes = 0;
        self.serial_fps = 0.0;
        self.serial_decoder.set_protocol(self.serial_protocol);
        self.serial_decoder.reset();
        let now = Instant::now();
        self.serial_started_at = Some(now);
        self.serial_fps_window_start = now;
        self.serial_fps_frame_count = 0;

        match SerialSession::open(&self.selected_port, self.serial_baud) {
            Ok(mut session) => {
                let _ = session.set_dtr(self.serial_dtr);
                let _ = session.set_rts(self.serial_rts);
                self.serial_log.push(format!(
                    "--- Connected {} @ {} baud ({}) ---",
                    session.port_name(),
                    session.baud(),
                    self.serial_protocol.label()
                ));
                self.serial_session = Some(session);
            }
            Err(err) => {
                self.serial_log.push(format!("Serial connect error: {err}"));
            }
        }
    }

    /// Close the active serial monitor session, if any.
    pub fn disconnect_serial(&mut self) {
        self.serial_auto_send = false;
        self.serial_started_at = None;
        self.serial_fps = 0.0;
        if let Some(session) = self.serial_session.take() {
            self.serial_log
                .push(format!("--- Disconnected {} ---", session.port_name()));
        }
    }

    /// Switch the active VOFA+ protocol engine (`FireWater`, `JustFloat`, or `RawData`).
    pub fn set_serial_protocol(&mut self, protocol: VofaProtocol) {
        self.serial_protocol = protocol;
        self.serial_decoder.set_protocol(protocol);
    }

    /// Apply current DTR and RTS control line states to the active serial port.
    pub fn apply_serial_lines(&mut self) {
        let (dtr, rts) = (self.serial_dtr, self.serial_rts);
        if let Some(session) = self.serial_session.as_mut() {
            if let Err(err) = session.set_dtr(dtr) {
                self.serial_log.push(format!("DTR error: {err}"));
            }
            if let Err(err) = session.set_rts(rts) {
                self.serial_log.push(format!("RTS error: {err}"));
            }
        }
    }

    /// Briefly pulse RTS to hardware-reset the attached microcontroller.
    pub fn reset_serial_target(&mut self) {
        if let Some(session) = self.serial_session.as_mut() {
            match session.pulse_reset() {
                Ok(()) => {
                    self.serial_rts = false;
                    self.serial_log.push("--- Target reset pulsed via RTS ---");
                }
                Err(err) => self.serial_log.push(format!("Reset error: {err}")),
            }
        } else {
            self.serial_log
                .push("Not connected — open a serial port before resetting.");
        }
    }

    /// Send a command payload using the current ASCII / HEX TX mode setting.
    pub fn send_serial(&mut self, line: &str) {
        self.send_serial_payload(line, self.serial_hex_tx);
    }

    /// Send either an ASCII command (with line ending) or raw HEX bytes over the connected port.
    pub fn send_serial_payload(&mut self, payload: &str, hex_mode: bool) {
        let trimmed = payload.trim();
        if trimmed.is_empty() {
            return;
        }
        let Some(session) = self.serial_session.as_mut() else {
            self.serial_log
                .push("Not connected — open a serial port before sending commands.");
            return;
        };

        let bytes = if hex_mode {
            match parse_hex_bytes(trimmed) {
                Ok(b) => b,
                Err(err) => {
                    self.serial_log.push(format!("TX HEX error: {err}"));
                    return;
                }
            }
        } else {
            let mut buf = trimmed.as_bytes().to_vec();
            buf.extend_from_slice(self.serial_line_ending.suffix());
            buf
        };

        match session.send_bytes(&bytes) {
            Ok(()) => {
                self.serial_tx_bytes += bytes.len() as u64;
                let entry = if hex_mode {
                    format!("> [HEX] {}", format_hex_dump(&bytes))
                } else {
                    format!("> {trimmed}")
                };
                self.serial_log.push(entry);
                if self.serial_tx_history.last().map(String::as_str) != Some(trimmed) {
                    self.serial_tx_history.push(trimmed.to_string());
                    if self.serial_tx_history.len() > 50 {
                        self.serial_tx_history.remove(0);
                    }
                }
                self.serial_tx_history_idx = None;
            }
            Err(err) => self.serial_log.push(format!("TX error: {err}")),
        }
    }

    /// Save user-customized macros and sliders to `~/.ravom/serial_controls.toml`.
    pub fn save_serial_controls_state(&mut self) {
        if let Err(err) = save_serial_controls(&self.serial_controls) {
            self.console
                .push(format!("Failed to save serial controls: {err}"));
        }
    }

    /// Export recorded oscilloscope waveforms to `ravom_waveform.csv`.
    pub fn export_plotter_csv(&mut self) {
        let csv = self.plotter.export_csv();
        let proj = self.project_path();
        let dir = if proj.is_dir() {
            proj
        } else {
            std::env::temp_dir()
        };
        let out_path = dir.join("ravom_waveform.csv");
        match std::fs::write(&out_path, csv) {
            Ok(()) => {
                let msg = format!("Exported waveform CSV -> {}", out_path.display());
                self.serial_log.push(msg.clone());
                self.console.push(msg);
            }
            Err(err) => {
                let msg = format!("CSV export failed ({}): {err}", out_path.display());
                self.serial_log.push(msg.clone());
                self.console.push(msg);
            }
        }
    }

    /// Initialize a new RavomIO project in `project_dir` using `board` and optional `framework`.
    pub fn run_init_here(&mut self, board: &str, framework: Option<&str>) {
        let dir = self.project_path();
        match ravom_core::project::init_project(&dir, board, framework) {
            Ok(res) => {
                self.console.push(format!(
                    "Initialized {} (board: {}, framework: {})",
                    res.root.display(),
                    res.board_id,
                    res.framework
                ));
                self.try_load_project();
            }
            Err(err) => {
                self.console.push(format!("Init error: {err}"));
                self.drawer_open = true;
                self.drawer_tab = DrawerTab::Console;
            }
        }
    }

    /// Migrate an existing `platformio.ini` in `project_dir` to `ravom.toml`.
    pub fn run_migrate(&mut self) {
        let dir = self.project_path();
        match ravom_core::migrate::migrate_platformio_project(&dir) {
            Ok((out_path, warnings)) => {
                self.console
                    .push(format!("Migrated {}", out_path.display()));
                for w in warnings {
                    self.console.push(format!("warning: {w}"));
                }
                self.try_load_project();
            }
            Err(err) => {
                self.console.push(format!("Migrate error: {err}"));
            }
        }
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
    }

    /// Print the dry-run build and flash command plan into the Console drawer.
    pub fn run_dry_run_plan(&mut self) {
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        let dir = self.project_path();
        let env = self.env_arg();
        match ravom_core::context::EnvContext::load(&dir, env.as_deref()) {
            Ok(ctx) => {
                let (backend, steps) = ravom_core::build::plan_build(&ctx, self.release_mode);
                self.console.push(format!(
                    "--- Plan env='{}' board={} backend={:?} profile={} ---",
                    ctx.env_name,
                    ctx.board.qualified_id(),
                    backend,
                    if self.release_mode {
                        "release"
                    } else {
                        "debug"
                    }
                ));
                for step in steps {
                    self.console.push(format!("  build: {step}"));
                }
                match ravom_core::flash::plan_flash(&ctx, self.release_mode, false) {
                    Ok(fp) => {
                        self.console.push(format!("  flash: {}", fp.command));
                        if let Some(post) = fp.post_command {
                            self.console.push(format!("  reset: {post}"));
                        }
                    }
                    Err(err) => self.console.push(format!("  flash plan: {err}")),
                }
            }
            Err(err) => self.console.push(format!("Plan error: {err}")),
        }
    }

    /// Scan local embedded toolchains, host environment notes, and Rust targets.
    pub fn run_doctor(&mut self) {
        self.doctor_tools = check_toolchains();
        self.doctor_warnings = check_host_setup();
        self.doctor_targets = list_rust_targets();
        if !self
            .doctor_tools
            .iter()
            .any(|t| t.name == self.selected_doctor_tool)
        {
            if let Some(first) = self.doctor_tools.first() {
                self.selected_doctor_tool = first.name.to_string();
            }
        }
        self.sync_doctor_path_draft();
    }

    /// Select a toolchain in Doctor and populate the custom-path draft field.
    pub fn select_doctor_tool(&mut self, name: &str) {
        self.selected_doctor_tool = name.to_string();
        self.sync_doctor_path_draft();
    }

    /// Synchronize `doctor_path_draft` with the selected tool's configured or detected binary path.
    pub fn sync_doctor_path_draft(&mut self) {
        self.doctor_path_draft = self
            .doctor_tools
            .iter()
            .find(|t| t.name == self.selected_doctor_tool)
            .and_then(|t| t.configured_path.as_ref().or(t.path.as_ref()))
            .map(|p| p.display().to_string())
            .unwrap_or_default();
    }

    /// Save the drafted executable override for `selected_doctor_tool` to `~/.ravom/tools.toml`.
    pub fn save_doctor_tool_path(&mut self) {
        let tool_name = self.selected_doctor_tool.trim().to_string();
        let draft = self.doctor_path_draft.trim().to_string();
        if tool_name.is_empty() {
            return;
        }
        if draft.is_empty() {
            self.clear_doctor_tool_path();
            return;
        }
        let path = Path::new(&draft);
        if !path.is_file() {
            self.console.push(format!(
                "Doctor error: `{draft}` does not exist or is not a regular file"
            ));
            self.drawer_open = true;
            self.drawer_tab = DrawerTab::Console;
            return;
        }
        match save_tool_override(&tool_name, Some(path)) {
            Ok(cfg_path) => {
                self.console.push(format!(
                    "Saved override for `{tool_name}` -> {} ({})",
                    path.display(),
                    cfg_path.display()
                ));
                self.run_doctor();
            }
            Err(err) => {
                self.console
                    .push(format!("Doctor error saving override: {err}"));
                self.drawer_open = true;
                self.drawer_tab = DrawerTab::Console;
            }
        }
    }

    /// Remove any custom path override for `selected_doctor_tool`, reverting to `PATH`.
    pub fn clear_doctor_tool_path(&mut self) {
        let tool_name = self.selected_doctor_tool.trim().to_string();
        if tool_name.is_empty() {
            return;
        }
        match save_tool_override(&tool_name, None) {
            Ok(cfg_path) => {
                self.console.push(format!(
                    "Cleared override for `{tool_name}` ({})",
                    cfg_path.display()
                ));
                self.run_doctor();
            }
            Err(err) => {
                self.console
                    .push(format!("Doctor error clearing override: {err}"));
                self.drawer_open = true;
                self.drawer_tab = DrawerTab::Console;
            }
        }
    }

    /// Open a non-blocking native file picker for selecting a custom tool binary.
    pub fn open_tool_file_dialog(&mut self) {
        if self.tool_picker_rx.is_some() {
            return;
        }
        let tool = self.selected_doctor_tool.clone();
        let (tx, rx) = mpsc::channel();
        self.tool_picker_rx = Some(rx);
        thread::spawn(move || {
            let picked = pick_tool_file_os(&tool);
            let _ = tx.send(picked);
        });
    }

    /// Run an automated tool installer (`brew` / `cargo` / `pip3`) in the background.
    pub fn spawn_doctor_install_tool(&mut self, tool_name: &str) {
        let name = tool_name.to_string();
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console
            .push(format!("Installing toolchain component `{name}`..."));
        self.tasks.spawn("Doctor", move |log| {
            install_tool_streaming(&name, log)
                .map(|()| format!("Installed `{name}`"))
                .map_err(|e| e.to_string())
        });
    }

    /// Install a Rust target triple via `rustup target add` in the background.
    pub fn spawn_doctor_install_target(&mut self, triple: &str) {
        let target = triple.trim().to_string();
        if target.is_empty() {
            return;
        }
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console
            .push(format!("Adding Rust target `{target}`..."));
        self.tasks.spawn("Doctor", move |log| {
            install_rust_target_streaming(&target, log)
                .map(|()| format!("Added Rust target `{target}`"))
                .map_err(|e| e.to_string())
        });
    }

    /// Remove an installed Rust target triple via `rustup target remove` in the background.
    pub fn spawn_doctor_remove_target(&mut self, triple: &str) {
        let target = triple.trim().to_string();
        if target.is_empty() {
            return;
        }
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console
            .push(format!("Removing Rust target `{target}`..."));
        self.tasks.spawn("Doctor", move |log| {
            remove_rust_target_streaming(&target, log)
                .map(|()| format!("Removed Rust target `{target}`"))
                .map_err(|e| e.to_string())
        });
    }

    /// Open a non-blocking native folder picker dialog (macOS / Linux).
    pub fn open_folder_dialog(&mut self) {
        if self.folder_picker_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.folder_picker_rx = Some(rx);
        thread::spawn(move || {
            let picked = pick_folder_os();
            let _ = tx.send(picked);
        });
    }

    /// Toggle a boolean (`y`) Zephyr Kconfig key in the active environment and persist `ravom.toml`.
    pub fn toggle_zephyr_kconfig(&mut self, key: &str) {
        let Some(cfg) = self.loaded_config.as_mut() else {
            return;
        };
        let env_name = self.selected_env.trim().to_string();
        let target_key = if env_name.is_empty() {
            cfg.env.keys().next().cloned()
        } else {
            Some(env_name)
        };
        let Some(name) = target_key else { return };
        let Some(env) = cfg.env.get_mut(&name) else {
            return;
        };
        let zcfg = env
            .zephyr
            .get_or_insert_with(ravom_core::zephyr::ZephyrConfig::default);
        let enabled = zcfg
            .kconfig
            .get(key)
            .is_some_and(|v| v.eq_ignore_ascii_case("y"));
        if enabled {
            zcfg.kconfig.remove(key);
        } else {
            zcfg.kconfig.insert(key.to_string(), "y".to_string());
        }
        match cfg.to_toml_string() {
            Ok(toml) => {
                if let Err(err) = std::fs::write(self.project_path().join("ravom.toml"), toml) {
                    self.console
                        .push(format!("Failed to save ravom.toml: {err}"));
                } else {
                    self.console.push(format!(
                        "Updated {key}={} in ravom.toml",
                        if enabled { "off" } else { "y" }
                    ));
                }
            }
            Err(err) => self
                .console
                .push(format!("Failed to serialize ravom.toml: {err}")),
        }
    }

    /// Persist `app.overlay` to the project root.
    pub fn save_overlay(&mut self) {
        let path = self.project_path().join("app.overlay");
        match std::fs::write(&path, &self.overlay_draft) {
            Ok(()) => {
                self.overlay_dirty = false;
                self.console.push(format!("Saved {}", path.display()));
            }
            Err(err) => self
                .console
                .push(format!("Failed to save {}: {err}", path.display())),
        }
    }

    /// Run a background firmware build and open the Console drawer.
    pub fn spawn_build(&mut self) {
        let (dir, env, release) = (self.project_path(), self.env_arg(), self.release_mode);
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console.push(format!(
            "Starting {} build...",
            if release { "release" } else { "debug" }
        ));
        self.tasks.spawn("Build", move |log| {
            ravom_core::build::build_project_with_log(&dir, env.as_deref(), release, log)
                .map(|res| {
                    format!(
                        "env={} board={} target={}",
                        res.env_name,
                        res.board_id,
                        res.target.as_deref().unwrap_or("host")
                    )
                })
                .map_err(|e| e.to_string())
        });
    }

    /// Disconnect any live serial monitor, flash firmware in the background, and open Console.
    pub fn spawn_flash(&mut self) {
        if self.serial_session.is_some() {
            self.disconnect_serial();
            self.console
                .push("Disconnected serial monitor prior to flashing.");
        }
        let (dir, env, release) = (self.project_path(), self.env_arg(), self.release_mode);
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console.push(format!(
            "Starting {} flash...",
            if release { "release" } else { "debug" }
        ));
        self.tasks.spawn("Flash", move |log| {
            ravom_core::flash::flash_project_with_log(&dir, env.as_deref(), release, log)
                .map(|res| {
                    format!(
                        "env={} chip={} ({})",
                        res.env_name,
                        res.chip,
                        res.firmware.display()
                    )
                })
                .map_err(|e| e.to_string())
        });
    }

    /// Compute the 4-stage pipeline states for the workspace stepper.
    pub fn workflow_steps(&self) -> [(&'static str, StepState); 4] {
        let has_manifest = self.loaded_config.is_some();
        let board_state = StepState::Done;
        let manifest_state = if has_manifest {
            StepState::Done
        } else {
            StepState::Active
        };
        let build_state = if !has_manifest {
            StepState::Pending
        } else if self.last_build_ok {
            StepState::Done
        } else {
            StepState::Active
        };
        let flash_state = if !has_manifest || !self.last_build_ok {
            StepState::Pending
        } else if self.last_flash_ok {
            StepState::Done
        } else {
            StepState::Active
        };
        [
            ("Target Board", board_state),
            ("Project Manifest", manifest_state),
            ("Build Firmware", build_state),
            ("Flash & Verify", flash_state),
        ]
    }

    /// Refresh the list of installed `ravom.toml` and `Cargo.toml` dependencies.
    pub fn refresh_installed_packages(&mut self) {
        let dir = self.project_path();
        let env = self.env_arg();
        match list_installed(&dir, env.as_deref()) {
            Ok(list) => self.installed_packages = list,
            Err(err) => self
                .console
                .push(format!("Failed to inspect project libraries: {err}")),
        }
    }

    /// Filter the offline starter catalog immediately and optionally query live community registries.
    pub fn run_pkg_search(&mut self, online: bool) {
        let query = SearchQuery {
            text: self.pkg_query.clone(),
            source: self.pkg_source,
            category: self.pkg_category,
            limit: 15,
            online: false,
        };
        let offline = search_packages(&query);
        self.pkg_results = offline.packages;
        self.pkg_remote_total = 0;
        self.pkg_warnings.clear();
        if self
            .selected_pkg
            .as_ref()
            .is_none_or(|id| !self.pkg_results.iter().any(|p| p.qualified_id() == *id))
        {
            self.selected_pkg = self.pkg_results.first().map(PackageRecord::qualified_id);
        }

        if !online {
            self.pkg_searching = false;
            self.pkg_search_rx = None;
            return;
        }

        let online_query = SearchQuery {
            online: true,
            ..query
        };
        let (tx, rx) = mpsc::channel();
        self.pkg_searching = true;
        self.pkg_search_rx = Some(rx);
        thread::spawn(move || {
            let outcome = search_packages(&online_query);
            let _ = tx.send(outcome);
        });
    }

    /// Install a library package into `ravom.toml` and local toolchain storage.
    pub fn spawn_pkg_install(&mut self, pkg: PackageRecord) {
        let (dir, env) = (self.project_path(), self.env_arg());
        let name = pkg.qualified_id();
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console.push(format!("Installing library {name}..."));
        self.tasks.spawn("Pkg", move |log| {
            let mut emit = |line: String| log(&line);
            install_package(&dir, env.as_deref(), &pkg, &mut emit)
                .map(|res| format!("installed {} ({})", res.name, res.spec))
                .map_err(|e| e.to_string())
        });
    }

    /// Resolve and install a custom `lib_deps` spec into `ravom.toml`.
    pub fn spawn_pkg_install_spec(&mut self, spec: String) {
        let trimmed = spec.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let (dir, env) = (self.project_path(), self.env_arg());
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console
            .push(format!("Resolving and installing {trimmed}..."));
        self.tasks.spawn("Pkg", move |log| {
            let mut emit = |line: String| log(&line);
            install_spec(&dir, env.as_deref(), &trimmed, &mut emit)
                .map(|res| format!("installed {} ({})", res.name, res.spec))
                .map_err(|e| e.to_string())
        });
    }

    /// Remove a package from `ravom.toml`, `.ravom/libdeps/<env>/`, and `Cargo.toml`.
    pub fn spawn_pkg_remove(&mut self, spec_or_name: String) {
        let (dir, env) = (self.project_path(), self.env_arg());
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console
            .push(format!("Removing library {spec_or_name}..."));
        self.tasks.spawn("Pkg", move |log| {
            let mut emit = |line: String| log(&line);
            remove_package(&dir, env.as_deref(), &spec_or_name, &mut emit)
                .map(|removed| {
                    if removed {
                        format!("removed {spec_or_name}")
                    } else {
                        format!("nothing removed for {spec_or_name}")
                    }
                })
                .map_err(|e| e.to_string())
        });
    }

    /// Sync all `lib_deps` configured in `ravom.toml` into local storage.
    pub fn spawn_pkg_sync(&mut self) {
        let (dir, env) = (self.project_path(), self.env_arg());
        self.drawer_open = true;
        self.drawer_tab = DrawerTab::Console;
        self.console.push("Syncing ravom.toml lib_deps...");
        self.tasks.spawn("Pkg", move |log| {
            let mut emit = |line: String| log(&line);
            sync_lib_deps(&dir, env.as_deref(), &mut emit)
                .map(|n| format!("synced {n} library entry(s)"))
                .map_err(|e| e.to_string())
        });
    }

    fn drain_task_events(&mut self) {
        let mut refresh_pkg = false;
        let mut refresh_doctor = false;
        for event in self.tasks.poll() {
            match event {
                TaskEvent::Log(line) => {
                    if line.starts_with("Build OK:") {
                        self.last_build_ok = true;
                    } else if line.starts_with("Build error:") {
                        self.last_build_ok = false;
                    } else if line.starts_with("Flash OK:") {
                        self.last_flash_ok = true;
                    } else if line.starts_with("Flash error:") {
                        self.last_flash_ok = false;
                    } else if line.starts_with("Pkg OK:") {
                        refresh_pkg = true;
                    } else if line.starts_with("Doctor OK:") {
                        refresh_doctor = true;
                    }
                    self.console.push(line);
                }
                TaskEvent::Finished => {}
            }
        }
        if refresh_pkg {
            self.try_load_project();
        }
        if refresh_doctor {
            self.run_doctor();
        }
    }

    fn format_serial_timestamp(&self) -> String {
        let elapsed = self
            .serial_started_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        let total_ms = elapsed.as_millis();
        let mins = (total_ms / 60_000) % 100;
        let secs = (total_ms / 1_000) % 60;
        let ms = total_ms % 1_000;
        format!("[{mins:02}:{secs:02}.{ms:03}] ")
    }

    fn drain_serial_events(&mut self) {
        let Some(session) = self.serial_session.as_ref() else {
            return;
        };
        let events = session.poll();
        let mut disconnected = false;
        for ev in events {
            match ev {
                SerialEvent::Bytes(chunk) => {
                    self.serial_rx_bytes += chunk.len() as u64;
                    let ts_prefix = if self.serial_timestamps {
                        self.format_serial_timestamp()
                    } else {
                        String::new()
                    };
                    let decoded = self.serial_decoder.feed(&chunk);
                    for frame in &decoded.frames {
                        self.serial_fps_frame_count += 1;
                        if !self.plotter.simulate {
                            self.plotter.push_frame(&frame.channels);
                        }
                    }

                    if !self.serial_pause_rx {
                        if self.serial_hex_rx {
                            self.serial_log
                                .push(format!("{ts_prefix}[HEX] {}", format_hex_dump(&chunk)));
                        } else if self.serial_protocol == VofaProtocol::JustFloat {
                            for frame in &decoded.frames {
                                let summary = frame
                                    .channels
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, (name, val))| match name.as_deref() {
                                        Some(label) => format!("{label}={val:.3}"),
                                        None => format!("I{idx}={val:.3}"),
                                    })
                                    .collect::<Vec<_>>()
                                    .join("  ");
                                if !summary.is_empty() {
                                    self.serial_log.push(format!("{ts_prefix}[JF] {summary}"));
                                }
                            }
                        } else {
                            for line in decoded.text_lines {
                                self.serial_log.push(format!("{ts_prefix}{line}"));
                            }
                        }
                    }
                }
                SerialEvent::Error(err) => {
                    self.serial_log.push(format!("Serial error: {err}"));
                    disconnected = true;
                }
                SerialEvent::Disconnected => {
                    self.serial_log.push("--- Serial disconnected ---");
                    disconnected = true;
                }
            }
        }

        let elapsed_fps = self.serial_fps_window_start.elapsed().as_secs_f32();
        if elapsed_fps >= 0.5 {
            self.serial_fps = self.serial_fps_frame_count as f32 / elapsed_fps;
            self.serial_fps_frame_count = 0;
            self.serial_fps_window_start = Instant::now();
        }

        if disconnected {
            self.serial_session = None;
            self.serial_auto_send = false;
            self.serial_started_at = None;
            self.serial_fps = 0.0;
        }
    }

    fn poll_folder_picker(&mut self) {
        let Some(rx) = self.folder_picker_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Some(path)) => {
                self.folder_picker_rx = None;
                self.project_dir = path;
                self.try_load_project();
            }
            Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                self.folder_picker_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn poll_pkg_search(&mut self) {
        let Some(rx) = self.pkg_search_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(outcome) => {
                self.pkg_search_rx = None;
                self.pkg_searching = false;
                self.pkg_results = outcome.packages;
                self.pkg_remote_total = outcome.remote_total;
                self.pkg_warnings = outcome.warnings;
                if self
                    .selected_pkg
                    .as_ref()
                    .is_none_or(|id| !self.pkg_results.iter().any(|p| p.qualified_id() == *id))
                {
                    self.selected_pkg = self.pkg_results.first().map(PackageRecord::qualified_id);
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pkg_search_rx = None;
                self.pkg_searching = false;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn poll_tool_picker(&mut self) {
        let Some(rx) = self.tool_picker_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Some(path)) => {
                self.tool_picker_rx = None;
                self.doctor_path_draft = path;
                self.save_doctor_tool_path();
            }
            Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                self.tool_picker_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        ctx.input(|i| {
            if !i.modifiers.command {
                return;
            }
            if i.key_pressed(egui::Key::Num1) {
                self.active_tab = NavTab::Workspace;
            } else if i.key_pressed(egui::Key::Num2) {
                self.refresh_installed_packages();
                self.active_tab = NavTab::Libraries;
            } else if i.key_pressed(egui::Key::Num3) {
                self.active_tab = NavTab::Boards;
            } else if i.key_pressed(egui::Key::Num4) {
                self.active_tab = NavTab::Monitor;
            } else if i.key_pressed(egui::Key::Num5) {
                self.active_tab = NavTab::Doctor;
                if self.doctor_tools.is_empty() {
                    self.run_doctor();
                }
            } else if i.key_pressed(egui::Key::J) {
                self.drawer_open = !self.drawer_open;
            } else if i.key_pressed(egui::Key::B) && self.tasks.running().is_none() {
                self.spawn_build();
            }
        });
    }
}

fn pick_folder_os() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("osascript")
            .args([
                "-e",
                "POSIX path of (choose folder with prompt \"Select Ravomi Workspace\")",
            ])
            .output()
            .ok()?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return (!s.is_empty()).then_some(s);
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        let out = std::process::Command::new("zenity")
            .args([
                "--file-selection",
                "--directory",
                "--title=Select Ravomi Workspace",
            ])
            .output()
            .ok()?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return (!s.is_empty()).then_some(s);
        }
        None
    }
}

fn pick_tool_file_os(tool: &str) -> Option<String> {
    let prompt = format!("Select executable for {tool}");
    #[cfg(target_os = "macos")]
    {
        let escaped = prompt.replace('"', "\\\"");
        let script = format!("POSIX path of (choose file with prompt \"{escaped}\")");
        let out = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .ok()?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return (!s.is_empty()).then_some(s);
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        let out = std::process::Command::new("zenity")
            .args(["--file-selection", &format!("--title={prompt}")])
            .output()
            .ok()?;
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return (!s.is_empty()).then_some(s);
        }
        None
    }
}

impl eframe::App for RavomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_task_events();
        self.drain_serial_events();
        if self.serial_auto_send && self.serial_session.is_some() {
            let interval =
                std::time::Duration::from_millis(self.serial_auto_send_ms.clamp(50, 60_000) as u64);
            if self.serial_last_auto_send.elapsed() >= interval {
                let payload = self.serial_tx_input.clone();
                if !payload.trim().is_empty() {
                    self.send_serial(&payload);
                }
                self.serial_last_auto_send = Instant::now();
            }
        }
        self.poll_folder_picker();
        self.poll_pkg_search();
        self.poll_tool_picker();
        self.handle_shortcuts(ctx);

        if self.tasks.running().is_some()
            || self.serial_session.is_some()
            || (self.serial_auto_send && self.serial_session.is_some())
            || self.folder_picker_rx.is_some()
            || self.tool_picker_rx.is_some()
            || self.pkg_searching
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }

        // 1. Unified Integrated Titlebar / Command Bar (44px)
        egui::TopBottomPanel::top("topbar")
            .exact_height(44.0)
            .show_separator_line(true)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_CHROME)
                    .stroke(egui::Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| panels::topbar::show(self, ui));

        // 2. Bottom Status Bar (26px)
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(26.0)
            .show_separator_line(true)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_CHROME)
                    .stroke(egui::Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(14, 4)),
            )
            .show(ctx, |ui| panels::statusbar::show(self, ui));

        // 3. Far-Left Activity Rail (56px)
        egui::SidePanel::left("activity_rail")
            .exact_width(56.0)
            .resizable(false)
            .show_separator_line(true)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_CHROME)
                    .stroke(egui::Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(5, 8)),
            )
            .show(ctx, |ui| panels::rail::show(self, ui));

        // 4. Collapsible Bottom Output Drawer
        if self.drawer_open {
            egui::TopBottomPanel::bottom("output_drawer")
                .resizable(true)
                .default_height(200.0)
                .min_height(140.0)
                .max_height(420.0)
                .show_separator_line(true)
                .frame(
                    egui::Frame::new()
                        .fill(theme::BG_SURFACE)
                        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                        .inner_margin(egui::Margin::symmetric(16, 10)),
                )
                .show(ctx, |ui| panels::drawer::show(self, ui));
        }

        // 5. Context-Driven Main Stage (Ma breathing space)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_CANVAS)
                    .inner_margin(egui::Margin::symmetric(28, 22)),
            )
            .show(ctx, |ui| match self.active_tab {
                NavTab::Workspace => panels::workspace::show(self, ui),
                NavTab::Libraries => panels::libraries::show(self, ui),
                NavTab::Boards => panels::boards::show(self, ui),
                NavTab::Monitor => panels::monitor::show(self, ui),
                NavTab::Doctor => panels::doctor::show(self, ui),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::{NavTab, RavomApp};
    use crate::theme::StepState;

    #[test]
    fn env_arg_trims_and_skips_empty() {
        let mut app = RavomApp::new();
        app.selected_env = "   ".to_string();
        assert_eq!(app.env_arg(), None);
        app.selected_env = " bluepill_f103c8 ".to_string();
        assert_eq!(app.env_arg().as_deref(), Some("bluepill_f103c8"));
    }

    #[test]
    fn workflow_steps_track_manifest_build_and_flash_progress() {
        let mut app = RavomApp::new();
        app.loaded_config = None;
        app.last_build_ok = false;
        app.last_flash_ok = false;
        let steps = app.workflow_steps();
        for (label, _) in &steps {
            assert!(
                !label.chars().next().is_some_and(|c| c.is_ascii_digit()),
                "step label must not start with duplicate digit: {label}"
            );
        }
        assert_eq!(steps[1].1, StepState::Active);
        assert_eq!(steps[2].1, StepState::Pending);

        app.active_tab = NavTab::Boards;
        assert_eq!(app.active_tab, NavTab::Boards);

        app.last_build_ok = true;
        assert_eq!(app.workflow_steps()[2].1, StepState::Pending);
    }
}
