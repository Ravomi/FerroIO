use crate::context::EnvContext;
use crate::process::CommandSpec;
use crate::registry::BoardRecord;
use crate::{CoreError, Result};
use std::path::Path;

/// Supported hardware simulation backends for `mio sim`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimBackend {
    Renode,
    Qemu,
    Wokwi,
}

impl SimBackend {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "renode" => Ok(Self::Renode),
            "qemu" => Ok(Self::Qemu),
            "wokwi" => Ok(Self::Wokwi),
            other => Err(CoreError::UnsupportedSimBackend(other.to_string())),
        }
    }
}

/// Renode platform description bundled with Renode for the board's MCU family.
pub fn renode_platform(board: &BoardRecord) -> Option<&'static str> {
    let mcu = board.mcu.to_ascii_uppercase();
    let table: &[(&str, &str)] = &[
        ("STM32F103", "platforms/cpus/stm32f103.repl"),
        ("STM32F4", "platforms/cpus/stm32f4.repl"),
        ("STM32F746", "platforms/cpus/stm32f746.repl"),
        ("STM32L072", "platforms/cpus/stm32l072.repl"),
        ("NRF52840", "platforms/cpus/nrf52840.repl"),
        ("EFR32MG12", "platforms/cpus/efr32mg12.repl"),
        ("FE310", "platforms/cpus/sifive-fe310.repl"),
    ];
    table
        .iter()
        .find(|(prefix, _)| mcu.starts_with(prefix))
        .map(|(_, repl)| *repl)
}

/// QEMU `-machine` emulating the board's MCU (QEMU only models a few MCUs).
pub fn qemu_machine(board: &BoardRecord) -> Option<&'static str> {
    let mcu = board.mcu.to_ascii_uppercase();
    let table: &[(&str, &str)] = &[
        ("STM32F405", "netduinoplus2"),
        ("STM32F205", "netduino2"),
        ("STM32F100", "stm32vldiscovery"),
        ("NRF51", "microbit"),
        ("LM3S6965", "lm3s6965evb"),
        ("LM3S811", "lm3s811evb"),
    ];
    table
        .iter()
        .find(|(prefix, _)| mcu.starts_with(prefix))
        .map(|(_, machine)| *machine)
}

/// Build the simulator command without executing it.
pub fn plan_sim(ctx: &EnvContext, release: bool, backend: SimBackend) -> Result<CommandSpec> {
    let firmware = ctx.firmware_path(release);
    let unsupported =
        || CoreError::UnsupportedSimTarget(ctx.board.qualified_id(), ctx.board.mcu.clone());
    let spec = match backend {
        SimBackend::Renode => {
            let repl = renode_platform(&ctx.board).ok_or_else(unsupported)?;
            let script = format!(
                "mach create; machine LoadPlatformDescription @{repl}; sysbus LoadELF @{}; start",
                firmware.display()
            );
            CommandSpec::new("renode").args(["--console", "--disable-xwt", "-e", script.as_str()])
        }
        SimBackend::Qemu => {
            let machine = qemu_machine(&ctx.board).ok_or_else(unsupported)?;
            CommandSpec::new("qemu-system-arm")
                .args(["-machine", machine, "-nographic", "-kernel"])
                .path_arg(&firmware)
        }
        SimBackend::Wokwi => CommandSpec::new("wokwi-cli").path_arg(&ctx.root),
    };
    Ok(spec.cwd(&ctx.root))
}

/// Run the project's built firmware inside a hardware simulator (`renode`, `qemu`, or `wokwi`).
pub fn simulate_project(
    dir: &Path,
    env_name: Option<&str>,
    release: bool,
    backend: SimBackend,
) -> Result<()> {
    let ctx = EnvContext::load(dir, env_name)?;
    let spec = plan_sim(&ctx, release, backend)?;
    let firmware = ctx.firmware_path(release);
    if backend != SimBackend::Wokwi && !firmware.exists() {
        return Err(CoreError::ArtifactNotFound(firmware));
    }
    spec.run(CoreError::SimFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RavomConfig;

    #[test]
    fn parses_supported_sim_backends() {
        assert_eq!(SimBackend::parse("renode").ok(), Some(SimBackend::Renode));
        assert_eq!(SimBackend::parse("QEMU").ok(), Some(SimBackend::Qemu));
        assert_eq!(SimBackend::parse("wokwi").ok(), Some(SimBackend::Wokwi));
        assert!(SimBackend::parse("unknown").is_err());
    }

    #[test]
    fn renode_plan_creates_machine_for_known_mcu_and_rejects_unknown() {
        let cfg = RavomConfig::from_toml_str(
            "[project]\nname = \"blink\"\n[env.bp]\nplatform = \"ststm32\"\n\
             board = \"bluepill_f103c8\"\nframework = \"embassy\"\n\
             [env.pico]\nplatform = \"raspberrypi\"\nboard = \"pico\"\nframework = \"embassy\"\n",
        )
        .expect("manifest");
        let bp = EnvContext::from_config(Path::new("/w"), cfg.clone(), Some("bp")).expect("ctx");
        let spec = plan_sim(&bp, false, SimBackend::Renode).expect("plan");
        assert!(spec.args[3].contains("stm32f103.repl"), "{spec}");
        assert!(spec.args[3]
            .replace('\\', "/")
            .contains("sysbus LoadELF @/w/target/thumbv7m-none-eabi/debug/blink"));

        let pico = EnvContext::from_config(Path::new("/w"), cfg, Some("pico")).expect("ctx");
        assert!(matches!(
            plan_sim(&pico, false, SimBackend::Qemu),
            Err(CoreError::UnsupportedSimTarget(..))
        ));
    }
}
