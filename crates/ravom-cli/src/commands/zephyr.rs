use super::CmdResult;
use crate::cli::ZephyrAction;
use ravom_core::context::EnvContext;
use ravom_core::zephyr::{
    default_zephyr_workspace, detect_zephyr_env, plan_zephyr_menuconfig, plan_zephyr_setup,
    resolve_zephyr_board, setup_zephyr, write_kconfig_fragment, ZephyrSetupOptions,
};
use ravom_core::CoreError;
use std::path::Path;

pub fn run(root: &Path, action: ZephyrAction) -> CmdResult {
    match action {
        ZephyrAction::Status { env } => status(root, env.as_deref()),
        ZephyrAction::Setup {
            dir,
            version,
            sdk_version,
            rust,
            skip_sdk,
            dry_run,
        } => {
            let workspace_dir = dir
                .or_else(default_zephyr_workspace)
                .ok_or("could not determine default Zephyr workspace directory; pass --dir")?;
            let opts = ZephyrSetupOptions {
                workspace_dir,
                zephyr_version: version,
                sdk_version,
                enable_rust: rust,
                skip_sdk,
            };
            if dry_run {
                println!(
                    "# Zephyr setup plan (workspace: {}, zephyr: {}, sdk: {}, rust: {})",
                    opts.workspace_dir.display(),
                    opts.zephyr_version,
                    if opts.skip_sdk {
                        "skipped"
                    } else {
                        opts.sdk_version.as_str()
                    },
                    opts.enable_rust
                );
                for step in plan_zephyr_setup(&opts) {
                    println!("{step}");
                }
                return Ok(());
            }
            setup_zephyr(&opts)?;
            println!(
                "Provisioned Zephyr workspace at {}",
                opts.workspace_dir.display()
            );
            Ok(())
        }
        ZephyrAction::Menuconfig { env, dry_run } => {
            let ctx = EnvContext::load(root, env.as_deref())?;
            if dry_run {
                let cmd = plan_zephyr_menuconfig(&ctx);
                println!("{cmd}");
                return Ok(());
            }
            write_kconfig_fragment(&ctx, false)?;
            plan_zephyr_menuconfig(&ctx).run(CoreError::BuildFailed)?;
            Ok(())
        }
    }
}

fn status(root: &Path, env_name: Option<&str>) -> CmdResult {
    let ctx = EnvContext::load(root, env_name).ok();
    let zcfg = ctx.as_ref().and_then(|c| c.env.zephyr.as_ref());
    let zenv = detect_zephyr_env(zcfg);

    println!("Zephyr environment:");
    println!("  west:        {}", zenv.west_bin);
    println!(
        "  workspace:   {}",
        zenv.workspace_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(not configured)".to_string())
    );
    println!(
        "  ZEPHYR_BASE: {}",
        zenv.zephyr_base
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(missing — run `mio zephyr setup` or set ZEPHYR_BASE)".to_string())
    );
    println!(
        "  Zephyr SDK:  {}",
        zenv.sdk_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| {
                "(missing — run `mio zephyr setup` or set ZEPHYR_SDK_INSTALL_DIR)".to_string()
            })
    );
    if let Some(ctx) = ctx {
        println!(
            "  project env: {} (board: {} -> zephyr_board: {})",
            ctx.env_name,
            ctx.board.qualified_id(),
            resolve_zephyr_board(&ctx.env, &ctx.board)
        );
    }
    Ok(())
}
