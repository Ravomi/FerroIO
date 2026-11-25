use clap::{Parser, Subcommand};
use ravom_core::config::builtin_boards;
use ravom_core::project::init_project;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "mio", version, about = "RavomIO embedded development CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new RavomIO project (PlatformIO-style layout)
    Init {
        /// Target board ID (e.g. stm32f401ccu6, esp32s3, rp2040)
        #[arg(short, long)]
        board: String,
        /// Target framework (defaults to board's default)
        #[arg(short, long)]
        framework: Option<String>,
        /// Project directory (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
    /// Build firmware for the current project
    Build {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
    },
    /// Flash firmware via probe-rs
    Flash {
        /// Environment name from ravom.toml
        #[arg(short, long)]
        env: Option<String>,
        /// Flash release build artifact
        #[arg(short, long)]
        release: bool,
        /// Open RTT monitor after flashing (`probe-rs run`)
        #[arg(short, long)]
        monitor: bool,
    },
    /// Open serial/RTT monitor
    Monitor {
        /// Serial port path
        #[arg(short, long)]
        port: Option<String>,
        /// Baud rate
        #[arg(short, long, default_value_t = 115200)]
        baud: u32,
    },
    /// List built-in supported boards
    Boards {
        /// Optional filter query
        query: Option<String>,
    },
    /// Launch the RavomIO egui desktop workbench
    Ui,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init {
            board,
            framework,
            dir,
        } => {
            let res = init_project(&dir, &board, framework.as_deref())?;
            println!(
                "Initialized RavomIO project at {} (board: {}, framework: {})",
                res.root.display(),
                res.board_id,
                res.framework
            );
        }
        Commands::Build { env, release } => {
            let res = ravom_core::build::build_project(
                &PathBuf::from("."),
                env.as_deref(),
                release,
            )?;
            println!(
                "Built environment '{}' (board: {}, target: {}, release: {})",
                res.env_name,
                res.board_id,
                res.target.as_deref().unwrap_or("host"),
                res.release
            );
        }
        Commands::Flash {
            env,
            release,
            monitor,
        } => {
            let res = ravom_core::flash::flash_project(
                &PathBuf::from("."),
                env.as_deref(),
                release,
                monitor,
            )?;
            println!(
                "Flashed environment '{}' to {} ({})",
                res.env_name,
                res.chip,
                res.firmware.display()
            );
        }
        Commands::Monitor { port, baud } => {
            println!(
                "mio monitor (port: {}, baud: {baud}) — serial/RTT monitor not yet implemented (Phase 1)",
                port.as_deref().unwrap_or("auto")
            );
        }
        Commands::Boards { query } => {
            let q = query.unwrap_or_default().to_lowercase();
            println!("{:<16} {:<12} {:<14} {}", "ID", "PLATFORM", "MCU", "NAME");
            for b in builtin_boards() {
                if q.is_empty()
                    || b.id.to_lowercase().contains(&q)
                    || b.name.to_lowercase().contains(&q)
                    || b.mcu.to_lowercase().contains(&q)
                {
                    println!("{:<16} {:<12} {:<14} {}", b.id, b.platform, b.mcu, b.name);
                }
            }
        }
        Commands::Ui => {
            println!("Run `cargo run -p ravom-ui` to launch the RavomIO desktop workbench.");
        }
    }
    Ok(())
}
