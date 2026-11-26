use super::CmdResult;
use ravom_core::registry::{
    default_boards_dir, import_pio_boards_dir, normalize_probe_rs_chips, registry, BoardRecord,
};
use std::path::PathBuf;

fn matches_query(b: &BoardRecord, q: &str) -> bool {
    q.is_empty()
        || [&b.id, &b.name, &b.mcu, &b.platform]
            .iter()
            .any(|f| f.to_lowercase().contains(q))
}

pub fn run(
    query: Option<String>,
    import: Option<PathBuf>,
    platform: &str,
    into: Option<PathBuf>,
    check: bool,
    normalize_chips: Option<PathBuf>,
) -> CmdResult {
    if let Some(src_dir) = import {
        let dest = into.unwrap_or_else(default_boards_dir);
        let count = import_pio_boards_dir(&src_dir, platform, &dest)?;
        println!(
            "Imported {count} PlatformIO board(s) for platform '{platform}' into {}",
            dest.display()
        );
        return Ok(());
    }

    if let Some(dir) = normalize_chips {
        let changes = normalize_probe_rs_chips(&dir)?;
        for change in &changes {
            println!(
                "{}: {} -> {}",
                change.board,
                change.old.as_deref().unwrap_or("-"),
                change.new.as_deref().unwrap_or("- (no probe-rs target)")
            );
        }
        println!("{} board file(s) updated", changes.len());
        return Ok(());
    }

    let reg = registry();
    if check {
        for issue in reg.issues() {
            eprintln!("{issue}");
        }
        let probe_rs = reg.boards().filter(|b| b.probe_rs_chip.is_some()).count();
        println!(
            "{} boards loaded, {probe_rs} flashable via probe-rs, {} issue(s)",
            reg.len(),
            reg.issues().len()
        );
        if !reg.issues().is_empty() {
            return Err(format!("{} board registry issue(s)", reg.issues().len()).into());
        }
        return Ok(());
    }

    let q = query.unwrap_or_default().to_lowercase();
    println!("{:<32} {:<26} {:<16} NAME", "ID", "PLATFORM", "MCU");
    for b in reg.boards().filter(|b| matches_query(b, &q)) {
        println!("{:<32} {:<26} {:<16} {}", b.id, b.platform, b.mcu, b.name);
    }
    Ok(())
}
