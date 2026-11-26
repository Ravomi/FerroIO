use super::CmdResult;
use ravom_core::migrate::migrate_platformio_project;
use ravom_core::project::init_project;
use std::path::Path;

pub fn init(dir: &Path, board: &str, framework: Option<&str>, lang: Option<&str>) -> CmdResult {
    let effective_fw = match (framework, lang) {
        (Some(fw), Some(l))
            if fw.eq_ignore_ascii_case("zephyr") && l.eq_ignore_ascii_case("rust") =>
        {
            Some("zephyr-rust")
        }
        (Some(fw), Some(l))
            if fw.eq_ignore_ascii_case("zephyr") && !l.eq_ignore_ascii_case("c") =>
        {
            return Err(format!("unsupported Zephyr language '{l}' (expected: c, rust)").into());
        }
        (fw, _) => fw,
    };
    let res = init_project(dir, board, effective_fw)?;
    println!(
        "Initialized RavomIO project at {} (board: {}, framework: {})",
        res.root.display(),
        res.board_id,
        res.framework
    );
    Ok(())
}

pub fn migrate(dir: &Path) -> CmdResult {
    let (out, warnings) = migrate_platformio_project(dir)?;
    println!("Migrated platformio.ini -> {}", out.display());
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    Ok(())
}
