//! End-to-end tests of the `mio` binary, run from a directory outside the
//! repository with an empty HOME so nothing depends on the developer machine.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn mio(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mio"))
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .env_remove("RAVOM_BOARDS_DIR")
        .env_remove("RAVOM_PROBE")
        .output()
        .expect("spawn mio")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn board_registry_works_outside_the_repository() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = mio(tmp.path(), tmp.path(), &["boards", "--check"]);
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("0 issue(s)"), "{}", stdout(&out));

    let list = mio(tmp.path(), tmp.path(), &["boards", "bluepill"]);
    assert!(stdout(&list).contains("bluepill_f103c8"));
}

#[test]
fn init_build_and_flash_dry_runs_produce_consistent_commands() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let proj = tmp.path().join("blink");
    let proj_s = proj.to_str().expect("utf8 path");

    let init = mio(
        tmp.path(),
        tmp.path(),
        &["init", "--board", "stm32f401ccu6", "--dir", proj_s],
    );
    assert!(init.status.success(), "{}", stderr(&init));

    let build = mio(
        tmp.path(),
        tmp.path(),
        &["-C", proj_s, "build", "--release", "--dry-run"],
    );
    assert!(build.status.success(), "{}", stderr(&build));
    assert!(
        stdout(&build).contains("cargo build --target thumbv7em-none-eabihf --release"),
        "{}",
        stdout(&build)
    );

    let flash = mio(
        tmp.path(),
        tmp.path(),
        &["-C", proj_s, "flash", "--release", "--dry-run"],
    );
    assert!(flash.status.success(), "{}", stderr(&flash));
    let expected_elf = proj.join("target/thumbv7em-none-eabihf/release/blink");
    assert!(
        stdout(&flash).contains(&format!(
            "probe-rs download --chip STM32F401CC {}",
            expected_elf.display()
        )),
        "{}",
        stdout(&flash)
    );

    let real_flash = mio(tmp.path(), tmp.path(), &["-C", proj_s, "flash"]);
    assert!(!real_flash.status.success());
    assert!(
        stderr(&real_flash).contains("build artifact not found"),
        "{}",
        stderr(&real_flash)
    );
}

#[test]
fn ambiguous_board_ids_are_rejected_with_candidates() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = mio(tmp.path(), tmp.path(), &["init", "--board", "pico"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("raspberrypi/pico") && err.contains("raspberrypi-earlephilhower/pico"),
        "{err}"
    );

    let ok = mio(
        tmp.path(),
        tmp.path(),
        &["init", "--board", "raspberrypi/pico"],
    );
    assert!(ok.status.success(), "{}", stderr(&ok));
}

#[test]
fn migrate_writes_manifest_and_reports_warnings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("platformio.ini"),
        "[env]\nframework = arduino\n\n[env:esp]\nplatform = espressif32@6.5.0\n\
         board = esp32dev\nupload_protocol = esptool\nboard_build.partitions = huge_app.csv\n",
    )
    .expect("write ini");
    let out = mio(tmp.path(), tmp.path(), &["migrate"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("esptool"), "{}", stderr(&out));
    let manifest = fs::read_to_string(tmp.path().join("ravom.toml")).expect("manifest");
    assert!(
        manifest.contains("platform = \"espressif32\""),
        "{manifest}"
    );
    assert!(manifest.contains("framework = \"arduino\""), "{manifest}");
    assert!(manifest.contains("huge_app.csv"), "{manifest}");
}

fn rust_target_installed(target: &str) -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.trim() == target)
        })
        .unwrap_or(false)
}

/// Real cross-compilation: the ELF `mio build` produces must be exactly where
/// `mio flash` will look for it.
#[test]
fn real_build_produces_the_elf_that_flash_expects() {
    if !rust_target_installed("thumbv7m-none-eabi") {
        eprintln!("skipping: rustup target thumbv7m-none-eabi not installed");
        return;
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let proj = tmp.path().join("bp");
    let proj_s = proj.to_str().expect("utf8 path");
    let init = mio(
        tmp.path(),
        tmp.path(),
        &[
            "init",
            "--board",
            "bluepill_f103c8",
            "--framework",
            "bare-metal",
            "--dir",
            proj_s,
        ],
    );
    assert!(init.status.success(), "{}", stderr(&init));

    let build = Command::new(env!("CARGO_BIN_EXE_mio"))
        .args(["-C", proj_s, "build"])
        .current_dir(tmp.path())
        .env("CARGO_TARGET_DIR", proj.join("target"))
        .env_remove("CARGO_MAKEFLAGS")
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .output()
        .expect("spawn mio build");
    assert!(
        build.status.success(),
        "{}{}",
        stdout(&build),
        stderr(&build)
    );
    let elf = proj.join("target/thumbv7m-none-eabi/debug/bp");
    let bytes = fs::read(&elf).unwrap_or_else(|e| panic!("{}: {e}", elf.display()));
    assert_eq!(&bytes[..4], b"\x7fELF");
    // e_machine == EM_ARM (40)
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 40);
    // Verify at least one PT_LOAD program header lands in STM32F103 flash (0x0800_0000..0x0801_0000)
    let e_phoff = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as usize;
    let e_phentsize = u16::from_le_bytes([bytes[42], bytes[43]]) as usize;
    let e_phnum = u16::from_le_bytes([bytes[44], bytes[45]]) as usize;
    let mut loads_into_stm32_flash = false;
    for i in 0..e_phnum {
        let off = e_phoff + i * e_phentsize;
        let p_type =
            u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
        let p_paddr = u32::from_le_bytes([
            bytes[off + 12],
            bytes[off + 13],
            bytes[off + 14],
            bytes[off + 15],
        ]);
        let p_filesz = u32::from_le_bytes([
            bytes[off + 16],
            bytes[off + 17],
            bytes[off + 18],
            bytes[off + 19],
        ]);
        if p_type == 1 && p_filesz > 0 {
            assert!(
                (0x0800_0000..0x0801_0000).contains(&p_paddr),
                "PT_LOAD p_paddr 0x{p_paddr:08X} outside STM32F103C8 flash"
            );
            loads_into_stm32_flash = true;
        }
    }
    assert!(
        loads_into_stm32_flash,
        "expected a non-empty PT_LOAD segment in STM32 flash"
    );
}

#[test]
fn zephyr_c_and_rust_init_build_flash_and_management_commands() {
    let tmp = tempfile::tempdir().expect("tempdir");

    // 1. Zephyr C project: init -> build --dry-run -> flash --dry-run (probe-rs on zephyr.elf)
    let c_proj = tmp.path().join("z_c_app");
    let c_s = c_proj.to_str().expect("utf8 path");
    let init_c = mio(
        tmp.path(),
        tmp.path(),
        &[
            "init",
            "--board",
            "stm32f401ccu6",
            "--framework",
            "zephyr",
            "--dir",
            c_s,
        ],
    );
    assert!(init_c.status.success(), "{}", stderr(&init_c));
    for file in [
        "ravom.toml",
        "CMakeLists.txt",
        "prj.conf",
        "app.overlay",
        "src/main.c",
    ] {
        assert!(c_proj.join(file).is_file(), "missing {file}");
    }
    assert!(!c_proj.join("Cargo.toml").exists());

    let build_c = mio(
        tmp.path(),
        tmp.path(),
        &["-C", c_s, "build", "--release", "--dry-run"],
    );
    assert!(build_c.status.success(), "{}", stderr(&build_c));
    let build_c_out = stdout(&build_c);
    assert!(
        build_c_out.contains("west build -b blackpill_f401cc -d build/stm32f401ccu6 -p auto --"),
        "{build_c_out}"
    );
    assert!(
        build_c_out.contains("-DEXTRA_CONF_FILE=build/stm32f401ccu6/ravom.conf"),
        "{build_c_out}"
    );
    assert!(
        build_c_out.contains("-DDTC_OVERLAY_FILE=app.overlay"),
        "{build_c_out}"
    );

    let flash_c = mio(tmp.path(), tmp.path(), &["-C", c_s, "flash", "--dry-run"]);
    assert!(flash_c.status.success(), "{}", stderr(&flash_c));
    let expected_elf = c_proj.join("build/stm32f401ccu6/zephyr/zephyr.elf");
    assert!(
        stdout(&flash_c).contains(&format!(
            "probe-rs download --chip STM32F401CC {}",
            expected_elf.display()
        )),
        "{}",
        stdout(&flash_c)
    );

    // 2. Zephyr Rust project: `--framework zephyr --lang rust`
    let rs_proj = tmp.path().join("z_rs_app");
    let rs_s = rs_proj.to_str().expect("utf8 path");
    let init_rs = mio(
        tmp.path(),
        tmp.path(),
        &[
            "init",
            "--board",
            "stm32f401ccu6",
            "--framework",
            "zephyr",
            "--lang",
            "rust",
            "--dir",
            rs_s,
        ],
    );
    assert!(init_rs.status.success(), "{}", stderr(&init_rs));
    for file in [
        "ravom.toml",
        "CMakeLists.txt",
        "prj.conf",
        "app.overlay",
        "Cargo.toml",
        "src/lib.rs",
    ] {
        assert!(rs_proj.join(file).is_file(), "missing {file}");
    }
    let cmake = fs::read_to_string(rs_proj.join("CMakeLists.txt")).expect("CMakeLists.txt");
    assert!(cmake.contains("rust_cargo_application()"), "{cmake}");

    let menuconfig = mio(
        tmp.path(),
        tmp.path(),
        &["-C", rs_s, "zephyr", "menuconfig", "--dry-run"],
    );
    assert!(menuconfig.status.success(), "{}", stderr(&menuconfig));
    assert!(
        stdout(&menuconfig)
            .contains("west build -b blackpill_f401cc -d build/stm32f401ccu6 -t menuconfig"),
        "{}",
        stdout(&menuconfig)
    );

    // 3. `mio zephyr status` & `mio zephyr setup --rust --dry-run`
    let status = mio(tmp.path(), tmp.path(), &["-C", rs_s, "zephyr", "status"]);
    assert!(status.status.success(), "{}", stderr(&status));
    assert!(
        stdout(&status).contains("zephyr_board: blackpill_f401cc"),
        "{}",
        stdout(&status)
    );

    let setup = mio(
        tmp.path(),
        tmp.path(),
        &["zephyr", "setup", "--rust", "--dry-run"],
    );
    assert!(setup.status.success(), "{}", stderr(&setup));
    let setup_out = stdout(&setup);
    assert!(setup_out.contains("west init"), "{setup_out}");
    assert!(
        setup_out.contains("manifest.project-filter +zephyr-lang-rust"),
        "{setup_out}"
    );
    assert!(setup_out.contains("sdk install --version"), "{setup_out}");
}
