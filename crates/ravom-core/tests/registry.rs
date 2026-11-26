//! Validates every board definition shipped in `boards/` (embedded at build time).

use ravom_core::chips::is_known_probe_rs_target;
use ravom_core::registry::{BoardRegistry, KNOWN_RUST_TARGETS};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn every_embedded_board_parses_and_validates() {
    let reg = BoardRegistry::embedded();
    let issues: Vec<String> = reg.issues().iter().map(ToString::to_string).collect();
    assert!(
        issues.is_empty(),
        "{} board registry issue(s):\n{}",
        issues.len(),
        issues.join("\n")
    );
    assert!(reg.len() >= 2300, "only {} boards embedded", reg.len());
}

#[test]
fn probe_rs_chips_and_rust_targets_are_valid() {
    let reg = BoardRegistry::embedded();
    for b in reg.boards() {
        if let Some(chip) = &b.probe_rs_chip {
            assert!(
                is_known_probe_rs_target(chip),
                "{}: {chip}",
                b.qualified_id()
            );
        }
        if let Some(t) = &b.rust_target {
            assert!(
                KNOWN_RUST_TARGETS.contains(&t.as_str()),
                "{}: {t}",
                b.qualified_id()
            );
        }
    }
    let flashable = reg.boards().filter(|b| b.probe_rs_chip.is_some()).count();
    assert!(flashable >= 1000, "only {flashable} probe-rs boards");
}

#[test]
fn qualified_ids_are_unique_and_platforms_cover_all_ecosystems() {
    let reg = BoardRegistry::embedded();
    let mut seen = BTreeSet::new();
    let mut per_platform: BTreeMap<&str, usize> = BTreeMap::new();
    for b in reg.boards() {
        assert!(
            seen.insert(b.qualified_id()),
            "duplicate {}",
            b.qualified_id()
        );
        *per_platform.entry(b.platform.as_str()).or_default() += 1;
    }
    for platform in [
        "ststm32",
        "espressif32",
        "raspberrypi",
        "nordicnrf52",
        "lattice_ice40",
        "gowin_fpga",
    ] {
        assert!(
            per_platform.contains_key(platform),
            "missing platform {platform}"
        );
    }
}

#[test]
fn well_known_boards_resolve_to_expected_probe_rs_targets() {
    let reg = BoardRegistry::embedded();
    let cases = [
        ("ststm32/bluepill_f103c8", Some("STM32F103C8")),
        ("stm32f401ccu6", Some("STM32F401CC")),
        ("raspberrypi/pico", Some("RP2040")),
        ("esp32s3", Some("esp32s3")),
        ("atmelavr/uno", None),
    ];
    for (id, chip) in cases {
        let board = reg
            .resolve(id, None)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(board.probe_rs_chip.as_deref(), chip, "{id}");
    }
}
