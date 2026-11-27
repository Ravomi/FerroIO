use crate::app::{NavTab, RavomApp};
use crate::icons::{self, Icon};
use crate::theme;
use eframe::egui::{self, Layout};

/// Render the 56px far-left vertical Activity Rail.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;

        if icons::rail_button(
            ui,
            Icon::Layers,
            "Project",
            app.active_tab == NavTab::Workspace,
            app.loaded_config.as_ref().map(|_| theme::FG_PRIMARY),
            "Project & Workflow (Cmd/Ctrl+1)",
        )
        .clicked()
        {
            app.active_tab = NavTab::Workspace;
        }

        let lib_dot = (!app.installed_packages.is_empty()).then_some(theme::FG_SECONDARY);
        if icons::rail_button(
            ui,
            Icon::Package,
            "Libs",
            app.active_tab == NavTab::Libraries,
            lib_dot,
            "Libraries & Packages (Cmd/Ctrl+2)",
        )
        .clicked()
        {
            app.refresh_installed_packages();
            app.active_tab = NavTab::Libraries;
        }

        if icons::rail_button(
            ui,
            Icon::Cpu,
            "Boards",
            app.active_tab == NavTab::Boards,
            None,
            "Board Catalog (Cmd/Ctrl+3)",
        )
        .clicked()
        {
            app.active_tab = NavTab::Boards;
        }

        let serial_dot = if app.serial_session.is_some() {
            Some(theme::FG_PRIMARY)
        } else if !app.visible_serial_ports().is_empty() {
            Some(theme::FG_MUTED)
        } else {
            None
        };
        if icons::rail_button(
            ui,
            Icon::Terminal,
            "Serial",
            app.active_tab == NavTab::Monitor,
            serial_dot,
            "Serial Studio & Scope (Cmd/Ctrl+4)",
        )
        .clicked()
        {
            app.refresh_ports();
            app.active_tab = NavTab::Monitor;
        }

        let doctor_dot =
            if !app.doctor_tools.is_empty() && app.doctor_tools.iter().any(|t| !t.installed) {
                Some(theme::FG_PRIMARY)
            } else {
                None
            };
        if icons::rail_button(
            ui,
            Icon::Activity,
            "Doctor",
            app.active_tab == NavTab::Doctor,
            doctor_dot,
            "Toolchain & Rust Targets (Cmd/Ctrl+5)",
        )
        .clicked()
        {
            if app.doctor_tools.is_empty() {
                app.run_doctor();
            }
            app.active_tab = NavTab::Doctor;
        }

        ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
            let task_dot = if app.tasks.running().is_some() {
                Some(theme::FG_PRIMARY)
            } else {
                None
            };
            if icons::rail_button(
                ui,
                Icon::PanelBottom,
                "Drawer",
                app.drawer_open,
                task_dot,
                "Output Drawer (Cmd/Ctrl+J)",
            )
            .clicked()
            {
                app.drawer_open = !app.drawer_open;
            }
        });
    });
}
