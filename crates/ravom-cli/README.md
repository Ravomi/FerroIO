# ravom-cli

## Overview
Binary crate (`mio`) providing the RavomIO embedded development CLI, parsing command-line arguments with `clap` and dispatching subcommands to `ravom-core` and `ravom-plugin` workflows.

## Interface
- Binary `mio` (`src/main.rs`)
- `cli::Cli` (`project_dir: PathBuf`, `command: Commands`)
- `cli::Commands` (`Init`, `Run`, `Build`, `Flash`, `Monitor`, `Boards`, `Migrate`, `Sim`, `Deploy`, `Doctor`, `Ui`)
- `commands::CmdResult` (`Result<(), Box<dyn std::error::Error>>`)
- `commands::run(cli: Cli) -> CmdResult`
- Submodules: `commands::boards`, `commands::build`, `commands::device`, `commands::doctor`, `commands::project`

## Invariants
- All subcommands receive `project_dir` (defaulting to `.`) as the root directory containing `ravom.toml` unless overridden by command-specific flags.
- `main()` exits with `ExitCode::SUCCESS` on `Ok(())` and `ExitCode::FAILURE` on `Err`, printing `error: {err}` to stderr.

## Side Effects
- Reads and writes project files (`ravom.toml`, board registries) and invokes external embedded toolchains (`probe-rs`, `openFPGALoader`, serial/RTT monitors, SSH deploy, UI launcher) via subcommand modules.

## Verification
- Run: `cargo test -p ravom-cli`
