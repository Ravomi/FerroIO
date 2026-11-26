//! Application state and frame orchestration.

use crate::board_index::BoardIndex;
use crate::console::Console;
use crate::panels;
use crate::plotter::Plotter;
use crate::tasks::{TaskEvent, TaskRunner};
use eframe::egui;
use ravom_core::config::RavomConfig;
use ravom_core::monitor::{list_serial_port_details, SerialPortEntry};
use ravom_core::registry::registry;
use std::path::PathBuf;

pub struct RavomApp {
    pub project_dir: String,
    pub board_query: String,
    pub boards: BoardIndex,
    /// Qualified `<platform>/<id>` of the selected board.
    pub selected_board: String,
    pub selected_env: String,
    /// Framework override used when clicking `Init` (`""` = board default).
    pub init_framework: String,
    pub release_mode: bool,
    pub loaded_config: Option<RavomConfig>,
    pub overlay_draft: String,
    pub overlay_dirty: bool,
    pub serial_ports: Vec<SerialPortEntry>,
    pub console: Console,
    pub tasks: TaskRunner,
    pub plotter: Plotter,
}

impl RavomApp {
    pub fn new() -> Self {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();
        let boards = BoardIndex::new(registry().boards().cloned().collect());
        let mut console = Console::default();
        console.push(format!(
            "RavomIO Workbench v{} ready ({} boards loaded).",
            ravom_core::version(),
            boards.len()
        ));
        let mut app = Self {
            project_dir: cwd,
            board_query: String::new(),
            boards,
            selected_board: "ststm32/bluepill_f103c8".to_string(),
            selected_env: String::new(),
            init_framework: String::new(),
            release_mode: false,
            loaded_config: None,
            overlay_draft: String::new(),
            overlay_dirty: false,
            serial_ports: list_serial_port_details(),
            console,
            tasks: TaskRunner::default(),
            plotter: Plotter::default(),
        };
        app.try_load_project();
        app
    }

    pub fn project_path(&self) -> PathBuf {
        PathBuf::from(&self.project_dir)
    }

    pub fn try_load_project(&mut self) {
        let dir = self.project_path();
        match RavomConfig::load_from_dir(&dir) {
            Ok(cfg) => {
                if let Ok((env_name, _)) = cfg.resolve_env(None) {
                    self.selected_env = env_name.to_string();
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
                self.console
                    .push(format!("No active project loaded: {err}"));
            }
        }
    }

    pub fn env_arg(&self) -> Option<String> {
        let trimmed = self.selected_env.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
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

    pub fn spawn_build(&mut self) {
        let (dir, env, release) = (self.project_path(), self.env_arg(), self.release_mode);
        self.console.push("Starting background build...");
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

    pub fn spawn_flash(&mut self) {
        let (dir, env, release) = (self.project_path(), self.env_arg(), self.release_mode);
        self.console.push("Starting background flash...");
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

    fn drain_task_events(&mut self) {
        for event in self.tasks.poll() {
            match event {
                TaskEvent::Log(line) => self.console.push(line),
                TaskEvent::Finished => {}
            }
        }
    }
}

impl eframe::App for RavomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_task_events();
        if self.tasks.running().is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        let panel_margin = egui::Margin::symmetric(14, 10);
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::BG_SURFACE)
                    .stroke(egui::Stroke::new(1.0_f32, crate::theme::BORDER))
                    .inner_margin(panel_margin),
            )
            .show(ctx, |ui| panels::toolbar::show(self, ui));

        egui::SidePanel::left("sidebar")
            .default_width(300.0)
            .min_width(250.0)
            .max_width(420.0)
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::BG_BASE)
                    .stroke(egui::Stroke::new(1.0_f32, crate::theme::BORDER))
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| panels::boards::show(self, ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::BG_BASE)
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ctx, |ui| panels::project::show(self, ui));
    }
}

#[cfg(test)]
mod tests {
    use super::RavomApp;

    #[test]
    fn env_arg_trims_and_skips_empty() {
        let mut app = RavomApp::new();
        app.selected_env = "   ".to_string();
        assert_eq!(app.env_arg(), None);
        app.selected_env = " bluepill_f103c8 ".to_string();
        assert_eq!(app.env_arg().as_deref(), Some("bluepill_f103c8"));
    }
}
