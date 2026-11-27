use crate::app::{LibrarySubTab, NavTab, RavomApp};
use crate::icons::{self, ButtonKind, Icon};
use crate::theme;
use eframe::egui::{self, Layout, Margin, RichText, Sense, Stroke};
use ravom_core::pkg::{
    sanitize_dir_name, PackageRecord, PkgCategory, RegistrySource, SearchSourceFilter,
};

/// Format download or star popularity counts into compact `k` / `M` strings.
pub fn format_popularity(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Render the Rikyu-styled monochrome Libraries & Package Manager hub.
pub fn show(app: &mut RavomApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        theme::hero_title(ui, "Libraries");
        ui.add_space(14.0);
        if theme::underline_tab(ui, "Registry", app.lib_subtab == LibrarySubTab::Registry).clicked()
        {
            app.lib_subtab = LibrarySubTab::Registry;
        }
        let inst_label = format!("Installed ({})", app.installed_packages.len());
        if theme::underline_tab(ui, &inst_label, app.lib_subtab == LibrarySubTab::Installed)
            .clicked()
        {
            app.refresh_installed_packages();
            app.lib_subtab = LibrarySubTab::Installed;
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if app.lib_subtab == LibrarySubTab::Registry {
                let count_text = if app.pkg_remote_total > 0 {
                    format!(
                        "{} · {} remote",
                        app.pkg_results.len(),
                        app.pkg_remote_total
                    )
                } else {
                    format!("{} curated", app.pkg_results.len())
                };
                ui.label(
                    RichText::new(count_text)
                        .font(egui::FontId::monospace(10.5))
                        .color(theme::FG_MUTED),
                );
                if app.pkg_searching {
                    ui.spinner();
                }
            }
        });
    });

    theme::hairline(ui);
    ui.add_space(14.0);

    match app.lib_subtab {
        LibrarySubTab::Registry => show_registry(app, ui),
        LibrarySubTab::Installed => show_installed(app, ui),
    }
}

fn show_registry(app: &mut RavomApp, ui: &mut egui::Ui) {
    let full_h = ui.available_height();
    let left_w = (ui.available_width() * 0.40).clamp(300.0, 390.0);
    let terms: Vec<String> = app
        .pkg_query
        .split_whitespace()
        .map(|s| s.to_ascii_lowercase())
        .collect();

    ui.horizontal_top(|ui| {
        // Left Master List: Search + Underline Filters + Hairline Package Rows
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, full_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                let mut trigger_live = false;
                let mut trigger_filter = false;
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 30.0),
                    Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if icons::button(
                            ui,
                            Icon::Search,
                            "Live",
                            ButtonKind::Primary,
                            !app.pkg_searching,
                        )
                        .on_hover_text("Query crates.io & PlatformIO Registry v3")
                        .clicked()
                        {
                            trigger_live = true;
                        }
                        let input_w = ui.available_width().max(100.0);
                        let search_id = ui.make_persistent_id("lib_pkg_search");
                        let resp = theme::text_input(
                            ui,
                            search_id,
                            &mut app.pkg_query,
                            "Search button, st7789, ssd1306, lvgl...",
                            input_w,
                            true,
                            false,
                        );
                        if resp.changed() {
                            trigger_filter = true;
                        }
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            trigger_live = true;
                        }
                    },
                );
                if trigger_live {
                    app.run_pkg_search(true);
                } else if trigger_filter {
                    app.run_pkg_search(false);
                }

                if !app.pkg_warnings.is_empty() {
                    ui.add_space(6.0);
                    for warn in &app.pkg_warnings {
                        ui.label(
                            RichText::new(format!("• {warn}"))
                                .size(11.0)
                                .color(theme::FG_PRIMARY),
                        );
                    }
                }

                ui.add_space(8.0);

                // Source underline filters
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 2.0);
                    for &src in SearchSourceFilter::ALL {
                        let active = app.pkg_source == src;
                        if theme::underline_tab(ui, src.label(), active).clicked() {
                            app.pkg_source = src;
                            let trigger_online = !app.pkg_query.trim().is_empty()
                                || app.pkg_category != PkgCategory::All;
                            app.run_pkg_search(trigger_online);
                        }
                    }
                });

                ui.add_space(2.0);

                // Category underline filters
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                    for &cat in PkgCategory::ALL {
                        let active = app.pkg_category == cat;
                        if theme::underline_tab(ui, cat.label(), active).clicked() {
                            app.pkg_category = cat;
                            app.run_pkg_search(
                                cat != PkgCategory::All || !app.pkg_query.trim().is_empty(),
                            );
                        }
                    }
                });
                theme::hairline(ui);

                let list_h = (ui.available_height() - 4.0).max(160.0);
                let results = app.pkg_results.clone();
                let mut next_selected = None;

                egui::ScrollArea::vertical()
                    .id_salt("lib_registry_list")
                    .max_height(list_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        if results.is_empty() {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("No matches — click Live to query online registries")
                                    .size(12.0)
                                    .color(theme::FG_FAINT),
                            );
                            return;
                        }

                        for pkg in &results {
                            let qid = pkg.qualified_id();
                            let selected = app.selected_pkg.as_deref() == Some(qid.as_str());
                            let is_installed = app
                                .installed_packages
                                .iter()
                                .any(|ip| pkg.matches_spec(&ip.spec));
                            let card_id = ui.id().with(&qid);
                            let prev_hov = ui
                                .ctx()
                                .data(|d| d.get_temp::<bool>(card_id.with("is_hov")))
                                .unwrap_or(false);
                            let hover_t =
                                theme::animate_spring(ui.ctx(), card_id.with("hov"), prev_hov);
                            let sel_t =
                                theme::animate_spring(ui.ctx(), card_id.with("sel"), selected);

                            let base_fill =
                                theme::lerp_color(theme::BG_CANVAS, theme::BG_SURFACE, hover_t);
                            let fill = theme::lerp_color(base_fill, theme::BG_SURFACE, sel_t);

                            let title_job = theme::highlighted_job(
                                &pkg.name,
                                &terms,
                                12.5,
                                theme::FG_PRIMARY,
                                selected,
                            );
                            let pop_str = match pkg.source {
                                RegistrySource::CratesIo => {
                                    format!("↓ {}", format_popularity(pkg.popularity))
                                }
                                RegistrySource::PlatformIo => {
                                    format!("★ {}", format_popularity(pkg.popularity))
                                }
                            };
                            let meta = match &pkg.owner {
                                Some(o) => format!("{o} · v{} · {pop_str}", pkg.version),
                                None => format!("v{} · {pop_str}", pkg.version),
                            };

                            let card = egui::Frame::new()
                                .fill(fill)
                                .inner_margin(Margin::symmetric(12, 9))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let hdr_w = ui.available_width();
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(hdr_w, 20.0),
                                        Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(pkg.source.short_tag())
                                                    .font(egui::FontId::monospace(10.0))
                                                    .color(theme::FG_MUTED),
                                            );
                                            if is_installed {
                                                ui.label(
                                                    RichText::new("Installed")
                                                        .font(egui::FontId::monospace(10.0))
                                                        .color(theme::FG_PRIMARY),
                                                );
                                            }
                                            let title_w = ui.available_width().max(40.0);
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(title_w, 20.0),
                                                Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.add(egui::Label::new(title_job).truncate());
                                                },
                                            );
                                        },
                                    );
                                    ui.label(RichText::new(meta).size(10.5).color(theme::FG_MUTED));
                                    if !pkg.description.is_empty() {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&pkg.description)
                                                    .size(11.0)
                                                    .color(theme::FG_SECONDARY),
                                            )
                                            .truncate(),
                                        );
                                    }
                                });

                            let r = card.response.rect;
                            ui.painter().line_segment(
                                [
                                    egui::pos2(r.left(), r.bottom()),
                                    egui::pos2(r.right(), r.bottom()),
                                ],
                                Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                            );
                            if sel_t > 0.01 {
                                let bar_h = (r.height() - 10.0).max(4.0) * sel_t;
                                let bar = egui::Rect::from_center_size(
                                    egui::pos2(r.left() + 1.0, r.center().y),
                                    egui::vec2(2.0, bar_h),
                                );
                                ui.painter().rect_filled(bar, 1.0, theme::FG_PRIMARY);
                            }

                            let resp = card
                                .response
                                .interact(Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            let now_hov = resp.hovered();
                            if now_hov != prev_hov {
                                ui.ctx()
                                    .data_mut(|d| d.insert_temp(card_id.with("is_hov"), now_hov));
                                ui.ctx().request_repaint();
                            }
                            if resp.clicked() {
                                next_selected = Some(qid);
                            }
                        }
                    });

                if let Some(qid) = next_selected {
                    app.selected_pkg = Some(qid);
                }
            },
        );

        ui.separator();

        // Right Detail Inspector on open canvas
        let detail_w = ui.available_width().max(260.0);
        ui.allocate_ui_with_layout(
            egui::vec2(detail_w, full_h),
            Layout::top_down(egui::Align::Min),
            |ui| {
                let selected_pkg = app
                    .selected_pkg
                    .as_deref()
                    .and_then(|id| app.pkg_results.iter().find(|p| p.qualified_id() == id))
                    .cloned();

                let Some(pkg) = selected_pkg else {
                    ui.label(
                        RichText::new("Select a library")
                            .size(12.5)
                            .color(theme::FG_FAINT),
                    );
                    return;
                };

                render_pkg_detail(app, ui, &pkg, full_h);
            },
        );
    });
}

fn render_pkg_detail(app: &mut RavomApp, ui: &mut egui::Ui, pkg: &PackageRecord, full_h: f32) {
    let installed_match = app
        .installed_packages
        .iter()
        .find(|ip| pkg.matches_spec(&ip.spec))
        .cloned();
    let idle = app.tasks.running().is_none();
    let has_project = app.loaded_config.is_some();

    egui::ScrollArea::vertical()
        .id_salt("lib_detail_scroll")
        .max_height(full_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Header Row: Title + Quick Install / Remove Action
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if has_project {
                        if let Some(inst) = &installed_match {
                            if icons::button(ui, Icon::Trash, "Remove", ButtonKind::Outline, idle)
                                .clicked()
                            {
                                app.spawn_pkg_remove(inst.spec.clone());
                            }
                        } else if icons::button(
                            ui,
                            Icon::Download,
                            "Install",
                            ButtonKind::Primary,
                            idle,
                        )
                        .clicked()
                        {
                            app.spawn_pkg_install(pkg.clone());
                        }
                    } else if icons::button(
                        ui,
                        Icon::Layers,
                        "Open Project",
                        ButtonKind::Outline,
                        true,
                    )
                    .clicked()
                    {
                        app.active_tab = NavTab::Workspace;
                    }

                    ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            theme::eyebrow(ui, pkg.source.label());
                            theme::hero_title(ui, &pkg.name);
                            ui.label(
                                RichText::new(pkg.qualified_id())
                                    .size(11.5)
                                    .monospace()
                                    .color(theme::FG_SECONDARY),
                            );
                        });
                    });
                });
            });

            ui.add_space(10.0);
            let pop_label = match pkg.source {
                RegistrySource::CratesIo => format!("↓ {}", format_popularity(pkg.popularity)),
                RegistrySource::PlatformIo => {
                    format!("★ {}", format_popularity(pkg.popularity))
                }
            };
            let mut meta_parts = vec![format!("v{}", pkg.version), pop_label];
            if let Some(lic) = &pkg.license {
                meta_parts.push(lic.clone());
            }
            if pkg.curated {
                meta_parts.push("Curated".to_string());
            }
            ui.label(
                RichText::new(meta_parts.join("  ·  "))
                    .font(egui::FontId::monospace(11.0))
                    .color(theme::FG_MUTED),
            );

            ui.add_space(12.0);
            theme::hairline(ui);
            ui.add_space(12.0);

            if !pkg.description.is_empty() {
                ui.add(
                    egui::Label::new(
                        RichText::new(&pkg.description)
                            .size(12.5)
                            .color(theme::FG_SECONDARY),
                    )
                    .wrap(),
                );
                ui.add_space(12.0);
            }

            if !pkg.keywords.is_empty() {
                ui.label(
                    RichText::new(
                        pkg.keywords
                            .iter()
                            .take(6)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(" · "),
                    )
                    .font(egui::FontId::monospace(10.5))
                    .color(theme::FG_MUTED),
                );
                ui.add_space(14.0);
            }

            // Dependency spec preview
            let dep_spec = pkg.dep_spec();
            let env_display = if app.selected_env.trim().is_empty() {
                "default".to_string()
            } else {
                app.selected_env.trim().to_string()
            };
            let usage_line = match pkg.source {
                RegistrySource::CratesIo => format!("{} = \"^{}\"", pkg.name, pkg.version),
                RegistrySource::PlatformIo => format!(
                    ".ravom/libdeps/{}/{}/",
                    sanitize_dir_name(&env_display),
                    sanitize_dir_name(&pkg.name)
                ),
            };

            theme::eyebrow(ui, "Dependency Spec");
            ui.add_space(6.0);
            theme::inset_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let box_w = ui.available_width();
                ui.allocate_ui_with_layout(
                    egui::vec2(box_w, 34.0),
                    Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        theme::copy_feedback_button(
                            ui,
                            &format!("spec_{}", pkg.qualified_id()),
                            "",
                            &dep_spec,
                        );
                        let left_w = ui.available_width().max(80.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(left_w, 34.0),
                            Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("lib_deps = [\"{dep_spec}\"]"))
                                            .size(11.5)
                                            .monospace()
                                            .color(theme::FG_PRIMARY),
                                    )
                                    .truncate(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(usage_line)
                                            .size(11.0)
                                            .monospace()
                                            .color(theme::FG_MUTED),
                                    )
                                    .truncate(),
                                );
                            },
                        );
                    },
                );
            });

            if pkg.repository.is_some() || pkg.docs_url.is_some() {
                ui.add_space(16.0);
                theme::hairline(ui);
                ui.add_space(10.0);
                if let Some(repo) = &pkg.repository {
                    render_link_row(ui, &format!("repo_{}", pkg.qualified_id()), "Repo", repo);
                }
                if let Some(docs) = &pkg.docs_url {
                    render_link_row(ui, &format!("docs_{}", pkg.qualified_id()), "Docs", docs);
                }
            }
        });
}

fn render_link_row(ui: &mut egui::Ui, id_salt: &str, label: &str, url: &str) {
    ui.horizontal(|ui| {
        theme::field_label(ui, 64.0, 24.0, label);
        let avail_w = ui.available_width().max(80.0);
        ui.allocate_ui_with_layout(
            egui::vec2(avail_w, 24.0),
            Layout::right_to_left(egui::Align::Center),
            |ui| {
                theme::copy_feedback_button(ui, id_salt, "", url);
                let url_w = ui.available_width().max(40.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(url_w, 24.0),
                    Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(url)
                                    .size(11.5)
                                    .monospace()
                                    .color(theme::FG_SECONDARY),
                            )
                            .truncate(),
                        );
                    },
                );
            },
        );
    });
}

fn show_installed(app: &mut RavomApp, ui: &mut egui::Ui) {
    let has_project = app.loaded_config.is_some();
    let idle = app.tasks.running().is_none();
    let env_display = if app.selected_env.trim().is_empty() {
        "default".to_string()
    } else {
        app.selected_env.trim().to_string()
    };

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("[env.{env_display}]"))
                .font(egui::FontId::monospace(11.5))
                .color(theme::FG_SECONDARY),
        );
        if icons::button(
            ui,
            Icon::Refresh,
            "Sync",
            ButtonKind::Primary,
            has_project && idle,
        )
        .clicked()
        {
            app.spawn_pkg_sync();
        }
        if icons::button(ui, Icon::Search, "Registry", ButtonKind::Outline, true).clicked() {
            app.lib_subtab = LibrarySubTab::Registry;
        }
    });

    ui.add_space(10.0);

    let can_add = has_project && idle && !app.custom_dep_spec.trim().is_empty();
    let mut do_add = false;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 30.0),
        Layout::right_to_left(egui::Align::Center),
        |ui| {
            if icons::button(ui, Icon::Plus, "Add", ButtonKind::Outline, can_add).clicked() {
                do_add = true;
            }
            let input_w = ui.available_width().max(120.0);
            let spec_id = ui.make_persistent_id("lib_custom_spec_input");
            let resp = theme::text_input(
                ui,
                spec_id,
                &mut app.custom_dep_spec,
                "Spec (e.g. mathertel/OneButton@^2.0.3 or crates:debouncr@^0.2.2)",
                input_w,
                false,
                false,
            );
            if can_add && resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_add = true;
            }
        },
    );
    if do_add {
        let spec = std::mem::take(&mut app.custom_dep_spec);
        app.spawn_pkg_install_spec(spec);
    }

    ui.add_space(12.0);
    theme::hairline(ui);
    ui.add_space(6.0);

    let list_h = (ui.available_height() - 4.0).max(160.0);
    let installed = app.installed_packages.clone();

    egui::ScrollArea::vertical()
        .id_salt("lib_installed_scroll")
        .max_height(list_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if installed.is_empty() {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("No libraries installed yet")
                            .size(12.0)
                            .color(theme::FG_MUTED),
                    );
                    if icons::button(ui, Icon::Search, "Browse Registry", ButtonKind::Ghost, true)
                        .clicked()
                    {
                        app.lib_subtab = LibrarySubTab::Registry;
                    }
                });
                return;
            }

            for pkg in &installed {
                let row = egui::Frame::new()
                    .fill(theme::BG_CANVAS)
                    .inner_margin(Margin::symmetric(4, 10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&pkg.name)
                                    .size(13.5)
                                    .strong()
                                    .color(theme::FG_PRIMARY),
                            );
                            let mut info =
                                format!("{} · {}", pkg.source.short_tag(), pkg.origin.label());
                            if let Some(ver) = &pkg.version {
                                info.push_str(&format!(" · {ver}"));
                            }
                            ui.label(
                                RichText::new(info)
                                    .font(egui::FontId::monospace(10.5))
                                    .color(theme::FG_MUTED),
                            );
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if icons::button(
                                    ui,
                                    Icon::Trash,
                                    "",
                                    ButtonKind::Ghost,
                                    has_project && idle,
                                )
                                .on_hover_text("Remove package")
                                .clicked()
                                {
                                    app.spawn_pkg_remove(pkg.spec.clone());
                                }
                            });
                        });
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(&pkg.spec)
                                .size(11.5)
                                .monospace()
                                .color(theme::FG_SECONDARY),
                        );
                        if let Some(local) = &pkg.local_dir {
                            ui.label(
                                RichText::new(local.display().to_string())
                                    .size(10.5)
                                    .monospace()
                                    .color(theme::FG_MUTED),
                            );
                        }
                    });
                let r = row.response.rect;
                ui.painter().line_segment(
                    [
                        egui::pos2(r.left(), r.bottom()),
                        egui::pos2(r.right(), r.bottom()),
                    ],
                    Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::format_popularity;

    #[test]
    fn format_popularity_scales_units_cleanly() {
        assert_eq!(format_popularity(0), "0");
        assert_eq!(format_popularity(720), "720");
        assert_eq!(format_popularity(1_250), "1.2k");
        assert_eq!(format_popularity(1_500_000), "1.5M");
    }
}
