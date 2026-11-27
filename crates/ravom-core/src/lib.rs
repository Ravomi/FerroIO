//! RavomIO 核心引擎：工程解析、板卡注册表、构建、probe-rs 烧录、串口。

pub mod build;
pub mod chips;
pub mod config;
pub mod context;
pub mod deploy;
pub mod flash;
pub mod migrate;
pub mod monitor;
pub mod pkg;
pub mod process;
pub mod project;
pub mod registry;
pub mod security;
pub mod sim;
pub mod toolchain;
pub mod vofa;
pub mod zephyr;

use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unknown board '{0}' (run `mio boards` to list supported boards)")]
    UnknownBoard(String),

    #[error("board id '{id}' exists on several platforms; use one of: {}", candidates.join(", "))]
    AmbiguousBoard { id: String, candidates: Vec<String> },

    #[error("project already initialized at {0}")]
    AlreadyInitialized(PathBuf),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to serialize TOML: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("failed to parse TOML: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("failed to parse PlatformIO board JSON: {0}")]
    JsonDe(#[from] serde_json::Error),

    #[error("ravom.toml not found in {0}")]
    ManifestNotFound(PathBuf),

    #[error("platformio.ini not found in {0}")]
    PlatformIoIniNotFound(PathBuf),

    #[error("environment '{0}' not found in ravom.toml")]
    UnknownEnv(String),

    #[error("no environments defined in ravom.toml")]
    NoEnvironments,

    #[error("`{0}` was not found in PATH (run `mio doctor` for install hints)")]
    ToolNotFound(String),

    #[error("build artifact not found: {0} (run `mio build` first)")]
    ArtifactNotFound(PathBuf),

    #[error("build failed with exit code {0:?}")]
    BuildFailed(Option<i32>),

    #[error("board '{0}' (MCU {1}) has no probe-rs target; flash it with the vendor tool or set upload_protocol")]
    MissingProbeRsChip(String, String),

    #[error("upload_protocol '{0}' is not supported yet (supported: probe-rs, stlink, jlink, cmsis-dap, west)")]
    UnsupportedUploadProtocol(String),

    #[error("flash failed with exit code {0:?}")]
    FlashFailed(Option<i32>),

    #[error("monitor failed with exit code {0:?}")]
    MonitorFailed(Option<i32>),

    #[error("serial port {0}: {1}")]
    SerialPort(String, String),

    #[error("unsupported simulator backend '{0}' (expected: renode, qemu, wokwi)")]
    UnsupportedSimBackend(String),

    #[error("board '{0}' (MCU {1}) has no model in this simulator backend")]
    UnsupportedSimTarget(String, String),

    #[error("simulation failed with exit code {0:?}")]
    SimFailed(Option<i32>),

    #[error("SSH deploy failed with exit code {0:?}")]
    DeployFailed(Option<i32>),

    #[error("package error: {0}")]
    Package(String),
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
