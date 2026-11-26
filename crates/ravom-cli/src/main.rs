mod cli;
mod commands;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    match commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
