use crate::config::RavomConfig;
use crate::registry::find_board_record;
use crate::{CoreError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of a firmware flash invocation via `probe-rs`.
#[derive(Debug, Clone)]
pub struct FlashResult {
    pub env_name: String,
    pub chip: String,
    pub firmware: PathBuf,
    pub monitor: bool,
}

/// Resolve the expected ELF path for the given project, target, and profile.
pub fn resolve_firmware_path(
    dir: &Path,
    project_name: &str,
    target: Option<&str>,
    release: bool,
) -> PathBuf {
    let profile = if release { "release" } else { "debug" };
    let mut p = dir.join("target");
    if let Some(t) = target {
        p = p.join(t);
    }
    p.join(profile).join(project_name)
}

/// Flash firmware to the target board using `probe-rs` CLI (`run` if `monitor` is true, else `download`).
pub fn flash_project(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    monitor: bool,
) -> Result<FlashResult> {
    let cfg = RavomConfig::load_from_dir(dir)?;
    let (resolved_env, env_cfg) = cfg.resolve_env(env_name)?;
    let board = find_board_record(&env_cfg.board)
        .ok_or_else(|| CoreError::UnknownBoard(env_cfg.board.clone()))?;

    let is_fpga = board.platform.starts_with("lattice_")
        || board.platform == "gowin_fpga"
        || board.platform.starts_with("xilinx_");

    let project_name = cfg
        .project
        .name
        .as_deref()
        .unwrap_or("ravomio-app")
        .replace(' ', "-");
    let firmware = resolve_firmware_path(dir, &project_name, board.rust_target.as_deref(), release);

    if is_fpga {
        let status = Command::new("openFPGALoader")
            .args(["-b", &board.id])
            .arg(&firmware)
            .current_dir(dir)
            .status()
            .map_err(|_| CoreError::FlashFailed(None))?;

        if !status.success() {
            return Err(CoreError::FlashFailed(status.code()));
        }

        return Ok(FlashResult {
            env_name: resolved_env.to_string(),
            chip: board.mcu,
            firmware,
            monitor: false,
        });
    }

    let chip = board
        .probe_rs_chip
        .clone()
        .ok_or_else(|| CoreError::MissingProbeRsChip(board.id.clone()))?;

    let subcmd = if monitor { "run" } else { "download" };
    let status = Command::new("probe-rs")
        .args([subcmd, "--chip", &chip])
        .arg(&firmware)
        .current_dir(dir)
        .status()
        .map_err(|_| CoreError::FlashFailed(None))?;

    if !status.success() {
        return Err(CoreError::FlashFailed(status.code()));
    }

    Ok(FlashResult {
        env_name: resolved_env.to_string(),
        chip,
        firmware,
        monitor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_debug_and_release_firmware_paths() {
        let root = Path::new("/tmp/demo");
        assert_eq!(
            resolve_firmware_path(root, "blink", Some("thumbv7em-none-eabihf"), false),
            PathBuf::from("/tmp/demo/target/thumbv7em-none-eabihf/debug/blink")
        );
        assert_eq!(
            resolve_firmware_path(root, "blink", Some("thumbv6m-none-eabi"), true),
            PathBuf::from("/tmp/demo/target/thumbv6m-none-eabi/release/blink")
        );
    }
}
