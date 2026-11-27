//! Board registry: boards embedded at build time from `boards/`, overlaid by
//! user boards in `~/.ravom/boards` and `$RAVOM_BOARDS_DIR`.

use crate::chips::{is_known_probe_rs_target, resolve_probe_rs_target};
use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Environment variable pointing at an extra board directory (`<platform>/<id>.toml`).
pub const BOARDS_DIR_ENV: &str = "RAVOM_BOARDS_DIR";

/// Rust target triples that board definitions may reference.
pub const KNOWN_RUST_TARGETS: &[&str] = &[
    "thumbv6m-none-eabi",
    "thumbv7m-none-eabi",
    "thumbv7em-none-eabi",
    "thumbv7em-none-eabihf",
    "thumbv8m.base-none-eabi",
    "thumbv8m.main-none-eabi",
    "thumbv8m.main-none-eabihf",
    "riscv32i-unknown-none-elf",
    "riscv32imc-unknown-none-elf",
    "riscv32imac-unknown-none-elf",
    "riscv32imafc-unknown-none-elf",
    "riscv64gc-unknown-none-elf",
    "riscv64gc-unknown-linux-gnu",
    "riscv64gc-unknown-linux-musl",
    "xtensa-esp32-none-elf",
    "xtensa-esp32s2-none-elf",
    "xtensa-esp32s3-none-elf",
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "armv7-unknown-linux-gnueabihf",
    "armv7-unknown-linux-musleabihf",
];

/// A board TOML file compiled into the binary by `build.rs`.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedBoard {
    pub platform: &'static str,
    pub stem: &'static str,
    pub toml: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/embedded_boards.rs"));

/// Unified owned board metadata (`boards/<platform>/<id>.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardRecord {
    pub id: String,
    pub name: String,
    pub vendor: Option<String>,
    pub platform: String,
    pub mcu: String,
    pub cpu: Option<String>,
    pub default_framework: String,
    #[serde(default)]
    pub frameworks: Vec<String>,
    pub probe_rs_chip: Option<String>,
    pub rust_target: Option<String>,
    /// Optional Zephyr HWMv2 board target (e.g. `nrf52840dk/nrf52840`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zephyr_board: Option<String>,
    pub flash_bytes: Option<u64>,
    pub ram_bytes: Option<u64>,
}

impl BoardRecord {
    /// Fully qualified `<platform>/<id>` name, unique across the registry.
    pub fn qualified_id(&self) -> String {
        format!("{}/{}", self.platform, self.id)
    }

    /// Semantic checks for a single record against its file location.
    pub fn validate(
        &self,
        expected_platform: Option<&str>,
        expected_id: Option<&str>,
    ) -> Vec<String> {
        let mut problems = Vec::new();
        if let Some(stem) = expected_id {
            if self.id != stem {
                problems.push(format!(
                    "id '{}' does not match file name '{stem}'",
                    self.id
                ));
            }
        }
        if let Some(platform) = expected_platform {
            if self.platform != platform {
                problems.push(format!(
                    "platform '{}' does not match directory '{platform}'",
                    self.platform
                ));
            }
        }
        if self.name.trim().is_empty() {
            problems.push("name is empty".to_string());
        }
        if self.mcu.trim().is_empty() {
            problems.push("mcu is empty".to_string());
        }
        if !self.frameworks.is_empty() && !self.frameworks.contains(&self.default_framework) {
            problems.push(format!(
                "default_framework '{}' is not listed in frameworks {:?}",
                self.default_framework, self.frameworks
            ));
        }
        if let Some(target) = &self.rust_target {
            if !KNOWN_RUST_TARGETS.contains(&target.as_str()) {
                problems.push(format!("unknown rust_target '{target}'"));
            }
        }
        if let Some(chip) = &self.probe_rs_chip {
            if !is_known_probe_rs_target(chip) {
                problems.push(format!("probe_rs_chip '{chip}' is not a probe-rs target"));
            }
        }
        if self.flash_bytes == Some(0) {
            problems.push("flash_bytes is 0".to_string());
        }
        if self.ram_bytes == Some(0) {
            problems.push("ram_bytes is 0".to_string());
        }
        problems
    }
}

/// Where a registry problem was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryIssue {
    pub location: String,
    pub message: String,
}

impl fmt::Display for RegistryIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

/// Subset of PlatformIO `boards/<id>.json` schema.
#[derive(Debug, Deserialize)]
struct PioBoardManifest {
    name: String,
    vendor: Option<String>,
    #[serde(default)]
    frameworks: Vec<String>,
    #[serde(default)]
    build: PioBuildSection,
    #[serde(default)]
    debug: PioDebugSection,
    #[serde(default)]
    upload: PioUploadSection,
}

#[derive(Debug, Default, Deserialize)]
struct PioBuildSection {
    mcu: Option<String>,
    cpu: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct PioDebugSection {
    jlink_device: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct PioUploadSection {
    maximum_size: Option<u64>,
    maximum_ram_size: Option<u64>,
}

/// Infer a Rust `--target` triple from PlatformIO's `build.cpu` and `build.mcu`.
pub fn infer_rust_target(cpu: Option<&str>, mcu: &str) -> Option<String> {
    let mcu_lower = mcu.to_ascii_lowercase();
    if mcu_lower.starts_with("esp32s3") {
        return Some("xtensa-esp32s3-none-elf".to_string());
    }
    if mcu_lower.starts_with("esp32s2") {
        return Some("xtensa-esp32s2-none-elf".to_string());
    }
    if mcu_lower.starts_with("esp32c3") || mcu_lower.starts_with("esp32c2") {
        return Some("riscv32imc-unknown-none-elf".to_string());
    }
    if mcu_lower.starts_with("esp32c6") || mcu_lower.starts_with("esp32h2") {
        return Some("riscv32imac-unknown-none-elf".to_string());
    }
    if mcu_lower.starts_with("esp32") {
        return Some("xtensa-esp32-none-elf".to_string());
    }

    let cpu_lower = cpu.unwrap_or_default().to_ascii_lowercase();
    match cpu_lower.as_str() {
        "cortex-m0" | "cortex-m0plus" | "cortex-m0+" => Some("thumbv6m-none-eabi".to_string()),
        "cortex-m3" => Some("thumbv7m-none-eabi".to_string()),
        "cortex-m4" | "cortex-m7" => Some("thumbv7em-none-eabihf".to_string()),
        "cortex-m33" => Some("thumbv8m.main-none-eabihf".to_string()),
        _ => None,
    }
}

/// Parse a PlatformIO `boards/<id>.json` file into a `BoardRecord`.
pub fn parse_pio_board_json(id: &str, platform: &str, json: &str) -> Result<BoardRecord> {
    let manifest: PioBoardManifest = serde_json::from_str(json)?;
    let mcu = manifest
        .build
        .mcu
        .clone()
        .unwrap_or_else(|| id.to_ascii_uppercase());
    let probe_rs_chip = resolve_probe_rs_target(
        manifest
            .debug
            .jlink_device
            .as_deref()
            .into_iter()
            .chain([mcu.as_str()]),
    );
    let rust_target = infer_rust_target(manifest.build.cpu.as_deref(), &mcu);
    let default_framework = manifest
        .frameworks
        .first()
        .cloned()
        .unwrap_or_else(|| "embassy".to_string());

    Ok(BoardRecord {
        id: id.to_string(),
        name: manifest.name,
        vendor: manifest.vendor,
        platform: platform.to_string(),
        mcu: mcu.to_ascii_uppercase(),
        cpu: manifest.build.cpu,
        default_framework,
        frameworks: manifest.frameworks,
        probe_rs_chip,
        rust_target,
        zephyr_board: crate::zephyr::known_zephyr_board(id).map(str::to_string),
        flash_bytes: manifest.upload.maximum_size,
        ram_bytes: manifest.upload.maximum_ram_size,
    })
}

/// Default local board registry directory (`~/.ravom/boards`).
pub fn default_boards_dir() -> PathBuf {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    match home {
        Some(home) => PathBuf::from(home).join(".ravom").join("boards"),
        None => PathBuf::from(".ravom").join("boards"),
    }
}

/// Import all valid PlatformIO `*.json` board definitions from `src_dir` into `dest_dir/<platform>/*.toml`.
pub fn import_pio_boards_dir(src_dir: &Path, platform: &str, dest_dir: &Path) -> Result<usize> {
    let target_dir = dest_dir.join(platform);
    fs::create_dir_all(&target_dir)?;

    let mut imported = 0usize;
    let mut paths: Vec<PathBuf> = fs::read_dir(src_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    paths.sort();
    for path in paths {
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let raw = fs::read_to_string(&path)?;
        if let Ok(record) = parse_pio_board_json(stem, platform, &raw) {
            let toml_text = toml::to_string_pretty(&record)?;
            fs::write(target_dir.join(format!("{stem}.toml")), toml_text)?;
            imported += 1;
        }
    }
    Ok(imported)
}

/// A board whose `probe_rs_chip` was rewritten by [`normalize_probe_rs_chips`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipChange {
    pub board: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// Re-resolve `probe_rs_chip` for every `<platform>/<id>.toml` under `root`
/// against the bundled `probe-rs` snapshot (and turn `0` memory sizes, which
/// PlatformIO uses for "unknown", into absent values), rewriting changed files.
pub fn normalize_probe_rs_chips(root: &Path) -> Result<Vec<ChipChange>> {
    let mut changes = Vec::new();
    let mut files = Vec::new();
    for platform in fs::read_dir(root)?.flatten() {
        if platform.path().is_dir() {
            for file in fs::read_dir(platform.path())?.flatten() {
                if file.path().extension().and_then(|e| e.to_str()) == Some("toml") {
                    files.push(file.path());
                }
            }
        }
    }
    files.sort();
    for path in files {
        let raw = fs::read_to_string(&path)?;
        let mut record: BoardRecord = toml::from_str(&raw)?;
        let candidates = record
            .probe_rs_chip
            .as_deref()
            .into_iter()
            .chain([record.mcu.as_str()]);
        let mut resolved = resolve_probe_rs_target(candidates);
        if let (Some(old), Some(new)) = (&record.probe_rs_chip, &resolved) {
            if old.eq_ignore_ascii_case(new) {
                // Same target, keep the existing spelling to avoid churn.
                resolved = Some(old.clone());
            }
        }
        let zero_sizes = record.flash_bytes == Some(0) || record.ram_bytes == Some(0);
        if resolved != record.probe_rs_chip || zero_sizes {
            changes.push(ChipChange {
                board: record.qualified_id(),
                old: record.probe_rs_chip.clone(),
                new: resolved.clone(),
            });
            record.probe_rs_chip = resolved;
            record.flash_bytes = record.flash_bytes.filter(|&b| b > 0);
            record.ram_bytes = record.ram_bytes.filter(|&b| b > 0);
            fs::write(&path, toml::to_string_pretty(&record)?)?;
        }
    }
    Ok(changes)
}

/// In-memory board registry keyed by `(platform, id)`.
#[derive(Debug, Default, Clone)]
pub struct BoardRegistry {
    boards: BTreeMap<(String, String), BoardRecord>,
    issues: Vec<RegistryIssue>,
}

impl BoardRegistry {
    /// Registry containing only the boards compiled into this binary.
    pub fn embedded() -> Self {
        let mut reg = Self::default();
        for board in EMBEDDED_BOARDS {
            let location = format!("boards/{}/{}.toml", board.platform, board.stem);
            match toml::from_str::<BoardRecord>(board.toml) {
                Ok(record) => reg.insert(record, Some(board.platform), Some(board.stem), &location),
                Err(err) => reg.issues.push(RegistryIssue {
                    location,
                    message: err.to_string().trim().to_string(),
                }),
            }
        }
        reg
    }

    /// Embedded boards overlaid with `~/.ravom/boards` and `$RAVOM_BOARDS_DIR`.
    pub fn load_default() -> Self {
        let mut reg = Self::embedded();
        reg.overlay_dir(&default_boards_dir());
        if let Some(extra) = std::env::var_os(BOARDS_DIR_ENV) {
            reg.overlay_dir(Path::new(&extra));
        }
        reg
    }

    /// Load `<root>/<platform>/<id>.{toml,json}` on top of the current boards.
    pub fn overlay_dir(&mut self, root: &Path) {
        let Ok(platforms) = fs::read_dir(root) else {
            return;
        };
        let mut platform_dirs: Vec<PathBuf> = platforms
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        platform_dirs.sort();
        for platform_path in platform_dirs {
            let Some(platform) = platform_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(files) = fs::read_dir(&platform_path) else {
                continue;
            };
            let mut files: Vec<PathBuf> = files.flatten().map(|e| e.path()).collect();
            files.sort();
            for file in files {
                let ext = file.extension().and_then(|e| e.to_str());
                if !matches!(ext, Some("toml" | "json")) {
                    continue;
                }
                let Some(stem) = file.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let location = file.display().to_string();
                let parsed = fs::read_to_string(&file)
                    .map_err(CoreError::from)
                    .and_then(|raw| match ext {
                        Some("json") => parse_pio_board_json(stem, platform, &raw),
                        _ => toml::from_str::<BoardRecord>(&raw).map_err(CoreError::from),
                    });
                match parsed {
                    Ok(record) => self.insert(record, Some(platform), Some(stem), &location),
                    Err(err) => self.issues.push(RegistryIssue {
                        location,
                        message: err.to_string().trim().to_string(),
                    }),
                }
            }
        }
    }

    fn insert(
        &mut self,
        record: BoardRecord,
        platform: Option<&str>,
        stem: Option<&str>,
        location: &str,
    ) {
        for message in record.validate(platform, stem) {
            self.issues.push(RegistryIssue {
                location: location.to_string(),
                message,
            });
        }
        let key = (
            record.platform.to_ascii_lowercase(),
            record.id.to_ascii_lowercase(),
        );
        self.boards.insert(key, record);
    }

    /// All boards ordered by `(platform, id)`.
    pub fn boards(&self) -> impl Iterator<Item = &BoardRecord> {
        self.boards.values()
    }

    pub fn len(&self) -> usize {
        self.boards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.boards.is_empty()
    }

    /// Parse and validation problems found while loading.
    pub fn issues(&self) -> &[RegistryIssue] {
        &self.issues
    }

    /// Every board whose id matches (case-insensitively), across platforms.
    pub fn matches(&self, id: &str) -> Vec<&BoardRecord> {
        self.boards
            .values()
            .filter(|b| b.id.eq_ignore_ascii_case(id))
            .collect()
    }

    /// Resolve `id` or `platform/id`, optionally constrained to `platform`.
    pub fn resolve(&self, id: &str, platform: Option<&str>) -> Result<&BoardRecord> {
        let (platform, id) = match id.split_once('/') {
            Some((p, i)) => (Some(p), i),
            None => (platform.filter(|p| !p.trim().is_empty()), id),
        };
        if let Some(platform) = platform {
            if let Some(board) = self
                .boards
                .get(&(platform.to_ascii_lowercase(), id.to_ascii_lowercase()))
            {
                return Ok(board);
            }
        }
        match self.matches(id).as_slice() {
            [] => Err(CoreError::UnknownBoard(id.to_string())),
            [only] => Ok(only),
            many => Err(CoreError::AmbiguousBoard {
                id: id.to_string(),
                candidates: many.iter().map(|b| b.qualified_id()).collect(),
            }),
        }
    }
}

/// Process-wide registry (loaded once). Unit tests use only the embedded
/// boards so results never depend on the developer's `~/.ravom/boards`.
pub fn registry() -> &'static BoardRegistry {
    static REGISTRY: OnceLock<BoardRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        if cfg!(test) {
            BoardRegistry::embedded()
        } else {
            BoardRegistry::load_default()
        }
    })
}

/// Load all boards from the process-wide registry.
pub fn load_all_boards() -> Vec<BoardRecord> {
    registry().boards().cloned().collect()
}

/// Look up a board by `id` or `platform/id`; `None` if unknown or ambiguous.
pub fn find_board_record(id: &str) -> Option<BoardRecord> {
    registry().resolve(id, None).ok().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_STM32_JSON: &str = r#"{
        "build": {
            "cpu": "cortex-m4",
            "mcu": "stm32f411ceu6"
        },
        "debug": {
            "jlink_device": "STM32F411CE"
        },
        "frameworks": ["arduino", "stm32cube"],
        "name": "WeAct Studio BlackPill V2.0 (STM32F411CE)",
        "upload": {
            "maximum_ram_size": 131072,
            "maximum_size": 524288
        },
        "vendor": "WeAct Studio"
    }"#;

    #[test]
    fn parses_platformio_board_json_and_infers_target() {
        let board = parse_pio_board_json("blackpill_f411ce", "ststm32", SAMPLE_STM32_JSON)
            .expect("valid PlatformIO board JSON should parse");
        assert_eq!(board.id, "blackpill_f411ce");
        assert_eq!(board.mcu, "STM32F411CEU6");
        let chip = board.probe_rs_chip.as_deref().expect("probe-rs chip");
        assert!(is_known_probe_rs_target(chip), "{chip}");
        assert!(chip.to_ascii_uppercase().starts_with("STM32F411CE"));
        assert_eq!(board.rust_target.as_deref(), Some("thumbv7em-none-eabihf"));
        assert_eq!(board.flash_bytes, Some(524288));
        assert_eq!(board.ram_bytes, Some(131072));
    }

    #[test]
    fn imports_directory_of_platformio_boards() {
        let src = tempfile::tempdir().expect("temp src");
        let dst = tempfile::tempdir().expect("temp dst");
        fs::write(src.path().join("blackpill_f411ce.json"), SAMPLE_STM32_JSON)
            .expect("write sample board json");
        fs::write(src.path().join("broken.json"), "{ not json").expect("write broken json");

        let count = import_pio_boards_dir(src.path(), "ststm32", dst.path())
            .expect("import_pio_boards_dir should succeed");
        assert_eq!(count, 1);
        let toml_path = dst.path().join("ststm32/blackpill_f411ce.toml");
        let raw_toml = fs::read_to_string(&toml_path).expect("read generated board toml");
        let parsed: BoardRecord = toml::from_str(&raw_toml).expect("parse generated board toml");
        assert_eq!(parsed.id, "blackpill_f411ce");
        assert!(parsed
            .validate(Some("ststm32"), Some("blackpill_f411ce"))
            .is_empty());
    }

    #[test]
    fn overlay_directory_overrides_embedded_and_reports_bad_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let platform_dir = dir.path().join("ststm32");
        fs::create_dir_all(&platform_dir).expect("mkdir");
        fs::write(
            platform_dir.join("bluepill_f103c8.toml"),
            "id = \"bluepill_f103c8\"\nname = \"Custom BluePill\"\nplatform = \"ststm32\"\n\
             mcu = \"STM32F103C8T6\"\ndefault_framework = \"embassy\"\n",
        )
        .expect("write override");
        fs::write(
            platform_dir.join("typo.toml"),
            "id = \"typo\"\nnmae = \"x\"\n",
        )
        .expect("write typo board");

        let mut reg = BoardRegistry::embedded();
        let before = reg.len();
        reg.overlay_dir(dir.path());
        assert_eq!(reg.len(), before, "override must replace, not add");
        let board = reg
            .resolve("bluepill_f103c8", None)
            .expect("override resolves");
        assert_eq!(board.name, "Custom BluePill");
        assert!(reg
            .issues()
            .iter()
            .any(|i| i.location.ends_with("typo.toml")));
    }

    #[test]
    fn ambiguous_ids_require_platform_qualification() {
        let reg = BoardRegistry::embedded();
        let err = reg.resolve("pico", None).expect_err("pico is ambiguous");
        assert!(
            matches!(err, CoreError::AmbiguousBoard { ref candidates, .. } if candidates.len() >= 2)
        );
        assert_eq!(
            reg.resolve("raspberrypi/pico", None)
                .expect("qualified")
                .platform,
            "raspberrypi"
        );
        assert_eq!(
            reg.resolve("pico", Some("raspberrypi"))
                .expect("platform hint")
                .platform,
            "raspberrypi"
        );
    }
}
