use crate::context::EnvContext;
use crate::process::CommandSpec;
use crate::{CoreError, Result};
use std::path::{Path, PathBuf};

pub use crate::context::resolve_firmware_path;

/// Result of a firmware flash invocation.
#[derive(Debug, Clone)]
pub struct FlashResult {
    pub env_name: String,
    pub chip: String,
    pub firmware: PathBuf,
    pub monitor: bool,
}

/// A resolved flash operation: which artifact goes to which chip with which command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashPlan {
    pub chip: String,
    pub artifact: PathBuf,
    pub command: CommandSpec,
    /// Optional follow-up command (e.g. `probe-rs reset` after `probe-rs download`
    /// so the newly flashed firmware starts running immediately).
    pub post_command: Option<CommandSpec>,
    pub monitor: bool,
}

/// Build the flash command without executing it.
///
/// * MCU boards: `probe-rs download --chip <chip> <elf>` followed by
///   `probe-rs reset --chip <chip>`, or `probe-rs run --chip <chip> <elf>` when
///   `monitor` is true (or `west flash` for Zephyr).
pub fn plan_flash(ctx: &EnvContext, release: bool, monitor: bool) -> Result<FlashPlan> {
    let protocol = ctx.env.upload_protocol.as_deref().unwrap_or("probe-rs");
    if protocol == "west" {
        let artifact = ctx.firmware_path(release);
        let command = crate::zephyr::plan_zephyr_flash(ctx);
        return Ok(FlashPlan {
            chip: crate::zephyr::resolve_zephyr_board(&ctx.env, &ctx.board),
            artifact,
            command,
            post_command: None,
            monitor: false,
        });
    }
    if !matches!(
        protocol,
        "probe-rs" | "stlink" | "jlink" | "cmsis-dap" | "custom"
    ) {
        return Err(CoreError::UnsupportedUploadProtocol(protocol.to_string()));
    }

    let chip = ctx.board.probe_rs_chip.clone().ok_or_else(|| {
        CoreError::MissingProbeRsChip(ctx.board.qualified_id(), ctx.board.mcu.clone())
    })?;
    let artifact = ctx.firmware_path(release);
    let probe_selector = std::env::var("RAVOM_PROBE")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut command = CommandSpec::new("probe-rs").args([
        if monitor { "run" } else { "download" },
        "--chip",
        chip.as_str(),
    ]);
    if let Some(ref selector) = probe_selector {
        command = command.args(["--probe", selector.as_str()]);
    }
    let command = command.path_arg(&artifact).cwd(&ctx.root);

    let post_command = if monitor {
        None
    } else {
        let mut reset = CommandSpec::new("probe-rs").args(["reset", "--chip", chip.as_str()]);
        if let Some(ref selector) = probe_selector {
            reset = reset.args(["--probe", selector.as_str()]);
        }
        Some(reset.cwd(&ctx.root))
    };

    Ok(FlashPlan {
        chip,
        artifact,
        command,
        post_command,
        monitor,
    })
}

/// Flash firmware to the target board (`probe-rs run` if `monitor`, else `download`).
pub fn flash_project(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    monitor: bool,
) -> Result<FlashResult> {
    flash_project_impl(dir, env_name, release, monitor, None)
}

/// Like [`flash_project`] (without RTT) but streams tool output into `log`.
pub fn flash_project_with_log(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    log: &mut dyn FnMut(&str),
) -> Result<FlashResult> {
    flash_project_impl(dir, env_name, release, false, Some(log))
}

fn flash_project_impl(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    monitor: bool,
    log: Option<&mut dyn FnMut(&str)>,
) -> Result<FlashResult> {
    let ctx = EnvContext::load(dir, env_name)?;
    let plan = plan_flash(&ctx, release, monitor)?;
    if !plan.artifact.exists() {
        return Err(CoreError::ArtifactNotFound(plan.artifact));
    }
    match log {
        Some(sink) => {
            sink(&format!("$ {}", plan.command));
            plan.command.run_streaming(CoreError::FlashFailed, sink)?;
            if let Some(post) = &plan.post_command {
                sink(&format!("$ {post}"));
                post.run_streaming(CoreError::FlashFailed, sink)?;
            }
        }
        None => {
            plan.command.run(CoreError::FlashFailed)?;
            if let Some(post) = &plan.post_command {
                post.run(CoreError::FlashFailed)?;
            }
        }
    }
    Ok(FlashResult {
        env_name: ctx.env_name,
        chip: plan.chip,
        firmware: plan.artifact,
        monitor: plan.monitor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RavomConfig;

    fn ctx(manifest: &str) -> EnvContext {
        let cfg = RavomConfig::from_toml_str(manifest).expect("manifest parses");
        EnvContext::from_config(Path::new("/tmp/demo"), cfg, None).expect("env resolves")
    }

    #[test]
    fn resolves_debug_and_release_firmware_paths() {
        let root = Path::new("/tmp/demo");
        assert_eq!(
            resolve_firmware_path(root, "blink", Some("thumbv7em-none-eabihf"), false),
            PathBuf::from("/tmp/demo/target/thumbv7em-none-eabihf/debug/blink")
        );
        assert_eq!(
            resolve_firmware_path(root, "blink", Some("thumbv6m-none-eabi"), true),
            PathBuf::from("/tmp/demo/target/thumbv6m-none-eabi/release/blink")
        );
    }

    #[test]
    fn plans_probe_rs_download_and_run() {
        let c = ctx(
            "[project]\nname = \"blink\"\n[env.bp]\nplatform = \"ststm32\"\n\
                     board = \"bluepill_f103c8\"\nframework = \"embassy\"\n",
        );
        let plan = plan_flash(&c, true, false).expect("plan");
        assert_eq!(plan.command.program, "probe-rs");
        assert_eq!(
            &plan.command.args[..3],
            ["download", "--chip", "STM32F103C8"]
        );
        assert_eq!(
            plan.artifact,
            PathBuf::from("/tmp/demo/target/thumbv7m-none-eabi/release/blink")
        );
        let post = plan
            .post_command
            .as_ref()
            .expect("download should schedule a post-flash reset");
        assert_eq!(post.program, "probe-rs");
        assert_eq!(post.args, ["reset", "--chip", "STM32F103C8"]);
        let run = plan_flash(&c, false, true).expect("plan");
        assert_eq!(run.command.args[0], "run");
        assert!(run.post_command.is_none());
    }

    #[test]
    fn rejects_boards_without_probe_rs_support() {
        let c =
            ctx("[env.uno]\nplatform = \"atmelavr\"\nboard = \"uno\"\nframework = \"arduino\"\n");
        let err = plan_flash(&c, false, false).expect_err("AVR is not supported by probe-rs");
        assert!(matches!(err, CoreError::MissingProbeRsChip(..)));
    }

    #[test]
    fn missing_artifact_is_reported_before_invoking_probe_rs() {
        let dir = tempfile::tempdir().expect("tempdir");
        crate::project::init_project(dir.path(), "bluepill_f103c8", None).expect("init");
        let err = flash_project(dir.path(), None, false, false).expect_err("no ELF yet");
        assert!(matches!(err, CoreError::ArtifactNotFound(_)));
    }
}
