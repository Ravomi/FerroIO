# RavomIO

> **A next-generation, Rust-powered cross-platform ecosystem for embedded development.**  
> Unified toolchain & library management, native `probe-rs` flashing and debugging, extensible plugin architecture, and a fast **egui**-based desktop workbench.

---

## Why RavomIO?

Embedded development shouldn't be slowed down by fragmented toolchains, brittle vendor IDEs, or manual environment setup. Inspired by **PlatformIO**, **RavomIO** brings a modern, reproducible, and fast workflow to both **C/C++** and **Rust** embedded projects—all from a single CLI and a lightweight native desktop workbench.

- **Zero-Hassle Toolchain Management**: Automatically provisions and isolates compilers, SDKs, and debuggers per target board and framework.
- **Declarative Project Configuration**: Define boards, frameworks, build flags, and dependencies cleanly in `ravom.toml` (with smooth migration from `platformio.ini`).
- **Native High-Speed Debug & Flash**: Built-in `probe-rs` integration for ARM Cortex-M, RISC-V, and more—supporting flashing, RTT real-time logging, and hardware debugging out of the box.
- **Fast Native Desktop Workbench**: Powered by **egui** (`eframe`), delivering a responsive project manager, build console, serial/RTT monitor, and real-time waveform plotter.
- **Extensible Plugin System**: Add new silicon platforms, board definitions, build hooks, and custom UI panels through a clean plugin interface.

---

## Key Features

- **Multi-Ecosystem Support**: First-class support for both **Rust** (`cargo`, `embassy`, `embedded-hal`) and **C/C++** (`arm-none-eabi-gcc`, `esp-idf`, Arduino) embedded workflows.
- **Board & Registry Compatibility**: Reuse and parse community board metadata to support hundreds of popular development boards.
- **Decoupled Core & UI**: Run `ravom-cli` (`mio`) headlessly in CI/CD pipelines and terminals, or launch `ravom-ui` for a full interactive desktop experience.
- **Cross-Platform**: Works consistently across Linux, macOS, and Windows.

---

## Architecture

```text
RavomIO/
├── Cargo.toml              # Workspace configuration
├── crates/
│   ├── ravom-core/         # Core engine: project parsing, toolchain manager, builder, probe-rs, serial
│   ├── ravom-plugin/       # Plugin protocol & loader (declarative / WASM / RPC)
│   ├── ravom-cli/          # CLI tool `mio` (init / build / flash / monitor / boards / ui)
│   └── ravom-ui/           # Native egui desktop workbench
└── plugins/                # Built-in platform & tool plugins
```

---

## Quick Start

```bash
# List built-in supported boards
mio boards

# Initialize a new project for a target board
mio init --board stm32f401ccu6

# Build firmware
mio build

# Flash firmware and open RTT monitor
mio flash --monitor

# Launch the desktop workbench
mio ui
```

---

## Roadmap

- [x] **Phase 0 — Workspace Bootstrap**: Rust workspace setup with `ravom-core`, `ravom-plugin`, `ravom-cli` (`mio`), and `ravom-ui` (`egui`).
- [x] **Phase 1 — Core & CLI MVP**: End-to-end `mio init` / `build` / `run` / `flash` / `monitor` / `boards` workflow (verified with STM32F103C8T6 Cortex-M3 ELF build & Google Rust Style lints).
- [x] **Phase 2 — Plugin System & Ecosystem**: Unified TOML board registry (36 platforms, 2,319 boards), `ravom-plugin` framework/RTOS plugins (Embassy, RTIC, ESP-IDF, FreeRTOS, Zephyr, RT-Thread), and `mio migrate` (`platformio.ini` → `ravom.toml`).
- [x] **Phase 3 — Desktop Workbench**: `ravom-ui` (`egui`) with 2,319-board live search, non-blocking background build/flash console, serial port discovery, and real-time signal plotter.

---

## License

Dual-licensed under **MIT** or **Apache-2.0** (matching the Rust ecosystem standard).
