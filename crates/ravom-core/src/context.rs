//! Resolved project environment shared by build, flash, monitor, sim and deploy.

use crate::config::{EnvConfig, RavomConfig};
use crate::registry::{registry, BoardRecord};
use crate::Result;
use std::path::{Path, PathBuf};

/// A `ravom.toml` environment resolved against the board registry.
#[derive(Debug, Clone)]
pub struct EnvContext {
    pub root: PathBuf,
    pub config: RavomConfig,
    pub env_name: String,
    pub env: EnvConfig,
    pub board: BoardRecord,
}

impl EnvContext {
    /// Load `ravom.toml` from `dir` and resolve `env_name` (or the default env).
    pub fn load(dir: &Path, env_name: Option<&str>) -> Result<Self> {
        let config = RavomConfig::load_from_dir(dir)?;
        Self::from_config(dir, config, env_name)
    }

    /// Resolve an already parsed configuration.
    pub fn from_config(dir: &Path, config: RavomConfig, env_name: Option<&str>) -> Result<Self> {
        let (name, env) = config.resolve_env(env_name)?;
        let (name, env) = (name.to_string(), env.clone());
        let board = registry().resolve(&env.board, Some(&env.platform))?.clone();
        Ok(Self {
            root: dir.to_path_buf(),
            config,
            env_name: name,
            env,
            board,
        })
    }

    /// Cargo package / binary name derived from `[project].name`.
    pub fn project_name(&self) -> String {
        self.config
            .project
            .name
            .as_deref()
            .unwrap_or("ravomio-app")
            .replace(' ', "-")
    }

    /// Expected firmware ELF for the active backend (Cargo or Zephyr).
    pub fn firmware_path(&self, release: bool) -> PathBuf {
        if crate::zephyr::is_zephyr_framework(&self.env.framework) {
            return crate::zephyr::zephyr_elf_path(&self.root, &self.env_name);
        }
        resolve_firmware_path(
            &self.root,
            &self.project_name(),
            self.board.rust_target.as_deref(),
            release,
        )
    }

    /// Expected FPGA bitstream produced by the synthesis pipeline.
    pub fn bitstream_path(&self) -> PathBuf {
        let ext = match self.board.platform.as_str() {
            "lattice_ice40" => "bin",
            "gowin_fpga" => "fs",
            _ => "bit",
        };
        self.root.join("build").join(format!("top.{ext}"))
    }
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
