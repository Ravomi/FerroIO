use super::CmdResult;
use ravom_core::build::{build_project, plan_build, BuildResult};
use ravom_core::context::EnvContext;
use ravom_core::flash::flash_project;
use std::path::Path;

fn report(res: &BuildResult) {
    println!(
        "Built environment '{}' (board: {}, backend: {:?}, target: {}, release: {})",
        res.env_name,
        res.board_id,
        res.backend,
        res.target.as_deref().unwrap_or("host"),
        res.release
    );
}

pub fn build(root: &Path, env: Option<&str>, release: bool, dry_run: bool) -> CmdResult {
    if dry_run {
        let ctx = EnvContext::load(root, env)?;
        let (backend, steps) = plan_build(&ctx, release);
        println!("# env '{}' backend {backend:?}", ctx.env_name);
        for step in steps {
            println!("{step}");
        }
        return Ok(());
    }
    report(&build_project(root, env, release)?);
    Ok(())
}

pub fn run(root: &Path, env: Option<&str>, target: Option<&str>, release: bool) -> CmdResult {
    report(&build_project(root, env, release)?);
    match target {
        None => {}
        Some("upload" | "flash") => {
            let res = flash_project(root, env, release, false)?;
            println!(
                "Uploaded environment '{}' to {} ({})",
                res.env_name,
                res.chip,
                res.firmware.display()
            );
        }
        Some(other) => {
            return Err(format!("unknown run target '{other}' (expected: upload)").into())
        }
    }
    Ok(())
}
