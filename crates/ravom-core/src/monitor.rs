use crate::config::{find_board, RavomConfig};
use crate::flash::resolve_firmware_path;
use crate::{CoreError, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Discover likely USB/ACM serial ports on Unix-like systems without native dependencies.
pub fn list_serial_ports() -> Vec<String> {
    let mut ports = Vec::new();
    if let Ok(entries) = fs::read_dir("/dev") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("ttyUSB")
                    || name.starts_with("ttyACM")
                    || name.starts_with("cu.usb")
                {
                    ports.push(entry.path().display().to_string());
                }
            }
        }
    }
    ports.sort();
    ports
}

/// Attach to an RTT session using `probe-rs attach --chip <chip> <elf>`.
pub fn monitor_rtt(dir: &Path, env_name: Option<&str>, release: bool) -> Result<()> {
    let cfg = RavomConfig::load_from_dir(dir)?;
    let (_resolved_env, env_cfg) = cfg.resolve_env(env_name)?;
    let board = find_board(&env_cfg.board)
        .ok_or_else(|| CoreError::UnknownBoard(env_cfg.board.clone()))?;

    let chip = board
        .probe_rs_chip
        .ok_or_else(|| CoreError::MissingProbeRsChip(board.id.to_string()))?;

    let project_name = cfg
        .project
        .name
        .as_deref()
        .unwrap_or("ravomio-app")
        .replace(' ', "-");
    let firmware = resolve_firmware_path(dir, &project_name, board.rust_target, release);

    let status = Command::new("probe-rs")
        .args(["attach", "--chip", chip])
        .arg(&firmware)
        .current_dir(dir)
        .status()
        .map_err(|_| CoreError::MonitorFailed(None))?;

    if !status.success() {
        return Err(CoreError::MonitorFailed(status.code()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_serial_ports_returns_sorted_paths() {
        let ports = list_serial_ports();
        let mut sorted = ports.clone();
        sorted.sort();
        assert_eq!(ports, sorted);
    }
}
