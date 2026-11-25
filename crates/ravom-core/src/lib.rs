//! RavomIO 核心引擎：工程解析、工具链管理、构建、probe-rs 烧录、串口。

pub mod build;
pub mod config;
pub mod flash;
pub mod monitor;
pub mod project;
pub mod registry;

use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unknown board '{0}' (run `mio boards` to list supported boards)")]
    UnknownBoard(String),

    #[error("project already initialized at {0}")]
    AlreadyInitialized(PathBuf),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to serialize ravom.toml: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("failed to parse ravom.toml: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("failed to parse PlatformIO board JSON: {0}")]
    JsonDe(#[from] serde_json::Error),

    #[error("ravom.toml not found in {0}")]
    ManifestNotFound(PathBuf),

    #[error("environment '{0}' not found in ravom.toml")]
    UnknownEnv(String),

    #[error("no environments defined in ravom.toml")]
    NoEnvironments,

    #[error("build failed with exit code {0:?}")]
    BuildFailed(Option<i32>),

    #[error("board '{0}' does not define a probe-rs chip name")]
    MissingProbeRsChip(String),

    #[error("flash failed with exit code {0:?} (is `probe-rs` installed and in PATH?)")]
    FlashFailed(Option<i32>),

    #[error("monitor failed with exit code {0:?} (is `probe-rs` installed and in PATH?)")]
    MonitorFailed(Option<i32>),
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
