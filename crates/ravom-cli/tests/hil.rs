//! Hardware-in-the-loop tests. They need a physical board + debug probe and are
//! `#[ignore]`d by default. Run on a bench machine with:
//!
//! ```text
//! RAVOM_HIL_BOARDS=all cargo test -p ravom-cli --test hil -- --ignored --nocapture
//! ```
//!
//! The non-ignored test below only verifies that every fixture cross-compiles,
//! so the firmware flashed on the bench is always buildable.

use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Bench {
    board: Vec<BenchBoard>,
}

#[derive(Debug, Deserialize)]
struct BenchBoard {
    id: String,
    fixture: String,
    marker: String,
    probe: Option<String>,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bench() -> Bench {
    let raw =
        std::fs::read_to_string(repo_root().join("hil/boards.toml")).expect("hil/boards.toml");
    toml::from_str(&raw).expect("valid hil/boards.toml")
}

fn mio(fixture: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mio"));
    cmd.arg("-C").arg(fixture);
    // Do not hand the outer `cargo test` jobserver to the nested `cargo build`.
    for var in ["CARGO_MAKEFLAGS", "MAKEFLAGS", "MFLAGS"] {
        cmd.env_remove(var);
    }
    cmd
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

#[test]
fn bench_matrix_references_existing_fixtures_and_boards() {
    let reg = ravom_core::registry::BoardRegistry::embedded();
    for b in bench().board {
        let board = reg
            .resolve(&b.id, None)
            .expect("bench board exists in registry");
        assert!(
            board.probe_rs_chip.is_some(),
            "{} is not flashable via probe-rs",
            b.id
        );
        let fixture = repo_root().join("hil/fixtures").join(&b.fixture);
        assert!(
            fixture.join("ravom.toml").exists(),
            "missing fixture {}",
            fixture.display()
        );
    }
}

#[test]
fn hil_fixtures_cross_compile() {
    for b in bench().board {
        let fixture = repo_root().join("hil/fixtures").join(&b.fixture);
        let ctx = ravom_core::context::EnvContext::load(&fixture, None).expect("fixture env");
        let target = ctx
            .board
            .rust_target
            .clone()
            .expect("fixture board has a rust target");
        if !rust_target_installed(&target) {
            eprintln!(
                "skipping {}: rustup target {target} not installed",
                b.fixture
            );
            continue;
        }
        let out = mio(&fixture)
            .args(["build", "--release"])
            .output()
            .expect("spawn mio build");
        assert!(
            out.status.success(),
            "{}: {}{}",
            b.fixture,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            ctx.firmware_path(true).exists(),
            "{} ELF missing",
            b.fixture
        );
    }
}

#[test]
#[ignore = "requires physical boards and debug probes (set RAVOM_HIL_BOARDS)"]
fn hil_flash_and_rtt_marker() {
    let selected = std::env::var("RAVOM_HIL_BOARDS").unwrap_or_default();
    assert!(
        !selected.is_empty(),
        "set RAVOM_HIL_BOARDS=all or a comma separated id list"
    );
    let wanted: Vec<&str> = selected.split(',').map(str::trim).collect();

    let mut failures = Vec::new();
    for b in bench().board {
        if !wanted.contains(&"all") && !wanted.contains(&b.id.as_str()) {
            continue;
        }
        let fixture = repo_root().join("hil/fixtures").join(&b.fixture);
        let build = mio(&fixture)
            .args(["build", "--release"])
            .status()
            .expect("mio build");
        if !build.success() {
            failures.push(format!("{}: build failed", b.id));
            continue;
        }

        let mut flash = mio(&fixture);
        flash
            .args(["flash", "--release", "--monitor"])
            .stdout(Stdio::piped());
        if let Some(probe) = &b.probe {
            flash.env("RAVOM_PROBE", probe);
        }
        let mut child = flash.spawn().expect("spawn mio flash");
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                println!("[rtt] {line}");
                if tx.send(line).is_err() {
                    break;
                }
            }
        });

        let deadline = Duration::from_secs(b.timeout_secs);
        let found = loop {
            match rx.recv_timeout(deadline) {
                Ok(line) if line.contains(&b.marker) => break true,
                Ok(_) => continue,
                Err(_) => break false,
            }
        };
        let exited_early = child.try_wait().ok().flatten();
        let _ = child.kill();
        let _ = child.wait();
        match (found, exited_early) {
            (true, _) => println!("{}: OK", b.id),
            (false, Some(status)) => failures.push(format!(
                "{}: `mio flash --monitor` exited ({status}) before printing '{}'",
                b.id, b.marker
            )),
            (false, None) => failures.push(format!(
                "{}: marker '{}' not seen within {}s",
                b.id, b.marker, b.timeout_secs
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "HIL failures:\n{}",
        failures.join("\n")
    );
}
