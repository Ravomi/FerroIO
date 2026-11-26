use crate::zephyr::ZephyrConfig;
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvConfig {
    pub platform: String,
    pub board: String,
    pub framework: String,
    /// C/C++ compiler flags (PlatformIO `build_flags`); not passed to rustc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub build_flags: Vec<String>,
    /// Extra rustc flags for the Cargo backend (`CARGO_ENCODED_RUSTFLAGS`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rustflags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lib_deps: Vec<String>,
    pub upload_protocol: Option<String>,
    pub monitor_speed: Option<u32>,
    /// Optional Zephyr RTOS configuration (`[env.<name>.zephyr]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zephyr: Option<ZephyrConfig>,
    /// PlatformIO options without a RavomIO equivalent, preserved verbatim by `mio migrate`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
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
                upload_protocol: Some("probe-rs".to_string()),
                monitor_speed: Some(115200),
                ..EnvConfig::default()
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

    #[test]
    fn resolves_default_and_named_envs() {
        let cfg = RavomConfig::from_toml_str(
            "[project]\ndefault_envs = [\"b\"]\n\
             [env.a]\nplatform = \"p\"\nboard = \"x\"\nframework = \"f\"\n\
             [env.b]\nplatform = \"p\"\nboard = \"y\"\nframework = \"f\"\n",
        )
        .expect("parse");
        assert_eq!(cfg.resolve_env(None).expect("default").0, "b");
        assert_eq!(cfg.resolve_env(Some("a")).expect("named").1.board, "x");
        assert!(matches!(
            cfg.resolve_env(Some("zzz")),
            Err(crate::CoreError::UnknownEnv(_))
        ));
        let empty = RavomConfig::from_toml_str("").expect("empty manifest");
        assert!(matches!(
            empty.resolve_env(None),
            Err(crate::CoreError::NoEnvironments)
        ));
    }
}
