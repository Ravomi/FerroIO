use crate::{CoreError, Result};
use std::process::Command;

/// Availability status of an external embedded or FPGA toolchain binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: &'static str,
    pub installed: bool,
    pub install_hint: &'static str,
}

fn is_in_path(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Inspect the local environment for Rust, MCU, FPGA, and simulation toolchains.
pub fn check_toolchains() -> Vec<ToolStatus> {
    let tools = [
        ("cargo", "https://rustup.rs"),
        ("rustup", "https://rustup.rs"),
        ("probe-rs", "cargo install probe-rs-tools --locked"),
        (
            "openFPGALoader",
            "Install via system package manager or oss-cad-suite",
        ),
        (
            "yosys",
            "Install via system package manager or oss-cad-suite",
        ),
        ("arm-none-eabi-gcc", "Install arm-none-eabi-gcc toolchain"),
        ("renode", "https://renode.io"),
        ("qemu-system-arm", "Install qemu-system-arm"),
    ];

    tools
        .into_iter()
        .map(|(name, install_hint)| ToolStatus {
            name,
            installed: is_in_path(name),
            install_hint,
        })
        .collect()
}

/// Install a Rust cross-compilation target via `rustup target add <triple>`.
pub fn install_rust_target(target: &str) -> Result<()> {
    let status = Command::new("rustup")
        .args(["target", "add", target])
        .status()
        .map_err(|_| CoreError::BuildFailed(None))?;
    if !status.success() {
        return Err(CoreError::BuildFailed(status.code()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_toolchains;

    #[test]
    fn detects_cargo_in_local_environment() {
        let statuses = check_toolchains();
        assert!(statuses.iter().any(|t| t.name == "cargo" && t.installed));
    }
}
