//! Firmware anti-reversing hardening planner, ELF binary leak auditor, and chip read-protection planner.

use crate::context::EnvContext;
use crate::process::CommandSpec;
use crate::registry::BoardRecord;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Build profile separating real-time HIL tuning from production anti-reversing hardening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityProfile {
    /// Unoptimized debug build with full DWARF symbols and RTT.
    Dev,
    /// Optimized (`-C opt-level=s`) for real-time PID control loop determinism with telemetry enabled and no obfuscation.
    Hil,
    /// Production build with symbol stripping, path remapping, `panic=abort`, fat LTO, and optional LLVM obfuscation passes.
    ReleaseHardened,
}

/// Planned Cargo command and hardening flags for a security profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardenedBuildPlan {
    pub profile: SecurityProfile,
    pub board_id: String,
    pub target: Option<String>,
    pub rustflags: Vec<String>,
    pub command: String,
}

/// Result of auditing a compiled ELF firmware binary for reverse-engineering leaks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareAuditReport {
    pub file_size_bytes: u64,
    pub is_elf: bool,
    pub has_debug_sections: bool,
    pub has_symtab: bool,
    pub leaked_host_paths: Vec<String>,
    pub has_panic_strings: bool,
    pub hardened: bool,
    pub recommendations: Vec<String>,
}

/// Planned hardware read-out protection / flash encryption commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChipProtectionPlan {
    pub board_id: String,
    pub mcu: String,
    pub reversible_commands: Vec<String>,
    pub irreversible_manual_commands: Vec<String>,
    pub warning: String,
}

/// Compute a stage-separated Cargo build plan (`Dev`, `Hil`, or `ReleaseHardened`).
pub fn plan_hardened_build(
    ctx: &EnvContext,
    profile: SecurityProfile,
    llvm_obfuscation_passes: &[String],
) -> HardenedBuildPlan {
    let mut rustflags = ctx.env.rustflags.clone();
    match profile {
        SecurityProfile::Dev => {}
        SecurityProfile::Hil => {
            rustflags.push("-C".to_string());
            rustflags.push("opt-level=s".to_string());
            rustflags.push("-C".to_string());
            rustflags.push("debuginfo=2".to_string());
        }
        SecurityProfile::ReleaseHardened => {
            let root_str = ctx.root.display().to_string();
            rustflags.extend([
                "-C".to_string(),
                "opt-level=z".to_string(),
                "-C".to_string(),
                "lto=fat".to_string(),
                "-C".to_string(),
                "codegen-units=1".to_string(),
                "-C".to_string(),
                "strip=symbols".to_string(),
                "-C".to_string(),
                "panic=abort".to_string(),
                format!("--remap-path-prefix={root_str}=/app"),
            ]);
            if let Ok(home) = std::env::var("HOME") {
                rustflags.push(format!("--remap-path-prefix={home}=/user"));
            }
            for pass in llvm_obfuscation_passes {
                let trimmed = pass.trim();
                if !trimmed.is_empty() {
                    rustflags.push("-C".to_string());
                    rustflags.push(format!("llvm-args={trimmed}"));
                }
            }
        }
    }

    let mut cmd = CommandSpec::new("cargo").arg("build").cwd(&ctx.root);
    if let Some(target) = &ctx.board.rust_target {
        cmd = cmd.args(["--target", target.as_str()]);
    }
    if matches!(
        profile,
        SecurityProfile::Hil | SecurityProfile::ReleaseHardened
    ) {
        cmd = cmd.arg("--release");
    }
    if !rustflags.is_empty() {
        cmd = cmd.env("CARGO_ENCODED_RUSTFLAGS", rustflags.join("\x1f"));
    }

    HardenedBuildPlan {
        profile,
        board_id: ctx.board.qualified_id(),
        target: ctx.board.rust_target.clone(),
        rustflags,
        command: cmd.to_string(),
    }
}

/// Inspect a compiled ELF binary for debug sections, symbol tables, host path strings, and panic markers.
pub fn audit_firmware_elf(path: &Path) -> Result<FirmwareAuditReport> {
    let bytes = fs::read(path)?;
    Ok(audit_firmware_bytes(&bytes))
}

/// Inspect raw firmware bytes for reverse-engineering exposure.
pub fn audit_firmware_bytes(bytes: &[u8]) -> FirmwareAuditReport {
    let is_elf = bytes.len() >= 4 && &bytes[0..4] == b"\x7fELF";
    let has_debug_sections = contains_subslice(bytes, b".debug_info")
        || contains_subslice(bytes, b".debug_line")
        || contains_subslice(bytes, b".debug_str");
    let has_symtab = contains_subslice(bytes, b".symtab");

    let mut leaked_host_paths = Vec::new();
    for marker in [
        "/home/",
        "/Users/",
        ".cargo/registry",
        ".rustup/toolchains",
        "\\Users\\",
    ] {
        if contains_subslice(bytes, marker.as_bytes()) {
            leaked_host_paths.push(marker.to_string());
        }
    }

    let has_panic_strings = contains_subslice(bytes, b"panicked at")
        || contains_subslice(bytes, b"core::panicking");

    let mut recommendations = Vec::new();
    if has_debug_sections {
        recommendations.push(
            "Strip DWARF debug sections (`-C strip=debuginfo` or `-C strip=symbols`).".to_string(),
        );
    }
    if has_symtab {
        recommendations.push(
            "Strip `.symtab` symbol table (`-C strip=symbols`) to hide function and variable names."
                .to_string(),
        );
    }
    if !leaked_host_paths.is_empty() {
        recommendations.push(
            "Scrub build host filesystem paths using `--remap-path-prefix=$HOME=/user`."
                .to_string(),
        );
    }
    if has_panic_strings {
        recommendations.push(
            "Use `-C panic=abort` and `panic_immediate_abort` (or `defmt`) to eliminate plaintext panic format strings."
                .to_string(),
        );
    }

    let hardened =
        is_elf && !has_debug_sections && !has_symtab && leaked_host_paths.is_empty() && !has_panic_strings;

    FirmwareAuditReport {
        file_size_bytes: bytes.len() as u64,
        is_elf,
        has_debug_sections,
        has_symtab,
        leaked_host_paths,
        has_panic_strings,
        hardened,
        recommendations,
    }
}

/// Generate dry-run chip read-out protection (RDP) and flash encryption plans.
///
/// Reversible protections (such as STM32 RDP Level 1) are separated from permanent
/// fuse operations (STM32 RDP Level 2, ESP32 eFuse burn), which are never auto-executed.
pub fn plan_chip_protection(board: &BoardRecord) -> ChipProtectionPlan {
    let platform = board.platform.as_str();
    let (reversible, irreversible) = if platform == "ststm32" {
        (
            vec![
                "STM32_Programmer_CLI -c port=SWD -ob RDP=0xBB (Enable RDP Level 1: blocks SWD flash readout; mass-erase recovers)".to_string(),
            ],
            vec![
                "STM32_Programmer_CLI -c port=SWD -ob RDP=0xCC (IRREVERSIBLE RDP Level 2: permanently disables SWD/JTAG)".to_string(),
            ],
        )
    } else if platform.starts_with("espressif") {
        (
            vec![
                "espsecure.py encrypt_flash_data --keyfile flash_key.bin --address 0x10000 -o firmware_enc.bin firmware.bin".to_string(),
            ],
            vec![
                "espefuse.py burn_key flash_encryption flash_key.bin (IRREVERSIBLE eFuse burn)".to_string(),
            ],
        )
    } else if platform == "nordicnrf52" {
        (
            vec![
                "nrfjprog --rbp ALL (Enable APPROTECT readback protection; `nrfjprog --recover` erases and unlocks)".to_string(),
            ],
            Vec::new(),
        )
    } else if platform == "atmelavr" {
        (
            vec![
                format!(
                    "avrdude -p {} -U lock:w:0xFC:m (Enable AVR flash read/write lock bits; chip erase recovers)",
                    board.mcu.to_ascii_lowercase()
                ),
            ],
            Vec::new(),
        )
    } else {
        (Vec::new(), Vec::new())
    };

    ChipProtectionPlan {
        board_id: board.qualified_id(),
        mcu: board.mcu.clone(),
        reversible_commands: reversible,
        irreversible_manual_commands: irreversible,
        warning: "Never enable RDP/APPROTECT during HIL/PID tuning; apply only after final release verification. Irreversible eFuse/RDP-L2 commands are dry-run only.".to_string(),
    }
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_detects_debug_symbols_paths_and_panics() {
        let leaky = b"\x7fELF\x01\x01\x01.symtab\0.debug_info\0/home/dev/.cargo/registry\0panicked at src/main.rs";
        let report = audit_firmware_bytes(leaky);
        assert!(report.is_elf);
        assert!(report.has_symtab);
        assert!(report.has_debug_sections);
        assert!(!report.leaked_host_paths.is_empty());
        assert!(report.has_panic_strings);
        assert!(!report.hardened);
        assert_eq!(report.recommendations.len(), 4);

        let clean = b"\x7fELF\x01\x01\x01.text\0.rodata\0";
        let clean_report = audit_firmware_bytes(clean);
        assert!(clean_report.hardened);
        assert!(clean_report.recommendations.is_empty());
    }
}
