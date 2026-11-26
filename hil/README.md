# Hardware-in-the-loop (HIL) bench

Static checks prove that `mio` *calls* `probe-rs` correctly; only real boards
prove that flashing, reset and RTT actually work. This directory holds the
bench definition and the firmware that is flashed during HIL runs.

```text
hil/
├── boards.toml                     # bench matrix: board id, fixture, RTT marker, probe selector
└── fixtures/
    └── rtt-hello-stm32f103/        # cortex-m-rt + rtt-target firmware printing RAVOM-HIL-OK
```

## Running locally

```bash
rustup target add thumbv7m-none-eabi
cargo install probe-rs-tools --locked
# Plug in the board + probe listed in hil/boards.toml, then:
RAVOM_HIL_BOARDS=all cargo test -p ravom-cli --test hil -- --ignored --nocapture
```

The test builds each fixture with `mio build --release`, flashes it with
`mio flash --release --monitor` (`probe-rs run`, probe pinned via `RAVOM_PROBE`)
and passes when the fixture's marker appears on RTT within `timeout_secs`.

Without hardware, `cargo test -p ravom-cli --test hil` still checks that the
bench matrix references real boards/fixtures and that every fixture
cross-compiles.

## Self-hosted runner

1. Register a GitHub Actions runner with labels `self-hosted, hil`.
2. Install probe-rs udev rules (`mio doctor` warns when they are missing).
3. Attach the boards from `boards.toml`; `probe` must match `probe-rs list`.
4. `.github/workflows/hil.yml` runs nightly and on demand.

## Adding a board

1. Create `fixtures/<name>/` (copy `rtt-hello-stm32f103`, adjust `memory.x`,
   HAL crate and `ravom.toml`).
2. Add a `[[board]]` entry to `boards.toml`.
3. `cargo test -p ravom-cli --test hil` must pass (fixture compiles).

## Subdirectories & Boundaries
- `rtt-hello-stm32f103`: [./fixtures/rtt-hello-stm32f103/README.md](./fixtures/rtt-hello-stm32f103/README.md) — `cortex-m-rt` + `rtt-target` smoke firmware printing `RAVOM-HIL-OK board=bluepill_f103c8`.
