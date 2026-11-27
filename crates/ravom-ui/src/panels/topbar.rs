use crate::app::{NavTab, RavomApp};
use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui::{self, Layout, RichText};

/// Leading clearance for macOS traffic-light buttons when not in fullscreen mode.
#[cfg(any(target_os = "macos", test))]
fn macos_leading_inset(fullscreen: bool) -> f32 {
    if fullscreen {
        0.0
    } else {
        68.0
    }
}

/// Render the 42px unified titlebar & IDE command bar.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    let bar_rect = ui.max_rect().expand2(egui::vec2(12.0, 6.0));
    let drag_resp = ui.interact(
        bar_rect,
        ui.id().with("unified_titlebar_drag"),
        egui::Sense::click_and_drag(),
    );
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
    let mut pointer_on_control = false;

    #[cfg(target_os = "macos")]
    let is_fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
    #[cfg(target_os = "macos")]
    if !is_fullscreen && pointer_pos.is_some_and(|p| p.x < bar_rect.left() + 78.0) {
        pointer_on_control = true;
    }

    let mut mark_control = |rect: egui::Rect| {
        if pointer_pos.is_some_and(|p| rect.contains(p)) {
            pointer_on_control = true;
        }
    };

    ui.horizontal_centered(|ui| {
        ui.spacing_mut().interact_size.y = 30.0;

        #[cfg(target_os = "macos")]
        {
            let inset = macos_leading_inset(is_fullscreen);
            if inset > 0.0 {
                ui.add_space(inset);
            }
        }

        // Left: Brand identity & separator slash (project details live in stage hero)
        let logo = app.brand_texture.get_or_insert_with(|| {
            ui.ctx().load_texture(
                "ravomio-brand-logo",
                theme::brand_logo_image(),
                egui::TextureOptions::LINEAR,
            )
        });
        ui.image(egui::load::SizedTexture::new(
            logo.id(),
            egui::vec2(20.0, 20.0),
        ));
        ui.label(
            RichText::new("Ravomi")
                .size(14.0)
                .strong()
                .color(theme::FG_PRIMARY),
        );
        ui.label(RichText::new("/").size(13.0).color(theme::FG_FAINT));
        if let Some(cfg) = &app.loaded_config {
            let name = cfg.project.name.as_deref().unwrap_or("project");
            ui.label(
                RichText::new(name)
                    .size(12.5)
                    .monospace()
                    .color(theme::FG_SECONDARY),
            );
        } else {
            ui.label(
                RichText::new(format!("v{}", ravom_core::version()))
                    .size(11.0)
                    .monospace()
                    .color(theme::FG_MUTED),
            );
        }

        theme::v_divider(ui, 14.0);

        // Right-aligned window controls (non-macOS) + primary build & flash actions
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            #[cfg(not(target_os = "macos"))]
            {
                let close_resp = ui
                    .add(
                        egui::Button::new(RichText::new("×").size(14.0).color(theme::FG_MUTED))
                            .frame(false),
                    )
                    .on_hover_text("Close");
                mark_control(close_resp.rect);
                if close_resp.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }

                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                let max_resp = ui
                    .add(
                        egui::Button::new(RichText::new("□").size(12.5).color(theme::FG_MUTED))
                            .frame(false),
                    )
                    .on_hover_text("Maximize / Restore");
                mark_control(max_resp.rect);
                if max_resp.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }

                let min_resp = ui
                    .add(
                        egui::Button::new(RichText::new("−").size(13.5).color(theme::FG_MUTED))
                            .frame(false),
                    )
                    .on_hover_text("Minimize");
                mark_control(min_resp.rect);
                if min_resp.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }

                theme::v_divider(ui, 16.0);
            }

            let idle = app.tasks.running().is_none();
            let has_project = app.loaded_config.is_some();

            let flash_resp = icons::button(
                ui,
                Icon::Zap,
                "Flash",
                ButtonKind::Primary,
                idle && has_project,
            )
            .on_hover_text("Flash firmware to target hardware (auto-disconnects serial)");
            mark_control(flash_resp.rect);
            if flash_resp.clicked() {
                app.spawn_flash();
            }

            let build_resp = icons::button(
                ui,
                Icon::Play,
                "Build",
                ButtonKind::Outline,
                idle && has_project,
            )
            .on_hover_text("Build active environment (Cmd/Ctrl+B)");
            mark_control(build_resp.rect);
            if build_resp.clicked() {
                app.spawn_build();
            }

            let toggle_scope = ui.scope(|ui| {
                theme::segmented_toggle(
                    ui,
                    "profile_mode_toggle",
                    "Debug",
                    "Release",
                    &mut app.release_mode,
                );
            });
            mark_control(toggle_scope.response.rect);

            if let Some(label) = app.tasks.running() {
                ui.spinner();
                ui.label(
                    RichText::new(format!("{label}..."))
                        .size(11.5)
                        .color(theme::FG_PRIMARY),
                );
            }

            theme::v_divider(ui, 16.0);

            // Center-left workspace path input + icon-only Refresh / Browse controls
            ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                let edit_w = (ui.available_width() - 92.0).clamp(110.0, 420.0);
                let path_id = ui.make_persistent_id("topbar_project_path");
                let path_scope = ui.scope(|ui| {
                    theme::text_input(
                        ui,
                        path_id,
                        &mut app.project_dir,
                        "/path/to/embedded/project",
                        edit_w,
                        false,
                        false,
                    )
                });
                mark_control(path_scope.response.rect);
                if path_scope.inner.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    app.try_load_project();
                    app.active_tab = NavTab::Workspace;
                }
                let load_resp = icons::button(ui, Icon::Refresh, "", ButtonKind::Ghost, true)
                    .on_hover_text("Reload ravom.toml");
                mark_control(load_resp.rect);
                if load_resp.clicked() {
                    app.try_load_project();
                }
                let browse_resp = icons::button(ui, Icon::Folder, "", ButtonKind::Ghost, true)
                    .on_hover_text("Open workspace folder");
                mark_control(browse_resp.rect);
                if browse_resp.clicked() {
                    app.open_folder_dialog();
                }
            });
        });
    });

    if !pointer_on_control {
        if drag_resp.drag_started_by(egui::PointerButton::Primary)
            && ui.input(|i| i.pointer.primary_down())
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        } else if drag_resp.double_clicked() {
            let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::macos_leading_inset;

    #[test]
    fn macos_leading_inset_accounts_for_fullscreen_state() {
        assert_eq!(macos_leading_inset(false), 68.0);
        assert_eq!(macos_leading_inset(true), 0.0);
    }
}
