use crate::config::RavomConfig;
use crate::flash::resolve_firmware_path;
use crate::registry::find_board_record;
use crate::{CoreError, Result};
use std::path::Path;
use std::process::Command;

/// Deploy a built Linux SBC binary to a remote device via `scp` (and optionally run via `ssh`).
pub fn deploy_ssh(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    host: &str,
    remote_dir: &str,
    run_after: bool,
) -> Result<String> {
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
    let binary = resolve_firmware_path(dir, &project_name, board.rust_target.as_deref(), release);
    let remote_path = format!("{}/{project_name}", remote_dir.trim_end_matches('/'));
    let scp_dest = format!("{host}:{remote_path}");

    let status = Command::new("scp")
        .arg(&binary)
        .arg(&scp_dest)
        .current_dir(dir)
        .status()
        .map_err(|_| CoreError::DeployFailed(None))?;

    if !status.success() {
        return Err(CoreError::DeployFailed(status.code()));
    }

    if run_after {
        let ssh_status = Command::new("ssh")
            .args([host, &format!("chmod +x {remote_path} && {remote_path}")])
            .status()
            .map_err(|_| CoreError::DeployFailed(None))?;
        if !ssh_status.success() {
            return Err(CoreError::DeployFailed(ssh_status.code()));
        }
    }

    Ok(scp_dest)
}
