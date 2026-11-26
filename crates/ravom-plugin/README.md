# ravom-plugin

## Overview
Plugin protocol and built-in framework/RTOS/HDL registry for RavomIO (supporting Embassy, RTIC, ESP-IDF, FreeRTOS, Zephyr, RT-Thread, bare-metal, RustHDL, Spade, Veryl, and Mirui).

## Interface
- `pub trait FrameworkPlugin: Send + Sync` (`id(&self) -> &'static str`, `display_name(&self) -> &'static str`, `is_rtos(&self) -> bool`, `starter_main_rs(&self, board_id: &str) -> String`)
- `pub fn builtin_framework_plugins() -> Vec<&'static dyn FrameworkPlugin>`
- `pub fn find_framework_plugin(id: &str) -> Option<&'static dyn FrameworkPlugin>`

## Invariants
- `FrameworkPlugin` implementations must be `Send + Sync`.
- `find_framework_plugin` performs case-insensitive ASCII matching against built-in framework IDs.
- Generated `starter_main_rs` templates target `#![no_std]` and `#![no_main]` with a panic handler and `_start` entry point.

## Side Effects
- None (pure in-memory static registry and string template generation).

## Verification
- Run: `cargo test -p ravom-plugin`
