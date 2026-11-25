use crate::config::{EnvConfig, ProjectMeta, RavomConfig};
use crate::{CoreError, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Parse a PlatformIO `platformio.ini` file into a RavomIO `RavomConfig`.
pub fn parse_platformio_ini(input: &str) -> RavomConfig {
    let mut default_envs: Option<Vec<String>> = None;
    let mut envs: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    let mut current_section: Option<String> = None;
    let mut current_key: Option<String> = None;

    for raw_line in input.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = Some(trimmed[1..trimmed.len() - 1].trim().to_string());
            current_key = None;
            continue;
        }

        let is_continuation = raw_line.starts_with(' ') || raw_line.starts_with('\t');
        if is_continuation {
            if let (Some(sec), Some(key)) = (&current_section, &current_key) {
                if let Some(env_name) = sec.strip_prefix("env:") {
                    envs.entry(env_name.to_string())
                        .or_default()
                        .entry(key.clone())
                        .or_default()
                        .push(trimmed.to_string());
                }
            }
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            current_key = Some(key.clone());

            match current_section.as_deref() {
                Some("platformio") if key == "default_envs" => {
                    let list: Vec<String> = val
                        .split([',', ' '])
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                    if !list.is_empty() {
                        default_envs = Some(list);
                    }
                }
                Some(sec) if sec.starts_with("env:") => {
                    let env_name = sec.trim_start_matches("env:").trim().to_string();
                    let entry = envs.entry(env_name).or_default().entry(key).or_default();
                    if !val.is_empty() {
                        entry.push(val.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let mut env_configs = BTreeMap::new();
    for (name, fields) in envs {
        let first = |k: &str| {
            fields
                .get(k)
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_default()
        };
        let monitor_speed = fields
            .get("monitor_speed")
            .and_then(|v| v.first())
            .and_then(|s| s.parse::<u32>().ok());
        let upload_protocol = fields
            .get("upload_protocol")
            .and_then(|v| v.first())
            .cloned();

        env_configs.insert(
            name,
            EnvConfig {
                platform: first("platform"),
                board: first("board"),
                framework: first("framework"),
                build_flags: fields.get("build_flags").cloned().unwrap_or_default(),
                lib_deps: fields.get("lib_deps").cloned().unwrap_or_default(),
                upload_protocol,
                monitor_speed,
            },
        );
    }

    RavomConfig {
        project: ProjectMeta {
            name: None,
            default_envs,
        },
        env: env_configs,
    }
}

/// Migrate `platformio.ini` in `dir` to `ravom.toml` and return the written path.
pub fn migrate_platformio_project(dir: &Path) -> Result<PathBuf> {
    let ini_path = dir.join("platformio.ini");
    if !ini_path.exists() {
        return Err(CoreError::PlatformIoIniNotFound(dir.to_path_buf()));
    }
    let raw = fs::read_to_string(&ini_path)?;
    let mut cfg = parse_platformio_ini(&raw);
    cfg.project.name = dir.file_name().and_then(|n| n.to_str()).map(str::to_string);

    let out_path = dir.join("ravom.toml");
    fs::write(&out_path, cfg.to_toml_string()?)?;
    Ok(out_path)
}

#[cfg(test)]
mod tests {
    use super::parse_platformio_ini;

    const SAMPLE_INI: &str = r#"
[platformio]
default_envs = bluepill

[env:bluepill]
platform = ststm32
board = bluepill_f103c8
framework = stm32cube
monitor_speed = 115200
upload_protocol = stlink
build_flags =
    -DDEBUG
    -O2
lib_deps =
    adafruit/Adafruit BusIO
"#;

    #[test]
    fn migrates_platformio_ini_to_ravom_config() {
        let cfg = parse_platformio_ini(SAMPLE_INI);
        assert_eq!(cfg.project.default_envs, Some(vec!["bluepill".to_string()]));
        let env = cfg.env.get("bluepill").expect("bluepill env should exist");
        assert_eq!(env.platform, "ststm32");
        assert_eq!(env.board, "bluepill_f103c8");
        assert_eq!(env.framework, "stm32cube");
        assert_eq!(env.monitor_speed, Some(115200));
        assert_eq!(env.upload_protocol.as_deref(), Some("stlink"));
        assert_eq!(env.build_flags, vec!["-DDEBUG", "-O2"]);
        assert_eq!(env.lib_deps, vec!["adafruit/Adafruit BusIO"]);
    }
}
