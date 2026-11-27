use crate::cli::PkgAction;
use crate::commands::CmdResult;
use ravom_core::pkg::{
    install_spec, list_installed, remove_package, search_packages, sync_lib_deps, PkgCategory,
    SearchQuery, SearchSourceFilter,
};
use std::path::Path;

pub fn run(root: &Path, action: PkgAction) -> CmdResult {
    match action {
        PkgAction::Search {
            query,
            source,
            limit,
            offline,
        } => {
            let src_filter = match source.to_ascii_lowercase().as_str() {
                "crates" | "crates.io" | "cargo" => SearchSourceFilter::CratesIo,
                "pio" | "platformio" => SearchSourceFilter::PlatformIo,
                "all" => SearchSourceFilter::All,
                other => {
                    return Err(format!(
                        "unknown registry source '{other}' (expected: all, crates, pio)"
                    )
                    .into());
                }
            };
            let outcome = search_packages(&SearchQuery {
                text: query,
                source: src_filter,
                category: PkgCategory::All,
                limit,
                online: !offline,
            });
            for warn in &outcome.warnings {
                eprintln!("warning: {warn}");
            }
            if outcome.packages.is_empty() {
                println!("No packages matched.");
                return Ok(());
            }
            println!(
                "{:<10}  {:<34}  {:<12}  {:<8}  DESCRIPTION",
                "SOURCE", "PACKAGE", "VERSION", "POP"
            );
            for pkg in outcome.packages {
                let id = pkg.qualified_id();
                let desc = truncate_single_line(&pkg.description, 56);
                println!(
                    "{:<10}  {:<34}  {:<12}  {:<8}  {desc}",
                    pkg.source.short_tag(),
                    truncate_single_line(&id, 34),
                    truncate_single_line(&pkg.version, 12),
                    format_pop(pkg.popularity),
                );
            }
            Ok(())
        }
        PkgAction::Install { spec, env } => {
            let installed = install_spec(root, env.as_deref(), &spec, &mut |line| {
                println!("{line}");
            })?;
            println!("Installed {} ({})", installed.name, installed.spec);
            Ok(())
        }
        PkgAction::Remove { name, env } => {
            let removed = remove_package(root, env.as_deref(), &name, &mut |line| {
                println!("{line}");
            })?;
            if removed {
                println!("Removed {name}");
            } else {
                println!("No matching package '{name}' found.");
            }
            Ok(())
        }
        PkgAction::List { env } => {
            let installed = list_installed(root, env.as_deref())?;
            if installed.is_empty() {
                println!("No packages installed.");
                return Ok(());
            }
            println!("{:<10}  {:<12}  {:<28}  SPEC", "SOURCE", "ORIGIN", "NAME");
            for pkg in installed {
                println!(
                    "{:<10}  {:<12}  {:<28}  {}",
                    pkg.source.short_tag(),
                    pkg.origin.label(),
                    truncate_single_line(&pkg.name, 28),
                    pkg.spec,
                );
            }
            Ok(())
        }
        PkgAction::Sync { env } => {
            let count = sync_lib_deps(root, env.as_deref(), &mut |line| {
                println!("{line}");
            })?;
            println!("Synced {count} package(s).");
            Ok(())
        }
    }
}

fn truncate_single_line(s: &str, max_chars: usize) -> String {
    let single: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = single.chars().count();
    if count <= max_chars {
        single
    } else if max_chars > 1 {
        let mut out: String = single.chars().take(max_chars - 1).collect();
        out.push('…');
        out
    } else {
        single.chars().take(max_chars).collect()
    }
}

fn format_pop(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
