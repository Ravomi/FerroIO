use crate::chips::probe_rs_snapshot_version;
use crate::process::{find_in_path, CommandSpec};
use crate::{CoreError, Result};
use std::path::PathBuf;
use std::process::Command;

/// Availability status of an external embedded or FPGA toolchain binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: &'static str,
    pub installed: bool,
    pub path: Option<PathBuf>,
    /// First line of `<tool> --version` (or equivalent), when available.
    pub version: Option<String>,
    pub install_hint: &'static str,
}

struct ToolSpec {
    name: &'static str,
    version_arg: &'static str,
    install_hint: &'static str,
}

const TOOLS: &[ToolSpec] = &[
    ToolSpec {
        name: "cargo",
        version_arg: "--version",
        install_hint: "https://rustup.rs",
    },
    ToolSpec {
        name: "rustup",
        version_arg: "--version",
        install_hint: "https://rustup.rs",
    },
    ToolSpec {
        name: "probe-rs",
        version_arg: "--version",
        install_hint: "cargo install probe-rs-tools --locked",
    },
    ToolSpec {
        name: "openFPGALoader",
        version_arg: "--Version",
        install_hint: "Install via system package manager or oss-cad-suite",
    },
    ToolSpec {
        name: "yosys",
        version_arg: "-V",
        install_hint: "Install via system package manager or oss-cad-suite",
    },
    ToolSpec {
        name: "arm-none-eabi-gcc",
        version_arg: "--version",
        install_hint: "Install the Arm GNU toolchain (arm-none-eabi-gcc)",
    },
    ToolSpec {
        name: "idf.py",
        version_arg: "--version",
        install_hint: "https://docs.espressif.com/projects/esp-idf/en/latest/get-started/",
    },
    ToolSpec {
        name: "west",
        version_arg: "--version",
        install_hint: "Run `mio zephyr setup` or `pip install west`",
    },
    ToolSpec {
        name: "cmake",
        version_arg: "--version",
        install_hint: "Install CMake >= 3.20 (required by Zephyr)",
    },
    ToolSpec {
        name: "ninja",
        version_arg: "--version",
        install_hint: "Install Ninja build tool (required by Zephyr)",
    },
    ToolSpec {
        name: "dtc",
        version_arg: "--version",
        install_hint: "Install DeviceTree compiler (dtc, required by Zephyr)",
    },
    ToolSpec {
        name: "renode",
        version_arg: "--version",
        install_hint: "https://renode.io",
    },
    ToolSpec {
        name: "qemu-system-arm",
        version_arg: "--version",
        install_hint: "Install qemu-system-arm",
    },
];

fn first_line_of_version(path: &PathBuf, arg: &str) -> Option<String> {
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

/// Inspect the local environment for Rust, MCU, FPGA, and simulation toolchains.
pub fn check_toolchains() -> Vec<ToolStatus> {
    TOOLS
        .iter()
        .map(|tool| {
            let path = find_in_path(tool.name);
            let version = path
                .as_ref()
                .and_then(|p| first_line_of_version(p, tool.version_arg));
            ToolStatus {
                name: tool.name,
                installed: path.is_some(),
                path,
                version,
                install_hint: tool.install_hint,
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

/// Install a Rust cross-compilation target via `rustup target add <triple>`.
pub fn install_rust_target(target: &str) -> Result<()> {
    CommandSpec::new("rustup")
        .args(["target", "add", target])
        .run(CoreError::BuildFailed)
}

#[cfg(test)]
mod tests {
    use super::check_toolchains;

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
}
