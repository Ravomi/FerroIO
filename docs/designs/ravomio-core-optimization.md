# RavomIO Core Pipeline, Plugin Lifecycle & Registry Optimization

*   **Author:** Ravomi
*   **Status:** Draft
*   **Date:** 2026-11-26

--------------------------------------------------------------------------------

## Overview

RavomIO is a unified embedded and FPGA build system, board registry, and GUI workbench supporting over 2,340 boards across 44 platforms. While the workspace compiles cleanly and passes unit tests, four structural gaps exist between project initialization, framework plugins, board registry loading, and the Zephyr integration module:

1.  **Broken `init` to `build` routing for non-Cargo frameworks**: `init_project` unconditionally generates `Cargo.toml` and `src/main.rs` for every framework except Zephyr, causing `select_build_backend` to misroute Arduino, ESP-IDF, C/C++, and HDL projects to `BuildBackend::Cargo`.
2.  **Disconnected `ravom-plugin` trait**: `FrameworkPlugin` only emits a placeholder `src/main.rs` string without generating framework dependencies, linker scripts, or non-Rust source layouts, while `ravom-core` bypasses the plugin trait via hardcoded string matches.
3.  **Redundant runtime board registry parsing**: `ravom-ui` invokes `load_all_boards()` directly on startup, re-parsing all 2,340+ embedded TOML strings instead of sharing the process-wide `OnceLock<BoardRegistry>` in `ravom-core`.
4.  **Monolithic `zephyr.rs` and scattered board mappings**: `crates/ravom-core/src/zephyr.rs` mixes configuration types, a hardcoded `known_zephyr_board` lookup table, project scaffolding, `west` command planning, and SDK management in an 816-line file.

This design resolves these four gaps to make project initialization and build routing consistent across all supported frameworks, eliminate redundant startup TOML parsing in the GUI, and split `zephyr.rs` into focused submodules.

## The Problem

### 1. Project Initialization Breaks Non-Cargo Build Routing

In `crates/ravom-core/src/project.rs`, `init_project` branches only on `if is_zephyr`. For every other framework—including `arduino`, `esp-idf`, `stm32cube`, `cmsis`, `c`, `cpp`, `veryl`, and `spade`—it writes a `Cargo.toml` and `src/main.rs` into the project root.

However, in `crates/ravom-core/src/build.rs`, `select_build_backend` checks `if !dir.join("Cargo.toml").exists()` before selecting `BuildBackend::EspIdf` or `BuildBackend::ArmGcc`. As a result, running `mio init --board atmelavr/uno --framework arduino` followed by `mio build` routes the Arduino project to `cargo build` instead of the C/C++ compiler toolchain.

### 2. `ravom-plugin` Does Not Drive Project Layout or Dependencies

`crates/ravom-plugin/src/lib.rs` defines `FrameworkPlugin` with a single behavior method (`starter_main_rs(&self, board_id: &str) -> String`). It has no concept of:
*   Primary source language or project layout (Rust Cargo crate vs. C/C++ source tree vs. HDL top module).
*   Manifest dependencies or build files required by a framework.
*   Build backend selection, which is duplicated as string `match` expressions in `crates/ravom-core/src/build.rs`.

### 3. Duplicate Runtime TOML Deserialization in `ravom-ui`

`crates/ravom-core/src/registry.rs` embeds 2,340+ board TOML files via `include!(concat!(env!("OUT_DIR"), "/embedded_boards.rs"))` and caches the parsed `BoardRegistry` in a static `OnceLock<BoardRegistry>` accessed through `ravom_core::registry::registry()`. However, `RavomApp::new()` in `crates/ravom-ui/src/app.rs` calls `load_all_boards()` directly, forcing a full re-parse of every embedded TOML string and filesystem overlay on UI initialization.

### 4. Monolithic `zephyr.rs` and Unused UI Lint Suppressions

`crates/ravom-core/src/zephyr.rs` has grown to 816 lines combining data models (`ZephyrConfig`, `ZephyrLang`), hardcoded board ID translations (`known_zephyr_board`), Kconfig/overlay file generation, `west` command planning, and workspace/SDK installation. In addition, `crates/ravom-ui/src/icons.rs` retains a file-level `#[allow(dead_code)]` suppression, and `.github/workflows/ci.yml` lacks Cargo build caching across Linux, macOS, and Windows runners.

## Goals

*   **Consistent `init -> build` lifecycle across all frameworks**: Ensure `init_project` generates framework-appropriate project files (Rust `Cargo.toml` + `src/main.rs` only for Rust frameworks; `src/main.c` or `src/main.cpp` for C/C++/Arduino/ESP-IDF; `top.v` or HDL sources for FPGA frameworks) so `select_build_backend` routes every initialized project to its intended `BuildBackend`.
*   **Unify framework metadata and scaffolding in `ravom-plugin`**: Extend `FrameworkPlugin` so each plugin declares its source kind / build backend family and generates the complete initial file set for its framework, removing duplicated framework string matching from `project.rs` and `build.rs`.
*   **Eliminate duplicate board TOML parsing**: Make `BoardIndex` in `ravom-ui` read from the cached `ravom_core::registry::registry()` (or only reload when explicitly requested) rather than calling `load_all_boards()` unconditionally.
*   **Modularize `zephyr.rs` and clean up workspace hygiene**: Split `crates/ravom-core/src/zephyr.rs` into focused submodules under `crates/ravom-core/src/zephyr/` (each under 350 lines), remove `#[allow(dead_code)]` in `crates/ravom-ui/src/icons.rs`, and add unit tests for `ravom-plugin` and `ravom-ui` state transitions.

## Non-Goals

*   **Dynamic third-party plugin loading**: Loading external `.so`/`.dll`/`.dylib` plugins or running out-of-process RPC plugins at runtime is out of scope. All framework plugins remain statically compiled into `ravom-plugin`.
*   **Breaking changes to `ravom.toml` schema**: Existing `ravom.toml` files and CLI flags must continue to parse and function without migration.
*   **Bundling external compiler toolchains**: Automatically downloading `arm-none-eabi-gcc`, `yosys`, or `idf.py` is out of scope; RavomIO plans and invokes external tools installed on the host.

## Open Questions

*   **Question 1 (Board Registry Storage Format) — Resolved**: Keep raw TOML embedding in `build.rs` and reuse the process-wide `OnceLock<BoardRegistry>` in `ravom-ui` (see Alternative B).
*   **Question 2 (Plugin Trait Boundary) — Resolved**: `FrameworkPlugin` owns project scaffolding metadata and `FrameworkKind` classification only; `ravom-core` retains `CommandSpec` execution planning.

## Critical User Journeys (CUJs)

### CUJ 1: Initializing and Building a Non-Cargo Embedded Project (Arduino / C / ESP-IDF)

*   **Given**: A user runs `mio init --board atmelavr/uno --framework arduino` (or `--board espressif32/esp32dev --framework esp-idf`) in an empty directory.
*   **When**: The user inspects the generated directory and runs `mio build --dry-run` (or `plan_build`).
*   **Then**: The project directory contains `ravom.toml` and `src/main.c` (or `src/main.cpp`) without a spurious `Cargo.toml`, and `select_build_backend` selects `BuildBackend::ArmGcc` (or `BuildBackend::EspIdf`) instead of `BuildBackend::Cargo`.

### CUJ 2: Initializing and Building an FPGA HDL Project

*   **Given**: A user runs `mio init --board lattice_ice40/icestick --framework verilog` (or `rust-hdl`).
*   **When**: The user runs `mio build`.
*   **Then**: For pure Verilog/HDL frameworks, `init_project` scaffolds `top.v` without an empty Rust binary crate; for `rust-hdl`, it scaffolds both `Cargo.toml`/`src/main.rs` and `top.v`, and `plan_build` emits the `yosys` synthesis command step.

### CUJ 3: Launching `ravom-ui` Workbench and Toggling Project Settings

*   **Given**: A user launches `ravom-ui` in a workspace with 2,340+ embedded boards.
*   **When**: `RavomApp::new()` initializes `BoardIndex` and the user switches boards or toggles Zephyr Kconfig options.
*   **Then**: `RavomApp` populates `BoardIndex` from the shared `BoardRegistry` cache without re-parsing embedded TOML strings a second time, and all state transitions are covered by automated unit tests.

## Technical Plan (Architecture)

### 1. Plugin-Driven Project Scaffolding & Build Routing

To eliminate the mismatch between `init_project` and `select_build_backend`, `ravom-plugin` becomes the single source of truth for framework layout and backend classification:

*   **`FrameworkKind` enum in `ravom-plugin`**:
    *   `RustCargo`: Generates `Cargo.toml`, `.cargo/config.toml`, and `src/main.rs` (used by `embassy`, `rtic`, `bare-metal`, `mirui`).
    *   `CCpp`: Generates `src/main.c` or `src/main.cpp` without `Cargo.toml` (used by `arduino`, `stm32cube`, `cmsis`, `esp-idf`, `freertos`, `rt-thread`).
    *   `Zephyr`: Delegates to `zephyr::scaffold_zephyr_project` (used by `zephyr`, `zephyr-rust`).
    *   `FpgaHdl { uses_cargo: bool }`: Generates `top.v` (and `Cargo.toml` + `src/main.rs` only when `uses_cargo` is `true`, such as `rust-hdl`; `spade` and `veryl` omit `Cargo.toml`).
*   **`FrameworkPlugin::scaffold_files(&self, project_name: &str, board_id: &str, rust_target: Option<&str>) -> Vec<(String, String)>`**: Returns the relative file paths and contents for `init_project` to write, replacing the hardcoded `Cargo.toml` + `src/main.rs` block in `crates/ravom-core/src/project.rs`.
*   **`select_build_backend` alignment**: `crates/ravom-core/src/build.rs` queries `ravom_plugin::find_framework_plugin(&fw)` for its `FrameworkKind` first, falling back to filesystem markers (`Cargo.toml`, `src/main.c`, `src/main.cpp`) only when no built-in plugin matches. For `FrameworkKind::CCpp`, `select_build_backend` inspects `board.platform` to route `atmelavr` to `BuildBackend::AvrGcc`, `espressif32`/`esp-idf` to `BuildBackend::EspIdf`, and ARM platforms to `BuildBackend::ArmGcc`.

```text
mio init --board <B> --framework <F>
  │
  ▼
ravom_plugin::find_framework_plugin(F)
  ├── RustCargo      ──▶ write Cargo.toml, .cargo/config.toml, src/main.rs ──▶ BuildBackend::Cargo
  ├── CCpp           ──▶ write src/main.c (or .cpp), NO Cargo.toml         ──▶ BuildBackend::ArmGcc / EspIdf
  ├── Zephyr         ──▶ zephyr::scaffold_zephyr_project(...)              ──▶ BuildBackend::Zephyr
  └── FpgaHdl        ──▶ write top.v (+ Cargo.toml if rust-hdl)            ──▶ BuildBackend::FpgaSynthesis
```

### 2. Shared `BoardRegistry` Cache in `ravom-ui`

*   `ravom_core::registry::registry()` already initializes a process-wide `OnceLock<BoardRegistry>` on first access.
*   `RavomApp::new()` in `crates/ravom-ui/src/app.rs` will call `ravom_core::registry::registry().all().cloned().collect()` (or `BoardIndex::from_registry(registry())`) instead of `load_all_boards()`, removing the duplicate parse of 2,340+ embedded TOML files at UI startup.

### 3. Modularizing `crates/ravom-core/src/zephyr.rs`

Split the 816-line `zephyr.rs` into a directory module `crates/ravom-core/src/zephyr/` with clear boundaries while preserving all `pub` re-exports from `zephyr/mod.rs` so callers (`project.rs`, `build.rs`, `flash.rs`, `ravom-cli`, `ravom-ui`) require zero import changes:

*   `zephyr/mod.rs`: `ZephyrConfig`, `ZephyrLang`, `is_zephyr_framework`, `is_zephyr_rust`, and public re-exports.
*   `zephyr/boards.rs`: `known_zephyr_board` lookup table and board target resolution.
*   `zephyr/scaffold.rs`: `scaffold_zephyr_project`, Kconfig fragment (`ravom.conf`) and overlay generation.
*   `zephyr/west.rs`: `west build` / `west flash` command planning and workspace/SDK management.

### 4. Workspace Hygiene & CI Caching

*   Remove `#[allow(dead_code)]` in `crates/ravom-ui/src/icons.rs` by pruning unused icon constants or wiring them into the UI.
*   Add `Swatinem/rust-cache@v2` to `.github/workflows/ci.yml` for Linux, macOS, and Windows jobs.
*   Extract pure state-transition helpers in `RavomApp` (e.g., `toggle_zephyr_kconfig`, `env_arg`) into testable methods and add unit tests in `ravom-ui` and `ravom-plugin`.

## Alternatives Considered

### Alternative A: Keep `init_project` Hardcoded in `ravom-core` with `if/match` Branches

*   **Pros**: Requires no changes to the `FrameworkPlugin` trait in `ravom-plugin`.
*   **Cons**: Continues duplicating framework classification logic across `project.rs`, `build.rs`, and `ravom-plugin/src/lib.rs`. Adding a new framework still requires editing three separate crates.
*   **Reason for Rejection**: Rejected by author: centralizing layout and backend metadata in `ravom-plugin` eliminates drift between `init` and `build` without adding runtime complexity.

### Alternative B: Compile-Time Static `BoardRecord` Code Generation in `build.rs`

*   **Pros**: Eliminates all runtime TOML parsing at startup (`OnceLock` initialization becomes instant).
*   **Cons**: Increases `ravom-core/build.rs` complexity, requires `serde`/`toml` as build-dependencies, and slows down incremental compilation whenever a board TOML is touched.
*   **Reason for Rejection**: Rejected by author: reusing the existing `OnceLock<BoardRegistry>` in `ravom-ui` already cuts duplicate parsing by 50% with a one-line change; compile-time codegen can be revisited later if cold-start profiling shows it is needed.

### Alternative C: Dynamic Plugin Loading via Shared Libraries (`.so` / `.dll`)

*   **Pros**: Allows third-party frameworks to be installed without recompiling `ravom-cli` or `ravom-ui`.
*   **Cons**: Introduces Rust ABI instability, cross-platform dynamic loading hazards, and distribution complexity.
*   **Reason for Rejection**: Rejected by author: RavomIO's target frameworks are well-known and statically compiled plugins are simpler, safer, and easier to test.

## Non-Functional Requirements

*   **Backward Compatibility**: All existing `ravom.toml` manifests and `ravom_core::zephyr::*` public paths remain unchanged.
*   **Performance**: `ravom-ui` startup avoids a second full pass over 2,340+ TOML files by reading from `ravom_core::registry::registry()`.
*   **CI Build Time**: Adding `Swatinem/rust-cache@v2` caches `target/` and Cargo registry artifacts across Linux, macOS, and Windows runners.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

### Target Files to Create/Modify

#### 1. `crates/ravom-plugin/src/lib.rs` (Modify)

*   **Type / Scope**: Plugin trait & built-in framework registry
*   **Proposed Changes**:
    *   Add `pub enum FrameworkKind { RustCargo, CCpp, Zephyr, FpgaHdl { uses_cargo: bool } }`.
    *   Extend `FrameworkPlugin` with `fn kind(&self) -> FrameworkKind` and `fn scaffold_files(&self, project_name: &str, board_id: &str, rust_target: Option<&str>) -> Vec<(String, String)>` (returning relative path + file contents).
    *   Map `embassy`, `rtic`, `bare-metal`, `mirui` to `RustCargo`; `arduino`, `stm32cube`, `cmsis`, `esp-idf`, `freertos`, `rt-thread` to `CCpp`; `zephyr`, `zephyr-rust` to `Zephyr`; `rust-hdl` to `FpgaHdl { uses_cargo: true }`; `spade`, `veryl`, `verilog` to `FpgaHdl { uses_cargo: false }`.
    *   Add unit tests covering `kind()` and `scaffold_files()` for each `FrameworkKind` variant.

#### 2. `crates/ravom-core/src/project.rs` (Modify)

*   **Type / Scope**: Project initialization
*   **Proposed Changes**:
    *   In `init_project`, look up `ravom_plugin::find_framework_plugin(&fw)` and branch on `FrameworkKind` instead of only `if is_zephyr`.
    *   For `Zephyr`, continue calling `scaffold_zephyr_project`. For all other kinds, write the files returned by `plugin.scaffold_files(...)` (or fallback `RustCargo` files if unknown) so non-Cargo frameworks do not receive a spurious `Cargo.toml`.
    *   Add unit tests verifying `init_project` with `arduino` (`atmelavr/uno`) and `verilog`/`veryl` does not create `Cargo.toml`.

#### 3. `crates/ravom-core/src/build.rs` (Modify)

*   **Type / Scope**: Build backend selection
*   **Proposed Changes**:
    *   Update `select_build_backend` to check `ravom_plugin::find_framework_plugin(&fw)` first, mapping `FrameworkKind::FpgaHdl` -> `BuildBackend::FpgaSynthesis`, `FrameworkKind::Zephyr` -> `BuildBackend::Zephyr`, `FrameworkKind::CCpp` -> `BuildBackend::EspIdf` (for `esp-idf`/`espidf`), `BuildBackend::AvrGcc` (when `board.platform == "atmelavr"`), or `BuildBackend::ArmGcc` (for ARM targets), and `FrameworkKind::RustCargo` -> `BuildBackend::Cargo`.
    *   Retain filesystem marker checks (`Cargo.toml`, `src/main.c`, `src/main.cpp`) as fallback when no plugin matches.
    *   Add unit tests verifying `init_project` + `select_build_backend` round-trips for `arduino`, `esp-idf`, `embassy`, and `veryl`.

#### 4. `crates/ravom-core/src/zephyr.rs` -> `crates/ravom-core/src/zephyr/` (Delete & Create)

*   **Type / Scope**: Module split (refactor only, no behavior change)
*   **Proposed Changes**:
    *   Split `crates/ravom-core/src/zephyr.rs` (816 lines) into:
        *   `crates/ravom-core/src/zephyr/mod.rs`: `ZephyrConfig`, `ZephyrLang`, `is_zephyr_framework`, `is_zephyr_rust`, and `pub use` re-exports.
        *   `crates/ravom-core/src/zephyr/boards.rs`: `known_zephyr_board` mapping table.
        *   `crates/ravom-core/src/zephyr/scaffold.rs`: `scaffold_zephyr_project`, Kconfig/overlay generation.
        *   `crates/ravom-core/src/zephyr/west.rs`: `west` command planning and workspace/SDK management, plus existing unit tests.
    *   Delete `crates/ravom-core/src/zephyr.rs`.

#### 5. `crates/ravom-ui/src/app.rs` & `crates/ravom-ui/src/icons.rs` (Modify)

*   **Type / Scope**: UI startup performance, lint cleanup, and state tests
*   **Proposed Changes**:
    *   In `crates/ravom-ui/src/app.rs`, replace `load_all_boards()` in `RavomApp::new()` with `ravom_core::registry::registry().all().cloned().collect()` so the shared `OnceLock` cache (which already includes embedded boards plus `~/.ravom/boards` and `$RAVOM_BOARDS_DIR` overlays) is reused.
    *   Add unit tests in `crates/ravom-ui/src/app.rs` for `env_arg()` and `toggle_zephyr_kconfig()`.
    *   In `crates/ravom-ui/src/icons.rs`, remove `#[allow(dead_code)]` and prune any unused icon definitions.

#### 6. `.github/workflows/ci.yml` (Modify)

*   **Type / Scope**: CI pipeline
*   **Proposed Changes**:
    *   Add `uses: Swatinem/rust-cache@v2` after `dtolnay/rust-toolchain@stable` in both `check-and-test` and `cross-platform` jobs.

### Verification Plan

*   **Formatting**: `cargo fmt --all -- --check`
*   **Static Analysis & Linting**: `cargo clippy --workspace --all-targets -- -D warnings`
*   **Unit & Integration Tests**: `cargo test --workspace` (verifying all existing tests plus new `ravom-plugin`, `init -> build` round-trip, and `ravom-ui` state tests pass)
*   **Board Registry Validation**: `cargo run -q -p ravom-cli -- boards --check`
