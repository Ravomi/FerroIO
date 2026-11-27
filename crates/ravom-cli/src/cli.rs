//! Command-line definitions for `mio`.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mio", version, about = "RavomIO embedded development CLI")]
pub struct Cli {
    /// Project directory containing `ravom.toml` (defaults to the current directory)
    #[arg(short = 'C', long, global = true, default_value = ".")]
    pub project_dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new RavomIO project (PlatformIO-style layout)
    Init {
        /// Target board ID or `<platform>/<id>` (e.g. stm32f401ccu6, raspberrypi/pico)
        #[arg(short, long)]
        board: String,
        /// Target framework (defaults to board's default; e.g. embassy, zephyr, zephyr-rust)
        #[arg(short, long)]
        framework: Option<String>,
        /// Application language for frameworks that support multiple languages (e.g. zephyr: c | rust)
        #[arg(long)]
        lang: Option<String>,
        /// Project directory (defaults to --project-dir)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Build (and optionally upload) firmware — PlatformIO `pio run` style
    Run {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Target action (e.g. `upload` to build and flash)
        #[arg(short, long)]
        target: Option<String>,
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
    },
    /// Build firmware for the current project
    Build {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
        /// Print the external commands instead of running them
        #[arg(long)]
        dry_run: bool,
    },
    /// Flash firmware via probe-rs or west
    Flash {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Flash release build artifact
        #[arg(short, long)]
        release: bool,
        /// Open RTT monitor after flashing (`probe-rs run`)
        #[arg(short, long)]
        monitor: bool,
        /// Print the flash command instead of running it
        #[arg(long)]
        dry_run: bool,
    },
    /// Open serial/RTT monitor
    Monitor {
        /// Serial port path
        #[arg(short, long)]
        port: Option<String>,
        /// Baud rate (defaults to the env's monitor_speed, then 115200)
        #[arg(short, long)]
        baud: Option<u32>,
        /// Attach via probe-rs RTT instead of serial
        #[arg(long)]
        rtt: bool,
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Use release ELF (used with --rtt)
        #[arg(short, long)]
        release: bool,
        /// List available serial ports and exit
        #[arg(short, long)]
        list: bool,
    },
    /// Run a stdio Model Context Protocol (MCP) server for AI agents
    Mcp {
        /// Security permission tier (`read-only`, `build`, or `hardware`)
        #[arg(long, default_value = "build")]
        mode: String,
        /// Shorthand for `--mode hardware` (enables flashing, serial sampling, and PID tuning)
        #[arg(long)]
        allow_hardware: bool,
    },
    /// List, validate or import board definitions
    Boards {
        /// Optional filter query
        query: Option<String>,
        /// Import PlatformIO `boards/` directory into the local RavomIO registry
        #[arg(long)]
        import: Option<PathBuf>,
        /// Platform name for imported boards (e.g. ststm32, espressif32, nordicnrf52)
        #[arg(long, default_value = "custom")]
        platform: String,
        /// Registry directory to import into (defaults to ~/.ravom/boards)
        #[arg(long, value_name = "DIR")]
        into: Option<PathBuf>,
        /// Validate every loaded board definition and exit non-zero on problems
        #[arg(long)]
        check: bool,
        /// Re-resolve `probe_rs_chip` in `<DIR>/<platform>/*.toml` against probe-rs
        #[arg(long, value_name = "DIR")]
        normalize_chips: Option<PathBuf>,
    },
    /// Migrate an existing `platformio.ini` project to `ravom.toml`
    Migrate {
        /// Project directory containing `platformio.ini` (defaults to --project-dir)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Run firmware in a hardware simulator (renode / qemu / wokwi)
    Sim {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Simulator backend (renode, qemu, wokwi)
        #[arg(short, long, default_value = "renode")]
        backend: String,
        /// Use release build artifact
        #[arg(short, long)]
        release: bool,
    },
    /// Deploy a built Linux SBC binary over SSH/SCP
    Deploy {
        /// Target SSH host (e.g. root@192.168.42.1)
        #[arg(long)]
        host: String,
        /// Remote destination directory
        #[arg(long, default_value = "/tmp")]
        remote_dir: String,
        /// Execute the binary after uploading
        #[arg(long)]
        run: bool,
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Deploy release build artifact
        #[arg(short, long)]
        release: bool,
    },
    /// Check installed embedded toolchains and optionally install a Rust target
    Doctor {
        /// Install a Rust target triple via `rustup target add <TRIPLE>`
        #[arg(long)]
        install_target: Option<String>,
    },
    /// Manage Zephyr RTOS workspace, SDK, and interactive Kconfig
    Zephyr {
        #[command(subcommand)]
        action: ZephyrAction,
    },
    /// Search, install, remove, list, and sync embedded packages (crates.io + PlatformIO)
    Pkg {
        #[command(subcommand)]
        action: PkgAction,
    },
    /// Launch the RavomIO egui desktop workbench
    Ui,
}

#[derive(Subcommand)]
pub enum PkgAction {
    /// Search curated and live community registries (crates.io + PlatformIO)
    Search {
        /// Search query (e.g. button, st7789, ssd1306, mpu6050)
        query: String,
        /// Registry source filter (`all`, `crates`, `pio`)
        #[arg(long, default_value = "all")]
        source: String,
        /// Maximum remote results per registry
        #[arg(long, default_value_t = 15)]
        limit: usize,
        /// Search offline curated starter catalog only
        #[arg(long)]
        offline: bool,
    },
    /// Add a library dependency to ravom.toml and install it locally
    Install {
        /// Package specification (e.g. mathertel/OneButton@^2.0.3 or crates:async-button@^0.2.0)
        spec: String,
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
    },
    /// Remove a library dependency from ravom.toml and local storage
    Remove {
        /// Library name or full spec to remove
        name: String,
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
    },
    /// List installed libraries in ravom.toml and Cargo.toml
    List {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
    },
    /// Ensure every dependency listed in ravom.toml lib_deps is downloaded locally
    Sync {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ZephyrAction {
    /// Show detected Zephyr workspace, ZEPHYR_BASE, SDK, and board mapping
    Status {
        /// Environment name from ravom.toml (when run inside a project)
        #[arg(short, long)]
        env: Option<String>,
    },
    /// Provision a RavomIO-managed Zephyr west workspace and SDK (defaults to ~/.ravom/zephyr)
    Setup {
        /// Target workspace directory (defaults to ~/.ravom/zephyr or $RAVOM_ZEPHYR_DIR)
        #[arg(short, long)]
        dir: Option<PathBuf>,
        /// Zephyr git tag or branch to initialize
        #[arg(long, default_value = ravom_core::zephyr::DEFAULT_ZEPHYR_VERSION)]
        version: String,
        /// Zephyr SDK version to install via `west sdk install`
        #[arg(long, default_value = ravom_core::zephyr::DEFAULT_ZEPHYR_SDK_VERSION)]
        sdk_version: String,
        /// Enable the official `zephyr-lang-rust` module in the west manifest
        #[arg(long)]
        rust: bool,
        /// Skip downloading the Zephyr SDK
        #[arg(long)]
        skip_sdk: bool,
        /// Print the setup commands instead of executing them
        #[arg(long)]
        dry_run: bool,
    },
    /// Launch interactive Zephyr Kconfig (`west build -t menuconfig`)
    Menuconfig {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Print the menuconfig command instead of running it
        #[arg(long)]
        dry_run: bool,
    },
}
