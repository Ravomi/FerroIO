use crate::config::RavomConfig;
use crate::flash::resolve_firmware_path;
use crate::registry::find_board_record;
use crate::{CoreError, Result};
use std::path::Path;
use std::process::Command;

/// Supported hardware simulation backends for `mio sim`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimBackend {
    Renode,
    Qemu,
    Wokwi,
}

impl SimBackend {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "renode" => Ok(Self::Renode),
            "qemu" => Ok(Self::Qemu),
            "wokwi" => Ok(Self::Wokwi),
            other => Err(CoreError::UnsupportedSimBackend(other.to_string())),
        }
    }
}

/// Run the project's built firmware inside a hardware simulator (`renode`, `qemu`, or `wokwi`).
pub fn simulate_project(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    backend: SimBackend,
) -> Result<()> {
    let cfg = RavomConfig::load_from_dir(dir)?;
    let (_resolved_env, env_cfg) = cfg.resolve_env(env_name)?;
    let board = find_board_record(&env_cfg.board)
        .ok_or_else(|| CoreError::UnknownBoard(env_cfg.board.clone()))?;

    let project_name = cfg
        .project
        .name
        .as_deref()
        .unwrap_or("ravomio-app")
        .replace(' ', "-");
    let firmware = resolve_firmware_path(dir, &project_name, board.rust_target.as_deref(), release);

    let mut cmd = match backend {
        SimBackend::Renode => {
            let mut c = Command::new("renode");
            c.args(["--console", "--disable-xwt", "-e"])
                .arg(format!("sysbus LoadELF @{}", firmware.display()));
            c
        }
        SimBackend::Qemu => {
            let mut c = Command::new("qemu-system-arm");
            c.args(["-machine", "lm3s6965evb", "-nographic", "-kernel"])
                .arg(&firmware);
            c
        }
        SimBackend::Wokwi => {
            let mut c = Command::new("wokwi-cli");
            c.arg(dir);
            c
        }
    };

    let status = cmd
        .current_dir(dir)
        .status()
        .map_err(|_| CoreError::SimFailed(None))?;

    if !status.success() {
        return Err(CoreError::SimFailed(status.code()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SimBackend;

    #[test]
    fn parses_supported_sim_backends() {
        assert_eq!(SimBackend::parse("renode").ok(), Some(SimBackend::Renode));
        assert_eq!(SimBackend::parse("QEMU").ok(), Some(SimBackend::Qemu));
        assert_eq!(SimBackend::parse("wokwi").ok(), Some(SimBackend::Wokwi));
        assert!(SimBackend::parse("unknown").is_err());
    }
}
