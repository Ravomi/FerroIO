use crate::context::EnvContext;
use crate::process::CommandSpec;
use crate::registry::BoardRecord;
use crate::{CoreError, Result};
use std::fs;
use std::path::Path;

/// Selected build backend based on framework, board platform, and project files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildBackend {
    Cargo,
    ArmGcc,
    AvrGcc,
    EspIdf,
    Zephyr,
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
    if board.is_fpga() {
        return BuildBackend::FpgaSynthesis;
    }

    if let Some(plugin) = ravom_plugin::find_framework_plugin(&fw) {
        return match plugin.kind() {
            ravom_plugin::FrameworkKind::FpgaHdl { .. } => BuildBackend::FpgaSynthesis,
            ravom_plugin::FrameworkKind::Zephyr => BuildBackend::Zephyr,
            ravom_plugin::FrameworkKind::CCpp => {
                if matches!(fw.as_str(), "esp-idf" | "espidf") {
                    BuildBackend::EspIdf
                } else if board.platform == "atmelavr" {
                    BuildBackend::AvrGcc
                } else {
                    BuildBackend::ArmGcc
                }
            }
            ravom_plugin::FrameworkKind::RustCargo => BuildBackend::Cargo,
        };
    }

    if matches!(fw.as_str(), "icestorm" | "apicula" | "openxc7") {
        return BuildBackend::FpgaSynthesis;
    }

    if crate::zephyr::is_zephyr_framework(&fw) {
        return BuildBackend::Zephyr;
    }

    if !dir.join("Cargo.toml").exists() {
        // PlatformIO spells it `espidf`, the RavomIO plugin `esp-idf`.
        if matches!(fw.as_str(), "esp-idf" | "espidf") {
            return BuildBackend::EspIdf;
        }
        if matches!(fw.as_str(), "arduino" | "stm32cube" | "cmsis" | "c" | "cpp")
            || dir.join("src/main.c").exists()
            || dir.join("src/main.cpp").exists()
        {
            return if board.platform == "atmelavr" {
                BuildBackend::AvrGcc
            } else {
                BuildBackend::ArmGcc
            };
        }
    }

    BuildBackend::Cargo
}

fn yosys_script(platform: &str) -> &'static str {
    match platform {
        "lattice_ice40" => "read_verilog top.v; synth_ice40 -json build/top.json",
        "lattice_ecp5" => "read_verilog top.v; synth_ecp5 -json build/top.json",
        "lattice_machxo2" => "read_verilog top.v; synth_lattice -family xo2 -json build/top.json",
        "gowin_fpga" => "read_verilog top.v; synth_gowin -json build/top.json",
        _ => "read_verilog top.v; synth_xilinx -json build/top.json",
    }
}

/// Compute the ordered list of external commands for a build, without running them.
pub fn plan_build(ctx: &EnvContext, release: bool) -> (BuildBackend, Vec<CommandSpec>) {
    let dir = ctx.root.as_path();
    let backend = select_build_backend(dir, &ctx.env.framework, &ctx.board);
    let steps = match backend {
        BuildBackend::Cargo => {
            let mut cmd = CommandSpec::new("cargo").arg("build").cwd(dir);
            if let Some(target) = &ctx.board.rust_target {
                cmd = cmd.args(["--target", target.as_str()]);
            }
            if release {
                cmd = cmd.arg("--release");
            }
            if !ctx.env.rustflags.is_empty() {
                cmd = cmd.env("CARGO_ENCODED_RUSTFLAGS", ctx.env.rustflags.join("\x1f"));
            }
            vec![cmd]
        }
        BuildBackend::FpgaSynthesis => {
            let mut steps = Vec::new();
            if ctx.env.framework.eq_ignore_ascii_case("rust-hdl") && dir.join("Cargo.toml").exists()
            {
                steps.push(
                    CommandSpec::new("cargo")
                        .args(["run", "--release"])
                        .cwd(dir),
                );
            }
            steps.push(
                CommandSpec::new("yosys")
                    .args(["-p", yosys_script(&ctx.board.platform)])
                    .cwd(dir),
            );
            steps
        }
        BuildBackend::ArmGcc => {
            let src = if dir.join("src/main.cpp").exists() {
                "src/main.cpp"
            } else {
                "src/main.c"
            };
            let mut cmd = CommandSpec::new("arm-none-eabi-gcc").cwd(dir);
            if let Some(cpu) = &ctx.board.cpu {
                cmd = cmd.arg(format!("-mcpu={cpu}")).arg("-mthumb");
            }
            cmd = cmd.arg(if release { "-O2" } else { "-Og" }).arg("-g");
            vec![cmd.args(ctx.env.build_flags.iter().cloned()).args([
                src,
                "-o",
                "build/firmware.elf",
            ])]
        }
        BuildBackend::AvrGcc => {
            let src = if dir.join("src/main.cpp").exists() {
                "src/main.cpp"
            } else {
                "src/main.c"
            };
            let mut cmd = CommandSpec::new("avr-gcc").cwd(dir);
            cmd = cmd.arg(format!("-mmcu={}", board_mcu_flag(&ctx.board.mcu)));
            cmd = cmd.arg(if release { "-Os" } else { "-Og" }).arg("-g");
            vec![cmd.args(ctx.env.build_flags.iter().cloned()).args([
                src,
                "-o",
                "build/firmware.elf",
            ])]
        }
        BuildBackend::EspIdf => vec![CommandSpec::new("idf.py").arg("build").cwd(dir)],
        BuildBackend::Zephyr => vec![crate::zephyr::plan_zephyr_build(ctx, release)],
    };
    (backend, steps)
}

fn board_mcu_flag(mcu: &str) -> String {
    mcu.to_ascii_lowercase()
}

/// Build the RavomIO project in `dir` for the selected (or default) environment,
/// with toolchain output inherited by the terminal.
pub fn build_project(dir: &Path, env_name: Option<&str>, release: bool) -> Result<BuildResult> {
    build_project_impl(dir, env_name, release, None)
}

/// Like [`build_project`] but streams every toolchain output line into `log`.
pub fn build_project_with_log(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    log: &mut dyn FnMut(&str),
) -> Result<BuildResult> {
    build_project_impl(dir, env_name, release, Some(log))
}

fn build_project_impl(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    mut log: Option<&mut dyn FnMut(&str)>,
) -> Result<BuildResult> {
    let ctx = EnvContext::load(dir, env_name)?;
    let (backend, steps) = plan_build(&ctx, release);
    if matches!(
        backend,
        BuildBackend::ArmGcc | BuildBackend::AvrGcc | BuildBackend::FpgaSynthesis
    ) {
        fs::create_dir_all(dir.join("build"))?;
    }
    if backend == BuildBackend::Zephyr {
        crate::zephyr::write_kconfig_fragment(&ctx, release)?;
    }
    for step in &steps {
        match log.as_mut() {
            Some(sink) => {
                sink(&format!("$ {step}"));
                step.run_streaming(CoreError::BuildFailed, *sink)?;
            }
            None => step.run(CoreError::BuildFailed)?,
        }
    }

    Ok(BuildResult {
        env_name: ctx.env_name,
        board_id: ctx.board.id,
        backend,
        target: ctx.board.rust_target,
        release,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RavomConfig;
    use crate::project::init_project;

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
            zephyr_board: None,
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
        for fw in ["esp-idf", "espidf"] {
            assert_eq!(
                select_build_backend(Path::new("/nonexistent"), fw, &stm_board),
                BuildBackend::EspIdf
            );
        }
        for fw in ["zephyr", "zephyr-rust"] {
            assert_eq!(
                select_build_backend(Path::new("/nonexistent"), fw, &stm_board),
                BuildBackend::Zephyr
            );
        }
        assert_eq!(
            select_build_backend(Path::new("/nonexistent"), "embassy", &stm_board),
            BuildBackend::Cargo
        );
    }

    #[test]
    fn cargo_plan_uses_board_target_and_rustflags_only() {
        let cfg = RavomConfig::from_toml_str(
            "[env.bp]\nplatform = \"ststm32\"\nboard = \"bluepill_f103c8\"\n\
             framework = \"embassy\"\nbuild_flags = [\"-DDEBUG\"]\n\
             rustflags = [\"-C\", \"link-arg=-Tlink.x\"]\n",
        )
        .expect("manifest");
        let ctx = EnvContext::from_config(Path::new("/nonexistent"), cfg, None).expect("ctx");
        let (backend, steps) = plan_build(&ctx, true);
        assert_eq!(backend, BuildBackend::Cargo);
        let cargo = &steps[0];
        assert_eq!(
            cargo.args,
            ["build", "--target", "thumbv7m-none-eabi", "--release"]
        );
        assert_eq!(
            cargo.env,
            [(
                "CARGO_ENCODED_RUSTFLAGS".to_string(),
                "-C\x1flink-arg=-Tlink.x".to_string()
            )],
            "C/C++ build_flags must not leak into RUSTFLAGS"
        );
    }

    #[test]
    fn build_fails_gracefully_without_cargo_toml() {
        let tmp = tempfile::tempdir().expect("tempdir");
        init_project(tmp.path(), "rp2040", Some("embassy")).expect("init_project should succeed");
        fs::remove_file(tmp.path().join("Cargo.toml")).expect("remove Cargo.toml");

        let err = build_project(tmp.path(), None, false)
            .expect_err("build should fail when Cargo.toml is missing");
        assert!(matches!(err, CoreError::BuildFailed(_)));
    }

    #[test]
    fn init_and_select_backend_round_trip_for_non_cargo_frameworks() {
        let avr_tmp = tempfile::tempdir().expect("tempdir");
        init_project(avr_tmp.path(), "atmelavr/uno", Some("arduino")).expect("init uno");
        let avr_ctx = EnvContext::load(avr_tmp.path(), None).expect("load uno");
        assert_eq!(
            select_build_backend(avr_tmp.path(), &avr_ctx.env.framework, &avr_ctx.board),
            BuildBackend::AvrGcc
        );

        let esp_tmp = tempfile::tempdir().expect("tempdir");
        init_project(esp_tmp.path(), "espressif32/esp32s3", Some("esp-idf")).expect("init esp32s3");
        let esp_ctx = EnvContext::load(esp_tmp.path(), None).expect("load esp32s3");
        assert_eq!(
            select_build_backend(esp_tmp.path(), &esp_ctx.env.framework, &esp_ctx.board),
            BuildBackend::EspIdf
        );
    }
}
