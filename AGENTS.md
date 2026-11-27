# RavomIO — AI Agent & Engineering Guide

This document is the authoritative reference for AI coding agents working in **RavomIO** (a Rust-first embedded build system, board registry, CLI `mio`, and **Ravomi** desktop workbench aiming to be the PlatformIO for Rust embedded development).

---

## 1. Repository Architecture (Quick Map)

- `crates/ravom-plugin`: Framework & RTOS plugin trait (`FrameworkPlugin`), `FrameworkKind` (`RustCargo`, `CCpp`, `Zephyr`), and project scaffolding templates.
- `crates/ravom-core`: Core domain logic — board registry (`registry.rs`), project init (`project.rs`), package manager (`pkg.rs`), toolchain & Rust target manager (`toolchain.rs`), build backend planning (`build.rs`), flashing (`flash.rs`), Zephyr integration (`zephyr.rs`), serial monitor (`monitor.rs`), and VOFA+ serial protocol engine (`vofa.rs`).
- `crates/ravom-cli`: CLI binary (`mio`) built with `clap` (also launches `Ravomi` via `mio ui`).
- `crates/ravom-ui`: Native monochrome desktop application **Ravomi** (`[[bin]] name = "Ravomi"`) built with `egui` / `eframe`.
- `boards/<platform>/<id>.toml`: 2,328 embedded board definitions across 39 platforms compiled into `ravom-core` via `build.rs`.
- `docs/designs/`: Architecture & optimization design documents (EGM).
- `docs/guide/` (git submodule → `Ravomi/ravomio-docs`): Official user guides (`quickstart`, `cli`, `ravom.toml`, `plugins`, `migration`). Push changes to `Ravomi/ravomio-docs` before bumping the submodule pointer here; validate links with the Paseo `docs-check` script.
- `examples/` (git submodule → `Ravomi/ravomio-examples`): Standalone MCU, Zephyr, and Linux SBC starter projects (excluded from the root Cargo workspace via `Cargo.toml`). Push changes to `Ravomi/ravomio-examples` before bumping the submodule pointer here; validate manifests with the Paseo `examples-check` script.
- `paseo.json` & `scripts/paseo-{setup,teardown}.sh`: Paseo worktree lifecycle hooks (auto-initializes git submodules and warms Cargo caches) and task scripts.

---

## 2. Rust Engineering Standards (Google Rust Style Guide)

All Rust code in this repository **MUST** follow the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html) and workspace lints in `Cargo.toml`:

1. **Error Handling & Safety**
   - **Never use `.unwrap()`** in production or library code (`clippy::unwrap_used = "warn"`). Propagate errors with `?` and `CoreError` (`thiserror`), or use `.expect("clear invariant")` only in tests.
   - **No `todo!()` or `unimplemented!()`** in committed code.
   - Minimize `unsafe`; every `unsafe fn` must have `#![warn(unsafe_op_in_unsafe_fn)]` compliance and a `// SAFETY:` comment explaining invariants.

2. **Imports, Formatting & Lints**
   - Avoid wildcard imports (`clippy::wildcard_imports = "warn"`).
   - Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` before concluding any task.
   - Keep functions focused and shallowly nested; use early returns.

3. **Documentation & Comments**
   - Every `pub` trait, struct, enum, and function must have concise `///` rustdoc comments explaining **what** it does and **why** (not restating types).
   - Explain non-obvious invariants with `//` comments.

4. **Testing**
   - Every new framework plugin, `init -> build` route, or state transition must include unit tests in `#[cfg(test)] mod tests` or integration tests under `crates/*/tests/`.

---

## 3. MCP (Model Context Protocol) Integration Guidelines

RavomIO includes a built-in stdio JSON-RPC 2.0 MCP server (`mio mcp [--mode read-only|build|hardware] [--allow-hardware]`):

- **Permission Tiers (`McpMode`)**:
  - `read-only`: `boards_search`, `board_inspect`, `doctor_check`, `device_list`, `plan_build`, `plan_hardened_build`, `audit_firmware`, `plan_chip_protection`.
  - `build` (default): All `read-only` tools plus `init_project`, `task_start(build)`, `task_poll`, `task_cancel`.
  - `hardware` (`--allow-hardware`): All `build` tools plus `task_start(flash)`, `monitor_sample` (bounded serial telemetry + PID error metrics), and `pid_tune` (clamped $K_p, K_i, K_d$ injection).
- **Stage-Separated Build & Anti-Reversing Hardening**:
  - Use `hil` profile (`plan_hardened_build` with `profile: "hil"`) during closed-loop PID tuning (`-C opt-level=s` for deterministic WCET, symbols/RTT retained, no obfuscation).
  - Use `release-hardened` profile for production (`-C strip=symbols`, `-C panic=abort`, `--remap-path-prefix`, fat LTO, and optional LLVM obfuscation passes), then verify with `audit_firmware` and `plan_chip_protection`.
- **Dry-Run / Planning First**: Prefer `plan_build` / `plan_hardened_build` / `pid_tune(dry_run=true)` before executing hardware-mutating actions. Irreversible fuses (STM32 RDP Level 2, ESP32 eFuse) are never auto-executed.

---

## 4. Agent Skill Authoring & Usage

When creating or updating `SKILL.md` workflows for RavomIO:

- **Standard Workflow Order**:
  1. Discover target board: `mio boards --search <query>` (or query `ravom_core::registry::registry()`).
  2. Initialize project: `mio init --board <platform>/<id> --framework <fw>`.
  3. Verify build plan / compile: `mio build` (ensure `select_build_backend` matches `FrameworkKind`).
  4. Flash & monitor (only when hardware/probe is attached): `mio flash` -> `mio monitor`.
- **Idempotency & Safety**: Skills must check for existing `ravom.toml` before `init` and never overwrite user source files without explicit confirmation.
