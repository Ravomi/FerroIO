//! VOFA+-compatible serial protocol decoders (`FireWater`, `JustFloat`, `RawData`),
//! HEX byte framing helpers, command template expansion, and persistent studio controls.

use crate::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Fixed 4-byte IEEE-754 `+inf` little-endian tail sentinel terminating `JustFloat` frames.
pub const JUSTFLOAT_TAIL: [u8; 4] = [0x00, 0x00, 0x80, 0x7F];

/// Environment variable overriding the persistent serial controls TOML location.
pub const RAVOM_SERIAL_CONTROLS_ENV: &str = "RAVOM_SERIAL_CONTROLS_CONFIG";

const MAX_DECODER_BUFFER: usize = 8192;
const MAX_JUSTFLOAT_CHANNELS: usize = 32;

/// Serial data-stream protocol engine matching VOFA+ conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VofaProtocol {
    #[default]
    FireWater,
    JustFloat,
    RawData,
}

impl VofaProtocol {
    /// All selectable VOFA+ protocol engines in toolbar display order.
    pub const ALL: &'static [Self] = &[Self::FireWater, Self::JustFloat, Self::RawData];

    /// Short selector label.
    pub fn label(self) -> &'static str {
        match self {
            Self::FireWater => "FireWater",
            Self::JustFloat => "JustFloat",
            Self::RawData => "RawData",
        }
    }

    /// Brief summary of how the engine decodes incoming bytes.
    pub fn hint(self) -> &'static str {
        match self {
            Self::FireWater => "ASCII CSV / key=value telemetry terminated by \\n",
            Self::JustFloat => "Little-endian f32 channels ending with 00 00 80 7F",
            Self::RawData => "Transparent terminal stream without waveform decoding",
        }
    }

    /// Minimal firmware code snippet illustrating the active protocol format.
    pub fn snippet(self) -> &'static str {
        match self {
            Self::FireWater => "printf(\"%.3f,%.3f,%.3f\\n\", ch0, ch1, ch2);",
            Self::JustFloat => {
                "float ch[3] = {a, b, c};\nconst uint8_t tail[4] = {0x00, 0x00, 0x80, 0x7F};\nwrite_bytes((uint8_t*)ch, sizeof(ch));\nwrite_bytes(tail, 4);"
            }
            Self::RawData => "// Raw stream: all incoming UART bytes shown directly",
        }
    }
}

/// Line ending appended to outgoing ASCII serial commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    CrLf,
    Lf,
    None,
}

impl LineEnding {
    /// All selectable TX line endings.
    pub const ALL: &'static [Self] = &[Self::CrLf, Self::Lf, Self::None];

    /// Short selector label.
    pub fn label(self) -> &'static str {
        match self {
            Self::CrLf => "CRLF",
            Self::Lf => "LF",
            Self::None => "None",
        }
    }

    /// Raw bytes appended to transmitted ASCII text.
    pub fn suffix(self) -> &'static [u8] {
        match self {
            Self::CrLf => b"\r\n",
            Self::Lf => b"\n",
            Self::None => b"",
        }
    }
}

/// Decoded multi-channel sample frame emitted by [`VofaDecoder`].
#[derive(Debug, Clone, PartialEq)]
pub struct VofaFrame {
    pub channels: Vec<(Option<String>, f32)>,
}

/// Result of feeding a raw serial byte chunk into [`VofaDecoder`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VofaDecodeOutput {
    pub frames: Vec<VofaFrame>,
    pub text_lines: Vec<String>,
}

/// Incremental stream decoder handling `FireWater`, `JustFloat`, and `RawData`.
#[derive(Debug, Clone)]
pub struct VofaDecoder {
    protocol: VofaProtocol,
    buf: Vec<u8>,
}

impl Default for VofaDecoder {
    fn default() -> Self {
        Self::new(VofaProtocol::FireWater)
    }
}

impl VofaDecoder {
    /// Create a decoder initialized with `protocol`.
    pub fn new(protocol: VofaProtocol) -> Self {
        Self {
            protocol,
            buf: Vec::new(),
        }
    }

    /// Currently active protocol mode.
    pub fn protocol(&self) -> VofaProtocol {
        self.protocol
    }

    /// Switch protocol mode and discard any incomplete buffered frame fragments.
    pub fn set_protocol(&mut self, protocol: VofaProtocol) {
        if self.protocol != protocol {
            self.protocol = protocol;
            self.buf.clear();
        }
    }

    /// Clear any unparsed trailing bytes in the decoder buffer.
    pub fn reset(&mut self) {
        self.buf.clear();
    }

    /// Feed raw incoming serial bytes and emit complete decoded frames and text lines.
    pub fn feed(&mut self, chunk: &[u8]) -> VofaDecodeOutput {
        let mut out = VofaDecodeOutput::default();
        if chunk.is_empty() && self.buf.is_empty() {
            return out;
        }
        self.buf.extend_from_slice(chunk);

        match self.protocol {
            VofaProtocol::JustFloat => self.decode_justfloat(&mut out),
            VofaProtocol::FireWater | VofaProtocol::RawData => self.decode_lines(&mut out),
        }
        out
    }

    fn decode_justfloat(&mut self, out: &mut VofaDecodeOutput) {
        while let Some(pos) = self.buf.windows(4).position(|w| w == JUSTFLOAT_TAIL) {
            if pos >= 4 && pos % 4 == 0 {
                let channel_count = pos / 4;
                if channel_count <= MAX_JUSTFLOAT_CHANNELS {
                    let mut channels = Vec::with_capacity(channel_count);
                    for &raw in self.buf[..pos].as_chunks::<4>().0 {
                        let val = f32::from_le_bytes(raw);
                        if val.is_finite() {
                            channels.push((None, val));
                        }
                    }
                    if !channels.is_empty() {
                        out.frames.push(VofaFrame { channels });
                    }
                }
            }
            self.buf.drain(..pos + 4);
        }

        if self.buf.len() > MAX_DECODER_BUFFER {
            let keep_from = self.buf.len().saturating_sub(3);
            self.buf.drain(..keep_from);
        }
    }

    fn decode_lines(&mut self, out: &mut VofaDecodeOutput) {
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line_bytes = self.buf[..pos]
                .strip_suffix(b"\r")
                .unwrap_or(&self.buf[..pos]);
            let line = String::from_utf8_lossy(line_bytes).into_owned();
            if self.protocol == VofaProtocol::FireWater {
                if let Some(frame) = parse_firewater_line(&line) {
                    out.frames.push(frame);
                }
            }
            out.text_lines.push(line);
            self.buf.drain(..=pos);
        }

        if self.buf.len() > MAX_DECODER_BUFFER {
            let line_bytes = self.buf.strip_suffix(b"\r").unwrap_or(&self.buf);
            let line = String::from_utf8_lossy(line_bytes).into_owned();
            if self.protocol == VofaProtocol::FireWater {
                if let Some(frame) = parse_firewater_line(&line) {
                    out.frames.push(frame);
                }
            }
            out.text_lines.push(line);
            self.buf.clear();
        }
    }
}

/// Parse one ASCII `FireWater` line into named or positional channels.
///
/// Supports:
/// - VOFA+ CSV (`1.2,3.4,5.6` or `label:1.2,3.4,5.6`)
/// - Key/value telemetry (`kp=1.2 err=-0.04 out=0.85` or `ravom-debug tick=5`)
/// - Standalone float (`12.345`)
pub fn parse_firewater_line(line: &str) -> Option<VofaFrame> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (prefix_label, csv_body) = match trimmed.split_once(':') {
        Some((lbl, rest)) => (Some(lbl.trim()), rest.trim()),
        None => (None, trimmed),
    };
    let clean_prefix = prefix_label.filter(|lbl| {
        !lbl.is_empty()
            && lbl
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    });

    if csv_body.contains(',') {
        let mut channels = Vec::new();
        let mut all_numeric = true;
        let mut saw_segment = false;
        for (idx, seg) in csv_body.split(',').enumerate() {
            let token = seg.trim();
            if token.is_empty() {
                continue;
            }
            saw_segment = true;
            if let Ok(val) = token.parse::<f32>() {
                if val.is_finite() {
                    let name = clean_prefix.map(|lbl| format!("{lbl}.{idx}"));
                    channels.push((name, val));
                    continue;
                }
            }
            all_numeric = false;
            break;
        }
        if saw_segment && all_numeric && !channels.is_empty() {
            return Some(VofaFrame { channels });
        }
    }

    let mut kv_channels = Vec::new();
    for raw_tok in trimmed.split(|c: char| c.is_whitespace() || c == ',') {
        let tok = raw_tok.trim_matches(|c: char| matches!(c, ';' | '(' | ')' | '[' | ']'));
        if let Some((k, rhs)) = tok.split_once('=') {
            let key = k.trim();
            if !key.is_empty() {
                if let Ok(val) = rhs.trim().parse::<f32>() {
                    if val.is_finite() {
                        kv_channels.push((Some(key.to_string()), val));
                    }
                }
            }
        }
    }
    if !kv_channels.is_empty() {
        return Some(VofaFrame {
            channels: kv_channels,
        });
    }

    if let Ok(val) = csv_body.parse::<f32>() {
        if val.is_finite() {
            return Some(VofaFrame {
                channels: vec![(clean_prefix.map(str::to_string), val)],
            });
        }
    }

    None
}

/// Parse space/comma-separated hex bytes (optional `0x` prefixes) or contiguous hex pairs.
pub fn parse_hex_bytes(input: &str) -> Result<Vec<u8>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CoreError::SerialPort(
            "hex".to_string(),
            "HEX payload cannot be empty".to_string(),
        ));
    }

    let mut bytes = Vec::new();
    for raw in trimmed.split(|c: char| c.is_whitespace() || c == ',') {
        if raw.is_empty() {
            continue;
        }
        let hex = raw
            .strip_prefix("0x")
            .or_else(|| raw.strip_prefix("0X"))
            .unwrap_or(raw);
        if hex.is_empty() || hex.len() % 2 != 0 || !hex.is_ascii() {
            return Err(CoreError::SerialPort(
                "hex".to_string(),
                format!("invalid HEX token '{raw}'"),
            ));
        }
        for pair in hex.as_bytes().as_chunks::<2>().0 {
            let pair_str = std::str::from_utf8(pair).map_err(|_| {
                CoreError::SerialPort("hex".to_string(), format!("invalid HEX token '{raw}'"))
            })?;
            let byte = u8::from_str_radix(pair_str, 16).map_err(|_| {
                CoreError::SerialPort("hex".to_string(), format!("invalid HEX token '{raw}'"))
            })?;
            bytes.push(byte);
        }
    }

    if bytes.is_empty() {
        return Err(CoreError::SerialPort(
            "hex".to_string(),
            "HEX payload produced zero bytes".to_string(),
        ));
    }
    Ok(bytes)
}

/// Format raw bytes as uppercase space-separated hexadecimal bytes (`00 00 80 7F`).
pub fn format_hex_dump(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Substitute `value` into a slider command template (`%.0f`..`%.6f`, `%f`, `%d`, `{val}`, `{val:.N}`).
///
/// Appends ` {value:.3}` when no placeholder is present.
pub fn format_command_template(template: &str, value: f32) -> String {
    let mut out = template.trim().to_string();
    let mut substituted = false;

    for prec in 0..=6 {
        let c_pat = format!("%.{prec}f");
        if out.contains(&c_pat) {
            out = out.replace(&c_pat, &format!("{value:.prec$}"));
            substituted = true;
        }
        let brace_pat = format!("{{val:.{prec}}}");
        if out.contains(&brace_pat) {
            out = out.replace(&brace_pat, &format!("{value:.prec$}"));
            substituted = true;
        }
    }

    if out.contains("{val}") {
        out = out.replace("{val}", &format!("{value:.3}"));
        substituted = true;
    }
    if out.contains("%f") {
        out = out.replace("%f", &format!("{value:.3}"));
        substituted = true;
    }
    if out.contains("%d") {
        let rounded = value.round() as i64;
        out = out.replace("%d", &rounded.to_string());
        substituted = true;
    }

    if substituted {
        out
    } else if out.is_empty() {
        format!("{value:.3}")
    } else {
        format!("{out} {value:.3}")
    }
}

/// User-defined quick-send command macro button displayed in the Serial Studio sidebar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SerialMacro {
    pub label: String,
    pub payload: String,
    #[serde(default)]
    pub hex: bool,
}

/// User-defined interactive parameter slider mapped to a formatted serial TX template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerialSlider {
    pub name: String,
    pub template: String,
    pub min: f32,
    pub max: f32,
    pub value: f32,
    #[serde(default)]
    pub auto_send: bool,
}

/// Persistent collection of VOFA+-style command macros and parameter sliders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerialControlsConfig {
    #[serde(default)]
    pub macros: Vec<SerialMacro>,
    #[serde(default)]
    pub sliders: Vec<SerialSlider>,
}

impl Default for SerialControlsConfig {
    fn default() -> Self {
        Self {
            macros: vec![
                SerialMacro {
                    label: "STATUS".to_string(),
                    payload: "STATUS".to_string(),
                    hex: false,
                },
                SerialMacro {
                    label: "INFO".to_string(),
                    payload: "INFO".to_string(),
                    hex: false,
                },
                SerialMacro {
                    label: "PING".to_string(),
                    payload: "PING".to_string(),
                    hex: false,
                },
                SerialMacro {
                    label: "RESET".to_string(),
                    payload: "RESET".to_string(),
                    hex: false,
                },
                SerialMacro {
                    label: "HEX 01".to_string(),
                    payload: "01".to_string(),
                    hex: true,
                },
            ],
            sliders: vec![
                SerialSlider {
                    name: "KP".to_string(),
                    template: "KP %.3f".to_string(),
                    min: 0.0,
                    max: 10.0,
                    value: 1.0,
                    auto_send: false,
                },
                SerialSlider {
                    name: "KI".to_string(),
                    template: "KI %.3f".to_string(),
                    min: 0.0,
                    max: 5.0,
                    value: 0.0,
                    auto_send: false,
                },
                SerialSlider {
                    name: "KD".to_string(),
                    template: "KD %.3f".to_string(),
                    min: 0.0,
                    max: 5.0,
                    value: 0.0,
                    auto_send: false,
                },
                SerialSlider {
                    name: "Target".to_string(),
                    template: "TARGET %.1f".to_string(),
                    min: -100.0,
                    max: 100.0,
                    value: 0.0,
                    auto_send: false,
                },
            ],
        }
    }
}

/// Return the path to RavomIO's persistent Serial Studio controls file (`~/.ravom/serial_controls.toml`).
pub fn serial_controls_config_path() -> PathBuf {
    if let Ok(explicit) = std::env::var(RAVOM_SERIAL_CONTROLS_ENV) {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".ravom").join("serial_controls.toml"))
        .unwrap_or_else(|| PathBuf::from(".ravom").join("serial_controls.toml"))
}

/// Load serial studio controls from `path`, returning defaults when missing or invalid.
pub fn load_serial_controls_from(path: &Path) -> SerialControlsConfig {
    let Ok(raw) = fs::read_to_string(path) else {
        return SerialControlsConfig::default();
    };
    toml::from_str::<SerialControlsConfig>(&raw).unwrap_or_default()
}

/// Load serial studio controls from [`serial_controls_config_path`].
pub fn load_serial_controls() -> SerialControlsConfig {
    load_serial_controls_from(&serial_controls_config_path())
}

/// Persist `cfg` as pretty TOML to `path`.
pub fn save_serial_controls_to(path: &Path, cfg: &SerialControlsConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(cfg)?;
    fs::write(path, raw)?;
    Ok(())
}

/// Persist `cfg` to [`serial_controls_config_path`] and return the path written.
pub fn save_serial_controls(cfg: &SerialControlsConfig) -> Result<PathBuf> {
    let path = serial_controls_config_path();
    save_serial_controls_to(&path, cfg)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn firewater_decodes_csv_labeled_csv_and_key_values() {
        let mut dec = VofaDecoder::new(VofaProtocol::FireWater);
        let out = dec.feed(b"1.5, -2.25, 3.0\nimu:0.1,0.2,0.3\nkp=1.2 err=-0.04\nBOOT READY\n");
        assert_eq!(out.text_lines.len(), 4);
        assert_eq!(out.frames.len(), 3);
        assert_eq!(
            out.frames[0].channels,
            vec![(None, 1.5), (None, -2.25), (None, 3.0)]
        );
        assert_eq!(
            out.frames[1].channels,
            vec![
                (Some("imu.0".to_string()), 0.1),
                (Some("imu.1".to_string()), 0.2),
                (Some("imu.2".to_string()), 0.3),
            ]
        );
        assert_eq!(
            out.frames[2].channels,
            vec![
                (Some("kp".to_string()), 1.2),
                (Some("err".to_string()), -0.04),
            ]
        );
    }

    #[test]
    fn justfloat_decodes_split_chunks_and_resyncs() {
        let mut dec = VofaDecoder::new(VofaProtocol::JustFloat);
        let mut packet = Vec::new();
        packet.extend_from_slice(&[0xDE, 0xAD]); // leading junk before tail
        packet.extend_from_slice(&JUSTFLOAT_TAIL); // misaligned 2-byte prefix -> dropped on resync
        packet.extend_from_slice(&12.5_f32.to_le_bytes());
        packet.extend_from_slice(&(-3.25_f32).to_le_bytes());
        packet.extend_from_slice(&JUSTFLOAT_TAIL);

        // Split right across the tail boundary
        let split = packet.len() - 2;
        let out1 = dec.feed(&packet[..split]);
        assert!(out1.frames.is_empty());

        let out2 = dec.feed(&packet[split..]);
        assert_eq!(out2.frames.len(), 1);
        assert_eq!(out2.frames[0].channels, vec![(None, 12.5), (None, -3.25)]);
    }

    #[test]
    fn rawdata_emits_lines_without_frames() {
        let mut dec = VofaDecoder::new(VofaProtocol::RawData);
        let out = dec.feed(b"1.0,2.0,3.0\r\nhello\n");
        assert_eq!(out.text_lines, vec!["1.0,2.0,3.0", "hello"]);
        assert!(out.frames.is_empty());
    }

    #[test]
    fn hex_parse_and_dump_roundtrip() {
        let bytes = parse_hex_bytes("0x01 AA, 807F").expect("hex bytes");
        assert_eq!(bytes, vec![0x01, 0xAA, 0x80, 0x7F]);
        assert_eq!(format_hex_dump(&bytes), "01 AA 80 7F");
        assert!(parse_hex_bytes("0x1").is_err());
        assert!(parse_hex_bytes("ZZ").is_err());
    }

    #[test]
    fn formats_command_templates() {
        assert_eq!(format_command_template("KP %.2f", 1.234), "KP 1.23");
        assert_eq!(format_command_template("PWM %d", 74.6), "PWM 75");
        assert_eq!(format_command_template("SET {val:.1}", -3.25), "SET -3.2");
        assert_eq!(format_command_template("SPEED", 42.0), "SPEED 42.000");
    }

    #[test]
    fn serial_controls_config_roundtrips_toml() {
        let tmp = tempdir().expect("tempdir");
        let cfg_path = tmp.path().join("serial_controls.toml");
        let mut cfg = SerialControlsConfig::default();
        cfg.macros.push(SerialMacro {
            label: "CALIB".to_string(),
            payload: "CAL START".to_string(),
            hex: false,
        });
        save_serial_controls_to(&cfg_path, &cfg).expect("save controls");
        let loaded = load_serial_controls_from(&cfg_path);
        assert_eq!(loaded, cfg);
    }
}
