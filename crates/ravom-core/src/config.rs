use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level `ravom.toml` project manifest (inspired by `platformio.ini`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RavomConfig {
    #[serde(default)]
    pub project: ProjectMeta,
    #[serde(default)]
    pub env: BTreeMap<String, EnvConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectMeta {
    pub name: Option<String>,
    pub default_envs: Option<Vec<String>>,
}

/// Environment configuration corresponding to `[env:<name>]` in PlatformIO.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvConfig {
    pub platform: String,
    pub board: String,
    pub framework: String,
    #[serde(default)]
    pub build_flags: Vec<String>,
    #[serde(default)]
    pub lib_deps: Vec<String>,
    pub upload_protocol: Option<String>,
    pub monitor_speed: Option<u32>,
}

impl RavomConfig {
    pub fn from_toml_str(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn load_from_dir(dir: &std::path::Path) -> crate::Result<Self> {
        let path = dir.join("ravom.toml");
        if !path.exists() {
            return Err(crate::CoreError::ManifestNotFound(dir.to_path_buf()));
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(Self::from_toml_str(&content)?)
    }

    pub fn resolve_env(&self, env_name: Option<&str>) -> crate::Result<(&str, &EnvConfig)> {
        if let Some(name) = env_name {
            return self
                .env
                .get_key_value(name)
                .map(|(k, v)| (k.as_str(), v))
                .ok_or_else(|| crate::CoreError::UnknownEnv(name.to_string()));
        }
        if let Some(defaults) = &self.project.default_envs {
            if let Some(first) = defaults.first() {
                if let Some((k, v)) = self.env.get_key_value(first) {
                    return Ok((k.as_str(), v));
                }
            }
        }
        self.env
            .iter()
            .next()
            .map(|(k, v)| (k.as_str(), v))
            .ok_or(crate::CoreError::NoEnvironments)
    }
}

/// Built-in board metadata (subset for Phase 1 scaffolding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub platform: &'static str,
    pub mcu: &'static str,
    pub default_framework: &'static str,
    pub probe_rs_chip: Option<&'static str>,
    pub rust_target: Option<&'static str>,
}

pub fn builtin_boards() -> &'static [BoardInfo] {
    &[
        BoardInfo {
            id: "stm32f401ccu6",
            name: "WeAct BlackPill V3.0 (STM32F401CCU6)",
            platform: "ststm32",
            mcu: "STM32F401CCU6",
            default_framework: "embassy",
            probe_rs_chip: Some("STM32F401CCUx"),
            rust_target: Some("thumbv7em-none-eabihf"),
        },
        BoardInfo {
            id: "esp32s3",
            name: "Espressif ESP32-S3-DevKitC-1",
            platform: "espressif32",
            mcu: "ESP32S3",
            default_framework: "esp-idf",
            probe_rs_chip: Some("ESP32S3"),
            rust_target: Some("xtensa-esp32s3-none-elf"),
        },
        BoardInfo {
            id: "rp2040",
            name: "Raspberry Pi Pico (RP2040)",
            platform: "raspberrypi",
            mcu: "RP2040",
            default_framework: "embassy",
            probe_rs_chip: Some("RP2040"),
            rust_target: Some("thumbv6m-none-eabi"),
        },
    ]
}

pub fn find_board(id: &str) -> Option<&'static BoardInfo> {
    builtin_boards()
        .iter()
        .find(|b| b.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ravom_toml() {
        let mut env = BTreeMap::new();
        env.insert(
            "blackpill".to_string(),
            EnvConfig {
                platform: "ststm32".to_string(),
                board: "stm32f401ccu6".to_string(),
                framework: "embassy".to_string(),
                build_flags: vec!["-DDEBUG".to_string()],
                lib_deps: vec![],
                upload_protocol: Some("probe-rs".to_string()),
                monitor_speed: Some(115200),
            },
        );
        let cfg = RavomConfig {
            project: ProjectMeta {
                name: Some("blink".to_string()),
                default_envs: Some(vec!["blackpill".to_string()]),
            },
            env,
        };
        let s = cfg
            .to_toml_string()
            .expect("RavomConfig should serialize to TOML");
        let parsed = RavomConfig::from_toml_str(&s)
            .expect("serialized RavomConfig should deserialize cleanly");
        assert_eq!(cfg, parsed);
    }
}
