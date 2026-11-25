use eframe::egui;
use ravom_core::config::RavomConfig;
use ravom_core::monitor::list_serial_ports;
use ravom_core::project::init_project;
use ravom_core::registry::{load_all_boards, BoardRecord};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 760.0]),
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
    board_query: String,
    all_boards: Vec<BoardRecord>,
    selected_board: String,
    selected_env: String,
    release_mode: bool,
    loaded_config: Option<RavomConfig>,
    serial_ports: Vec<String>,
    logs: Vec<String>,
    log_tx: Sender<String>,
    log_rx: Receiver<String>,
    simulate_signal: bool,
    signal_phase: f32,
    signal_samples: VecDeque<f32>,
}

impl RavomApp {
    fn new() -> Self {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .display()
            .to_string();
        let (log_tx, log_rx) = mpsc::channel();
        let all_boards = load_all_boards();
        let board_count = all_boards.len();
        let mut app = Self {
            project_dir: cwd,
            board_query: String::new(),
            all_boards,
            selected_board: "bluepill_f103c8".to_string(),
            selected_env: String::new(),
            release_mode: false,
            loaded_config: None,
            serial_ports: list_serial_ports(),
            logs: vec![format!(
                "RavomIO Workbench v{} ready ({board_count} boards loaded).",
                ravom_core::version()
            )],
            log_tx,
            log_rx,
            simulate_signal: false,
            signal_phase: 0.0,
            signal_samples: VecDeque::with_capacity(160),
        };
        app.try_load_project();
        app
    }

    fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
    }

    fn drain_background_logs(&mut self) {
        while let Ok(line) = self.log_rx.try_recv() {
            self.logs.push(line);
        }
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

    fn env_arg(&self) -> Option<String> {
        let trimmed = self.selected_env.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn spawn_build(&mut self) {
        let dir = PathBuf::from(&self.project_dir);
        let env = self.env_arg();
        let release = self.release_mode;
        let tx = self.log_tx.clone();
        self.log("Starting background build...");
        thread::spawn(move || {
            let msg = match ravom_core::build::build_project(&dir, env.as_deref(), release) {
                Ok(res) => format!(
                    "Build OK: env={} board={} target={}",
                    res.env_name,
                    res.board_id,
                    res.target.as_deref().unwrap_or("host")
                ),
                Err(err) => format!("Build error: {err}"),
            };
            let _ = tx.send(msg);
        });
    }

    fn spawn_flash(&mut self) {
        let dir = PathBuf::from(&self.project_dir);
        let env = self.env_arg();
        let release = self.release_mode;
        let tx = self.log_tx.clone();
        self.log("Starting background flash...");
        thread::spawn(move || {
            let msg = match ravom_core::flash::flash_project(&dir, env.as_deref(), release, false) {
                Ok(res) => format!(
                    "Flash OK: env={} chip={} ({})",
                    res.env_name,
                    res.chip,
                    res.firmware.display()
                ),
                Err(err) => format!("Flash error: {err}"),
            };
            let _ = tx.send(msg);
        });
    }

    fn draw_signal_plotter(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.heading("Signal Plotter");
            ui.checkbox(&mut self.simulate_signal, "Simulate Signal");
            if ui.small_button("Reset").clicked() {
                self.signal_samples.clear();
            }
        });

        if self.simulate_signal {
            self.signal_phase += 0.15;
            let sample = (self.signal_phase.sin() * 0.7) + ((self.signal_phase * 2.7).cos() * 0.25);
            if self.signal_samples.len() >= 160 {
                self.signal_samples.pop_front();
            }
            self.signal_samples.push_back(sample);
            ctx.request_repaint();
        }

        let desired = egui::vec2(ui.available_width(), 120.0);
        let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, egui::Color32::from_gray(24));

        if self.signal_samples.len() >= 2 {
            let n = self.signal_samples.len() as f32;
            let points: Vec<egui::Pos2> = self
                .signal_samples
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let x = rect.left() + (i as f32 / (n - 1.0).max(1.0)) * rect.width();
                    let y = rect.center().y - v * (rect.height() * 0.42);
                    egui::pos2(x, y)
                })
                .collect();
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(80, 220, 140)),
            ));
        }
    }
}

impl eframe::App for RavomApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_background_logs();

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
                    self.spawn_build();
                }
                if ui.button("Flash").clicked() {
                    self.spawn_flash();
                }
                if ui.button("Doctor").clicked() {
                    self.log("--- Toolchain Doctor ---");
                    for tool in ravom_core::toolchain::check_toolchains() {
                        let status = if tool.installed { "OK" } else { "MISSING" };
                        self.log(format!("{:<18} {}", tool.name, status));
                    }
                }
            });
        });

        egui::SidePanel::left("sidebar")
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading(format!("Boards ({})", self.all_boards.len()));
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.board_query);
                });
                let q = self.board_query.trim().to_ascii_lowercase();
                egui::ScrollArea::vertical()
                    .id_salt("boards_scroll")
                    .max_height(360.0)
                    .show(ui, |ui| {
                        let mut shown = 0usize;
                        for b in &self.all_boards {
                            if !q.is_empty()
                                && !b.id.to_ascii_lowercase().contains(&q)
                                && !b.mcu.to_ascii_lowercase().contains(&q)
                                && !b.platform.to_ascii_lowercase().contains(&q)
                                && !b.name.to_ascii_lowercase().contains(&q)
                            {
                                continue;
                            }
                            let selected = self.selected_board == b.id;
                            if ui
                                .selectable_label(selected, format!("{} [{}]", b.id, b.platform))
                                .clicked()
                            {
                                self.selected_board = b.id.clone();
                            }
                            shown += 1;
                            if shown >= 200 {
                                ui.label("... refine filter to see more boards");
                                break;
                            }
                        }
                    });

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
            self.draw_signal_plotter(ui, ctx);

            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("Console");
                if ui.small_button("Clear").clicked() {
                    self.logs.clear();
                }
            });
            egui::ScrollArea::vertical()
                .id_salt("console_scroll")
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.logs {
                        ui.monospace(line);
                    }
                });
        });
    }
}
