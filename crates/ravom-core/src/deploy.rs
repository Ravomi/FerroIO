use crate::context::EnvContext;
use crate::process::CommandSpec;
use crate::{CoreError, Result};
use std::path::Path;

/// Commands that upload (and optionally run) a Linux SBC binary over SSH.
pub fn plan_deploy(
    ctx: &EnvContext,
    release: bool,
    host: &str,
    remote_dir: &str,
    run_after: bool,
) -> (String, Vec<CommandSpec>) {
    let binary = ctx.firmware_path(release);
    let remote_path = format!(
        "{}/{}",
        remote_dir.trim_end_matches('/'),
        ctx.project_name()
    );
    let scp_dest = format!("{host}:{remote_path}");
    let mut steps = vec![CommandSpec::new("scp")
        .path_arg(&binary)
        .arg(scp_dest.as_str())
        .cwd(&ctx.root)];
    if run_after {
        steps.push(
            CommandSpec::new("ssh")
                .arg(host)
                .arg(format!("chmod +x '{remote_path}' && '{remote_path}'")),
        );
    }
    (scp_dest, steps)
}

/// Deploy a built Linux SBC binary to a remote device via `scp` (and optionally run via `ssh`).
pub fn deploy_ssh(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    host: &str,
    remote_dir: &str,
    run_after: bool,
) -> Result<String> {
    let ctx = EnvContext::load(dir, env_name)?;
    let binary = ctx.firmware_path(release);
    if !binary.exists() {
        return Err(CoreError::ArtifactNotFound(binary));
    }
    let (dest, steps) = plan_deploy(&ctx, release, host, remote_dir, run_after);
    for step in steps {
        step.run(CoreError::DeployFailed)?;
    }
    Ok(dest)
}
