//! Embed the repository board registry (`boards/<platform>/<id>.toml`) into
//! `ravom-core` so `mio` resolves boards independently of the working directory.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let boards_dir = manifest_dir.join("../../boards");
    println!("cargo:rerun-if-changed={}", boards_dir.display());

    let mut entries: Vec<(String, String, PathBuf)> = Vec::new();
    if let Ok(platforms) = fs::read_dir(&boards_dir) {
        for platform in platforms.flatten() {
            let platform_path = platform.path();
            if !platform_path.is_dir() {
                continue;
            }
            let platform_name = file_name(&platform_path);
            let Ok(files) = fs::read_dir(&platform_path) else {
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                let abs = path.canonicalize().unwrap_or(path);
                entries.push((platform_name.clone(), stem, abs));
            }
        }
    }
    entries.sort();

    let mut out = String::from("pub(crate) static EMBEDDED_BOARDS: &[EmbeddedBoard] = &[\n");
    for (platform, stem, path) in &entries {
        let _ = writeln!(
            out,
            "    EmbeddedBoard {{ platform: {platform:?}, stem: {stem:?}, toml: include_str!({:?}) }},",
            path.display().to_string()
        );
    }
    out.push_str("];\n");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("embedded_boards.rs");
    fs::write(out_path, out).expect("write embedded_boards.rs");
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}
