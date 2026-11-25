use eframe::egui;
use ravom_core::config::{builtin_boards, RavomConfig};
use ravom_core::monitor::list_serial_ports;
use ravom_core::project::init_project;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RavomIO Workbench",
        options,
        Box::new(|_cc| Ok(Box::new(RavomApp::new()))),
    )
}

struct RavomApp {
    project_dir: String,
    selected_board: String,
    selected_env: String,
    release_mode: bool,
    loaded_config: Option<RavomConfig>,
    serial_ports: Vec<String>,
    logs: Vec<String>,
}

impl RavomApp {
    fn new() -> Self {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();
        let mut app = Self {
            project_dir: cwd,
            selected_board: "stm32f401ccu6".to_string(),
            selected_env: String::new(),
            release_mode: false,
            loaded_config: None,
            serial_ports: list_serial_ports(),
            logs: vec![format!(
                "RavomIO Workbench v{} ready.",
                ravom_core::version()
            )],
        };
        app.try_load_project();
        app
    }

    fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
    }

    fn try_load_project(&mut self) {
        let dir = PathBuf::from(&self.project_dir);
        match RavomConfig::load_from_dir(&dir) {
            Ok(cfg) => {
                if let Ok((env_name, _)) = cfg.resolve_env(None) {
                    self.selected_env = env_name.to_string();
                }
                self.loaded_config = Some(cfg);
                self.log(format!("Loaded ravom.toml from {}", dir.display()));
            }
            Err(err) => {
                self.loaded_config = None;
                self.log(format!("No active project loaded: {err}"));
            }
        }
    }

    fn env_arg(&self) -> Option<&str> {
        let trimmed = self.selected_env.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}

impl eframe::App for RavomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("RavomIO v{}", ravom_core::version()));
                ui.separator();
                ui.label("Project:");
                ui.text_edit_singleline(&mut self.project_dir);
                if ui.button("Load").clicked() {
                    self.try_load_project();
                }
                if ui.button("Init").clicked() {
                    let dir = PathBuf::from(&self.project_dir);
                    match init_project(&dir, &self.selected_board, None) {
                        Ok(res) => {
                            self.log(format!(
                                "Initialized {} (board: {}, framework: {})",
                                res.root.display(),
                                res.board_id,
                                res.framework
                            ));
                            self.try_load_project();
                        }
                        Err(err) => self.log(format!("Init error: {err}")),
                    }
                }
                ui.checkbox(&mut self.release_mode, "Release");
                if ui.button("Build").clicked() {
                    let dir = PathBuf::from(&self.project_dir);
                    let env = self.env_arg().map(str::to_string);
                    match ravom_core::build::build_project(&dir, env.as_deref(), self.release_mode)
                    {
                        Ok(res) => self.log(format!(
                            "Build OK: env={} board={} target={}",
                            res.env_name,
                            res.board_id,
                            res.target.as_deref().unwrap_or("host")
                        )),
                        Err(err) => self.log(format!("Build error: {err}")),
                    }
                }
                if ui.button("Flash").clicked() {
                    let dir = PathBuf::from(&self.project_dir);
                    let env = self.env_arg().map(str::to_string);
                    match ravom_core::flash::flash_project(
                        &dir,
                        env.as_deref(),
                        self.release_mode,
                        false,
                    ) {
                        Ok(res) => self.log(format!(
                            "Flash OK: env={} chip={} ({})",
                            res.env_name,
                            res.chip,
                            res.firmware.display()
                        )),
                        Err(err) => self.log(format!("Flash error: {err}")),
                    }
                }
            });
        });

        egui::SidePanel::left("sidebar")
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("Boards");
                for b in builtin_boards() {
                    let selected = self.selected_board == b.id;
                    if ui
                        .selectable_label(selected, format!("{} ({})", b.id, b.mcu))
                        .clicked()
                    {
                        self.selected_board = b.id.to_string();
                    }
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.heading("Serial Ports");
                    if ui.small_button("Refresh").clicked() {
                        self.serial_ports = list_serial_ports();
                    }
                });
                if self.serial_ports.is_empty() {
                    ui.label("No serial ports detected.");
                } else {
                    for p in &self.serial_ports {
                        ui.monospace(p);
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Configuration");
            if let Some(cfg) = &self.loaded_config {
                ui.label(format!(
                    "Name: {}",
                    cfg.project.name.as_deref().unwrap_or("(unnamed)")
                ));
                ui.horizontal(|ui| {
                    ui.label("Active Env:");
                    ui.text_edit_singleline(&mut self.selected_env);
                });
                for (name, env) in &cfg.env {
                    ui.group(|ui| {
                        ui.strong(format!("[env.{name}]"));
                        ui.label(format!(
                            "platform = {} | board = {} | framework = {}",
                            env.platform, env.board, env.framework
                        ));
                    });
                }
            } else {
                ui.label("No ravom.toml loaded. Choose a board on the left and click Init, or enter an existing project path.");
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("Console");
                if ui.small_button("Clear").clicked() {
                    self.logs.clear();
                }
            });
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.logs {
                        ui.monospace(line);
                    }
                });
        });
    }
}
