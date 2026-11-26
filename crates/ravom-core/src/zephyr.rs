//! Zephyr RTOS integration: board mapping, Kconfig/DeviceTree generation,
//! `west build` planning, C/Rust project scaffolding, and workspace/SDK management.

use crate::config::EnvConfig;
use crate::context::EnvContext;
use crate::process::{find_in_path, CommandSpec};
use crate::registry::BoardRecord;
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment variable overriding the managed RavomIO Zephyr workspace (`~/.ravom/zephyr`).
pub const RAVOM_ZEPHYR_DIR_ENV: &str = "RAVOM_ZEPHYR_DIR";

/// Default Zephyr release tag used by `mio zephyr setup`.
pub const DEFAULT_ZEPHYR_VERSION: &str = "v4.1.0";

/// Default Zephyr SDK version used by `mio zephyr setup`.
pub const DEFAULT_ZEPHYR_SDK_VERSION: &str = "0.17.0";

/// Application language for a Zephyr project.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ZephyrLang {
    #[default]
    C,
    Rust,
}

impl ZephyrLang {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Rust => "rust",
        }
    }
}

/// Optional `[env.<name>.zephyr]` configuration inside `ravom.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZephyrConfig {
    /// Zephyr HWMv2 board target (e.g. `nrf52840dk/nrf52840`, `blackpill_f401cc`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board: Option<String>,
    /// Application language (`c` or `rust`).
    #[serde(default, skip_serializing_if = "is_default_lang")]
    pub lang: ZephyrLang,
    /// Inline Kconfig entries rendered to `build/<env>/ravom.conf`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub kconfig: BTreeMap<String, String>,
    /// Additional Kconfig fragment files passed via `-DEXTRA_CONF_FILE=...`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conf_files: Vec<String>,
    /// DeviceTree overlay files passed via `-DDTC_OVERLAY_FILE=...`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlays: Vec<String>,
    /// Zephyr shields passed via `-DSHIELD=...`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shields: Vec<String>,
    /// Pristine build mode (`auto`, `always`, `never`; defaults to `auto`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pristine: Option<String>,
    /// Custom west workspace root (overrides `ZEPHYR_BASE` and `~/.ravom/zephyr`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<PathBuf>,
    /// Custom Zephyr SDK directory (overrides `ZEPHYR_SDK_INSTALL_DIR`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk_dir: Option<PathBuf>,
}

fn is_default_lang(lang: &ZephyrLang) -> bool {
    *lang == ZephyrLang::C
}

/// Whether `framework` selects the Zephyr RTOS backend.
pub fn is_zephyr_framework(framework: &str) -> bool {
    matches!(
        framework.to_ascii_lowercase().as_str(),
        "zephyr" | "zephyr-rust" | "zephyr_rust"
    )
}

/// Whether the environment uses Rust on Zephyr (`zephyr-lang-rust`).
pub fn is_zephyr_rust(env: &EnvConfig) -> bool {
    matches!(
        env.framework.to_ascii_lowercase().as_str(),
        "zephyr-rust" | "zephyr_rust"
    ) || env
        .zephyr
        .as_ref()
        .is_some_and(|z| z.lang == ZephyrLang::Rust)
}

/// Curated mapping from RavomIO/PlatformIO board IDs to Zephyr HWMv2 board targets.
pub fn known_zephyr_board(board_id: &str) -> Option<&'static str> {
    match board_id {
        // ST STM32
        "stm32f401ccu6" | "blackpill_f401cc" => Some("blackpill_f401cc"),
        "stm32f411ceu6" | "blackpill_f411ce" => Some("blackpill_f411ce"),
        "bluepill_f103c8" | "bluepill_f103c8_128k" => Some("stm32_min_dev"),
        "nucleo_f401re" => Some("nucleo_f401re"),
        "nucleo_f411re" => Some("nucleo_f411re"),
        "nucleo_f446re" => Some("nucleo_f446re"),
        "nucleo_f103rb" => Some("nucleo_f103rb"),
        "nucleo_g474re" => Some("nucleo_g474re"),
        "nucleo_h743zi" => Some("nucleo_h743zi"),
        "nucleo_l476rg" => Some("nucleo_l476rg"),
        "disco_f407vg" => Some("stm32f4_disco"),
        "disco_f429zi" => Some("stm32f429i_disc1"),
        // Nordic nRF
        "nrf52840_dk" | "nrf52840dk" => Some("nrf52840dk/nrf52840"),
        "nrf52_dk" | "nrf52dk" => Some("nrf52dk/nrf52832"),
        "nrf52840_dk_adafruit" | "adafruit_feather_nrf52840" => {
            Some("adafruit_feather_nrf52840/nrf52840")
        }
        "xiao_ble" | "xiaoblesense" => Some("xiao_ble/nrf52840"),
        "nrf5340_dk" => Some("nrf5340dk/nrf5340/cpuapp"),
        // Raspberry Pi
        "pico" | "rp2040" => Some("rpi_pico"),
        "rpipicow" | "pico_w" => Some("rpi_pico/rp2040/w"),
        "rpipico2" | "pico2" => Some("rpi_pico2/rp2350a/m33"),
        // Espressif ESP32
        "esp32dev" | "esp32doit-devkit-v1" => Some("esp32_devkitc/esp32/procpu"),
        "esp32-c3-devkitm-1" => Some("esp32c3_devkitm"),
        "esp32-c6-devkitc-1" => Some("esp32c6_devkitc/esp32c6/hpcore"),
        "esp32-s3-devkitc-1" => Some("esp32s3_devkitc/esp32s3/procpu"),
        // Teensy / NXP
        "teensy40" => Some("teensy40"),
        "teensy41" => Some("teensy41"),
        // QEMU virtual targets
        "qemu_cortex_m3" => Some("qemu_cortex_m3"),
        _ => None,
    }
}

/// Resolve the Zephyr board target for a project environment.
///
/// Priority: `env.zephyr.board` -> `board.zephyr_board` -> built-in mapping -> `board.id`.
pub fn resolve_zephyr_board(env: &EnvConfig, board: &BoardRecord) -> String {
    if let Some(explicit) = env.zephyr.as_ref().and_then(|z| z.board.as_deref()) {
        if !explicit.trim().is_empty() {
            return explicit.trim().to_string();
        }
    }
    if let Some(mapped) = board.zephyr_board.as_deref() {
        if !mapped.trim().is_empty() {
            return mapped.trim().to_string();
        }
    }
    if let Some(known) = known_zephyr_board(&board.id) {
        return known.to_string();
    }
    board.id.clone()
}

/// Default directory for the RavomIO-managed Zephyr workspace (`~/.ravom/zephyr`).
pub fn default_zephyr_workspace() -> Option<PathBuf> {
    if let Ok(override_dir) = std::env::var(RAVOM_ZEPHYR_DIR_ENV) {
        if !override_dir.trim().is_empty() {
            return Some(PathBuf::from(override_dir));
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;
    Some(home.join(".ravom").join("zephyr"))
}

/// Resolved local Zephyr environment (workspace, `ZEPHYR_BASE`, SDK, `west`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZephyrEnvironment {
    pub west_bin: String,
    pub workspace_dir: Option<PathBuf>,
    pub zephyr_base: Option<PathBuf>,
    pub sdk_dir: Option<PathBuf>,
}

fn find_sdk_in_dir(parent: &Path) -> Option<PathBuf> {
    let mut matches: Vec<PathBuf> = fs::read_dir(parent)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("zephyr-sdk"))
        })
        .collect();
    matches.sort();
    matches.pop()
}

/// Detect the active Zephyr environment (workspace, `ZEPHYR_BASE`, SDK, and `west` binary).
pub fn detect_zephyr_env(cfg: Option<&ZephyrConfig>) -> ZephyrEnvironment {
    let managed = default_zephyr_workspace();

    // 1. Resolve workspace & ZEPHYR_BASE
    let explicit_ws = cfg.and_then(|c| c.workspace.clone());
    let env_base = std::env::var_os("ZEPHYR_BASE")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());

    let (workspace_dir, zephyr_base) = if let Some(ws) = explicit_ws {
        let base = if ws.join("CMakeLists.txt").is_file() && ws.join("Kconfig.zephyr").is_file() {
            ws.clone()
        } else {
            ws.join("zephyr")
        };
        (Some(ws), Some(base))
    } else if let Some(base) = env_base {
        let ws = base.parent().map(Path::to_path_buf);
        (ws, Some(base))
    } else if let Some(ws) = managed.as_ref() {
        let base = ws.join("zephyr");
        if base.is_dir() {
            (Some(ws.clone()), Some(base))
        } else {
            (Some(ws.clone()), None)
        }
    } else {
        (None, None)
    };

    // 2. Resolve west executable (prefer managed venv if present)
    let venv_west = workspace_dir.as_ref().map(|ws| {
        if cfg!(windows) {
            ws.join(".venv").join("Scripts").join("west.exe")
        } else {
            ws.join(".venv").join("bin").join("west")
        }
    });
    let west_bin = venv_west
        .filter(|p| p.is_file())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "west".to_string());

    // 3. Resolve ZEPHYR_SDK_INSTALL_DIR
    let explicit_sdk = cfg.and_then(|c| c.sdk_dir.clone());
    let env_sdk = std::env::var_os("ZEPHYR_SDK_INSTALL_DIR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty());
    let sdk_dir = explicit_sdk.or(env_sdk).or_else(|| {
        let mut search_dirs = Vec::new();
        if let Some(ws) = &workspace_dir {
            search_dirs.push(ws.join("sdk"));
            search_dirs.push(ws.clone());
        }
        if let Some(home) = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
        {
            search_dirs.push(home);
        }
        search_dirs.push(PathBuf::from("/opt"));
        search_dirs.into_iter().find_map(|d| find_sdk_in_dir(&d))
    });

    ZephyrEnvironment {
        west_bin,
        workspace_dir,
        zephyr_base,
        sdk_dir,
    }
}

/// Build directory for a Zephyr environment: `<root>/build/<env_name>`.
pub fn zephyr_build_dir(root: &Path, env_name: &str) -> PathBuf {
    root.join("build").join(env_name)
}

/// Expected Zephyr ELF artifact: `<root>/build/<env_name>/zephyr/zephyr.elf`.
pub fn zephyr_elf_path(root: &Path, env_name: &str) -> PathBuf {
    zephyr_build_dir(root, env_name)
        .join("zephyr")
        .join("zephyr.elf")
}

/// Path to the RavomIO-generated Kconfig fragment: `<root>/build/<env_name>/ravom.conf`.
pub fn generated_kconfig_path(root: &Path, env_name: &str) -> PathBuf {
    zephyr_build_dir(root, env_name).join("ravom.conf")
}

/// Format a Kconfig value (leaving `y`/`n`/`m`/numbers unquoted, quoting strings).
fn format_kconfig_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if matches!(trimmed, "y" | "n" | "m")
        || (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || trimmed.parse::<i64>().is_ok()
        || trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .is_some_and(|hex| !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()))
    {
        trimmed.to_string()
    } else {
        format!("\"{}\"", trimmed.replace('"', "\\\""))
    }
}

/// Collect the effective inline Kconfig map for an environment + build profile.
pub fn effective_kconfig(env: &EnvConfig, release: bool) -> BTreeMap<String, String> {
    let mut map = env
        .zephyr
        .as_ref()
        .map(|z| z.kconfig.clone())
        .unwrap_or_default();

    if is_zephyr_rust(env) && !map.contains_key("CONFIG_RUST") {
        map.insert("CONFIG_RUST".to_string(), "y".to_string());
    }

    if release {
        let has_opt = [
            "CONFIG_SIZE_OPTIMIZATIONS",
            "CONFIG_SPEED_OPTIMIZATIONS",
            "CONFIG_DEBUG_OPTIMIZATIONS",
            "CONFIG_NO_OPTIMIZATIONS",
        ]
        .iter()
        .any(|k| map.contains_key(*k));
        if !has_opt {
            map.insert("CONFIG_SIZE_OPTIMIZATIONS".to_string(), "y".to_string());
        }
    }

    map
}

/// Render a Kconfig map into a Zephyr `.conf` fragment string.
pub fn render_kconfig_fragment(entries: &BTreeMap<String, String>) -> String {
    let mut out = String::from("# Generated by RavomIO from ravom.toml — do not edit by hand\n");
    for (key, value) in entries {
        let normalized_key = if key.starts_with("CONFIG_") {
            key.clone()
        } else {
            format!("CONFIG_{key}")
        };
        out.push_str(&format!(
            "{normalized_key}={}\n",
            format_kconfig_value(value)
        ));
    }
    out
}

/// Write `build/<env>/ravom.conf` if there are any effective Kconfig entries.
pub fn write_kconfig_fragment(ctx: &EnvContext, release: bool) -> Result<Option<PathBuf>> {
    let entries = effective_kconfig(&ctx.env, release);
    if entries.is_empty() {
        return Ok(None);
    }
    let build_dir = zephyr_build_dir(&ctx.root, &ctx.env_name);
    fs::create_dir_all(&build_dir)?;
    let path = generated_kconfig_path(&ctx.root, &ctx.env_name);
    fs::write(&path, render_kconfig_fragment(&entries))?;
    Ok(Some(path))
}

/// Plan the `west build` command for a Zephyr environment without executing it.
pub fn plan_zephyr_build(ctx: &EnvContext, release: bool) -> CommandSpec {
    let zcfg = ctx.env.zephyr.as_ref();
    let zenv = detect_zephyr_env(zcfg);
    let board = resolve_zephyr_board(&ctx.env, &ctx.board);
    let build_dir = format!("build/{}", ctx.env_name);
    let pristine = zcfg.and_then(|z| z.pristine.as_deref()).unwrap_or("auto");

    let mut cmd = CommandSpec::new(&zenv.west_bin)
        .args([
            "build",
            "-b",
            board.as_str(),
            "-d",
            build_dir.as_str(),
            "-p",
            pristine,
        ])
        .cwd(&ctx.root);

    if let Some(base) = &zenv.zephyr_base {
        cmd = cmd.env("ZEPHYR_BASE", base.display().to_string());
    }
    if let Some(sdk) = &zenv.sdk_dir {
        cmd = cmd.env("ZEPHYR_SDK_INSTALL_DIR", sdk.display().to_string());
    }

    let mut cmake_args = Vec::new();

    // Extra Kconfig fragments (explicit conf_files + generated ravom.conf)
    let mut conf_files: Vec<String> = zcfg.map(|z| z.conf_files.clone()).unwrap_or_default();
    if !effective_kconfig(&ctx.env, release).is_empty() {
        conf_files.push(format!("build/{}/ravom.conf", ctx.env_name));
    }
    if !conf_files.is_empty() {
        cmake_args.push(format!("-DEXTRA_CONF_FILE={}", conf_files.join(";")));
    }

    // DeviceTree overlays (explicit overlays, or auto-detected app.overlay)
    let overlays: Vec<String> = match zcfg {
        Some(z) if !z.overlays.is_empty() => z.overlays.clone(),
        _ if ctx.root.join("app.overlay").is_file() => vec!["app.overlay".to_string()],
        _ => Vec::new(),
    };
    if !overlays.is_empty() {
        cmake_args.push(format!("-DDTC_OVERLAY_FILE={}", overlays.join(";")));
    }

    // Shields
    if let Some(z) = zcfg {
        if !z.shields.is_empty() {
            cmake_args.push(format!("-DSHIELD={}", z.shields.join(";")));
        }
    }

    // Extra C compiler flags from ravom.toml `build_flags`
    if !ctx.env.build_flags.is_empty() {
        cmake_args.push(format!("-DEXTRA_CFLAGS={}", ctx.env.build_flags.join(" ")));
    }

    if !cmake_args.is_empty() {
        cmd = cmd.arg("--").args(cmake_args);
    }

    cmd
}

/// Plan `west flash -d build/<env>` for projects opting into `upload_protocol = "west"`.
pub fn plan_zephyr_flash(ctx: &EnvContext) -> CommandSpec {
    let zenv = detect_zephyr_env(ctx.env.zephyr.as_ref());
    let build_dir = format!("build/{}", ctx.env_name);
    let mut cmd = CommandSpec::new(&zenv.west_bin)
        .args(["flash", "-d", build_dir.as_str()])
        .cwd(&ctx.root);
    if let Some(base) = &zenv.zephyr_base {
        cmd = cmd.env("ZEPHYR_BASE", base.display().to_string());
    }
    if let Some(sdk) = &zenv.sdk_dir {
        cmd = cmd.env("ZEPHYR_SDK_INSTALL_DIR", sdk.display().to_string());
    }
    cmd
}

/// Plan `west build -t menuconfig -d build/<env>` for interactive Kconfig editing.
pub fn plan_zephyr_menuconfig(ctx: &EnvContext) -> CommandSpec {
    let zenv = detect_zephyr_env(ctx.env.zephyr.as_ref());
    let board = resolve_zephyr_board(&ctx.env, &ctx.board);
    let build_dir = format!("build/{}", ctx.env_name);
    let mut cmd = CommandSpec::new(&zenv.west_bin)
        .args([
            "build",
            "-b",
            board.as_str(),
            "-d",
            build_dir.as_str(),
            "-t",
            "menuconfig",
        ])
        .cwd(&ctx.root);
    if let Some(base) = &zenv.zephyr_base {
        cmd = cmd.env("ZEPHYR_BASE", base.display().to_string());
    }
    if let Some(sdk) = &zenv.sdk_dir {
        cmd = cmd.env("ZEPHYR_SDK_INSTALL_DIR", sdk.display().to_string());
    }
    cmd
}

/// Scaffold a Zephyr C or Rust application layout in `dir`.
pub fn scaffold_zephyr_project(
    dir: &Path,
    project_name: &str,
    board: &BoardRecord,
    lang: ZephyrLang,
) -> Result<()> {
    let clean_name = project_name.replace(' ', "-");
    let zboard = known_zephyr_board(&board.id).unwrap_or(&board.id);

    // Starter DeviceTree overlay
    fs::write(
        dir.join("app.overlay"),
        format!(
            "/* RavomIO Zephyr DeviceTree overlay for `{zboard}` */\n\
             /* Add custom pinmux, UART, I2C, SPI, or LED aliases here. */\n"
        ),
    )?;

    match lang {
        ZephyrLang::C => {
            fs::write(
                dir.join("CMakeLists.txt"),
                format!(
                    "cmake_minimum_required(VERSION 3.20.0)\n\
                     find_package(Zephyr REQUIRED HINTS $ENV{{ZEPHYR_BASE}})\n\
                     project({clean_name})\n\n\
                     target_sources(app PRIVATE src/main.c)\n"
                ),
            )?;
            fs::write(
                dir.join("prj.conf"),
                "# RavomIO Zephyr Kconfig defaults\n\
                 CONFIG_PRINTK=y\n\
                 CONFIG_LOG=y\n",
            )?;
            fs::write(
                dir.join("src/main.c"),
                format!(
                    "/* RavomIO Zephyr C starter for `{zboard}` */\n\
                     #include <zephyr/kernel.h>\n\
                     #include <zephyr/sys/printk.h>\n\n\
                     int main(void)\n\
                     {{\n\
                         printk(\"RavomIO Zephyr booted on {zboard}\\n\");\n\
                         while (1) {{\n\
                             k_msleep(1000);\n\
                         }}\n\
                         return 0;\n\
                     }}\n"
                ),
            )?;
            fs::write(dir.join(".gitignore"), "build/\n.ravom/\n")?;
        }
        ZephyrLang::Rust => {
            fs::write(
                dir.join("CMakeLists.txt"),
                format!(
                    "cmake_minimum_required(VERSION 3.20.0)\n\
                     find_package(Zephyr REQUIRED HINTS $ENV{{ZEPHYR_BASE}})\n\
                     project({clean_name})\n\n\
                     rust_cargo_application()\n"
                ),
            )?;
            fs::write(
                dir.join("prj.conf"),
                "# RavomIO Zephyr Rust Kconfig defaults (requires zephyr-lang-rust module)\n\
                 CONFIG_RUST=y\n\
                 CONFIG_PRINTK=y\n\
                 CONFIG_LOG=y\n",
            )?;
            fs::write(
                dir.join("Cargo.toml"),
                format!(
                    "[package]\n\
                     name = \"{clean_name}\"\n\
                     version = \"0.1.0\"\n\
                     edition = \"2021\"\n\n\
                     [lib]\n\
                     crate-type = [\"staticlib\"]\n\n\
                     [dependencies]\n\
                     # Provided automatically by Zephyr's `zephyr-lang-rust` CMake integration.\n\
                     zephyr = \"0.1\"\n"
                ),
            )?;
            fs::write(
                dir.join("src/lib.rs"),
                format!(
                    "// RavomIO Zephyr Rust starter for `{zboard}`\n\
                     #![no_std]\n\n\
                     use zephyr::time::{{sleep, Duration}};\n\n\
                     #[no_mangle]\n\
                     pub extern \"C\" fn rust_main() {{\n\
                         zephyr::printkln!(\"RavomIO Zephyr Rust booted on {zboard}\");\n\
                         loop {{\n\
                             sleep(Duration::millis_at_least(1000));\n\
                         }}\n\
                     }}\n"
                ),
            )?;
            fs::write(dir.join(".gitignore"), "build/\ntarget/\n.ravom/\n")?;
        }
    }

    Ok(())
}

/// Options for provisioning a managed Zephyr workspace and SDK (`mio zephyr setup`).
#[derive(Debug, Clone)]
pub struct ZephyrSetupOptions {
    pub workspace_dir: PathBuf,
    pub zephyr_version: String,
    pub sdk_version: String,
    pub enable_rust: bool,
    pub skip_sdk: bool,
}

impl ZephyrSetupOptions {
    pub fn default_with_dir(workspace_dir: PathBuf) -> Self {
        Self {
            workspace_dir,
            zephyr_version: DEFAULT_ZEPHYR_VERSION.to_string(),
            sdk_version: DEFAULT_ZEPHYR_SDK_VERSION.to_string(),
            enable_rust: false,
            skip_sdk: false,
        }
    }
}

/// Build the ordered command plan for `mio zephyr setup`.
pub fn plan_zephyr_setup(opts: &ZephyrSetupOptions) -> Vec<CommandSpec> {
    let ws = &opts.workspace_dir;
    let venv_dir = ws.join(".venv");
    let (venv_pip, venv_west) = if cfg!(windows) {
        (
            venv_dir.join("Scripts").join("pip.exe"),
            venv_dir.join("Scripts").join("west.exe"),
        )
    } else {
        (
            venv_dir.join("bin").join("pip"),
            venv_dir.join("bin").join("west"),
        )
    };
    let west_s = venv_west.display().to_string();
    let use_uv = find_in_path("uv").is_some();

    let mut steps = Vec::new();

    // 1. Create isolated Python virtual environment + install west
    if use_uv {
        steps.push(CommandSpec::new("uv").args(["venv"]).path_arg(&venv_dir));
        steps.push(
            CommandSpec::new("uv")
                .args(["pip", "install", "--python"])
                .path_arg(&venv_dir)
                .arg("west"),
        );
    } else {
        steps.push(
            CommandSpec::new("python3")
                .args(["-m", "venv"])
                .path_arg(&venv_dir),
        );
        steps.push(CommandSpec::new(venv_pip.display().to_string()).args([
            "install",
            "--upgrade",
            "pip",
            "west",
        ]));
    }

    // 2. Initialize Zephyr west workspace (shallow clone for speed)
    steps.push(
        CommandSpec::new(&west_s)
            .args([
                "init",
                "-m",
                "https://github.com/zephyrproject-rtos/zephyr",
                "--mr",
                opts.zephyr_version.as_str(),
            ])
            .path_arg(ws),
    );

    // 3. Optionally enable the official Rust language module (`zephyr-lang-rust`)
    if opts.enable_rust {
        steps.push(
            CommandSpec::new(&west_s)
                .args(["config", "manifest.project-filter", "+zephyr-lang-rust"])
                .cwd(ws),
        );
    }

    // 4. Fetch modules and export Zephyr CMake package
    steps.push(
        CommandSpec::new(&west_s)
            .args(["update", "--narrow", "-o=--depth=1"])
            .cwd(ws),
    );
    steps.push(CommandSpec::new(&west_s).args(["zephyr-export"]).cwd(ws));

    // 5. Install Python requirements for Zephyr build scripts
    steps.push(
        CommandSpec::new(&west_s)
            .args(["packages", "pip", "--install"])
            .cwd(ws),
    );

    // 6. Install Zephyr SDK via `west sdk install`
    if !opts.skip_sdk {
        let sdk_parent = ws.join("sdk");
        steps.push(
            CommandSpec::new(&west_s)
                .args([
                    "sdk",
                    "install",
                    "--version",
                    opts.sdk_version.as_str(),
                    "--install-base",
                ])
                .path_arg(&sdk_parent)
                .cwd(ws),
        );
    }

    steps
}

/// Execute `mio zephyr setup` to provision the managed Zephyr workspace and SDK.
pub fn setup_zephyr(opts: &ZephyrSetupOptions) -> Result<()> {
    fs::create_dir_all(&opts.workspace_dir)?;
    for step in plan_zephyr_setup(opts) {
        step.run(CoreError::BuildFailed)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RavomConfig;

    #[test]
    fn maps_popular_boards_to_zephyr_hwmv2_names() {
        assert_eq!(
            known_zephyr_board("stm32f401ccu6"),
            Some("blackpill_f401cc")
        );
        assert_eq!(
            known_zephyr_board("nrf52840_dk"),
            Some("nrf52840dk/nrf52840")
        );
        assert_eq!(known_zephyr_board("pico"), Some("rpi_pico"));
        assert_eq!(
            known_zephyr_board("esp32-s3-devkitc-1"),
            Some("esp32s3_devkitc/esp32s3/procpu")
        );
    }

    #[test]
    fn renders_kconfig_fragment_with_proper_quoting() {
        let mut entries = BTreeMap::new();
        entries.insert("CONFIG_BT".to_string(), "y".to_string());
        entries.insert("LOG_DEFAULT_LEVEL".to_string(), "3".to_string());
        entries.insert(
            "CONFIG_BT_DEVICE_NAME".to_string(),
            "RavomIO BLE".to_string(),
        );
        entries.insert(
            "CONFIG_SRAM_BASE_ADDRESS".to_string(),
            "0x20000000".to_string(),
        );

        let text = render_kconfig_fragment(&entries);
        assert!(text.contains("CONFIG_BT=y\n"), "{text}");
        assert!(text.contains("CONFIG_LOG_DEFAULT_LEVEL=3\n"), "{text}");
        assert!(
            text.contains("CONFIG_BT_DEVICE_NAME=\"RavomIO BLE\"\n"),
            "{text}"
        );
        assert!(
            text.contains("CONFIG_SRAM_BASE_ADDRESS=0x20000000\n"),
            "{text}"
        );
    }

    #[test]
    fn plans_west_build_with_kconfig_overlays_shields_and_rust() {
        let cfg = RavomConfig::from_toml_str(
            "[env.ble]\n\
             platform = \"ststm32\"\n\
             board = \"stm32f401ccu6\"\n\
             framework = \"zephyr\"\n\
             build_flags = [\"-DAPP_DEBUG=1\"]\n\
             [env.ble.zephyr]\n\
             lang = \"rust\"\n\
             overlays = [\"boards/blackpill.overlay\"]\n\
             shields = [\"ssd1306_128x64\"]\n\
             [env.ble.zephyr.kconfig]\n\
             CONFIG_USE_SEGGER_RTT = \"y\"\n",
        )
        .expect("parse ravom.toml");
        let ctx = EnvContext::from_config(Path::new("/tmp/zapp"), cfg, None).expect("ctx");

        let cmd = plan_zephyr_build(&ctx, true);
        assert!(cmd.args.starts_with(&[
            "build".to_string(),
            "-b".to_string(),
            "blackpill_f401cc".to_string(),
            "-d".to_string(),
            "build/ble".to_string(),
            "-p".to_string(),
            "auto".to_string(),
            "--".to_string(),
        ]));
        assert!(cmd
            .args
            .contains(&"-DEXTRA_CONF_FILE=build/ble/ravom.conf".to_string()));
        assert!(cmd
            .args
            .contains(&"-DDTC_OVERLAY_FILE=boards/blackpill.overlay".to_string()));
        assert!(cmd.args.contains(&"-DSHIELD=ssd1306_128x64".to_string()));
        assert!(cmd
            .args
            .contains(&"-DEXTRA_CFLAGS=-DAPP_DEBUG=1".to_string()));

        let kconfig = effective_kconfig(&ctx.env, true);
        assert_eq!(kconfig.get("CONFIG_RUST").map(String::as_str), Some("y"));
        assert_eq!(
            kconfig.get("CONFIG_SIZE_OPTIMIZATIONS").map(String::as_str),
            Some("y")
        );
        assert_eq!(
            kconfig.get("CONFIG_USE_SEGGER_RTT").map(String::as_str),
            Some("y")
        );
    }

    #[test]
    fn plans_zephyr_setup_with_rust_module_and_sdk() {
        let mut opts = ZephyrSetupOptions::default_with_dir(PathBuf::from("/tmp/ravom-zephyr"));
        opts.enable_rust = true;
        let plan = plan_zephyr_setup(&opts);
        let rendered: Vec<String> = plan.iter().map(ToString::to_string).collect();
        assert!(rendered.iter().any(|s| s.contains("west init")));
        assert!(rendered
            .iter()
            .any(|s| s.contains("manifest.project-filter +zephyr-lang-rust")));
        assert!(rendered.iter().any(|s| s.contains("west update --narrow")));
        assert!(rendered.iter().any(|s| s.contains("sdk install --version")));
    }
}
