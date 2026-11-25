use crate::config::builtin_boards;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Unified owned board metadata (from built-in list or imported PlatformIO `boards/*.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardRecord {
    pub id: String,
    pub name: String,
    pub vendor: Option<String>,
    pub platform: String,
    pub mcu: String,
    pub cpu: Option<String>,
    pub default_framework: String,
    pub frameworks: Vec<String>,
    pub probe_rs_chip: Option<String>,
    pub rust_target: Option<String>,
    pub flash_bytes: Option<u64>,
    pub ram_bytes: Option<u64>,
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
    let probe_rs_chip = manifest
        .debug
        .jlink_device
        .or_else(|| Some(mcu.to_ascii_uppercase()));
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
        flash_bytes: manifest.upload.maximum_size,
        ram_bytes: manifest.upload.maximum_ram_size,
    })
}

/// Default local board registry directory (`~/.ravom/boards`).
pub fn default_boards_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".ravom").join("boards");
    }
    PathBuf::from(".ravom").join("boards")
}

/// Import all valid PlatformIO `*.json` board definitions from `src_dir` into `dest_dir/<platform>/*.toml`.
pub fn import_pio_boards_dir(src_dir: &Path, platform: &str, dest_dir: &Path) -> Result<usize> {
    let target_dir = dest_dir.join(platform);
    fs::create_dir_all(&target_dir)?;

    let mut imported = 0usize;
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
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

fn load_boards_from_root(root: &Path, map: &mut BTreeMap<String, BoardRecord>) {
    let Ok(platforms) = fs::read_dir(root) else {
        return;
    };
    for platform_entry in platforms.flatten() {
        let platform_path = platform_entry.path();
        if !platform_path.is_dir() {
            continue;
        }
        let Some(platform_name) = platform_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Ok(files) = fs::read_dir(&platform_path) else {
            continue;
        };
        for file_entry in files.flatten() {
            let file_path = file_entry.path();
            let ext = file_path.extension().and_then(|e| e.to_str());
            let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(raw) = fs::read_to_string(&file_path) else {
                continue;
            };
            let parsed = match ext {
                Some("toml") => toml::from_str::<BoardRecord>(&raw).ok(),
                Some("json") => parse_pio_board_json(stem, platform_name, &raw).ok(),
                _ => None,
            };
            if let Some(record) = parsed {
                map.insert(record.id.to_ascii_lowercase(), record);
            }
        }
    }
}

/// Load all boards: built-in fallback boards merged with imported PlatformIO boards.
pub fn load_all_boards() -> Vec<BoardRecord> {
    let mut map = BTreeMap::new();
    for b in builtin_boards() {
        map.insert(
            b.id.to_ascii_lowercase(),
            BoardRecord {
                id: b.id.to_string(),
                name: b.name.to_string(),
                vendor: None,
                platform: b.platform.to_string(),
                mcu: b.mcu.to_string(),
                cpu: None,
                default_framework: b.default_framework.to_string(),
                frameworks: vec![b.default_framework.to_string()],
                probe_rs_chip: b.probe_rs_chip.map(str::to_string),
                rust_target: b.rust_target.map(str::to_string),
                flash_bytes: None,
                ram_bytes: None,
            },
        );
    }

    load_boards_from_root(&default_boards_dir(), &mut map);
    load_boards_from_root(Path::new("boards"), &mut map);
    map.into_values().collect()
}

/// Look up a board by ID across built-in and imported PlatformIO boards.
pub fn find_board_record(id: &str) -> Option<BoardRecord> {
    load_all_boards()
        .into_iter()
        .find(|b| b.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        assert_eq!(board.probe_rs_chip.as_deref(), Some("STM32F411CE"));
        assert_eq!(board.rust_target.as_deref(), Some("thumbv7em-none-eabihf"));
        assert_eq!(board.flash_bytes, Some(524288));
        assert_eq!(board.ram_bytes, Some(131072));
    }

    #[test]
    fn imports_directory_of_platformio_boards() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH")
            .as_nanos();
        let src = std::env::temp_dir().join(format!("ravomio_pio_src_{nanos}"));
        let dst = std::env::temp_dir().join(format!("ravomio_pio_dst_{nanos}"));
        fs::create_dir_all(&src).expect("create temp src dir");
        fs::write(src.join("blackpill_f411ce.json"), SAMPLE_STM32_JSON)
            .expect("write sample board json");

        let count = import_pio_boards_dir(&src, "ststm32", &dst)
            .expect("import_pio_boards_dir should succeed");
        assert_eq!(count, 1);
        let toml_path = dst.join("ststm32/blackpill_f411ce.toml");
        assert!(toml_path.exists());
        let raw_toml = fs::read_to_string(&toml_path).expect("read generated board toml");
        let parsed: BoardRecord = toml::from_str(&raw_toml).expect("parse generated board toml");
        assert_eq!(parsed.id, "blackpill_f411ce");
        assert_eq!(parsed.rust_target.as_deref(), Some("thumbv7em-none-eabihf"));

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dst);
    }
}
