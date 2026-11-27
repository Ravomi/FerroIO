use crate::chips::probe_rs_snapshot_version;
use crate::process::{find_in_path, CommandSpec};
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Environment variable overriding the persistent custom toolchain override TOML file.
pub const RAVOM_TOOLS_CONFIG_ENV: &str = "RAVOM_TOOLS_CONFIG";

/// Functional category for grouping external embedded toolchains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolCategory {
    #[default]
    All,
    Core,
    Flash,
    McuC,
    Zephyr,
    Simulator,
}

impl ToolCategory {
    /// All selectable toolchain filter categories in display order.
    pub const ALL: &'static [Self] = &[
        Self::All,
        Self::Core,
        Self::Flash,
        Self::McuC,
        Self::Zephyr,
        Self::Simulator,
    ];

    /// Short filter chip label.
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Core => "Core",
            Self::Flash => "Flash",
            Self::McuC => "MCU C",
            Self::Zephyr => "Zephyr",
            Self::Simulator => "Sim",
        }
    }
}

/// Availability and configuration status of an external embedded toolchain binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: &'static str,
    pub category: ToolCategory,
    pub installed: bool,
    pub path: Option<PathBuf>,
    /// Explicit path override saved in `~/.ravom/tools.toml`, if any.
    pub configured_path: Option<PathBuf>,
    /// First line of `<tool> --version` (or equivalent), when available.
    pub version: Option<String>,
    pub install_hint: &'static str,
    pub docs_url: &'static str,
    /// Automated install command available on this host, if any.
    pub auto_install_cmd: Option<String>,
}

/// Curated status entry for an embedded Rust compilation target (`rustup target`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustTargetStatus {
    pub triple: &'static str,
    pub label: &'static str,
    pub boards_hint: &'static str,
    pub installed: bool,
}

/// Persistent custom tool binary overrides stored in `~/.ravom/tools.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolOverrides {
    #[serde(default)]
    pub tools: BTreeMap<String, PathBuf>,
}

struct ToolSpec {
    name: &'static str,
    category: ToolCategory,
    version_arg: &'static str,
    install_hint: &'static str,
    docs_url: &'static str,
}

struct TargetSpec {
    triple: &'static str,
    label: &'static str,
    boards_hint: &'static str,
}

const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "cargo",
        category: ToolCategory::Core,
        version_arg: "--version",
        install_hint: "Install Rust toolchain from https://rustup.rs",
        docs_url: "https://rustup.rs",
    },
    ToolSpec {
        name: "rustup",
        category: ToolCategory::Core,
        version_arg: "--version",
        install_hint: "Install Rust toolchain manager from https://rustup.rs",
        docs_url: "https://rustup.rs",
    },
    ToolSpec {
        name: "probe-rs",
        category: ToolCategory::Flash,
        version_arg: "--version",
        install_hint: "Install probe-rs CLI (brew install probe-rs/tap/probe-rs or cargo install)",
        docs_url: "https://probe.rs",
    },
    ToolSpec {
        name: "espflash",
        category: ToolCategory::Flash,
        version_arg: "--version",
        install_hint: "Install espflash for ESP32 USB flashing (brew/cargo)",
        docs_url: "https://github.com/esp-rs/espflash",
    },
    ToolSpec {
        name: "arm-none-eabi-gcc",
        category: ToolCategory::McuC,
        version_arg: "--version",
        install_hint: "Install the Arm GNU embedded toolchain (arm-none-eabi-gcc)",
        docs_url: "https://developer.arm.com/Tools%20and%20Software/GNU%20Toolchain",
    },
    ToolSpec {
        name: "idf.py",
        category: ToolCategory::McuC,
        version_arg: "--version",
        install_hint: "Install ESP-IDF and source export.sh (or point to idf.py below)",
        docs_url: "https://docs.espressif.com/projects/esp-idf/en/latest/get-started/",
    },
    ToolSpec {
        name: "west",
        category: ToolCategory::Zephyr,
        version_arg: "--version",
        install_hint: "Run `mio zephyr setup` or install Zephyr west via pip3",
        docs_url: "https://docs.zephyrproject.org/latest/develop/west/",
    },
    ToolSpec {
        name: "cmake",
        category: ToolCategory::Zephyr,
        version_arg: "--version",
        install_hint: "Install CMake >= 3.20 (required by Zephyr and C/C++ builds)",
        docs_url: "https://cmake.org/download/",
    },
    ToolSpec {
        name: "ninja",
        category: ToolCategory::Zephyr,
        version_arg: "--version",
        install_hint: "Install Ninja build tool (required by Zephyr)",
        docs_url: "https://ninja-build.org/",
    },
    ToolSpec {
        name: "dtc",
        category: ToolCategory::Zephyr,
        version_arg: "--version",
        install_hint: "Install DeviceTree compiler (dtc, required by Zephyr)",
        docs_url: "https://devicetree.org/",
    },
    ToolSpec {
        name: "renode",
        category: ToolCategory::Simulator,
        version_arg: "--version",
        install_hint: "Install Renode multi-node embedded simulator",
        docs_url: "https://renode.io",
    },
    ToolSpec {
        name: "qemu-system-arm",
        category: ToolCategory::Simulator,
        version_arg: "--version",
        install_hint: "Install qemu-system-arm system emulator",
        docs_url: "https://www.qemu.org/",
    },
];

const CURATED_RUST_TARGETS: &[TargetSpec] = &[
    TargetSpec {
        triple: "riscv32imc-unknown-none-elf",
        label: "ESP32-C3 / RISC-V IMC",
        boards_hint: "esp32-c3-devkitm-1, xiao_esp32c3",
    },
    TargetSpec {
        triple: "riscv32imac-unknown-none-elf",
        label: "ESP32-C6 / ESP32-H2",
        boards_hint: "esp32-c6-devkitc-1, esp32-h2",
    },
    TargetSpec {
        triple: "thumbv6m-none-eabi",
        label: "ARM Cortex-M0 / M0+",
        boards_hint: "RP2040 Pico, STM32F0, nRF51",
    },
    TargetSpec {
        triple: "thumbv7m-none-eabi",
        label: "ARM Cortex-M3",
        boards_hint: "STM32F1 BluePill, LPC1768",
    },
    TargetSpec {
        triple: "thumbv7em-none-eabi",
        label: "ARM Cortex-M4 (soft-float)",
        boards_hint: "STM32F3/F4 soft-float, SAMD51",
    },
    TargetSpec {
        triple: "thumbv7em-none-eabihf",
        label: "ARM Cortex-M4F / M7",
        boards_hint: "STM32F4 BlackPill, nRF52840, Teensy 4",
    },
    TargetSpec {
        triple: "thumbv8m.main-none-eabihf",
        label: "ARM Cortex-M33",
        boards_hint: "RP2350 Pico 2, nRF5340, STM32U5",
    },
];

/// Return the path to RavomIO's persistent toolchain overrides file (`~/.ravom/tools.toml`).
pub fn tools_config_path() -> PathBuf {
    if let Ok(explicit) = std::env::var(RAVOM_TOOLS_CONFIG_ENV) {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".ravom").join("tools.toml"))
        .unwrap_or_else(|| PathBuf::from(".ravom").join("tools.toml"))
}

/// Read custom toolchain overrides from `config_path` (returns empty on missing/unreadable files).
pub fn load_tool_overrides_from(config_path: &Path) -> ToolOverrides {
    let Ok(raw) = fs::read_to_string(config_path) else {
        return ToolOverrides::default();
    };
    toml::from_str::<ToolOverrides>(&raw).unwrap_or_default()
}

/// Read custom toolchain overrides from [`tools_config_path`].
pub fn load_tool_overrides() -> ToolOverrides {
    load_tool_overrides_from(&tools_config_path())
}

/// Save or remove a custom executable path for `tool_name` in `config_path`.
pub fn save_tool_override_to(
    config_path: &Path,
    tool_name: &str,
    custom_path: Option<&Path>,
) -> Result<PathBuf> {
    let key = tool_name.trim();
    if key.is_empty() {
        return Err(CoreError::Package("tool name cannot be empty".to_string()));
    }
    let mut overrides = load_tool_overrides_from(config_path);
    match custom_path {
        Some(path) => {
            let trimmed = path.to_string_lossy().trim().to_string();
            if trimmed.is_empty() {
                return Err(CoreError::Package(format!(
                    "custom executable path for '{key}' cannot be empty"
                )));
            }
            overrides
                .tools
                .insert(key.to_string(), PathBuf::from(trimmed));
        }
        None => {
            overrides.tools.remove(key);
        }
    }
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let encoded = toml::to_string_pretty(&overrides)?;
    fs::write(config_path, encoded)?;
    Ok(config_path.to_path_buf())
}

/// Save or remove a custom executable path for `tool_name` in [`tools_config_path`].
pub fn save_tool_override(tool_name: &str, custom_path: Option<&Path>) -> Result<PathBuf> {
    save_tool_override_to(&tools_config_path(), tool_name, custom_path)
}

/// Resolve a configured binary override for `program` from `config_path` when it points to a file.
pub fn resolve_configured_tool_from(config_path: &Path, program: &str) -> Option<PathBuf> {
    let overrides = load_tool_overrides_from(config_path);
    overrides
        .tools
        .get(program)
        .filter(|path| path.is_file())
        .cloned()
}

/// Resolve a configured binary override for `program` from [`tools_config_path`].
pub fn resolve_configured_tool(program: &str) -> Option<PathBuf> {
    resolve_configured_tool_from(&tools_config_path(), program)
}

fn first_line_of_version(path: &Path, arg: &str) -> Option<String> {
    let output = Command::new(path).arg(arg).output().ok()?;
    let text = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    String::from_utf8_lossy(&text)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

/// Plan an automated host install command for `tool_name`, when supported on this machine.
pub fn install_tool_plan(tool_name: &str) -> Option<CommandSpec> {
    let has_brew = cfg!(target_os = "macos") && find_in_path("brew").is_some();
    let has_cargo = find_in_path("cargo").is_some();
    let has_pip = find_in_path("pip3").is_some();

    match tool_name {
        "probe-rs" => {
            if has_brew {
                Some(CommandSpec::new("brew").args(["install", "probe-rs/tap/probe-rs"]))
            } else if has_cargo {
                Some(CommandSpec::new("cargo").args(["install", "probe-rs-tools", "--locked"]))
            } else {
                None
            }
        }
        "espflash" => {
            if has_brew {
                Some(CommandSpec::new("brew").args(["install", "espflash"]))
            } else if has_cargo {
                Some(CommandSpec::new("cargo").args(["install", "espflash", "--locked"]))
            } else {
                None
            }
        }
        "cmake" if has_brew => Some(CommandSpec::new("brew").args(["install", "cmake"])),
        "ninja" if has_brew => Some(CommandSpec::new("brew").args(["install", "ninja"])),
        "dtc" if has_brew => Some(CommandSpec::new("brew").args(["install", "dtc"])),
        "arm-none-eabi-gcc" if has_brew => {
            Some(CommandSpec::new("brew").args(["install", "--cask", "gcc-arm-embedded"]))
        }
        "qemu-system-arm" if has_brew => Some(CommandSpec::new("brew").args(["install", "qemu"])),
        "west" if has_pip => Some(CommandSpec::new("pip3").args(["install", "--user", "west"])),
        _ => None,
    }
}

/// Run an automated toolchain installation and stream progress lines to `log`.
pub fn install_tool_streaming(tool_name: &str, log: &mut dyn FnMut(&str)) -> Result<()> {
    let cmd = install_tool_plan(tool_name).ok_or_else(|| {
        CoreError::Package(format!(
            "no automatic installer available for '{tool_name}'; configure a local binary path instead"
        ))
    })?;
    log(&format!("$ {cmd}"));
    cmd.run_streaming(CoreError::BuildFailed, log)
}

/// Inspect the local environment for Rust, MCU, Zephyr, and simulation toolchains.
pub fn check_toolchains() -> Vec<ToolStatus> {
    let overrides = load_tool_overrides();
    TOOLS
        .iter()
        .map(|tool| {
            let configured_path = overrides.tools.get(tool.name).cloned();
            let path = configured_path
                .as_ref()
                .filter(|p| p.is_file())
                .cloned()
                .or_else(|| find_in_path(tool.name));
            let version = path
                .as_ref()
                .and_then(|p| first_line_of_version(p, tool.version_arg));
            let auto_install_cmd = install_tool_plan(tool.name).map(|cmd| cmd.to_string());
            ToolStatus {
                name: tool.name,
                category: tool.category,
                installed: path.is_some(),
                path,
                configured_path,
                version,
                install_hint: tool.install_hint,
                docs_url: tool.docs_url,
                auto_install_cmd,
            }
        })
        .collect()
}

/// Host-level problems that typically break flashing on real hardware.
pub fn check_host_setup() -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(probe_rs) = check_toolchains()
        .into_iter()
        .find(|t| t.name == "probe-rs" && t.installed)
    {
        if let (Some(installed), Some(snapshot)) = (
            probe_rs
                .version
                .as_deref()
                .and_then(|v| v.split_whitespace().nth(1)),
            probe_rs_snapshot_version(),
        ) {
            if installed != snapshot {
                warnings.push(format!(
                    "probe-rs {installed} is installed but the board registry was validated against \
                     probe-rs {snapshot}; chip names may differ"
                ));
            }
        }
    }
    if cfg!(target_os = "linux") {
        let has_rules = [
            "/etc/udev/rules.d",
            "/usr/lib/udev/rules.d",
            "/lib/udev/rules.d",
        ]
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|it| it.flatten())
        .any(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            name.contains("probe-rs") || name.contains("stlink") || name.contains("cmsis")
        });
        if !has_rules {
            warnings.push(
                "no probe-rs/ST-Link udev rules found; debug probes may need root \
                 (see https://probe.rs/docs/getting-started/probe-setup/)"
                    .to_string(),
            );
        }
    }
    warnings
}

/// List curated embedded Rust target triples and whether each is installed via `rustup`.
pub fn list_rust_targets() -> Vec<RustTargetStatus> {
    let rustup = resolve_configured_tool("rustup").unwrap_or_else(|| PathBuf::from("rustup"));
    let installed_set: BTreeSet<String> = Command::new(rustup)
        .args(["target", "list", "--installed"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default();

    CURATED_RUST_TARGETS
        .iter()
        .map(|spec| RustTargetStatus {
            triple: spec.triple,
            label: spec.label,
            boards_hint: spec.boards_hint,
            installed: installed_set.contains(spec.triple),
        })
        .collect()
}

/// Install a Rust cross-compilation target via `rustup target add <triple>`.
pub fn install_rust_target(target: &str) -> Result<()> {
    CommandSpec::new("rustup")
        .args(["target", "add", target])
        .run(CoreError::BuildFailed)
}

/// Install a Rust cross-compilation target and stream output lines to `log`.
pub fn install_rust_target_streaming(target: &str, log: &mut dyn FnMut(&str)) -> Result<()> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Package(
            "Rust target triple cannot be empty".to_string(),
        ));
    }
    let cmd = CommandSpec::new("rustup").args(["target", "add", trimmed]);
    log(&format!("$ {cmd}"));
    cmd.run_streaming(CoreError::BuildFailed, log)
}

/// Remove an installed Rust cross-compilation target and stream output lines to `log`.
pub fn remove_rust_target_streaming(target: &str, log: &mut dyn FnMut(&str)) -> Result<()> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Package(
            "Rust target triple cannot be empty".to_string(),
        ));
    }
    let cmd = CommandSpec::new("rustup").args(["target", "remove", trimmed]);
    log(&format!("$ {cmd}"));
    cmd.run_streaming(CoreError::BuildFailed, log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cargo_with_version_in_local_environment() {
        let statuses = check_toolchains();
        let cargo = statuses
            .iter()
            .find(|t| t.name == "cargo")
            .expect("cargo entry");
        assert!(cargo.installed);
        assert!(cargo
            .version
            .as_deref()
            .is_some_and(|v| v.starts_with("cargo ")));
    }

    #[test]
    fn tool_overrides_save_load_resolve_and_clear_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cfg_path = tmp.path().join("tools.toml");
        let dummy_bin = tmp.path().join("custom-probe-rs");
        fs::write(&dummy_bin, "#!/bin/sh\n").expect("write dummy bin");

        save_tool_override_to(&cfg_path, "probe-rs", Some(&dummy_bin)).expect("save override");
        let loaded = load_tool_overrides_from(&cfg_path);
        assert_eq!(loaded.tools.get("probe-rs"), Some(&dummy_bin));
        assert_eq!(
            resolve_configured_tool_from(&cfg_path, "probe-rs").as_ref(),
            Some(&dummy_bin)
        );

        save_tool_override_to(&cfg_path, "probe-rs", None).expect("clear override");
        assert_eq!(resolve_configured_tool_from(&cfg_path, "probe-rs"), None);
    }

    #[test]
    fn list_rust_targets_returns_curated_embedded_targets() {
        let targets = list_rust_targets();
        assert_eq!(targets.len(), 7);
        assert!(targets
            .iter()
            .any(|t| t.triple == "riscv32imc-unknown-none-elf"));
        assert!(targets.iter().any(|t| t.triple == "thumbv7em-none-eabihf"));
    }
}
