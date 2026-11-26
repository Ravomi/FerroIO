<div align="center">

<img src="./docs/assets/ravomio-icon.jpg" alt="RavomIO Logo" width="128" />

# RavomIO

**The PlatformIO for Rust Embedded & FPGA Development**  
Unified board & toolchain management, native `probe-rs` flashing, **AI-native MCP server (HIL + PID tuning + anti-reversing)**, and a fast **egui** desktop workbench.

[![CI](https://github.com/Ravomi/RavomIO/actions/workflows/ci.yml/badge.svg)](https://github.com/Ravomi/RavomIO/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)](https://www.rust-lang.org/)
[![Boards](https://img.shields.io/badge/boards-2%2C340%2B%20across%2044%20platforms-brightgreen.svg)](#feature-status)

[Quick Start](#quick-start) · [Features](#key-features) · [AI & MCP](#ai--mcp-integration) · [Architecture](#architecture) · [Roadmap](#roadmap) · [Community](#community)

</div>

---

## Why RavomIO?

Embedded development shouldn't be slowed down by fragmented toolchains, brittle vendor IDEs, or manual environment setup. Inspired by **PlatformIO**, **RavomIO** brings a reproducible, Rust-first workflow to **MCU**, **Embedded Linux**, **C/C++**, and **Open-Source FPGA** projects—from a single CLI (`mio`) and a lightweight native desktop workbench (`ravom-ui`).

| Pillar | What You Get |
| :--- | :--- |
| **2,340+ Built-in Boards** | 44 platforms compiled into the binary and validated in CI (`1,225+` verified `probe-rs` targets), overridable via `~/.ravom/boards` or `$RAVOM_BOARDS_DIR`. |
| **Declarative `ravom.toml`** | Define boards, frameworks, flags, and Zephyr Kconfig/overlays in one manifest—with `mio migrate` from `platformio.ini`. |
| **Native `probe-rs` & Serial** | Fast MCU flashing, RTT real-time logging, cross-platform serial monitor, and live waveform plotter. |
| **AI-Native MCP & Security** | Built-in `mio mcp` stdio server with 3 permission tiers, async background builds, bounded HIL/PID tuning, and stage-separated anti-reversing hardening. |
| **Fast `egui` Workbench** | Native desktop GUI (`eframe`) with instant board search, non-blocking build/flash console, and live signal plotting. |

---

## Quick Start

```bash
# 1. Discover boards and validate the embedded registry
mio boards --search stm32f4
mio boards --check

# 2. Initialize a new project (Rust Embassy, RTIC, Zephyr, Arduino, Linux SBC, or FPGA)
mio init --board stm32f401ccu6

# 3. Build firmware (add --dry-run to inspect exact toolchain commands)
mio build

# 4. Flash via probe-rs and attach RTT monitor
mio flash --monitor

# 5. Launch the native egui desktop workbench
mio ui

# 6. Start the stdio MCP server for AI agents (default: build mode; add --allow-hardware for HIL/PID)
mio mcp --allow-hardware
```

---

## AI & MCP Integration

RavomIO is designed from the ground up to work seamlessly with AI coding agents (see [AGENTS.md](./AGENTS.md) for the Google Rust Style, MCP, and Skill contract):

- **Three-Tier Permission Isolation (`McpMode`)**:
  - `mio mcp --mode read-only`: Board discovery (`boards_search`, `board_inspect`), toolchain diagnostics (`doctor_check`), USB device discovery (`device_list`), dry-run build/security plans, and ELF binary audits.
  - `mio mcp --mode build` *(default)*: Adds non-interactive scaffolding (`init_project`) and asynchronous background task management (`task_start`, `task_poll`, `task_cancel`).
  - `mio mcp --allow-hardware`: Enables physical flashing, bounded serial telemetry sampling (`monitor_sample` with automatic PID error metrics), and clamped PID gain injection (`pid_tune`).
- **Stage-Separated Anti-Reversing Hardening**:
  - **`hil` profile**: Optimized (`-C opt-level=s`) for deterministic real-time PID loop timing while keeping symbols and RTT telemetry intact.
  - **`release-hardened` profile**: Applies `-C strip=symbols`, `-C panic=abort`, `--remap-path-prefix`, fat LTO, and optional LLVM obfuscation passes—verified with `audit_firmware` and paired with dry-run chip read-protection plans (`plan_chip_protection`).

---

## Key Features

- **Multi-Ecosystem Support**: **Rust** (`cargo`, `embassy`, `rtic`, `embedded-hal`, [`mirui`](https://github.com/W-Mai/mirui) `no_std` embedded UI), **Embedded Linux SBC** (`aarch64`/`armv7` with `rust-lld`), **Zephyr RTOS** (C & Rust via `west`), **C/C++** (`arm-none-eabi-gcc`, `avr-gcc`, `esp-idf`), and **Open-Source FPGA** (`rust-hdl`, `spade`, `veryl`, `verilog` + `yosys` / `openFPGALoader`).
- **Decoupled Core & UI**: Run `ravom-cli` (`mio`) headlessly in CI/CD pipelines and terminals, or launch `ravom-ui` for a full interactive desktop experience.
- **Cross-Platform**: Tested in CI across **Linux**, **macOS**, and **Windows**.

---

## Feature Status

| Area | Status | Notes |
| :--- | :--- | :--- |
| Board registry (`mio boards`) | **Stable** | 2,340+ boards embedded at build time, overridable via `~/.ravom/boards` / `$RAVOM_BOARDS_DIR`, `mio boards --check` validation |
| `platformio.ini` migration | **Stable** | `[env]` inheritance, `extends`, `${section.option}` / `${this.*}` / `${sysenv.*}` interpolation |
| Rust build (`cargo`) | **Stable** | Real cross-compilation covered by tests (`thumbv7m-none-eabi`) + Linux SBC `std` support |
| Zephyr RTOS (`west`, C & Rust) | **Beta** | `mio init --framework zephyr [--lang rust]`, `ravom.toml` Kconfig/DeviceTree/Shields, `west build`, `mio zephyr setup/status/menuconfig` |
| `probe-rs` flash / RTT | **Beta** | Command plans unit-tested, chip names validated against `probe-rs`; physical-board runs via `hil/` |
| Serial monitor | **Beta** | Cross-platform `serialport` backend (Linux / macOS / Windows) |
| MCP Server (`mio mcp`) | **Beta** | Stdio JSON-RPC 2.0, 3 permission tiers, async background tasks, bounded HIL telemetry, clamped PID tuning |
| Anti-Reversing & ELF Audit | **Beta** | `dev` / `hil` / `release-hardened` profiles + `audit_firmware` ELF leak scanner + `plan_chip_protection` |
| C/C++ (`arm-none-eabi-gcc`, `avr-gcc`, `esp-idf`) | **Experimental** | Plugin-driven `init -> build` routing without spurious `Cargo.toml` |
| FPGA (`yosys` → bitstream) | **Experimental** | Synthesis wired; `nextpnr` place-and-route + packing planned |
| Simulators (Renode / QEMU / Wokwi) | **Experimental** | Supported for MCUs with known Renode/QEMU machines |

---

## Architecture

```text
RavomIO/
├── Cargo.toml              # Workspace configuration & Google Rust lints
├── AGENTS.md               # AI agent engineering guide (Google Rust Style, MCP & Skill contract)
├── crates/
│   ├── ravom-core/         # Core engine: registry, security/ELF audit, build / flash / monitor / zephyr
│   ├── ravom-plugin/       # FrameworkPlugin trait, FrameworkKind & project scaffolding templates
│   ├── ravom-cli/          # CLI binary `mio` (subcommands + stdio MCP server + E2E/HIL tests)
│   └── ravom-ui/           # Native egui desktop workbench (app / panels / tasks / console / plotter)
├── boards/                 # 2,340+ board definitions (<platform>/<id>.toml), embedded at build time
├── docs/                   # EGM design documents & visual assets
├── hil/                    # Hardware-in-the-loop bench matrix + firmware fixtures
└── scripts/                # probe-rs target snapshot + PlatformIO board sync tooling
```

---

## Roadmap

- [x] **Phase 0 — Workspace Bootstrap**: Rust workspace setup with `ravom-core`, `ravom-plugin`, `ravom-cli` (`mio`), and `ravom-ui` (`egui`).
- [x] **Phase 1 — Core & CLI MVP**: End-to-end `mio init` / `build` / `run` / `flash` / `monitor` / `boards` workflow.
- [x] **Phase 2 — Plugin System & Ecosystem**: Unified TOML board registry (44 platforms, 2,340+ boards), `ravom-plugin` (`FrameworkKind` + scaffolding), and `mio migrate`.
- [x] **Phase 3 — Desktop Workbench & AI-Native MCP**: `ravom-ui` (`egui`) workbench, `mio mcp` (HIL + PID tuning + anti-reversing), and `AGENTS.md`.

### Upcoming TODO

- [ ] **Hardware-in-the-Loop Runs**: Wire a physical bench (STM32, ESP32, RP2040/RP2350, STEP-MXO2-LPC, Tang Nano) to the HIL runner.
- [ ] **C/C++ Build Pipeline**: Linker scripts, startup code, and Arduino core management; `esp-idf` env bootstrap.
- [ ] **FPGA Place & Route**: `nextpnr-{ice40,ecp5,gowin}` + `icepack` / `ecppack` / `gowin_pack`.
- [ ] **Bootable Rust Templates**: `cortex-m-rt` + auto-generated `memory.x` + `defmt` templates.
- [ ] **Toolchain Provisioning**: Automatic download and pinning of `arm-none-eabi-gcc`, `probe-rs`, and `oss-cad-suite` under `.ravom/`.

---

## Acknowledgments

RavomIO stands on the shoulders of the open-source embedded, FPGA, and Rust communities:

- **[PlatformIO](https://github.com/platformio)** & community platform maintainers (**[CommunityGD32Cores/platform-gd32](https://github.com/CommunityGD32Cores/platform-gd32)**, **[Community-PIO-CH32V/platform-ch32v](https://github.com/Community-PIO-CH32V/platform-ch32v)**, **[maxgerhardt/platform-raspberrypi](https://github.com/maxgerhardt/platform-raspberrypi)**)
- **[probe-rs](https://github.com/probe-rs/probe-rs)** — Rust-native MCU flashing, debugging, and RTT logging
- **[egui / eframe](https://github.com/emilk/egui)** — immediate-mode GUI powering `ravom-ui`
- **[mirui](https://github.com/W-Mai/mirui)**, **[Embassy](https://github.com/embassy-rs/embassy)**, **[RTIC](https://github.com/rtic-rs/rtic)**, **[RustHDL](https://github.com/samitbasu/rust-hdl)**, **[Yosys](https://github.com/YosysHQ/yosys)**, **[nextpnr](https://github.com/YosysHQ/nextpnr)**, **[openFPGALoader](https://github.com/trabucayre/openFPGALoader)**, **[Renode](https://github.com/renode/renode)**, **[QEMU](https://www.qemu.org/)**, and **[Wokwi](https://wokwi.com/)**

---

## Subdirectories & System Boundaries
- `crates`: [./crates/README.md](./crates/README.md) — Workspace crates (`ravom-core`, `ravom-plugin`, `ravom-cli`, `ravom-ui`).
- `hil`: [./hil/README.md](./hil/README.md) — Hardware-in-the-loop bench matrix and firmware fixtures (`./hil/fixtures/rtt-hello-stm32f103/README.md`).
- `scripts`: [./scripts/README.md](./scripts/README.md) — PlatformIO board sync and `probe-rs` target snapshot maintenance scripts.

## Global Invariants
- Board definitions in `boards/` are embedded into `ravom-core` at build time and validated against `crates/ravom-core/data/probe-rs-targets.txt` via `mio boards --check`.
- `ravom-core` and `ravom-plugin` remain UI-agnostic; both `ravom-cli` (`mio`) and `ravom-ui` delegate project, build, flash, and monitor operations to `ravom-core`.

## Global Side Effects
- Reads/writes project manifests (`ravom.toml`, `platformio.ini`) and build artifacts on disk.
- Invokes external toolchains (`cargo`, `probe-rs`, `openFPGALoader`, `arm-none-eabi-gcc`, `avr-gcc`, `yosys`, simulators, SSH) and accesses host serial ports.

## System Verification
- Run: `cargo test`

---

## Community

- **QQ 交流群**: [点击加入【RavomIO交流群】](https://qm.qq.com/q/hZVzfJmAVy)

<img src="./assets/qq-group-qr.jpg" width="220" alt="RavomIO QQ 交流群二维码" />

---

## License

Dual-licensed under **MIT** or **Apache-2.0** (matching the Rust ecosystem standard).
