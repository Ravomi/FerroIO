# rtt-hello-stm32f103

## Overview
Hardware-in-the-loop (HIL) smoke firmware fixture for `bluepill_f103c8` (`thumbv7m-none-eabi`). Prints a startup marker and periodic ticks over RTT so the host can verify flashing, reset, and RTT attach on a physical board.

## Interface
- `main() -> !` (`src/main.rs`) — `#![no_std]` / `#![no_main]` `cortex_m_rt::entry` loop printing `RAVOM-HIL-OK board=bluepill_f103c8` and periodic `tick <n>` messages via `rtt_target::rprintln!`.

## Invariants
- Must remain `#![no_std]` and `#![no_main]` with `panic_halt` and `cortex-m-rt` entry.
- Startup RTT output must include `RAVOM-HIL-OK board=bluepill_f103c8` for HIL marker matching.

## Side Effects
- Initializes RTT control block in target RAM and writes log frames over SWD/RTT.

## Verification
- Run: `cargo test -p ravom-cli --test hil`
