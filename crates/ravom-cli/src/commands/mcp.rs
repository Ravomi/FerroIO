//! Stdio Model Context Protocol (MCP) server (`mio mcp`) with permission tiers,
//! asynchronous background tasks, HIL telemetry sampling, guarded PID tuning,
//! and firmware anti-reversing hardening / ELF binary audit tools.

use ravom_core::build::{build_project_with_log, plan_build};
use ravom_core::context::EnvContext;
use ravom_core::flash::flash_project_with_log;
use ravom_core::monitor::{list_serial_port_details, sample_serial_lines, write_serial_frame};
use ravom_core::project::init_project;
use ravom_core::registry::registry;
use ravom_core::security::{
    audit_firmware_elf, plan_chip_protection, plan_hardened_build, SecurityProfile,
};
use ravom_core::toolchain::check_toolchains;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Permission tier controlling which MCP tools may execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum McpMode {
    /// Only discovery, dry-run planning, and binary audit tools are permitted.
    ReadOnly,
    /// ReadOnly + project scaffolding and background builds (default).
    Build,
    /// Build + physical hardware flashing, serial telemetry sampling, and guarded PID tuning.
    Hardware,
}

impl McpMode {
    /// Parse a CLI `--mode` flag (`read-only`, `build`, `hardware`).
    pub fn parse(mode: &str, allow_hardware: bool) -> Self {
        if allow_hardware || mode.eq_ignore_ascii_case("hardware") {
            Self::Hardware
        } else if mode.eq_ignore_ascii_case("read-only") || mode.eq_ignore_ascii_case("readonly") {
            Self::ReadOnly
        } else {
            Self::Build
        }
    }
}

/// Lifecycle state of an asynchronous background build or flash job.
#[derive(Debug, Clone)]
pub struct TaskEntry {
    pub id: u64,
    pub action: String,
    pub status: String,
    pub logs: Vec<String>,
    pub summary: Option<String>,
    pub cancel: Arc<AtomicBool>,
}

/// Thread-safe background job table for long-running MCP builds and flashes.
#[derive(Default, Clone)]
pub struct TaskRegistry {
    inner: Arc<Mutex<(u64, BTreeMap<u64, TaskEntry>)>>,
}

impl TaskRegistry {
    /// Start a background `build` or `flash` job and return its `task_id`.
    pub fn spawn_job(&self, action: &str, dir: PathBuf, env: Option<String>, release: bool) -> u64 {
        let cancel = Arc::new(AtomicBool::new(false));
        let task_id = {
            let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            guard.0 += 1;
            let id = guard.0;
            guard.1.insert(
                id,
                TaskEntry {
                    id,
                    action: action.to_string(),
                    status: "running".to_string(),
                    logs: Vec::new(),
                    summary: None,
                    cancel: Arc::clone(&cancel),
                },
            );
            id
        };

        let table = Arc::clone(&self.inner);
        let act = action.to_string();
        std::thread::spawn(move || {
            let push_log = |line: &str| {
                let mut g = table.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(entry) = g.1.get_mut(&task_id) {
                    entry.logs.push(line.to_string());
                }
            };
            let outcome = if act == "flash" {
                let mut sink = |line: &str| push_log(line);
                flash_project_with_log(&dir, env.as_deref(), release, &mut sink).map(|r| {
                    format!(
                        "Flashed {} ({}) to {}",
                        r.env_name,
                        r.firmware.display(),
                        r.chip
                    )
                })
            } else {
                let mut sink = |line: &str| push_log(line);
                build_project_with_log(&dir, env.as_deref(), release, &mut sink)
                    .map(|r| format!("Built {} ({:?})", r.env_name, r.backend))
            };

            let mut g = table.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = g.1.get_mut(&task_id) {
                if entry.cancel.load(Ordering::Relaxed) {
                    entry.status = "cancelled".to_string();
                } else {
                    match outcome {
                        Ok(msg) => {
                            entry.status = "completed".to_string();
                            entry.summary = Some(msg);
                        }
                        Err(err) => {
                            entry.status = "failed".to_string();
                            entry.summary = Some(err.to_string());
                        }
                    }
                }
            }
        });

        task_id
    }

    /// Retrieve current status and logs of a background task.
    pub fn poll(&self, task_id: u64) -> Option<TaskEntry> {
        let g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.1.get(&task_id).cloned()
    }

    /// Mark a background task as cancelled.
    pub fn cancel(&self, task_id: u64) -> bool {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = g.1.get_mut(&task_id) {
            entry.cancel.store(true, Ordering::Relaxed);
            entry.status = "cancelled".to_string();
            true
        } else {
            false
        }
    }
}

/// Run the stdio JSON-RPC 2.0 MCP server until EOF.
pub fn run_mcp_server(
    mode_str: &str,
    allow_hardware: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mode = McpMode::parse(mode_str, allow_hardware);
    let tasks = TaskRegistry::default();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(err) => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("parse error: {err}") }
                });
                writeln!(out, "{resp}")?;
                out.flush()?;
                continue;
            }
        };

        if let Some(resp) = handle_request_with_state(&req, mode, &tasks) {
            writeln!(out, "{resp}")?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Handle a single JSON-RPC 2.0 MCP request; returns `None` for notifications.
pub fn handle_request_with_state(
    req: &Value,
    mode: McpMode,
    tasks: &TaskRegistry,
) -> Option<Value> {
    let id = req.get("id")?.clone();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "ravom-mcp",
                "version": env!("CARGO_PKG_VERSION"),
                "mode": format!("{mode:?}")
            }
        })),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => handle_tool_call(&params, mode, tasks),
        _ => Err((-32601, format!("method not found: {method}"))),
    };

    Some(match result {
        Ok(val) => json!({ "jsonrpc": "2.0", "id": id, "result": val }),
        Err((code, message)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        }
    })
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "boards_search",
            "description": "Search RavomIO's embedded MCU & Linux SBC board registry by ID, MCU, or platform.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "board_inspect",
            "description": "Return full BoardRecord metadata (Flash/RAM bytes, rust_target, probe_rs_chip, frameworks).",
            "inputSchema": {
                "type": "object",
                "properties": { "board": { "type": "string" } },
                "required": ["board"]
            }
        },
        {
            "name": "doctor_check",
            "description": "Check installed embedded toolchains (rustc, probe-rs, espflash, arm-none-eabi-gcc, west).",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "device_list",
            "description": "List attached USB serial and debug probe ports.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "plan_build",
            "description": "Inspect the selected BuildBackend and external commands for a project without executing them.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dir": { "type": "string" },
                    "env": { "type": "string" },
                    "release": { "type": "boolean" }
                },
                "required": ["dir"]
            }
        },
        {
            "name": "plan_hardened_build",
            "description": "Plan a stage-separated build (`dev`, `hil` for real-time PID tuning, or `release-hardened` with symbol stripping, path remapping, and LLVM obfuscation passes).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dir": { "type": "string" },
                    "env": { "type": "string" },
                    "profile": { "type": "string", "enum": ["dev", "hil", "release-hardened"] },
                    "obfuscation_passes": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["dir", "profile"]
            }
        },
        {
            "name": "audit_firmware",
            "description": "Audit a compiled ELF binary for reverse-engineering leaks (.symtab, .debug_*, host paths, panic strings).",
            "inputSchema": {
                "type": "object",
                "properties": { "elf_path": { "type": "string" } },
                "required": ["elf_path"]
            }
        },
        {
            "name": "plan_chip_protection",
            "description": "Generate reversible and manual-only hardware read-protection / flash encryption plans (RDP / APPROTECT / eFuse).",
            "inputSchema": {
                "type": "object",
                "properties": { "board": { "type": "string" } },
                "required": ["board"]
            }
        },
        {
            "name": "init_project",
            "description": "Initialize a new RavomIO project non-interactively (requires Build or Hardware mode).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dir": { "type": "string" },
                    "board": { "type": "string" },
                    "framework": { "type": "string" }
                },
                "required": ["dir", "board"]
            }
        },
        {
            "name": "task_start",
            "description": "Start an asynchronous background `build` or `flash` job (`flash` requires Hardware mode).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["build", "flash"] },
                    "dir": { "type": "string" },
                    "env": { "type": "string" },
                    "release": { "type": "boolean" }
                },
                "required": ["action", "dir"]
            }
        },
        {
            "name": "task_poll",
            "description": "Poll status and logs of a background task started with `task_start`.",
            "inputSchema": {
                "type": "object",
                "properties": { "task_id": { "type": "integer" } },
                "required": ["task_id"]
            }
        },
        {
            "name": "task_cancel",
            "description": "Cancel a running background task.",
            "inputSchema": {
                "type": "object",
                "properties": { "task_id": { "type": "integer" } },
                "required": ["task_id"]
            }
        },
        {
            "name": "monitor_sample",
            "description": "Capture bounded serial telemetry (`duration_ms`, `max_lines`) and compute PID error metrics (requires Hardware mode).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "port": { "type": "string" },
                    "baud": { "type": "integer" },
                    "duration_ms": { "type": "integer" },
                    "max_lines": { "type": "integer" },
                    "until_pattern": { "type": "string" }
                },
                "required": ["port"]
            }
        },
        {
            "name": "pid_tune",
            "description": "Inject clamped PID gains (`kp`, `ki`, `kd`) over serial with mandatory safety bounds validation (requires Hardware mode).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "port": { "type": "string" },
                    "baud": { "type": "integer" },
                    "kp": { "type": "number" },
                    "ki": { "type": "number" },
                    "kd": { "type": "number" },
                    "max_kp": { "type": "number" },
                    "max_ki": { "type": "number" },
                    "max_kd": { "type": "number" },
                    "dry_run": { "type": "boolean" }
                },
                "required": ["port", "kp", "ki", "kd"]
            }
        }
    ])
}

fn require_mode(current: McpMode, required: McpMode, tool: &str) -> Result<(), (i64, String)> {
    if current < required {
        Err((
            -32001,
            format!(
                "permission denied: tool `{tool}` requires mode `{required:?}`, but server is running in `{current:?}` mode (pass `--allow-hardware` or `--mode hardware`)"
            ),
        ))
    } else {
        Ok(())
    }
}

/// Compute PID step-response metrics from telemetry lines containing `err=<f64>` or `sp=<f64>,pv=<f64>`.
pub fn parse_pid_telemetry_metrics(lines: &[String]) -> Value {
    let mut errors = Vec::new();
    for line in lines {
        for token in line.split(|c: char| c == ',' || c.is_whitespace()) {
            if let Some(val) = token.strip_prefix("err=") {
                if let Ok(e) = val.parse::<f64>() {
                    if e.is_finite() {
                        errors.push(e);
                    }
                }
            }
        }
    }
    if errors.is_empty() {
        return json!({ "samples": 0 });
    }
    let max_abs_error = errors.iter().map(|e| e.abs()).fold(0.0_f64, f64::max);
    let final_error = *errors.last().unwrap_or(&0.0);
    let mean_abs_error = errors.iter().map(|e| e.abs()).sum::<f64>() / (errors.len() as f64);
    json!({
        "samples": errors.len(),
        "max_abs_error": max_abs_error,
        "mean_abs_error": mean_abs_error,
        "final_error": final_error,
    })
}

fn validate_pid_gains(
    kp: f64,
    ki: f64,
    kd: f64,
    max_kp: f64,
    max_ki: f64,
    max_kd: f64,
) -> Result<String, (i64, String)> {
    if !kp.is_finite() || !ki.is_finite() || !kd.is_finite() {
        return Err((-32602, "PID gains must be finite numbers".to_string()));
    }
    if kp < 0.0 || ki < 0.0 || kd < 0.0 {
        return Err((-32602, "PID gains must be non-negative".to_string()));
    }
    if kp > max_kp || ki > max_ki || kd > max_kd {
        return Err((
            -32602,
            format!(
                "PID safety clamp exceeded: (kp={kp}, ki={ki}, kd={kd}) exceeds limits (max_kp={max_kp}, max_ki={max_ki}, max_kd={max_kd})"
            ),
        ));
    }
    Ok(format!("PID:kp={kp:.6},ki={ki:.6},kd={kd:.6}\n"))
}

fn handle_tool_call(
    params: &Value,
    mode: McpMode,
    tasks: &TaskRegistry,
) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing tool name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);

    let payload = match name {
        "boards_search" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            let matches: Vec<Value> = registry()
                .boards()
                .filter(|b| {
                    b.id.to_ascii_lowercase().contains(&query)
                        || b.mcu.to_ascii_lowercase().contains(&query)
                        || b.platform.to_ascii_lowercase().contains(&query)
                })
                .take(limit)
                .map(|b| {
                    json!({
                        "qualified_id": b.qualified_id(),
                        "mcu": b.mcu,
                        "default_framework": b.default_framework,
                        "frameworks": b.frameworks,
                        "rust_target": b.rust_target,
                    })
                })
                .collect();
            json!({ "count": matches.len(), "boards": matches })
        }
        "board_inspect" => {
            let board_id = args
                .get("board")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing board".to_string()))?;
            let b = registry()
                .resolve(board_id, None)
                .map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_value(b).map_err(|e| (-32000, e.to_string()))?
        }
        "doctor_check" => {
            let checks: Vec<Value> = check_toolchains()
                .into_iter()
                .map(|c| {
                    json!({
                        "name": c.name,
                        "installed": c.installed,
                        "path": c.path,
                        "version": c.version,
                        "install_hint": c.install_hint,
                    })
                })
                .collect();
            json!({ "toolchains": checks })
        }
        "device_list" => {
            let ports: Vec<Value> = list_serial_port_details()
                .into_iter()
                .map(|p| json!({ "port": p.path, "description": p.description }))
                .collect();
            json!({ "devices": ports })
        }
        "plan_build" => {
            let dir = args
                .get("dir")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing dir".to_string()))?;
            let env = args.get("env").and_then(Value::as_str);
            let release = args
                .get("release")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let ctx = EnvContext::load(Path::new(dir), env).map_err(|e| (-32000, e.to_string()))?;
            let (backend, steps) = plan_build(&ctx, release);
            let commands: Vec<String> = steps.iter().map(ToString::to_string).collect();
            json!({
                "env": ctx.env_name,
                "board": ctx.board.qualified_id(),
                "backend": format!("{backend:?}"),
                "commands": commands,
            })
        }
        "plan_hardened_build" => {
            let dir = args
                .get("dir")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing dir".to_string()))?;
            let env = args.get("env").and_then(Value::as_str);
            let profile_str = args
                .get("profile")
                .and_then(Value::as_str)
                .unwrap_or("release-hardened");
            let profile = match profile_str {
                "dev" => SecurityProfile::Dev,
                "hil" => SecurityProfile::Hil,
                _ => SecurityProfile::ReleaseHardened,
            };
            let passes: Vec<String> = args
                .get("obfuscation_passes")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let ctx = EnvContext::load(Path::new(dir), env).map_err(|e| (-32000, e.to_string()))?;
            let plan = plan_hardened_build(&ctx, profile, &passes);
            serde_json::to_value(plan).map_err(|e| (-32000, e.to_string()))?
        }
        "audit_firmware" => {
            let elf_path = args
                .get("elf_path")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing elf_path".to_string()))?;
            let report =
                audit_firmware_elf(Path::new(elf_path)).map_err(|e| (-32000, e.to_string()))?;
            serde_json::to_value(report).map_err(|e| (-32000, e.to_string()))?
        }
        "plan_chip_protection" => {
            let board_id = args
                .get("board")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing board".to_string()))?;
            let b = registry()
                .resolve(board_id, None)
                .map_err(|e| (-32000, e.to_string()))?;
            let plan = plan_chip_protection(b);
            serde_json::to_value(plan).map_err(|e| (-32000, e.to_string()))?
        }
        "init_project" => {
            require_mode(mode, McpMode::Build, name)?;
            let dir = args
                .get("dir")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing dir".to_string()))?;
            let board = args
                .get("board")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing board".to_string()))?;
            let framework = args.get("framework").and_then(Value::as_str);
            let res = init_project(Path::new(dir), board, framework)
                .map_err(|e| (-32000, e.to_string()))?;
            json!({
                "root": res.root,
                "board_id": res.board_id,
                "framework": res.framework,
                "manifest_path": res.manifest_path,
            })
        }
        "task_start" => {
            let action = args
                .get("action")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing action".to_string()))?;
            if action == "flash" {
                require_mode(mode, McpMode::Hardware, "task_start(flash)")?;
            } else {
                require_mode(mode, McpMode::Build, "task_start(build)")?;
            }
            let dir = args
                .get("dir")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing dir".to_string()))?;
            let env = args.get("env").and_then(Value::as_str).map(str::to_string);
            let release = args
                .get("release")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let task_id = tasks.spawn_job(action, PathBuf::from(dir), env, release);
            json!({ "task_id": task_id, "status": "running", "action": action })
        }
        "task_poll" => {
            let task_id = args
                .get("task_id")
                .and_then(Value::as_u64)
                .ok_or((-32602, "missing task_id".to_string()))?;
            let entry = tasks
                .poll(task_id)
                .ok_or((-32000, format!("unknown task_id: {task_id}")))?;
            json!({
                "id": entry.id,
                "action": entry.action,
                "status": entry.status,
                "logs": entry.logs,
                "summary": entry.summary,
            })
        }
        "task_cancel" => {
            let task_id = args
                .get("task_id")
                .and_then(Value::as_u64)
                .ok_or((-32602, "missing task_id".to_string()))?;
            let cancelled = tasks.cancel(task_id);
            json!({ "task_id": task_id, "cancelled": cancelled })
        }
        "monitor_sample" => {
            require_mode(mode, McpMode::Hardware, name)?;
            let port_name = args
                .get("port")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing port".to_string()))?;
            let baud = args.get("baud").and_then(Value::as_u64).unwrap_or(115_200) as u32;
            let duration_ms = args
                .get("duration_ms")
                .and_then(Value::as_u64)
                .unwrap_or(2_000)
                .min(15_000);
            let max_lines = args
                .get("max_lines")
                .and_then(Value::as_u64)
                .unwrap_or(100)
                .min(500) as usize;
            let until_pattern = args.get("until_pattern").and_then(Value::as_str);

            let (lines, matched_pattern) =
                sample_serial_lines(port_name, baud, duration_ms, max_lines, until_pattern)
                    .map_err(|e| (-32000, e.to_string()))?;
            let pid_metrics = parse_pid_telemetry_metrics(&lines);
            json!({
                "port": port_name,
                "lines_captured": lines.len(),
                "matched_pattern": matched_pattern,
                "pid_metrics": pid_metrics,
                "lines": lines,
            })
        }
        "pid_tune" => {
            require_mode(mode, McpMode::Hardware, name)?;
            let port_name = args
                .get("port")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing port".to_string()))?;
            let baud = args.get("baud").and_then(Value::as_u64).unwrap_or(115_200) as u32;
            let kp = args
                .get("kp")
                .and_then(Value::as_f64)
                .ok_or((-32602, "missing kp".to_string()))?;
            let ki = args
                .get("ki")
                .and_then(Value::as_f64)
                .ok_or((-32602, "missing ki".to_string()))?;
            let kd = args
                .get("kd")
                .and_then(Value::as_f64)
                .ok_or((-32602, "missing kd".to_string()))?;
            let max_kp = args.get("max_kp").and_then(Value::as_f64).unwrap_or(100.0);
            let max_ki = args.get("max_ki").and_then(Value::as_f64).unwrap_or(50.0);
            let max_kd = args.get("max_kd").and_then(Value::as_f64).unwrap_or(50.0);
            let dry_run = args
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            let frame = validate_pid_gains(kp, ki, kd, max_kp, max_ki, max_kd)?;
            if !dry_run {
                write_serial_frame(port_name, baud, &frame).map_err(|e| (-32000, e.to_string()))?;
            }
            json!({
                "port": port_name,
                "frame": frame.trim_end(),
                "dry_run": dry_run,
                "clamps": { "max_kp": max_kp, "max_ki": max_ki, "max_kd": max_kd }
            })
        }
        _ => return Err((-32601, format!("unknown tool: {name}"))),
    };

    Ok(json!({
        "content": [{ "type": "text", "text": payload.to_string() }]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mcp_permission_isolation_blocks_hardware_in_build_mode() {
        let tasks = TaskRegistry::default();
        let blocked = handle_request_with_state(
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "pid_tune",
                    "arguments": { "port": "/dev/ttyACM0", "kp": 1.0, "ki": 0.1, "kd": 0.01, "dry_run": true }
                }
            }),
            McpMode::Build,
            &tasks,
        )
        .expect("response");
        assert_eq!(blocked["error"]["code"], -32001);

        let allowed = handle_request_with_state(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "pid_tune",
                    "arguments": { "port": "/dev/ttyACM0", "kp": 1.0, "ki": 0.1, "kd": 0.01, "dry_run": true }
                }
            }),
            McpMode::Hardware,
            &tasks,
        )
        .expect("response");
        assert!(allowed.get("result").is_some());
    }

    #[test]
    fn pid_tune_rejects_gains_exceeding_safety_clamps() {
        let tasks = TaskRegistry::default();
        let over_limit = handle_request_with_state(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "pid_tune",
                    "arguments": {
                        "port": "/dev/ttyACM0",
                        "kp": 250.0,
                        "ki": 0.1,
                        "kd": 0.01,
                        "max_kp": 100.0,
                        "dry_run": true
                    }
                }
            }),
            McpMode::Hardware,
            &tasks,
        )
        .expect("response");
        assert_eq!(over_limit["error"]["code"], -32602);
    }

    #[test]
    fn pid_telemetry_parser_extracts_error_metrics() {
        let lines = vec![
            "TELEM: t=10,err=12.5".to_string(),
            "TELEM: t=20,err=-4.0".to_string(),
            "TELEM: t=30,err=0.5".to_string(),
        ];
        let metrics = parse_pid_telemetry_metrics(&lines);
        assert_eq!(metrics["samples"], 3);
        assert_eq!(metrics["max_abs_error"], 12.5);
        assert_eq!(metrics["final_error"], 0.5);
    }
}
