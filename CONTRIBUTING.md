# Contributing to RavomIO

Thank you for helping build **RavomIO** (`mio`)! Contributions of all kinds—new board definitions, framework plugins, bug fixes, and documentation—are welcome.

## 1. Adding a New Board (No Rust Required!)

All boards are declarative TOML files under `boards/<platform>/<board_id>.toml`.

1. Create `boards/<platform>/<board_id>.toml`:
   ```toml
   id = "my_board"
   name = "Vendor My Board (MCU)"
   vendor = "Vendor"
   platform = "ststm32"
   mcu = "STM32F411CEU6"
   cpu = "cortex-m4"
   default_framework = "embassy"
   frameworks = ["embassy", "rtic"]
   probe_rs_chip = "STM32F411CEUx"
   rust_target = "thumbv7em-none-eabihf"
   flash_bytes = 524288
   ram_bytes = 131072
   ```
2. Verify it loads cleanly:
   ```bash
   cargo run -p ravom-cli -- boards my_board
   ```
3. Open a Pull Request!

## 2. Contributing Rust Code

RavomIO follows the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html):
- Avoid `.unwrap()` in production code (propagate `Result` or use `.expect("reason")`).
- Avoid glob imports (`use foo::*;`) outside preludes and unit tests.
- Run the full check suite before opening a PR:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```

## 3. Adding Framework / RTOS / FPGA Plugins

See `crates/ravom-plugin/src/lib.rs` to implement `FrameworkPlugin` and register your framework.
