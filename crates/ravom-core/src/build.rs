use crate::config::{find_board, RavomConfig};
use crate::{CoreError, Result};
use std::path::Path;
use std::process::Command;

/// Result of a firmware build invocation.
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub env_name: String,
    pub board_id: String,
    pub target: Option<String>,
    pub release: bool,
}

/// Build the RavomIO project in `dir` for the selected (or default) environment.
pub fn build_project(dir: &Path, env_name: Option<&str>, release: bool) -> Result<BuildResult> {
    let cfg = RavomConfig::load_from_dir(dir)?;
    let (resolved_env, env_cfg) = cfg.resolve_env(env_name)?;
    let board =
        find_board(&env_cfg.board).ok_or_else(|| CoreError::UnknownBoard(env_cfg.board.clone()))?;

    let mut cmd = Command::new("cargo");
    cmd.arg("build").current_dir(dir);

    if let Some(target) = board.rust_target {
        cmd.args(["--target", target]);
    }
    if release {
        cmd.arg("--release");
    }
    if !env_cfg.build_flags.is_empty() {
        cmd.env("CARGO_ENCODED_RUSTFLAGS", env_cfg.build_flags.join("\x1f"));
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err(CoreError::BuildFailed(status.code()));
    }

    Ok(BuildResult {
        env_name: resolved_env.to_string(),
        board_id: board.id.to_string(),
        target: board.rust_target.map(str::to_string),
        release,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::init_project;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn build_fails_gracefully_without_cargo_toml() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("ravomio_build_test_{nanos}"));
        init_project(&tmp, "rp2040", None).expect("init_project should succeed in temp dir");
        let _ = fs::remove_file(tmp.join("Cargo.toml"));

        let err = build_project(&tmp, None, false)
            .expect_err("build should fail when Cargo.toml is missing");
        assert!(matches!(err, CoreError::BuildFailed(_)));

        let _ = fs::remove_dir_all(&tmp);
    }
}
