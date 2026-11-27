use crate::context::EnvContext;
use crate::process::CommandSpec;
use crate::{CoreError, Result};
use std::io::{self, BufRead, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// A serial port detected on the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialPortEntry {
    pub path: String,
    /// Human readable description (`USB 0483:5740 STMicroelectronics ...`).
    pub description: String,
}

/// Discover serial ports (USB CDC/ACM, USB-UART bridges, PCI, ...) on Linux, macOS and Windows.
pub fn list_serial_port_details() -> Vec<SerialPortEntry> {
    let mut ports: Vec<SerialPortEntry> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            let description = match p.port_type {
                serialport::SerialPortType::UsbPort(usb) => format!(
                    "USB {:04x}:{:04x} {} {}",
                    usb.vid,
                    usb.pid,
                    usb.manufacturer.unwrap_or_default(),
                    usb.product.unwrap_or_default()
                )
                .trim()
                .to_string(),
                serialport::SerialPortType::PciPort => "PCI".to_string(),
                serialport::SerialPortType::BluetoothPort => "Bluetooth".to_string(),
                // Legacy on-board UARTs (`/dev/ttyS*`) are almost never a dev board.
                serialport::SerialPortType::Unknown => return None,
            };
            Some(SerialPortEntry {
                path: p.port_name,
                description,
            })
        })
        .collect();
    ports.sort_by(|a, b| a.path.cmp(&b.path));
    ports.dedup_by(|a, b| a.path == b.path);
    ports
}

/// Paths of detected serial ports, sorted.
pub fn list_serial_ports() -> Vec<String> {
    list_serial_port_details()
        .into_iter()
        .map(|p| p.path)
        .collect()
}

/// Build the `probe-rs attach` command used for RTT / defmt logging.
pub fn plan_rtt(ctx: &EnvContext, release: bool) -> Result<CommandSpec> {
    let chip = ctx.board.probe_rs_chip.clone().ok_or_else(|| {
        CoreError::MissingProbeRsChip(ctx.board.qualified_id(), ctx.board.mcu.clone())
    })?;
    Ok(CommandSpec::new("probe-rs")
        .args(["attach", "--chip", chip.as_str()])
        .path_arg(&ctx.firmware_path(release))
        .cwd(&ctx.root))
}

/// Attach to an RTT session using `probe-rs attach --chip <chip> <elf>`.
pub fn monitor_rtt(dir: &Path, env_name: Option<&str>, release: bool) -> Result<()> {
    let ctx = EnvContext::load(dir, env_name)?;
    let firmware = ctx.firmware_path(release);
    if !firmware.exists() {
        return Err(CoreError::ArtifactNotFound(firmware));
    }
    plan_rtt(&ctx, release)?.run(CoreError::MonitorFailed)
}

/// Open an interactive serial terminal: device output to stdout, stdin to the device.
pub fn monitor_serial(port: &str, baud: u32) -> Result<()> {
    let mut reader = serialport::new(port, baud)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|err| CoreError::SerialPort(port.to_string(), err.to_string()))?;
    let mut writer = reader
        .try_clone()
        .map_err(|err| CoreError::SerialPort(port.to_string(), err.to_string()))?;

    thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        let mut buf = [0u8; 256];
        while let Ok(n) = stdin.read(&mut buf) {
            if n == 0 || writer.write_all(&buf[..n]).is_err() {
                break;
            }
            let _ = writer.flush();
        }
    });

    let mut stdout = io::stdout().lock();
    let mut buf = [0u8; 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                stdout.write_all(&buf[..n])?;
                stdout.flush()?;
            }
            Err(err) if err.kind() == io::ErrorKind::TimedOut => continue,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(CoreError::SerialPort(port.to_string(), err.to_string())),
        }
    }
}

/// Sample bounded lines from a serial port until `duration_ms`, `max_lines`, or `until_pattern` is reached.
pub fn sample_serial_lines(
    port_name: &str,
    baud: u32,
    duration_ms: u64,
    max_lines: usize,
    until_pattern: Option<&str>,
) -> Result<(Vec<String>, bool)> {
    let port = serialport::new(port_name, baud)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|err| CoreError::SerialPort(port_name.to_string(), err.to_string()))?;
    let mut reader = io::BufReader::new(port);
    let deadline = Instant::now() + Duration::from_millis(duration_ms.min(15_000));
    let cap = max_lines.min(500);
    let mut lines = Vec::new();
    let mut matched = false;

    while Instant::now() < deadline && lines.len() < cap {
        let mut buf = String::new();
        match reader.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                let line = buf.trim_end().to_string();
                if let Some(pat) = until_pattern {
                    if line.contains(pat) {
                        matched = true;
                    }
                }
                lines.push(line);
                if matched {
                    break;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
            Err(_) => break,
        }
    }
    Ok((lines, matched))
}

/// Write a single command frame (e.g. clamped PID tuning parameters) to a serial port.
pub fn write_serial_frame(port_name: &str, baud: u32, frame: &str) -> Result<()> {
    let mut port = serialport::new(port_name, baud)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|err| CoreError::SerialPort(port_name.to_string(), err.to_string()))?;
    port.write_all(frame.as_bytes())
        .map_err(|err| CoreError::SerialPort(port_name.to_string(), err.to_string()))?;
    port.flush()
        .map_err(|err| CoreError::SerialPort(port_name.to_string(), err.to_string()))?;
    Ok(())
}

/// Event produced by a background [`SerialSession`] reader thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialEvent {
    Bytes(Vec<u8>),
    Error(String),
    Disconnected,
}

/// A live, non-blocking serial monitor session backed by a background raw-byte reader thread.
pub struct SerialSession {
    port_name: String,
    baud: u32,
    writer: Box<dyn serialport::SerialPort>,
    rx: Receiver<SerialEvent>,
    stop: Arc<AtomicBool>,
}

impl SerialSession {
    /// Open `port_name` at `baud` and start streaming raw incoming serial bytes.
    pub fn open(port_name: &str, baud: u32) -> Result<Self> {
        let port = serialport::new(port_name, baud)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|err| CoreError::SerialPort(port_name.to_string(), err.to_string()))?;
        let mut reader = port
            .try_clone()
            .map_err(|err| CoreError::SerialPort(port_name.to_string(), err.to_string()))?;
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);

        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            while !stop_flag.load(Ordering::Relaxed) {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx.send(SerialEvent::Disconnected);
                        break;
                    }
                    Ok(n) => {
                        if tx.send(SerialEvent::Bytes(buf[..n].to_vec())).is_err() {
                            return;
                        }
                    }
                    Err(err)
                        if err.kind() == io::ErrorKind::TimedOut
                            || err.kind() == io::ErrorKind::Interrupted =>
                    {
                        continue;
                    }
                    Err(err) => {
                        let _ = tx.send(SerialEvent::Error(err.to_string()));
                        break;
                    }
                }
            }
        });

        Ok(Self {
            port_name: port_name.to_string(),
            baud,
            writer: port,
            rx,
            stop,
        })
    }

    /// Drain any pending events received since the previous poll.
    pub fn poll(&self) -> Vec<SerialEvent> {
        self.rx.try_iter().collect()
    }

    /// Transmit arbitrary raw bytes over the connected serial port.
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer
            .write_all(bytes)
            .map_err(|err| CoreError::SerialPort(self.port_name.clone(), err.to_string()))?;
        self.writer
            .flush()
            .map_err(|err| CoreError::SerialPort(self.port_name.clone(), err.to_string()))?;
        Ok(())
    }

    /// Transmit a single command line terminated by `\r\n`.
    pub fn send_line(&mut self, line: &str) -> Result<()> {
        let payload = format!("{line}\r\n");
        self.send_bytes(payload.as_bytes())
    }

    /// Drive the hardware Data Terminal Ready (DTR) control line.
    pub fn set_dtr(&mut self, level: bool) -> Result<()> {
        self.writer
            .write_data_terminal_ready(level)
            .map_err(|err| CoreError::SerialPort(self.port_name.clone(), err.to_string()))
    }

    /// Drive the hardware Request To Send (RTS) control line.
    pub fn set_rts(&mut self, level: bool) -> Result<()> {
        self.writer
            .write_request_to_send(level)
            .map_err(|err| CoreError::SerialPort(self.port_name.clone(), err.to_string()))
    }

    /// Pulse the RTS line briefly (`true` -> 25ms -> `false`) to hardware-reset common dev boards.
    pub fn pulse_reset(&mut self) -> Result<()> {
        self.set_rts(true)?;
        thread::sleep(Duration::from_millis(25));
        self.set_rts(false)
    }

    /// Connected serial port path.
    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// Active baud rate.
    pub fn baud(&self) -> u32 {
        self.baud
    }
}

impl Drop for SerialSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Extract the first numeric telemetry sample (e.g. `key=123.4` or plain float) from a serial line.
pub fn extract_numeric_sample(line: &str) -> Option<f32> {
    for token in line.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| matches!(c, ',' | ';' | ')' | '('));
        if let Some((_, rhs)) = trimmed.split_once('=') {
            if let Ok(v) = rhs.parse::<f32>() {
                if v.is_finite() {
                    return Some(v);
                }
            }
        }
    }
    for token in line.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| matches!(c, ',' | ';' | ')' | '('));
        if let Ok(v) = trimmed.parse::<f32>() {
            if v.is_finite() {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RavomConfig;

    #[test]
    fn list_serial_ports_returns_sorted_unique_paths() {
        let ports = list_serial_ports();
        let mut sorted = ports.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ports, sorted);
    }

    #[test]
    fn opening_missing_serial_port_is_a_typed_error() {
        let err = monitor_serial("/dev/ravomio-no-such-port", 115200).expect_err("must fail");
        assert!(matches!(err, CoreError::SerialPort(..)));
    }

    #[test]
    fn serial_session_rejects_missing_port() {
        let err = SerialSession::open("/dev/ravomio-no-such-port", 115200)
            .err()
            .expect("must fail");
        assert!(matches!(err, CoreError::SerialPort(..)));
    }

    #[test]
    fn extracts_numeric_samples_from_telemetry_lines() {
        assert_eq!(extract_numeric_sample("ravom-debug tick=5"), Some(5.0));
        assert_eq!(extract_numeric_sample("batteryMv=4120"), Some(4120.0));
        assert_eq!(extract_numeric_sample("12.345"), Some(12.345));
        assert_eq!(extract_numeric_sample("BOOT OK ready"), None);
    }

    #[test]
    fn plans_rtt_attach() {
        let cfg = RavomConfig::from_toml_str(
            "[project]\nname = \"blink\"\n[env.pico]\nplatform = \"raspberrypi\"\n\
             board = \"pico\"\nframework = \"embassy\"\n",
        )
        .expect("manifest");
        let ctx = EnvContext::from_config(Path::new("/w"), cfg, None).expect("ctx");
        let spec = plan_rtt(&ctx, false).expect("plan");
        assert_eq!(spec.program, "probe-rs");
        assert_eq!(&spec.args[..3], ["attach", "--chip", "RP2040"]);
    }
}
