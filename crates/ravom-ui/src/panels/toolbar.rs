use crate::app::RavomApp;
use crate::icons::{self, Icon};
use crate::theme;
use eframe::egui::{self, Layout, RichText};
use ravom_core::project::init_project;

pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    // Row 1: Brand + primary build/flash actions (never clipped).
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("RavomIO")
                .size(15.0)
                .strong()
                .color(theme::FG_PRIMARY),
        );
        theme::badge(ui, format!("v{}", ravom_core::version()), theme::FG_MUTED);

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let idle = app.tasks.running().is_none();

            if icons::button(ui, Icon::Play, "Build", true, idle).clicked() {
                app.spawn_build();
            }
            if icons::button(ui, Icon::Zap, "Flash", false, idle).clicked() {
                app.spawn_flash();
            }
            if icons::button(ui, Icon::Activity, "Doctor", false, true).clicked() {
                app.console.push("--- Toolchain Doctor ---");
                for tool in ravom_core::toolchain::check_toolchains() {
                    let status = if tool.installed { "OK" } else { "MISSING" };
                    let detail = tool.version.unwrap_or_default();
                    app.console
                        .push(format!("{:<18} {:<8} {detail}", tool.name, status));
                }
                for warning in ravom_core::toolchain::check_host_setup() {
                    app.console.push(format!("warning: {warning}"));
                }
            }
            ui.checkbox(&mut app.release_mode, "Release");

            if let Some(label) = app.tasks.running() {
                ui.label(
                    RichText::new(format!("{label}..."))
                        .small()
                        .color(theme::FG_MUTED),
                );
                ui.spinner();
            }
        });
    });

    ui.add_space(6.0);

    // Row 2: Workspace path + project initialization controls (aligned at 26px height).
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().interact_size.y = 26.0;
        ui.label(RichText::new("Workspace").size(12.0).color(theme::FG_MUTED));
        ui.add(
            egui::TextEdit::singleline(&mut app.project_dir)
                .desired_width(240.0)
                .hint_text("/path/to/project"),
        );
        if icons::button(ui, Icon::Folder, "Load", false, true).clicked() {
            app.try_load_project();
        }

        ui.separator();

        let fw_label = match app.init_framework.as_str() {
            "" => "Board Default",
            "zephyr" => "Zephyr (C)",
            "zephyr-rust" => "Zephyr (Rust)",
            "embassy" => "Embassy",
            other => other,
        };
        egui::ComboBox::from_id_salt("init_framework")
            .selected_text(fw_label)
            .width(120.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.init_framework, String::new(), "Board Default");
                ui.selectable_value(&mut app.init_framework, "zephyr".to_string(), "Zephyr (C)");
                ui.selectable_value(
                    &mut app.init_framework,
                    "zephyr-rust".to_string(),
                    "Zephyr (Rust)",
                );
                ui.selectable_value(&mut app.init_framework, "embassy".to_string(), "Embassy");
            });

        if icons::button(ui, Icon::Plus, "Init", false, true).clicked() {
            let fw = (!app.init_framework.is_empty()).then_some(app.init_framework.as_str());
            match init_project(&app.project_path(), &app.selected_board, fw) {
                Ok(res) => {
                    app.console.push(format!(
                        "Initialized {} (board: {}, framework: {})",
                        res.root.display(),
                        res.board_id,
                        res.framework
                    ));
                    app.try_load_project();
                }
                Err(err) => app.console.push(format!("Init error: {err}")),
            }
        }
    });
}
