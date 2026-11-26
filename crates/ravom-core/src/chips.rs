//! Mapping of vendor MCU names onto `probe-rs` target names.
//!
//! PlatformIO board manifests describe MCUs with ordering codes
//! (`stm32f411ceu6`), J-Link device names (`STM32H747XI_M7`), or family names.
//! `probe-rs` only accepts the target variant names from its own database
//! (`STM32F411CEUx`, `STM32H747XI`, `RP2040`). Passing an unknown name makes
//! `probe-rs download` fail at runtime on real hardware, so the board registry
//! resolves names against a checked-in snapshot of `probe-rs chip list`.

use std::collections::HashSet;
use std::sync::OnceLock;

/// Snapshot of `probe-rs chip list` (regenerate with `scripts/update-probe-rs-targets.sh`).
const PROBE_RS_TARGETS: &str = include_str!("../data/probe-rs-targets.txt");

/// Maximum number of trailing ordering-code characters (package, temperature
/// range, revision; e.g. NXP `DVL6A`) that may follow a matched `probe-rs` variant name.
const MAX_SUFFIX_LEN: usize = 5;

/// Maximum number of characters a `probe-rs` variant may add beyond a short requested name.
const MAX_EXTENSION_LEN: usize = 2;

/// Requested names at least this long are specific enough to accept longer
/// package-qualified variants (`LPC54608J512` -> `LPC54608J512BD208`).
const SPECIFIC_NAME_LEN: usize = 8;
const MAX_SPECIFIC_EXTENSION_LEN: usize = 6;

/// Return every target variant from the bundled `probe-rs` snapshot.
pub fn probe_rs_targets() -> &'static [&'static str] {
    static TARGETS: OnceLock<Vec<&'static str>> = OnceLock::new();
    TARGETS.get_or_init(|| {
        PROBE_RS_TARGETS
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    })
}

/// Version of `probe-rs` the bundled target snapshot was generated from.
pub fn probe_rs_snapshot_version() -> Option<&'static str> {
    PROBE_RS_TARGETS
        .lines()
        .filter_map(|l| l.strip_prefix("# probe-rs "))
        .map(str::trim)
        .next()
}

fn uppercase_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        probe_rs_targets()
            .iter()
            .map(|t| t.to_ascii_uppercase())
            .collect()
    })
}

/// Whether `name` is accepted verbatim (case-insensitively) by `probe-rs`.
pub fn is_known_probe_rs_target(name: &str) -> bool {
    uppercase_set().contains(&name.to_ascii_uppercase())
}

/// Strip J-Link / PlatformIO core selectors such as `_M7`, `_M4`, `_M0_0`, `_M33_0`.
fn strip_core_suffix(name: &str) -> &str {
    let upper = name.to_ascii_uppercase();
    if let Some(idx) = upper.rfind("_M") {
        let tail = &upper[idx + 2..];
        if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit() || c == '_') {
            return &name[..idx];
        }
    }
    name
}

/// Score how well `variant` describes `requested`; `None` if incompatible.
///
/// A lowercase `x` on either side is a wildcard (`STM32F401CCUx`, `nRF52840_xxAA`).
fn match_score(variant: &str, requested: &str) -> Option<i64> {
    let v: Vec<char> = variant.chars().collect();
    let r: Vec<char> = requested.chars().collect();
    let common = v.len().min(r.len());
    let mut exact = 0i64;
    for i in 0..common {
        let (vc, rc) = (v[i], r[i]);
        if vc == 'x' || rc == 'x' {
            continue;
        }
        if !vc.eq_ignore_ascii_case(&rc) {
            return None;
        }
        exact += 1;
    }
    if r.len() >= v.len() {
        let suffix = r.len() - v.len();
        if suffix > MAX_SUFFIX_LEN {
            return None;
        }
        Some(exact * 10 - suffix as i64)
    } else {
        let extension = v.len() - r.len();
        let limit = if r.len() >= SPECIFIC_NAME_LEN {
            MAX_SPECIFIC_EXTENSION_LEN
        } else {
            MAX_EXTENSION_LEN
        };
        if extension > limit {
            return None;
        }
        Some(exact * 10 - 5 * extension as i64)
    }
}

/// Resolve the best `probe-rs` target for an MCU given candidate names
/// (for example the J-Link device name and the PlatformIO `build.mcu`).
///
/// Returns the canonical spelling from the `probe-rs` database, or `None`
/// when `probe-rs` has no matching target (AVR, 8051, ESP8266, ...).
pub fn resolve_probe_rs_target<'a>(
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut best: Option<(i64, &'static str)> = None;
    for raw in candidates {
        let requested = strip_core_suffix(raw.trim());
        if requested.len() < 3 {
            continue;
        }
        for &variant in probe_rs_targets() {
            if variant.eq_ignore_ascii_case(requested) {
                return Some(variant.to_string());
            }
            if let Some(score) = match_score(variant, requested) {
                let better = match best {
                    None => true,
                    Some((s, v)) => score > s || (score == s && variant < v),
                };
                if better {
                    best = Some((score, variant));
                }
            }
        }
    }
    best.map(|(_, v)| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_loaded_with_version() {
        assert!(probe_rs_targets().len() > 1000);
        assert!(probe_rs_snapshot_version().is_some());
        assert!(is_known_probe_rs_target("rp2040"));
        assert!(!is_known_probe_rs_target("ATMEGA328P"));
    }

    #[test]
    fn resolves_ordering_codes_and_core_selectors() {
        let cases = [
            (vec!["STM32H747XI_M7"], Some("STM32H747XI")),
            (vec!["RP2040_M0_0"], Some("RP2040")),
            (vec!["CH32V003F4P6"], Some("CH32V003")),
            (vec!["ATSAMD21G18"], Some("ATSAMD21G18A")),
            (vec!["ESP32S3"], Some("esp32s3")),
            (vec!["MIMXRT1064DVL6A"], Some("MIMXRT1064")),
            (vec!["LPC54608J512"], Some("LPC54608J512BD208")),
            (vec!["ATMEGA328P"], None),
            (vec!["8051"], None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                resolve_probe_rs_target(input.iter().copied()).as_deref(),
                expected,
                "input {input:?}"
            );
        }
    }

    #[test]
    fn resolved_names_are_always_known_targets() {
        for name in [
            "stm32f411ceu6",
            "STM32F103C8",
            "nRF52840_xxAA",
            "MIMXRT1062DVL6A",
        ] {
            if let Some(resolved) = resolve_probe_rs_target([name]) {
                assert!(is_known_probe_rs_target(&resolved), "{name} -> {resolved}");
            }
        }
    }
}
