# FerroIO

> **下一代 Rust 驱动的跨平台嵌入式开发工作台**  
> 融合类 PlatformIO 的统一工具链与生态管理、原生 `probe-rs` 调试烧录、可扩展插件体系，以及基于 **GPUI** 的高性能桌面端应用。

---

## 什么是 FerroIO？

**FerroIO** 是一个面向嵌入式系统的开源开发平台与桌面工作台。它旨在解决嵌入式开发中工具链碎片化、环境配置繁琐、IDE 笨重的问题，同时为 **C/C++** 与 **Rust** 嵌入式项目提供一致的开发体验。

- **统一构建与依赖管理**：使用声明式配置（`ferro.toml`，支持平滑导入 `platformio.ini`）管理开发板、框架、工具链与第三方库。
- **原生高速烧录与调试**：深度集成 `probe-rs`，开箱支持 ARM Cortex-M、RISC-V 等主流架构的烧录、RTT 实时日志与硬件调试。
- **GPUI 高性能桌面端**：基于 Zed 同款 GPU 加速 UI 框架 **GPUI** 构建，提供低延迟的工程管理、编译输出面板、串口终端与实时波形监视器。
- **插件化架构**：支持通过插件扩展芯片平台（Platform）、开发板定义（Board）、构建钩子与桌面端工具应用。

---

## 核心特性

- **多生态兼容**：同时支持 Rust (`cargo` / `embassy` / `embedded-hal`) 与传统 C/C++ (`arm-none-eabi-gcc` / `esp-idf` / Arduino) 嵌入式工程。
- **零配置工具链分发**：自动拉取并隔离管理各目标芯片所需的编译器、SDK 与调试工具。
- **Board & Registry 兼容**：支持解析与复用社区现有的开发板元数据，快速适配数百款常见开发板。
- **Core 与 UI 解耦**：既可以作为纯命令行工具（`ferro-cli`）运行在 CI/CD 与无头环境中，也可以通过 `ferro-ui` 获得完整的桌面端交互体验。

---

## 项目结构（规划）

```text
FerroIO/
├── Cargo.toml              # Workspace 配置
├── crates/
│   ├── ferro-core/         # 核心引擎：工程解析、工具链管理、构建、probe-rs 烧录、串口
│   ├── ferro-plugin/       # 插件协议与加载器（声明式配置 / WASM / 子进程 RPC）
│   ├── ferro-cli/          # 命令行工具（init / build / flash / monitor）
│   └── ferro-ui/           # GPUI 桌面应用与插件工作台
└── plugins/                # 内置平台与工具插件
```

---

## 快速开始（规划中）

```bash
# 初始化一个新工程
ferro init --board stm32f401ccu6

# 编译工程
ferro build

# 烧录固件并打开 RTT / 串口监视器
ferro flash --monitor

# 启动 GPUI 桌面工作台
ferro ui
```

---

## 路线图

- [ ] **Phase 1 — Core & CLI MVP**：完成 Workspace 初始化，打通 `init` / `build` / `flash` / `monitor` 闭环（优先支持 STM32 / ESP32）。
- [ ] **Phase 2 — 插件体系与生态兼容**：确定插件接口规范，支持导入 PlatformIO 板卡配置。
- [ ] **Phase 3 — GPUI 桌面端**：构建工程管理、编译控制台、串口/RTT 监视器与插件面板。

---

## 许可证

待定（拟采用 MIT / Apache-2.0 双许可）。
