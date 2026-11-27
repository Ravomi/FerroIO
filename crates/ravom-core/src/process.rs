//! External tool invocation as inspectable, unit-testable command plans.

use crate::{CoreError, Result};
use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// A fully resolved external command (program, arguments, working directory, env).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn path_arg(self, path: &Path) -> Self {
        self.arg(path.display().to_string())
    }

    pub fn cwd(mut self, dir: &Path) -> Self {
        self.cwd = Some(dir.to_path_buf());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Convert into a `std::process::Command`.
    pub fn to_command(&self) -> Command {
        let program = if let Some(custom) = crate::toolchain::resolve_configured_tool(&self.program)
        {
            custom.display().to_string()
        } else if self.program == "cargo" {
            resolve_cargo_program()
        } else {
            self.program.clone()
        };
        let mut cmd = Command::new(program);
        cmd.args(&self.args);
        if let Some(dir) = &self.cwd {
            cmd.current_dir(dir);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd
    }

    fn spawn_error(&self, err: std::io::Error) -> CoreError {
        if err.kind() == std::io::ErrorKind::NotFound {
            CoreError::ToolNotFound(self.program.clone())
        } else {
            CoreError::Io(err)
        }
    }

    /// Run with inherited stdio; map a missing program to `ToolNotFound` and a
    /// non-zero exit status through `on_failure`.
    pub fn run(&self, on_failure: fn(Option<i32>) -> CoreError) -> Result<()> {
        let status = self
            .to_command()
            .status()
            .map_err(|e| self.spawn_error(e))?;
        if status.success() {
            Ok(())
        } else {
            Err(on_failure(status.code()))
        }
    }

    /// Run with stdout and stderr captured line by line into `sink` (used by
    /// the desktop workbench, which has no terminal to inherit).
    pub fn run_streaming(
        &self,
        on_failure: fn(Option<i32>) -> CoreError,
        sink: &mut dyn FnMut(&str),
    ) -> Result<()> {
        let mut child = self
            .to_command()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| self.spawn_error(e))?;
        let (tx, rx) = mpsc::channel::<String>();
        let streams: Vec<Box<dyn Read + Send>> = [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        .collect();
        for stream in streams {
            let tx = tx.clone();
            thread::spawn(move || {
                for line in BufReader::new(stream)
                    .lines()
                    .map_while(std::result::Result::ok)
                {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for line in rx {
            sink(&line);
        }
        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(on_failure(status.code()))
        }
    }
}

impl fmt::Display for CommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program)?;
        for arg in &self.args {
            if arg.contains(char::is_whitespace) {
                write!(f, " {arg:?}")?;
            } else {
                write!(f, " {arg}")?;
            }
        }
        Ok(())
    }
}

/// Locate an executable on `PATH` without shelling out to `which` (works on Windows too).
pub fn find_in_path(program: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string())
            .split(';')
            .map(str::to_string)
            .collect()
    } else {
        vec![String::new()]
    };
    std::env::split_paths(&path_var).find_map(|dir| {
        exts.iter().find_map(|ext| {
            let candidate = dir.join(format!("{program}{ext}"));
            candidate.is_file().then_some(candidate)
        })
    })
}

fn is_script_file(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 2];
    file.read_exact(&mut magic).is_ok() && magic == *b"#!"
}

/// Resolve the real Rust `cargo` executable.
///
/// Honors `RAVOM_CARGO` when set. If the first `cargo` on `PATH` is a shell
/// wrapper script (e.g. a shim that intercepts `cargo build`), falls back to
/// `rustup which cargo` or `$CARGO_HOME/bin/cargo` so `mio build` still works.
pub fn resolve_cargo_program() -> String {
    if let Ok(explicit) = std::env::var("RAVOM_CARGO") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(found) = find_in_path("cargo") {
        if !is_script_file(&found) {
            return "cargo".to_string();
        }
        if let Ok(out) = Command::new("rustup").args(["which", "cargo"]).output() {
            if out.status.success() {
                let real = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !real.is_empty() && Path::new(&real).is_file() {
                    return real;
                }
            }
        }
        let cargo_bin = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))
            .map(|d| {
                d.join("bin")
                    .join(if cfg!(windows) { "cargo.exe" } else { "cargo" })
            });
        if let Some(path) = cargo_bin {
            if path.is_file() && !is_script_file(&path) {
                return path.display().to_string();
            }
        }
    }
    "cargo".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_quotes_arguments_with_spaces() {
        let spec = CommandSpec::new("arm-none-eabi-gcc").args(["-D", "FOO BAR"]);
        assert_eq!(spec.to_string(), "arm-none-eabi-gcc -D \"FOO BAR\"");
    }

    #[test]
    fn missing_program_maps_to_tool_not_found() {
        let err = CommandSpec::new("ravomio-definitely-missing-tool")
            .run(CoreError::BuildFailed)
            .expect_err("missing tool must fail");
        assert!(
            matches!(err, CoreError::ToolNotFound(ref p) if p == "ravomio-definitely-missing-tool")
        );
    }

    #[cfg(unix)]
    #[test]
    fn streaming_run_captures_stdout_stderr_and_exit_code() {
        let mut lines = Vec::new();
        let err = CommandSpec::new("sh")
            .args(["-c", "echo out; echo err >&2; exit 3"])
            .run_streaming(CoreError::BuildFailed, &mut |l| lines.push(l.to_string()))
            .expect_err("exit 3 must fail");
        lines.sort();
        assert_eq!(lines, ["err", "out"]);
        assert!(matches!(err, CoreError::BuildFailed(Some(3))));
    }

    #[test]
    fn finds_cargo_on_path() {
        assert!(find_in_path("cargo").is_some());
        assert!(find_in_path("ravomio-definitely-missing-tool").is_none());
    }
}
