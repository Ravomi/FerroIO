use crate::config::{EnvConfig, ProjectMeta, RavomConfig};
use crate::registry::find_board_record;
use crate::{CoreError, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Summary of an initialized RavomIO project.
#[derive(Debug, Clone)]
pub struct InitResult {
    pub root: PathBuf,
    pub board_id: String,
    pub framework: String,
    pub manifest_path: PathBuf,
}

/// Initialize a PlatformIO-style project layout (`ravom.toml`, `src/`, `include/`, `lib/`).
pub fn init_project(dir: &Path, board_id: &str, framework: Option<&str>) -> Result<InitResult> {
    let board =
        find_board_record(board_id).ok_or_else(|| CoreError::UnknownBoard(board_id.to_string()))?;

    let fw = framework.unwrap_or(&board.default_framework).to_string();
    fs::create_dir_all(dir)?;

    let manifest_path = dir.join("ravom.toml");
    if manifest_path.exists() {
        return Err(CoreError::AlreadyInitialized(manifest_path));
    }

    for sub in ["src", "include", "lib"] {
        fs::create_dir_all(dir.join(sub))?;
    }

    let project_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ravomio-app")
        .to_string();

    let mut env = BTreeMap::new();
    env.insert(
        board.id.to_string(),
        EnvConfig {
            platform: board.platform.to_string(),
            board: board.id.to_string(),
            framework: fw.clone(),
            build_flags: vec![],
            lib_deps: vec![],
            upload_protocol: Some("probe-rs".to_string()),
            monitor_speed: Some(115200),
        },
    );

    let cfg = RavomConfig {
        project: ProjectMeta {
            name: Some(project_name.clone()),
            default_envs: Some(vec![board.id.to_string()]),
        },
        env,
    };

    fs::write(&manifest_path, cfg.to_toml_string()?)?;
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
            project_name.replace(' ', "-")
        ),
    )?;
    if let Some(target) = &board.rust_target {
        fs::create_dir_all(dir.join(".cargo"))?;
        fs::write(
            dir.join(".cargo/config.toml"),
            format!("[build]\ntarget = \"{target}\"\n"),
        )?;
    }
    fs::write(
        dir.join("src/main.rs"),
        "// RavomIO entry point\nfn main() {\n    // TODO: initialize peripherals\n}\n",
    )?;
    fs::write(dir.join(".gitignore"), "target/\n.ravom/\n")?;

    Ok(InitResult {
        root: dir.to_path_buf(),
        board_id: board.id.to_string(),
        framework: fw,
        manifest_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn init_creates_platformio_like_layout() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("ravomio_test_{nanos}"));
        let res = init_project(&tmp, "stm32f401ccu6", None)
            .expect("init_project should succeed for stm32f401ccu6");

        assert!(res.manifest_path.exists());
        assert!(tmp.join("Cargo.toml").exists());
        assert!(tmp.join(".cargo/config.toml").exists());
        assert!(tmp.join("src/main.rs").exists());
        assert!(tmp.join("include").is_dir());
        assert!(tmp.join("lib").is_dir());
        assert!(tmp.join(".gitignore").exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}
