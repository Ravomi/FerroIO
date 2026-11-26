use super::CmdResult;
use ravom_core::context::EnvContext;
use ravom_core::deploy::deploy_ssh;
use ravom_core::flash::{flash_project, plan_flash};
use ravom_core::monitor::{list_serial_port_details, monitor_rtt, monitor_serial};
use ravom_core::sim::{simulate_project, SimBackend};
use std::path::Path;

const DEFAULT_BAUD: u32 = 115_200;

pub fn flash(
    root: &Path,
    env: Option<&str>,
    release: bool,
    monitor: bool,
    dry_run: bool,
) -> CmdResult {
    if dry_run {
        let ctx = EnvContext::load(root, env)?;
        let plan = plan_flash(&ctx, release, monitor)?;
        println!("{}", plan.command);
        if let Some(post) = &plan.post_command {
            println!("{post}");
        }
        return Ok(());
    }
    let res = flash_project(root, env, release, monitor)?;
    println!(
        "Flashed environment '{}' to {} ({})",
        res.env_name,
        res.chip,
        res.firmware.display()
    );
    Ok(())
}

pub fn monitor(
    root: &Path,
    port: Option<String>,
    baud: Option<u32>,
    rtt: bool,
    env: Option<&str>,
    release: bool,
    list: bool,
) -> CmdResult {
    if list {
        let ports = list_serial_port_details();
        if ports.is_empty() {
            println!("No serial ports detected.");
        }
        for p in ports {
            println!("{:<24} {}", p.path, p.description);
        }
        return Ok(());
    }
    if rtt {
        monitor_rtt(root, env, release)?;
        return Ok(());
    }
    let baud = baud
        .or_else(|| {
            EnvContext::load(root, env)
                .ok()
                .and_then(|ctx| ctx.env.monitor_speed)
        })
        .unwrap_or(DEFAULT_BAUD);
    let selected = port.or_else(|| {
        list_serial_port_details()
            .into_iter()
            .next()
            .map(|p| p.path)
    });
    let Some(port) = selected else {
        return Err("no serial port found; specify --port <PATH> or use --rtt".into());
    };
    println!("Opening serial monitor on {port} @ {baud} baud (Ctrl+C to exit)...");
    monitor_serial(&port, baud)?;
    Ok(())
}

pub fn sim(root: &Path, env: Option<&str>, backend: &str, release: bool) -> CmdResult {
    simulate_project(root, env, release, SimBackend::parse(backend)?)?;
    Ok(())
}

pub fn deploy(
    root: &Path,
    env: Option<&str>,
    release: bool,
    host: &str,
    remote_dir: &str,
    run: bool,
) -> CmdResult {
    let dest = deploy_ssh(root, env, release, host, remote_dir, run)?;
    println!("Deployed binary to {dest}");
    Ok(())
}
