use super::CmdResult;
use ravom_core::process::find_in_path;
use ravom_core::toolchain::{check_host_setup, check_toolchains, install_rust_target};
use std::process::Command;

pub fn run(install_target: Option<&str>) -> CmdResult {
    if let Some(target) = install_target {
        install_rust_target(target)?;
        println!("Installed Rust target '{target}' via rustup.");
        return Ok(());
    }
    println!("{:<20} {:<8} VERSION / HINT", "TOOL", "STATUS");
    for tool in check_toolchains() {
        let (state, detail) = if tool.installed {
            ("OK", tool.version.unwrap_or_else(|| "-".to_string()))
        } else {
            ("MISSING", tool.install_hint.to_string())
        };
        println!("{:<20} {:<8} {}", tool.name, state, detail);
    }
    for warning in check_host_setup() {
        println!("warning: {warning}");
    }
    let zenv = ravom_core::zephyr::detect_zephyr_env(None);
    println!(
        "zephyr: ZEPHYR_BASE={}, SDK={}",
        zenv.zephyr_base
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not set (run `mio zephyr setup`)".to_string()),
        zenv.sdk_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not set".to_string())
    );
    Ok(())
}

/// Launch the `Ravomi` GUI binary installed next to `mio` or on `PATH`.
pub fn launch_ui() -> CmdResult {
    let sibling_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
    let ui = ["Ravomi", "ravom-ui"]
        .iter()
        .find_map(|name| {
            let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            sibling_dir
                .as_ref()
                .map(|dir| dir.join(&exe))
                .filter(|p| p.is_file())
                .or_else(|| find_in_path(name))
        })
        .ok_or_else(|| {
            "`Ravomi` GUI binary not found; install it with `cargo install --path crates/ravom-ui`"
                .to_string()
        })?;
    let status = Command::new(ui).status()?;
    if !status.success() {
        return Err(format!("Ravomi exited with {status}").into());
    }
    Ok(())
}
