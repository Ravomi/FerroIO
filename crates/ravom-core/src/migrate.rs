//! `platformio.ini` → `ravom.toml` migration.
//!
//! Supports the PlatformIO features real projects rely on: the shared `[env]`
//! section, `extends`, `${section.option}` / `${this.option}` / `${sysenv.VAR}`
//! interpolation, multi-line and single-line multi-value options, inline `;`
//! comments and versioned/URL platform specs. Options without a RavomIO
//! equivalent are preserved under `extra` and reported as warnings.

use crate::config::{EnvConfig, ProjectMeta, RavomConfig};
use crate::registry::registry;
use crate::{CoreError, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

type Section = BTreeMap<String, String>;

const MAX_DEPTH: usize = 16;

/// Result of a migration: the generated config plus human-readable warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub config: RavomConfig,
    pub warnings: Vec<String>,
}

fn strip_inline_comment(value: &str) -> &str {
    // PlatformIO treats ` ;` (whitespace + semicolon) as an inline comment.
    let bytes = value.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i] == b';' && bytes[i - 1].is_ascii_whitespace() {
            return value[..i].trim_end();
        }
    }
    value
}

/// Parse INI text into ordered sections; continuation lines are joined with `\n`.
fn parse_ini(input: &str) -> BTreeMap<String, Section> {
    let mut sections: BTreeMap<String, Section> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut current_key: Option<String> = None;

    for raw_line in input.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        let is_continuation = raw_line.starts_with([' ', '\t']);
        if is_continuation {
            if let (Some(sec), Some(key)) = (&current, &current_key) {
                let value = strip_inline_comment(trimmed);
                let entry = sections
                    .entry(sec.clone())
                    .or_default()
                    .entry(key.clone())
                    .or_default();
                if !entry.is_empty() {
                    entry.push('\n');
                }
                entry.push_str(value);
            }
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let name = trimmed[1..trimmed.len() - 1].trim().to_string();
            sections.entry(name.clone()).or_default();
            current = Some(name);
            current_key = None;
            continue;
        }
        if let (Some(sec), Some((k, v))) = (&current, trimmed.split_once('=')) {
            let key = k.trim().to_ascii_lowercase();
            let value = strip_inline_comment(v.trim()).to_string();
            sections
                .entry(sec.clone())
                .or_default()
                .insert(key.clone(), value);
            current_key = Some(key);
        }
    }
    sections
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(['\n', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Shell-like split honouring single and double quotes (`-DNAME="a b"`).
fn split_flags(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in value.lines() {
        let mut cur = String::new();
        let mut quote: Option<char> = None;
        let mut has_token = false;
        for c in line.chars() {
            match (quote, c) {
                (Some(q), c) if c == q => {
                    quote = None;
                    cur.push(c);
                }
                (Some(_), c) => cur.push(c),
                (None, '"' | '\'') => {
                    quote = Some(c);
                    has_token = true;
                    cur.push(c);
                }
                (None, c) if c.is_whitespace() => {
                    if has_token || !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                        has_token = false;
                    }
                }
                (None, c) => {
                    has_token = true;
                    cur.push(c);
                }
            }
        }
        if has_token || !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}

/// Merge `[env]`, `extends` parents and `[env:<name>]` (later wins).
fn merged_env(
    sections: &BTreeMap<String, Section>,
    name: &str,
    depth: usize,
    warnings: &mut Vec<String>,
) -> Section {
    let mut merged = sections.get("env").cloned().unwrap_or_default();
    let Some(own) = sections.get(&format!("env:{name}")) else {
        return merged;
    };
    if let Some(parents) = own.get("extends") {
        for parent in split_list(parents) {
            if depth >= MAX_DEPTH {
                warnings.push(format!(
                    "env:{name}: `extends` nesting too deep, ignoring {parent}"
                ));
                continue;
            }
            if let Some(parent_env) = parent.strip_prefix("env:") {
                merged.extend(merged_env(sections, parent_env, depth + 1, warnings));
            } else if let Some(section) = sections.get(&parent) {
                merged.extend(section.clone());
            } else {
                warnings.push(format!("env:{name}: extends unknown section [{parent}]"));
            }
        }
    }
    merged.extend(own.iter().map(|(k, v)| (k.clone(), v.clone())));
    merged.remove("extends");
    merged
}

/// Expand `${section.option}`, `${this.option}` and `${sysenv.VAR}`.
fn interpolate(
    value: &str,
    this: &Section,
    sections: &BTreeMap<String, Section>,
    warnings: &mut Vec<String>,
) -> String {
    let mut out = value.to_string();
    for _ in 0..MAX_DEPTH {
        let Some(start) = out.find("${") else {
            break;
        };
        let Some(len) = out[start..].find('}') else {
            break;
        };
        let expr = out[start + 2..start + len].to_string();
        let replacement = match expr.split_once('.') {
            Some(("this", key)) => this.get(key).cloned(),
            Some(("sysenv", var)) => std::env::var(var).ok(),
            Some((section, key)) => sections.get(section).and_then(|s| s.get(key)).cloned(),
            None => None,
        };
        let replacement = replacement.unwrap_or_else(|| {
            warnings.push(format!("unresolved interpolation ${{{expr}}}"));
            String::new()
        });
        out.replace_range(start..start + len + 1, &replacement);
    }
    out
}

/// `espressif32@6.5.0`, `platformio/ststm32@^17`, git URLs → bare platform name.
fn normalize_platform(spec: &str) -> String {
    let spec = spec.trim();
    let without_ref = spec.split('#').next().unwrap_or(spec);
    let base = if without_ref.contains("://") || without_ref.ends_with(".git") {
        without_ref
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(spec)
            .trim_start_matches("platform-")
    } else {
        spec.split('@').next().unwrap_or(spec)
    };
    base.rsplit('/').next().unwrap_or(base).trim().to_string()
}

const MAPPED_KEYS: &[&str] = &[
    "platform",
    "board",
    "framework",
    "build_flags",
    "lib_deps",
    "upload_protocol",
    "monitor_speed",
];

/// Parse a PlatformIO `platformio.ini` file into a `RavomConfig` with migration warnings.
pub fn migrate_platformio_ini(input: &str) -> MigrationReport {
    let sections = parse_ini(input);
    let mut warnings = Vec::new();

    let default_envs = sections
        .get("platformio")
        .and_then(|s| s.get("default_envs"))
        .map(|v| {
            v.split([',', ' ', '\n'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());

    let mut env_configs = BTreeMap::new();
    for section_name in sections.keys() {
        let Some(name) = section_name.strip_prefix("env:") else {
            continue;
        };
        let name = name.trim();
        let merged = merged_env(&sections, name, 0, &mut warnings);
        let fields: Section = merged
            .iter()
            .map(|(k, v)| (k.clone(), interpolate(v, &merged, &sections, &mut warnings)))
            .collect();
        let get = |k: &str| {
            fields
                .get(k)
                .map(|v| v.trim().to_string())
                .unwrap_or_default()
        };

        let platform = normalize_platform(&get("platform"));
        let frameworks = split_list(&get("framework"));
        if frameworks.len() > 1 {
            warnings.push(format!(
                "env:{name}: multiple frameworks {frameworks:?}; using '{}'",
                frameworks[0]
            ));
        }
        let monitor_speed = fields.get("monitor_speed").and_then(|s| {
            let parsed = s.trim().parse::<u32>().ok();
            if parsed.is_none() {
                warnings.push(format!("env:{name}: invalid monitor_speed '{s}'"));
            }
            parsed
        });
        let upload_protocol = fields
            .get("upload_protocol")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(protocol) = &upload_protocol {
            if !matches!(
                protocol.as_str(),
                "stlink" | "jlink" | "cmsis-dap" | "probe-rs"
            ) {
                warnings.push(format!(
                    "env:{name}: upload_protocol '{protocol}' is not supported by `mio flash` yet"
                ));
            }
        }

        let board = get("board");
        if !board.is_empty() && registry().resolve(&board, Some(&platform)).is_err() {
            warnings.push(format!(
                "env:{name}: board '{board}' (platform '{platform}') is not in the RavomIO registry"
            ));
        }

        let extra: BTreeMap<String, String> = fields
            .iter()
            .filter(|(k, _)| !MAPPED_KEYS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for key in extra.keys() {
            warnings.push(format!(
                "env:{name}: option '{key}' has no RavomIO equivalent; kept under `extra`"
            ));
        }

        env_configs.insert(
            name.to_string(),
            EnvConfig {
                platform,
                board,
                framework: frameworks.first().cloned().unwrap_or_default(),
                build_flags: split_flags(&get("build_flags")),
                rustflags: Vec::new(),
                lib_deps: split_list(&get("lib_deps")),
                upload_protocol,
                monitor_speed,
                zephyr: None,
                extra,
            },
        );
    }

    if let Some(defaults) = &default_envs {
        for env in defaults {
            if !env_configs.contains_key(env) {
                warnings.push(format!("default_envs references unknown env '{env}'"));
            }
        }
    }

    MigrationReport {
        config: RavomConfig {
            project: ProjectMeta {
                name: None,
                default_envs,
            },
            env: env_configs,
        },
        warnings,
    }
}

/// Parse a PlatformIO `platformio.ini` file into a RavomIO `RavomConfig`.
pub fn parse_platformio_ini(input: &str) -> RavomConfig {
    migrate_platformio_ini(input).config
}

/// Migrate `platformio.ini` in `dir` to `ravom.toml`; returns the written path and warnings.
pub fn migrate_platformio_project(dir: &Path) -> Result<(PathBuf, Vec<String>)> {
    let ini_path = dir.join("platformio.ini");
    if !ini_path.exists() {
        return Err(CoreError::PlatformIoIniNotFound(dir.to_path_buf()));
    }
    let raw = fs::read_to_string(&ini_path)?;
    let mut report = migrate_platformio_ini(&raw);
    let name = dir
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string));
    report.config.project.name = name;

    let out_path = dir.join("ravom.toml");
    if out_path.exists() {
        return Err(CoreError::AlreadyInitialized(out_path));
    }
    fs::write(&out_path, report.config.to_toml_string()?)?;
    Ok((out_path, report.warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    const ADVANCED_INI: &str = r#"
; shared settings
[platformio]
default_envs = release, missing

[common]
flags = -DCOMMON=1 -DNAME="hello world"
libs =
    bblanchon/ArduinoJson @ ^7
    knolleary/PubSubClient

[env]
framework = arduino
monitor_speed = 921600 ; fast

[env:base]
platform = espressif32@6.5.0
board = esp32dev
build_flags = ${common.flags} -Os
lib_deps = ${common.libs}
board_build.partitions = huge_app.csv

[env:release]
extends = env:base
build_flags = ${env:base.build_flags} -DRELEASE
upload_protocol = esptool

[env:git]
platform = https://github.com/platformio/platform-ststm32.git#v17.0.0
board = ${this.board_alias}
board_alias = bluepill_f103c8
framework = arduino, cmsis
"#;

    #[test]
    fn supports_env_inheritance_interpolation_and_extra_options() {
        let report = migrate_platformio_ini(ADVANCED_INI);
        let cfg = &report.config;
        let release = cfg.env.get("release").expect("release env");
        assert_eq!(release.platform, "espressif32");
        assert_eq!(release.board, "esp32dev");
        assert_eq!(release.framework, "arduino", "inherited from [env]");
        assert_eq!(
            release.monitor_speed,
            Some(921600),
            "inline comment stripped"
        );
        assert_eq!(
            release.build_flags,
            vec!["-DCOMMON=1", "-DNAME=\"hello world\"", "-Os", "-DRELEASE"]
        );
        assert_eq!(
            release.lib_deps,
            vec!["bblanchon/ArduinoJson @ ^7", "knolleary/PubSubClient"]
        );
        assert_eq!(
            release
                .extra
                .get("board_build.partitions")
                .map(String::as_str),
            Some("huge_app.csv")
        );

        let git = cfg.env.get("git").expect("git env");
        assert_eq!(git.platform, "ststm32");
        assert_eq!(git.board, "bluepill_f103c8");
        assert_eq!(git.framework, "arduino");

        let joined = report.warnings.join("\n");
        assert!(joined.contains("esptool"), "{joined}");
        assert!(joined.contains("multiple frameworks"), "{joined}");
        assert!(joined.contains("unknown env 'missing'"), "{joined}");
        assert!(joined.contains("board_build.partitions"), "{joined}");
    }

    #[test]
    fn splits_flags_respecting_quotes() {
        assert_eq!(
            split_flags("-DA=1 -DB='x y'\n  -Wall"),
            vec!["-DA=1", "-DB='x y'", "-Wall"]
        );
        assert_eq!(normalize_platform("platformio/ststm32@^17.0.0"), "ststm32");
        assert_eq!(normalize_platform("native"), "native");
    }

    #[test]
    fn migrate_project_writes_manifest_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("platformio.ini"), SAMPLE_INI).expect("write ini");
        let (out, warnings) = migrate_platformio_project(dir.path()).expect("migrate");
        assert!(warnings.is_empty(), "{warnings:?}");
        let cfg = RavomConfig::load_from_dir(dir.path()).expect("written manifest parses");
        assert!(cfg.env.contains_key("bluepill"));
        assert!(out.ends_with("ravom.toml"));
        assert!(matches!(
            migrate_platformio_project(dir.path()),
            Err(CoreError::AlreadyInitialized(_))
        ));
    }
}
