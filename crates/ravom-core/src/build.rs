use crate::config::RavomConfig;
use crate::registry::{find_board_record, BoardRecord};
use crate::{CoreError, Result};
use std::path::Path;
use std::process::Command;

/// Selected build backend based on framework, board platform, and project files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildBackend {
    Cargo,
    ArmGcc,
    EspIdf,
    FpgaSynthesis,
}

/// Result of a firmware or bitstream build invocation.
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub env_name: String,
    pub board_id: String,
    pub backend: BuildBackend,
    pub target: Option<String>,
    pub release: bool,
}

/// Determine the appropriate build backend for a project environment.
pub fn select_build_backend(dir: &Path, framework: &str, board: &BoardRecord) -> BuildBackend {
    let fw = framework.to_ascii_lowercase();
    let is_fpga_platform = board.platform.starts_with("lattice_")
        || board.platform == "gowin_fpga"
        || board.platform.starts_with("xilinx_");
    if is_fpga_platform
        || matches!(
            fw.as_str(),
            "rust-hdl" | "spade" | "veryl" | "verilog" | "icestorm" | "apicula" | "openxc7"
        )
    {
        return BuildBackend::FpgaSynthesis;
    }

    if !dir.join("Cargo.toml").exists() {
        if fw == "esp-idf" {
            return BuildBackend::EspIdf;
        }
        if matches!(fw.as_str(), "arduino" | "stm32cube" | "cmsis" | "c" | "cpp")
            || dir.join("src/main.c").exists()
            || dir.join("src/main.cpp").exists()
        {
            return BuildBackend::ArmGcc;
        }
    }

    BuildBackend::Cargo
}

fn run_fpga_pipeline(dir: &Path, framework: &str, board: &BoardRecord) -> Result<()> {
    if framework.eq_ignore_ascii_case("rust-hdl") && dir.join("Cargo.toml").exists() {
        let status = Command::new("cargo")
            .args(["run", "--release"])
            .current_dir(dir)
            .status()?;
        if !status.success() {
            return Err(CoreError::BuildFailed(status.code()));
        }
    }

    let synth_cmd = match board.platform.as_str() {
        "lattice_ice40" => "read_verilog top.v; synth_ice40 -json build/top.json",
        "lattice_ecp5" => "read_verilog top.v; synth_ecp5 -json build/top.json",
        "lattice_machxo2" => "read_verilog top.v; synth_nexus -json build/top.json",
        "gowin_fpga" => "read_verilog top.v; synth_gowin -json build/top.json",
        _ => "read_verilog top.v; synth_xilinx -json build/top.json",
    };

    let status = Command::new("yosys")
        .args(["-p", synth_cmd])
        .current_dir(dir)
        .status()
        .map_err(|_| CoreError::BuildFailed(None))?;
    if !status.success() {
        return Err(CoreError::BuildFailed(status.code()));
    }
    Ok(())
}

fn run_arm_gcc(dir: &Path, board: &BoardRecord, build_flags: &[String]) -> Result<()> {
    let src = if dir.join("src/main.cpp").exists() {
        "src/main.cpp"
    } else {
        "src/main.c"
    };
    let mut cmd = Command::new("arm-none-eabi-gcc");
    cmd.current_dir(dir);
    if let Some(cpu) = &board.cpu {
        cmd.arg(format!("-mcpu={cpu}")).arg("-mthumb");
    }
    cmd.args(build_flags)
        .args([src, "-o", "build/firmware.elf"]);

    let status = cmd.status().map_err(|_| CoreError::BuildFailed(None))?;
    if !status.success() {
        return Err(CoreError::BuildFailed(status.code()));
    }
    Ok(())
}

/// Build the RavomIO project in `dir` for the selected (or default) environment.
pub fn build_project(dir: &Path, env_name: Option<&str>, release: bool) -> Result<BuildResult> {
    let cfg = RavomConfig::load_from_dir(dir)?;
    let (resolved_env, env_cfg) = cfg.resolve_env(env_name)?;
    let board = find_board_record(&env_cfg.board)
        .ok_or_else(|| CoreError::UnknownBoard(env_cfg.board.clone()))?;

    let backend = select_build_backend(dir, &env_cfg.framework, &board);

    match backend {
        BuildBackend::Cargo => {
            let mut cmd = Command::new("cargo");
            cmd.arg("build").current_dir(dir);
            if let Some(target) = &board.rust_target {
                cmd.args(["--target", target]);
            }
            if release {
                cmd.arg("--release");
            }
            if !env_cfg.build_flags.is_empty() {
                cmd.env("CARGO_ENCODED_RUSTFLAGS", env_cfg.build_flags.join("\x1f"));
            }
            let status = cmd.status()?;
            if !status.success() {
                return Err(CoreError::BuildFailed(status.code()));
            }
        }
        BuildBackend::FpgaSynthesis => {
            run_fpga_pipeline(dir, &env_cfg.framework, &board)?;
        }
        BuildBackend::ArmGcc => {
            run_arm_gcc(dir, &board, &env_cfg.build_flags)?;
        }
        BuildBackend::EspIdf => {
            let status = Command::new("idf.py")
                .arg("build")
                .current_dir(dir)
                .status()
                .map_err(|_| CoreError::BuildFailed(None))?;
            if !status.success() {
                return Err(CoreError::BuildFailed(status.code()));
            }
        }
    }

    Ok(BuildResult {
        env_name: resolved_env.to_string(),
        board_id: board.id,
        backend,
        target: board.rust_target,
        release,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_project, select_build_backend, BuildBackend};
    use crate::project::init_project;
    use crate::registry::BoardRecord;
    use crate::CoreError;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_board(platform: &str) -> BoardRecord {
        BoardRecord {
            id: "sample".to_string(),
            name: "Sample".to_string(),
            vendor: None,
            platform: platform.to_string(),
            mcu: "SAMPLE".to_string(),
            cpu: Some("cortex-m3".to_string()),
            default_framework: "embassy".to_string(),
            frameworks: vec![],
            probe_rs_chip: None,
            rust_target: None,
            flash_bytes: None,
            ram_bytes: None,
        }
    }

    #[test]
    fn selects_rust_c_espidf_and_fpga_backends() {
        let fpga_board = sample_board("lattice_machxo2");
        assert_eq!(
            select_build_backend(Path::new("/nonexistent"), "rust-hdl", &fpga_board),
            BuildBackend::FpgaSynthesis
        );

        let stm_board = sample_board("ststm32");
        assert_eq!(
            select_build_backend(Path::new("/nonexistent"), "stm32cube", &stm_board),
            BuildBackend::ArmGcc
        );
        assert_eq!(
            select_build_backend(Path::new("/nonexistent"), "esp-idf", &stm_board),
            BuildBackend::EspIdf
        );
        assert_eq!(
            select_build_backend(Path::new("/nonexistent"), "embassy", &stm_board),
            BuildBackend::Cargo
        );
    }

    #[test]
    fn build_fails_gracefully_without_cargo_toml() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("ravomio_build_test_{nanos}"));
        init_project(&tmp, "rp2040", Some("embassy")).expect("init_project should succeed");
        let _ = fs::remove_file(tmp.join("Cargo.toml"));

        let err = build_project(&tmp, None, false)
            .expect_err("build should fail when Cargo.toml is missing");
        assert!(matches!(err, CoreError::BuildFailed(_)));

        let _ = fs::remove_dir_all(&tmp);
    }
}
