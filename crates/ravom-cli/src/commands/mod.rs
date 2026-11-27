//! `mio` subcommand implementations, one module per area.

mod boards;
mod build;
mod device;
mod doctor;
mod mcp;
mod pkg;
mod project;
mod zephyr;

use crate::cli::{Cli, Commands};

pub type CmdResult = Result<(), Box<dyn std::error::Error>>;

/// Dispatch a parsed command line.
pub fn run(cli: Cli) -> CmdResult {
    let root = cli.project_dir;
    match cli.command {
        Commands::Init {
            board,
            framework,
            lang,
            dir,
        } => project::init(
            &dir.unwrap_or(root),
            &board,
            framework.as_deref(),
            lang.as_deref(),
        ),
        Commands::Migrate { dir } => project::migrate(&dir.unwrap_or(root)),
        Commands::Run {
            env,
            target,
            release,
        } => build::run(&root, env.as_deref(), target.as_deref(), release),
        Commands::Build {
            env,
            release,
            dry_run,
        } => build::build(&root, env.as_deref(), release, dry_run),
        Commands::Flash {
            env,
            release,
            monitor,
            dry_run,
        } => device::flash(&root, env.as_deref(), release, monitor, dry_run),
        Commands::Monitor {
            port,
            baud,
            rtt,
            env,
            release,
            list,
        } => device::monitor(&root, port, baud, rtt, env.as_deref(), release, list),
        Commands::Mcp {
            mode,
            allow_hardware,
        } => mcp::run_mcp_server(&mode, allow_hardware),
        Commands::Sim {
            env,
            backend,
            release,
        } => device::sim(&root, env.as_deref(), &backend, release),
        Commands::Deploy {
            host,
            remote_dir,
            run,
            env,
            release,
        } => device::deploy(&root, env.as_deref(), release, &host, &remote_dir, run),
        Commands::Boards {
            query,
            import,
            platform,
            into,
            check,
            normalize_chips,
        } => boards::run(query, import, &platform, into, check, normalize_chips),
        Commands::Doctor { install_target } => doctor::run(install_target.as_deref()),
        Commands::Zephyr { action } => zephyr::run(&root, action),
        Commands::Pkg { action } => pkg::run(&root, action),
        Commands::Ui => doctor::launch_ui(),
    }
}
