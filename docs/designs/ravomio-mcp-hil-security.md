# RavomIO MCP Server: Full-Loop HIL, PID Tuning, Async Tasks & Anti-Reversing Hardening

*   **Author:** Ravomi
*   **Status:** Approved
*   **Date:** 2026-11-26

--------------------------------------------------------------------------------

## Overview

RavomIO's stdio Model Context Protocol server (`mio mcp`) enables AI agents to autonomously discover boards, scaffold projects, compile firmware, flash hardware, tune control loops (such as PID controllers), and harden release binaries against reverse engineering.

To resolve the contradiction between real-time closed-loop debugging and production anti-reversing protection—while preventing MCP request timeouts and accidental hardware damage—this design introduces:
1.  **Three-tier permission isolation** (`read-only`, `build`, `hardware`).
2.  **In-process asynchronous background task queue** (`task_start`, `task_poll`, `task_cancel`) for long-running builds and flashes.
3.  **Bounded HIL telemetry sampling and guarded PID parameter injection** (`monitor_sample`, `pid_tune`, `device_list`, `doctor_check`).
4.  **Stage-separated build profiles (`dev` / `hil` / `release-hardened`) and ELF anti-reversing audit (`audit_firmware`, `plan_hardened_build`, `plan_chip_protection`)**.

## The Problem

1.  **Debug vs. Production Hardening Conflict**: Unoptimized `dev` builds (`opt-level = 0`) often overflow MCU Flash or miss real-time PID control deadlines, whereas full production hardening (symbol stripping, path remapping, LLVM obfuscation, and chip read-out protection) removes the telemetry and debug symbols required for closed-loop tuning.
2.  **Blocking Streams & Long Builds in MCP**: Standard `mio monitor` blocks indefinitely, which deadlocks request-response MCP `tools/call` invocations. Similarly, `west build` or cold Cargo cross-compilations can exceed client RPC timeouts.
3.  **Unconstrained Hardware & Fuse Mutation**: Allowing an AI agent to send arbitrary PID gains or execute irreversible option-byte/eFuse lock commands (`RDP Level 2`, ESP32 eFuse burn) risks mechanical runaway or permanently bricking development boards.

## Goals

*   **Permission-gated MCP execution**: Enforce `--mode <read-only|build|hardware>` (defaulting to `build`) so physical flash and serial write tools cannot run without explicit opt-in.
*   **Non-blocking background task execution**: Provide `task_start`, `task_poll`, and `task_cancel` tools backed by a thread-safe job table in `mio mcp`.
*   **Closed-loop HIL & guarded PID tuning**: Provide `device_list`, `monitor_sample` (bounded by `duration_ms`, `max_lines`, and `until_pattern` with automatic PID step-response metric extraction), and `pid_tune` (enforcing min/max clamping on $K_p, K_i, K_d$ before writing to serial).
*   **Anti-reversing hardening & ELF binary audit**:
    *   Separate `hil` profile (optimized for real-time PID loop timing, symbols/RTT intact, obfuscation off) from `release-hardened` profile (`-C strip=symbols`, `-C panic=abort`, `--remap-path-prefix` to scrub host filesystem paths, and optional LLVM obfuscation passes).
    *   Provide `audit_firmware` to inspect compiled ELF binaries for leaked debug sections (`.debug_*`, `.symtab`), host path strings (`/home/`, `/Users/`, `.cargo/`), and panic format strings, returning a structured hardening report.
    *   Provide `plan_chip_protection` to generate chip read-protection commands (STM32 Option Bytes RDP Level 1, ESP32 Flash Encryption) as dry-run plans without automatically executing irreversible fuse burns.

## Non-Goals

*   **Automatic execution of irreversible eFuse / RDP Level 2 locks**: MCP will only emit dry-run command plans for permanent hardware fuses.
*   **Forking or bundling a custom LLVM compiler**: LLVM obfuscation flags (`-C llvm-args=...` or custom wrapper passes) are configured through `plan_hardened_build`, using the host's installed Rust/LLVM toolchain.

## Critical User Journeys (CUJs)

### CUJ 1: Automated HIL Build -> Flash -> Bounded Telemetry & PID Tuning

*   **Given**: `mio mcp --allow-hardware` is running with an STM32 or ESP32 board attached over USB serial/probe.
*   **When**: The AI agent starts a `hil` build via `task_start`, flashes the target, samples 2,000 ms of serial telemetry via `monitor_sample`, and calls `pid_tune` with updated $K_p, K_i, K_d$ within configured clamp bounds.
*   **Then**: `monitor_sample` returns bounded serial lines plus computed step-response metrics (overshoot, peak error, steady-state error), and `pid_tune` rejects any gain exceeding safety bounds before transmitting the command frame.

### CUJ 2: Release Anti-Reversing Hardening & ELF Audit

*   **Given**: The PID loop is tuned and the user asks the AI agent to prepare a production release firmware.
*   **When**: The agent calls `plan_hardened_build` and runs `audit_firmware` on the resulting `.elf` binary.
*   **Then**: `plan_hardened_build` injects `--remap-path-prefix`, `-C strip=symbols`, `-C panic=abort`, and `-C lto=fat`; `audit_firmware` scans the ELF binary and reports whether `.symtab`, `.debug_info`, or plaintext host paths remain, along with reversible `RDP Level 1` protection commands via `plan_chip_protection`.

## Technical Plan (Architecture)

```text
mio mcp [--mode read-only | build | hardware]
  │
  ├── Tier 1: Read-Only Discovery & Security Audit (Always Enabled)
  │     ├── boards_search / board_inspect
  │     ├── doctor_check / device_list
  │     ├── plan_build / plan_hardened_build / plan_chip_protection
  │     └── audit_firmware (ELF section, symbol, and plaintext path leak scanner)
  │
  ├── Tier 2: Build & Project Scaffolding (Enabled in `build` & `hardware` modes)
  │     ├── init_project / migrate_platformio
  │     └── task_start(action="build") / task_poll / task_cancel
  │
  └── Tier 3: Physical Hardware & HIL Closed-Loop (Requires `--mode hardware` / `--allow-hardware`)
        ├── task_start(action="flash")
        ├── monitor_sample (bounded duration_ms / max_lines + PID telemetry parser)
        └── pid_tune (clamped Kp/Ki/Kd serial command injector)
```

### 1. Stage-Separated Build Profiles (`dev` vs. `hil` vs. `release-hardened`)

| Profile | Optimization | Symbols & RTT | Anti-Reversing Hardening | Chip Read Protection |
| :--- | :--- | :--- | :--- | :--- |
| `dev` | `-C opt-level=0` | Full DWARF + RTT | Off | Off |
| `hil` | `-C opt-level=s` | Symbols + RTT + PID Telemetry | Off (deterministic WCET) | Off |
| `release-hardened` | `-C opt-level=z -C lto=fat -C codegen-units=1` | Stripped (`-C strip=symbols`) | `--remap-path-prefix`, `-C panic=abort`, optional function-level LLVM obfuscation args | Reversible RDP Level 1 plan |

### 2. ELF Anti-Reversing Scanner (`audit_firmware`)

Implemented in `crates/ravom-core/src/security.rs` with zero heavy external ELF crate dependencies (parses standard ELF32/ELF64 section headers and scans raw binary bytes for leaked signatures):
*   Detects presence of `.symtab`, `.strtab`, and `.debug_*` section names in the section header string table.
*   Scans binary payload for host path leaks (`/home/`, `/Users/`, `.cargo/registry`, `src/main.rs`) and Rust panic formatting markers (`panicked at`, `core::panicking`).
*   Computes a `hardened: bool` verdict and actionable remediation flags.

### 3. Bounded HIL Telemetry & Guarded PID Tuning

*   `monitor_sample`: Opens the serial port with a strict wall-clock deadline (`Instant::now() + Duration::from_millis(duration_ms.min(15_000))`) and line cap (`max_lines.min(500)`). Parses `sp=<f64>,pv=<f64>` or `err=<f64>` tokens from lines to compute `max_abs_error`, `overshoot`, and `final_error`.
*   `pid_tune`: Validates `0.0 <= kp <= max_kp`, `0.0 <= ki <= max_ki`, `0.0 <= kd <= max_kd` (all finite `f64`). Rejects out-of-range or `NaN` values with JSON-RPC error `-32602` before opening the serial port.

## Alternatives Considered

### Alternative A: Global LLVM Control-Flow Flattening on All builds

*   **Pros**: Maximum resistance to static CFG decompilation in Ghidra/IDA.
*   **Cons**: Inflates MCU Flash usage by 2x–5x and adds unpredictable jitter to interrupt service routines and PID loops.
*   **Reason for Rejection**: Rejected by author: separating `hil` from `release-hardened` and supporting selective LLVM passes preserves real-time control loop determinism during tuning and keeps Flash size within MCU limits.

### Alternative B: Streaming Serial Monitor Over MCP Notifications

*   **Pros**: Continuous real-time stream.
*   **Cons**: Most MCP clients only consume synchronous `tools/call` responses and ignore unsolicited server notifications, causing buffer overflow or hangs.
*   **Reason for Rejection**: Rejected by author: bounded `monitor_sample` windows (e.g., 1–5 seconds per tuning iteration) work reliably across every MCP client.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

### Target Files to Create/Modify

#### 1. `crates/ravom-core/src/security.rs` & `crates/ravom-core/src/lib.rs` (Create / Modify)

*   **Type / Scope**: Firmware anti-reversing planner, ELF binary auditor, and chip protection planner
*   **Proposed Changes**:
    *   Define `SecurityProfile` (`Dev`, `Hil`, `ReleaseHardened`), `HardenedBuildPlan`, `FirmwareAuditReport`, and `ChipProtectionPlan`.
    *   Implement `plan_hardened_build(ctx: &EnvContext, profile: SecurityProfile, llvm_obfuscation_passes: &[String]) -> HardenedBuildPlan` preserving existing `ctx.env.rustflags` (such as `-C link-arg=-Tlink.x`) and appending profile/hardening flags joined by `\x1f` in `CARGO_ENCODED_RUSTFLAGS`.
    *   Implement `audit_firmware_elf(path: &Path) -> Result<FirmwareAuditReport>` (using `CoreError::Io`) and pure `audit_firmware_bytes(bytes: &[u8])` with bounds-safe slice window scanning for `.debug_*`, `.symtab`, host path prefixes, and panic strings.
    *   Implement `plan_chip_protection(board: &BoardRecord) -> ChipProtectionPlan` covering `ststm32` (RDP L1 vs. irreversible L2), `espressif*` (flash encryption vs. irreversible eFuse), `nordicnrf52` (`APPROTECT`), and `atmelavr` (`avrdude` lock bits).
    *   Add unit tests verifying `plan_hardened_build` flags, `audit_firmware_bytes` leak detection vs. clean binaries, and `plan_chip_protection`.

#### 2. `crates/ravom-cli/src/cli.rs` & `crates/ravom-cli/src/commands/mod.rs` (Modify)

*   **Type / Scope**: CLI flags for `mio mcp` permission isolation
*   **Proposed Changes**:
    *   Add both `--mode <read-only|build|hardware>` (defaulting to `build`) and `--allow-hardware` (which upgrades the effective mode to `Hardware`) to `Commands::Mcp` in `crates/ravom-cli/src/cli.rs` and forward them to `mcp::run_mcp_server`.

#### 3. `crates/ravom-cli/src/commands/mcp.rs` & `crates/ravom-core/src/monitor.rs` (Modify)

*   **Type / Scope**: Full-featured MCP server implementation & bounded serial helpers
*   **Proposed Changes**:
    *   Add `McpMode` (`ReadOnly`, `Build`, `Hardware`) permission check before tool dispatch, while preserving backwards-compatible `handle_request(&Value)` delegating to `handle_request_with_state`.
    *   Add thread-safe background `TaskRegistry` with `Arc<AtomicBool>` cancellation state supporting `task_start` (`build` / `flash`), `task_poll`, and `task_cancel`.
    *   Add read-only & security tools: `board_inspect`, `doctor_check`, `device_list`, `plan_hardened_build`, `audit_firmware`, `plan_chip_protection`.
    *   Add HIL & PID tools (gated by `McpMode::Hardware`): `monitor_sample` (backed by `ravom_core::monitor::sample_serial_lines` and pure `parse_pid_telemetry_metrics`) and `pid_tune` (validating default bounds `max_kp=100.0, max_ki=50.0, max_kd=50.0`, formatting `PID:kp=<kp:.6>,ki=<ki:.6>,kd=<kd:.6>\n`, and supporting `dry_run: bool`).
    *   Add unit tests covering permission isolation rejections, PID safety clamp validation, and telemetry metric parsing.

### Verification Plan

*   **Static Analysis & Linting**: `cargo clippy --workspace --all-targets -- -D warnings`
*   **Unit Tests**: Verify `crates/ravom-core/src/security.rs` and `crates/ravom-cli/src/commands/mcp.rs` unit tests cover all MCP tools, permission guards, PID bounds checks, and ELF anti-reversing audits.

--------------------------------------------------------------------------------

*Drafted with EGM*
