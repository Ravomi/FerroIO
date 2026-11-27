//! PlatformIO-inspired embedded package manager and community library registry.
//!
//! Searches across both **crates.io** (Rust `no_std` embedded crates) and the
//! **PlatformIO Registry v3** (C/C++/Arduino/ESP-IDF libraries), maintains an
//! instant offline curated embedded starter catalog, and manages `ravom.toml`
//! `lib_deps` alongside `Cargo.toml` / `.ravom/libdeps/<env>/` installations.

use crate::config::RavomConfig;
use crate::{CoreError, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const CRATES_IO_API_BASE: &str = "https://crates.io/api/v1/crates";
const PIO_REGISTRY_SEARCH_BASE: &str = "https://api.registry.platformio.org/v3/search";
const USER_AGENT: &str = "RavomIO/0.1 (https://github.com/Ravomi/RavomIO)";

/// Upstream ecosystem where an embedded package lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistrySource {
    CratesIo,
    PlatformIo,
}

impl RegistrySource {
    /// Human-readable ecosystem name.
    pub fn label(self) -> &'static str {
        match self {
            Self::CratesIo => "crates.io",
            Self::PlatformIo => "PlatformIO",
        }
    }

    /// Compact pill tag for dense UI lists and tables.
    pub fn short_tag(self) -> &'static str {
        match self {
            Self::CratesIo => "crates.io",
            Self::PlatformIo => "PIO",
        }
    }
}

/// Registry filter selector for package queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchSourceFilter {
    #[default]
    All,
    CratesIo,
    PlatformIo,
}

impl SearchSourceFilter {
    /// All selectable source filters in display order.
    pub const ALL: &'static [Self] = &[Self::All, Self::CratesIo, Self::PlatformIo];

    /// Display label for UI filter chips and CLI output.
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::CratesIo => "crates.io",
            Self::PlatformIo => "PlatformIO",
        }
    }

    /// Return `true` when `source` passes this filter.
    pub fn matches(self, source: RegistrySource) -> bool {
        match self {
            Self::All => true,
            Self::CratesIo => source == RegistrySource::CratesIo,
            Self::PlatformIo => source == RegistrySource::PlatformIo,
        }
    }
}

/// Functional category for embedded drivers and middleware libraries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PkgCategory {
    #[default]
    All,
    ButtonsInput,
    DisplayGui,
    SensorsImu,
    MotorPwm,
    WirelessIot,
    StorageBus,
}

impl PkgCategory {
    /// All embedded categories in display order.
    pub const ALL: &'static [Self] = &[
        Self::All,
        Self::ButtonsInput,
        Self::DisplayGui,
        Self::SensorsImu,
        Self::MotorPwm,
        Self::WirelessIot,
        Self::StorageBus,
    ];

    /// Display label for UI category chips.
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::ButtonsInput => "Buttons & Input",
            Self::DisplayGui => "Display & GUI",
            Self::SensorsImu => "Sensors & IMU",
            Self::MotorPwm => "Motor & PWM",
            Self::WirelessIot => "Wireless & IoT",
            Self::StorageBus => "Storage & Bus",
        }
    }

    /// Default remote search term when querying this category without user text.
    pub fn query_terms(self) -> &'static str {
        match self {
            Self::All => "",
            Self::ButtonsInput => "button",
            Self::DisplayGui => "display",
            Self::SensorsImu => "sensor",
            Self::MotorPwm => "motor",
            Self::WirelessIot => "wifi",
            Self::StorageBus => "storage",
        }
    }
}

/// Metadata for a community or curated embedded library package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRecord {
    pub source: RegistrySource,
    pub owner: Option<String>,
    pub name: String,
    pub version: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub category: PkgCategory,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub docs_url: Option<String>,
    pub download_url: Option<String>,
    pub popularity: u64,
    pub curated: bool,
}

impl PackageRecord {
    /// Return canonical identifier (`crates:<name>` or `<owner>/<name>`).
    pub fn qualified_id(&self) -> String {
        match (self.source, &self.owner) {
            (RegistrySource::CratesIo, _) => format!("crates:{}", self.name),
            (RegistrySource::PlatformIo, Some(owner)) if !owner.is_empty() => {
                format!("{owner}/{}", self.name)
            }
            (RegistrySource::PlatformIo, _) => self.name.clone(),
        }
    }

    /// Render the canonical `ravom.toml` `lib_deps` dependency specification.
    pub fn dep_spec(&self) -> String {
        let ver = self.version.trim();
        let suffix = if ver.is_empty() {
            String::new()
        } else {
            format!("@^{ver}")
        };
        match (self.source, &self.owner) {
            (RegistrySource::CratesIo, _) => format!("crates:{}{suffix}", self.name),
            (RegistrySource::PlatformIo, Some(owner)) if !owner.is_empty() => {
                format!("{owner}/{}{suffix}", self.name)
            }
            (RegistrySource::PlatformIo, _) => format!("{}{suffix}", self.name),
        }
    }

    /// Return `true` when `spec` identifies the same package as `self`.
    pub fn matches_spec(&self, spec: &str) -> bool {
        let (src, owner, name, _) = parse_dep_spec(spec);
        if src != self.source {
            return false;
        }
        match self.source {
            RegistrySource::CratesIo => name.eq_ignore_ascii_case(&self.name),
            RegistrySource::PlatformIo => {
                if !name.eq_ignore_ascii_case(&self.name) {
                    return false;
                }
                match (&self.owner, &owner) {
                    (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                    _ => true,
                }
            }
        }
    }

    fn dedup_key(&self) -> (RegistrySource, String, String) {
        let name_key = self.name.to_ascii_lowercase();
        let owner_key = self.owner.as_deref().unwrap_or("").to_ascii_lowercase();
        (self.source, name_key, owner_key)
    }
}

/// Where an installed project dependency is declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstalledOrigin {
    RavomManifest,
    CargoToml,
    Both,
}

impl InstalledOrigin {
    /// Human-readable origin badge label.
    pub fn label(self) -> &'static str {
        match self {
            Self::RavomManifest => "ravom.toml",
            Self::CargoToml => "Cargo.toml",
            Self::Both => "Synced",
        }
    }
}

/// Dependency currently configured or unpacked inside a project environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPackage {
    pub spec: String,
    pub name: String,
    pub owner: Option<String>,
    pub version: Option<String>,
    pub source: RegistrySource,
    pub origin: InstalledOrigin,
    pub local_dir: Option<PathBuf>,
}

/// Parameters controlling curated and remote package searches.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub source: SearchSourceFilter,
    pub category: PkgCategory,
    pub limit: usize,
    pub online: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            source: SearchSourceFilter::All,
            category: PkgCategory::All,
            limit: 15,
            online: true,
        }
    }
}

/// Merged results and diagnostics returned by [`search_packages`].
#[derive(Debug, Clone, Default)]
pub struct SearchOutcome {
    pub packages: Vec<PackageRecord>,
    pub remote_total: usize,
    pub warnings: Vec<String>,
}

/// Curated offline starter catalog of widely used embedded Rust & C/C++ libraries.
pub fn curated_catalog() -> Vec<PackageRecord> {
    let mut list = vec![
        // Buttons & Input
        curated_crate(
            "async-button",
            "0.2.0",
            "Async button debouncing and press/hold state machine for no_std embedded targets.",
            &["button", "debounce", "async", "no_std"],
            PkgCategory::ButtonsInput,
            "https://github.com/avsaase/async-button",
            6_200,
        ),
        curated_crate(
            "button-driver",
            "0.2.0",
            "Platform-agnostic embedded-hal push-button driver with short, double, and long press.",
            &["button", "input", "embedded-hal"],
            PkgCategory::ButtonsInput,
            "https://github.com/embassy-rs/embassy",
            3_400,
        ),
        curated_crate(
            "debouncr",
            "0.2.2",
            "Simple and fast branchless signal debouncer for embedded GPIO buttons in Rust.",
            &["debounce", "button", "gpio", "no_std"],
            PkgCategory::ButtonsInput,
            "https://github.com/dbrgn/debouncr",
            28_500,
        ),
        curated_crate(
            "rotary-encoder-hal",
            "0.6.0",
            "Platform-agnostic rotary encoder driver built on embedded-hal GPIO input pins.",
            &["encoder", "rotary", "input", "embedded-hal"],
            PkgCategory::ButtonsInput,
            "https://github.com/ostrowski/rotary-encoder-hal",
            19_400,
        ),
        curated_pio(
            "mathertel/OneButton",
            "2.0.3",
            "Arduino library using one input pin to detect single click, double click, and long press.",
            &["button", "pushbutton", "debounce", "click"],
            PkgCategory::ButtonsInput,
            Some("BSD-3-Clause"),
            720,
        ),
        curated_pio(
            "lennarthennigs/Button2",
            "2.3.2",
            "Lightweight C++ button library supporting callback handlers, click counting, and long holds.",
            &["button", "input", "arduino", "esp32"],
            PkgCategory::ButtonsInput,
            Some("MIT"),
            265,
        ),
        curated_pio(
            "evert-arias/EasyButton",
            "2.0.3",
            "Non-blocking Arduino push-button debouncer with sequence and interrupt callback support.",
            &["button", "debounce", "interrupt"],
            PkgCategory::ButtonsInput,
            Some("MIT"),
            340,
        ),
        // Display & GUI
        curated_crate(
            "embedded-graphics",
            "0.8.1",
            "2D no_std graphics library for driving monochrome OLEDs, e-paper, and TFT LCDs.",
            &["graphics", "display", "oled", "lcd"],
            PkgCategory::DisplayGui,
            "https://github.com/embedded-graphics/embedded-graphics",
            490_000,
        ),
        curated_crate(
            "mipidsi",
            "0.9.0",
            "Universal MIPI DCS display driver for ST7789, ST7735, ILI9341, and RM67162 screens.",
            &["st7789", "ili9341", "display", "spi", "tft"],
            PkgCategory::DisplayGui,
            "https://github.com/almindor/mipidsi",
            86_000,
        ),
        curated_crate(
            "ssd1306",
            "0.10.0",
            "I2C and SPI no_std driver for Solomon Systech SSD1306 OLED displays.",
            &["oled", "ssd1306", "i2c", "display"],
            PkgCategory::DisplayGui,
            "https://github.com/rust-embedded-community/ssd1306",
            629_600,
        ),
        curated_crate(
            "slint",
            "1.9.2",
            "Declarative reactive UI toolkit supporting bare-metal MCU displays and embedded Linux.",
            &["gui", "ui", "display", "mcu"],
            PkgCategory::DisplayGui,
            "https://github.com/slint-ui/slint",
            310_000,
        ),
        curated_pio(
            "bodmer/TFT_eSPI",
            "2.5.43",
            "High-speed DMA TFT LCD & Touch driver for ESP32, RP2040, and STM32 (ST7789/ILI9341).",
            &["tft", "st7789", "ili9341", "esp32", "display"],
            PkgCategory::DisplayGui,
            Some("BSD-3-Clause"),
            3_600,
        ),
        curated_pio(
            "lvgl/lvgl",
            "9.2.2",
            "Light and Versatile Graphics Library for embedded touchscreens and monochrome displays.",
            &["lvgl", "gui", "display", "widgets"],
            PkgCategory::DisplayGui,
            Some("MIT"),
            17_200,
        ),
        curated_pio(
            "adafruit/Adafruit SSD1306",
            "2.5.13",
            "Adafruit driver library for monochrome 128x64 and 128x32 SSD1306 OLED displays.",
            &["ssd1306", "oled", "i2c", "adafruit"],
            PkgCategory::DisplayGui,
            Some("BSD-3-Clause"),
            890,
        ),
        curated_pio(
            "olikraus/U8g2",
            "2.35.30",
            "Monochrome graphics and font library supporting SSD1306, SH1106, and ST7920 displays.",
            &["oled", "font", "monochrome", "display"],
            PkgCategory::DisplayGui,
            Some("BSD-2-Clause"),
            4_700,
        ),
        // Sensors & IMU
        curated_crate(
            "bme280",
            "0.5.1",
            "embedded-hal driver for Bosch BME280 temperature, humidity, and barometric sensors.",
            &["sensor", "bme280", "i2c", "spi"],
            PkgCategory::SensorsImu,
            "https://github.com/versable/bme280-rs",
            48_000,
        ),
        curated_crate(
            "mpu6050",
            "0.1.6",
            "no_std I2C driver for InvenSense MPU6050 6-axis accelerometer and gyroscope IMU.",
            &["imu", "mpu6050", "gyro", "sensor"],
            PkgCategory::SensorsImu,
            "https://github.com/juliangaal/mpu6050",
            32_000,
        ),
        curated_crate(
            "ina219",
            "0.2.0",
            "embedded-hal driver for TI INA219 I2C shunt current and battery voltage monitors.",
            &["ina219", "battery", "power", "sensor"],
            PkgCategory::SensorsImu,
            "https://github.com/scttnlsn/ina219",
            14_500,
        ),
        curated_pio(
            "adafruit/Adafruit BME280 Library",
            "2.2.4",
            "Arduino library for Bosch BME280 temperature, humidity, and pressure sensors.",
            &["bme280", "sensor", "i2c"],
            PkgCategory::SensorsImu,
            Some("BSD-3-Clause"),
            420,
        ),
        curated_pio(
            "electroniccats/MPU6050",
            "1.4.1",
            "InvenSense MPU6050 6-axis IMU motion processor & DMP library for MCU projects.",
            &["imu", "mpu6050", "sensor"],
            PkgCategory::SensorsImu,
            Some("MIT"),
            510,
        ),
        curated_pio(
            "adafruit/DHT sensor library",
            "1.4.6",
            "Arduino library for DHT11 and DHT22 single-wire temperature and humidity sensors.",
            &["dht11", "dht22", "temperature", "sensor"],
            PkgCategory::SensorsImu,
            Some("MIT"),
            580,
        ),
        // Motor & PWM
        curated_crate(
            "pid",
            "4.0.0",
            "no_std PID closed-loop controller with clamped derivative and anti-windup integral.",
            &["pid", "control", "motor", "no_std"],
            PkgCategory::MotorPwm,
            "https://github.com/braincore/pid-rs",
            140_000,
        ),
        curated_crate(
            "smart-leds",
            "0.4.0",
            "Trait and utilities for addressable RGB LEDs (WS2812, APA102, SK6812) in no_std Rust.",
            &["ws2812", "led", "pwm", "smart-leds"],
            PkgCategory::MotorPwm,
            "https://github.com/smart-leds-rs/smart-leds",
            220_000,
        ),
        curated_crate(
            "ws2812-spi",
            "0.5.1",
            "SPI MOSI driver for WS2812 / NeoPixel RGB LEDs built on embedded-hal.",
            &["ws2812", "spi", "led"],
            PkgCategory::MotorPwm,
            "https://github.com/smart-leds-rs/ws2812-spi-rs",
            98_000,
        ),
        curated_pio(
            "fastled/FastLED",
            "3.9.8",
            "High-performance C++ library for WS2812B/APA102 addressable LEDs on ESP32 & AVR.",
            &["led", "ws2812", "neopixel", "pwm"],
            PkgCategory::MotorPwm,
            Some("MIT"),
            7_900,
        ),
        curated_pio(
            "askuric/Simple FOC",
            "2.3.4",
            "Arduino Field Oriented Control library for BLDC and stepper motor closed-loop drive.",
            &["motor", "bldc", "foc", "pwm"],
            PkgCategory::MotorPwm,
            Some("MIT"),
            1_850,
        ),
        // Wireless & IoT
        curated_crate(
            "embassy-net",
            "0.6.0",
            "Async TCP/IP, UDP, DHCP, and DNS network stack for Embassy no_std applications.",
            &["wifi", "network", "embassy", "iot"],
            PkgCategory::WirelessIot,
            "https://github.com/embassy-rs/embassy",
            360_000,
        ),
        curated_crate(
            "esp-wifi",
            "0.12.0",
            "Bare-metal Wi-Fi and BLE wireless driver for Espressif ESP32/ESP32-C3/ESP32-S3.",
            &["esp32", "wifi", "ble", "wireless"],
            PkgCategory::WirelessIot,
            "https://github.com/esp-rs/esp-hal",
            175_000,
        ),
        curated_crate(
            "lora-phy",
            "3.0.1",
            "Async/sync LoRa radio driver for SX126x and SX127x sub-GHz transceivers.",
            &["lora", "radio", "sx1262", "wireless"],
            PkgCategory::WirelessIot,
            "https://github.com/lora-rs/lora-rs",
            39_000,
        ),
        curated_pio(
            "bblanchon/ArduinoJson",
            "7.3.0",
            "Zero-overhead C++ JSON serializer and deserializer for ESP32, STM32, and IoT nodes.",
            &["json", "iot", "wifi", "parser"],
            PkgCategory::WirelessIot,
            Some("MIT"),
            6_800,
        ),
        curated_pio(
            "knolleary/PubSubClient",
            "2.8",
            "Lightweight Arduino MQTT client library for Wi-Fi and Ethernet IoT telemetry.",
            &["mqtt", "iot", "wifi", "pubsub"],
            PkgCategory::WirelessIot,
            Some("MIT"),
            4_100,
        ),
        // Storage & Bus
        curated_crate(
            "embedded-hal",
            "1.0.0",
            "Hardware Abstraction Layer traits for SPI, I2C, GPIO, PWM, and Delay across Rust MCUs.",
            &["hal", "spi", "i2c", "gpio"],
            PkgCategory::StorageBus,
            "https://github.com/rust-embedded/embedded-hal",
            2_900_000,
        ),
        curated_crate(
            "embedded-sdmmc",
            "0.8.1",
            "Pure-Rust no_std FAT16/FAT32 SD card and MMC block filesystem driver.",
            &["sd", "fat32", "storage", "spi"],
            PkgCategory::StorageBus,
            "https://github.com/rust-embedded-community/embedded-sdmmc-rs",
            132_000,
        ),
        curated_crate(
            "littlefs2",
            "0.6.0",
            "Safe Rust wrapper around littlefs power-fail-safe flash filesystem for microcontrollers.",
            &["flash", "filesystem", "storage", "littlefs"],
            PkgCategory::StorageBus,
            "https://github.com/trussed-dev/littlefs2",
            91_000,
        ),
        curated_pio(
            "greiman/SdFat",
            "2.3.0",
            "Fast FAT16/FAT32/exFAT SD card DMA storage library for Arduino, Teensy, and STM32.",
            &["sd", "fat32", "storage", "spi"],
            PkgCategory::StorageBus,
            Some("MIT"),
            1_450,
        ),
    ];

    list.sort_by_key(|a| std::cmp::Reverse(a.popularity));
    list
}

fn curated_crate(
    name: &str,
    version: &str,
    description: &str,
    keywords: &[&str],
    category: PkgCategory,
    repository: &str,
    popularity: u64,
) -> PackageRecord {
    PackageRecord {
        source: RegistrySource::CratesIo,
        owner: None,
        name: name.to_string(),
        version: version.to_string(),
        description: description.to_string(),
        keywords: keywords.iter().map(|s| (*s).to_string()).collect(),
        category,
        license: Some("MIT OR Apache-2.0".to_string()),
        repository: Some(repository.to_string()),
        docs_url: Some(format!("https://docs.rs/{name}")),
        download_url: None,
        popularity,
        curated: true,
    }
}

fn curated_pio(
    repo_id: &str,
    version: &str,
    description: &str,
    keywords: &[&str],
    category: PkgCategory,
    license: Option<&str>,
    popularity: u64,
) -> PackageRecord {
    let (owner, name) = repo_id.split_once('/').unwrap_or(("", repo_id));
    PackageRecord {
        source: RegistrySource::PlatformIo,
        owner: (!owner.is_empty()).then(|| owner.to_string()),
        name: name.to_string(),
        version: version.to_string(),
        description: description.to_string(),
        keywords: keywords.iter().map(|s| (*s).to_string()).collect(),
        category,
        license: license.map(str::to_string),
        repository: None,
        docs_url: Some(format!(
            "https://registry.platformio.org/libraries/{owner}/{}",
            percent_encode(name)
        )),
        download_url: Some(format!(
            "https://dl.registry.platformio.org/download/{owner}/library/{}/{version}/{}-{version}.tar.gz",
            percent_encode(name),
            percent_encode(name)
        )),
        popularity,
        curated: true,
    }
}

/// Search the offline curated embedded catalog using `query`.
pub fn search_curated(query: &SearchQuery) -> Vec<PackageRecord> {
    let terms: Vec<String> = query
        .text
        .split_whitespace()
        .map(|s| s.to_ascii_lowercase())
        .collect();

    curated_catalog()
        .into_iter()
        .filter(|pkg| query.source.matches(pkg.source))
        .filter(|pkg| query.category == PkgCategory::All || pkg.category == query.category)
        .filter(|pkg| {
            if terms.is_empty() {
                return true;
            }
            let owner_lc = pkg.owner.as_deref().unwrap_or("").to_ascii_lowercase();
            let name_lc = pkg.name.to_ascii_lowercase();
            let desc_lc = pkg.description.to_ascii_lowercase();
            let kw_lc = pkg
                .keywords
                .iter()
                .map(|k| k.to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            terms.iter().all(|t| {
                name_lc.contains(t)
                    || owner_lc.contains(t)
                    || desc_lc.contains(t)
                    || kw_lc.contains(t)
            })
        })
        .collect()
}

/// Query curated + optional live crates.io and PlatformIO Registry v3 APIs.
pub fn search_packages(query: &SearchQuery) -> SearchOutcome {
    let curated_hits = search_curated(query);
    let mut by_key: BTreeMap<(u8, String, String), PackageRecord> = BTreeMap::new();
    let mut order: Vec<(u8, String, String)> = Vec::new();
    let source_ord = |src: RegistrySource| match src {
        RegistrySource::CratesIo => 0,
        RegistrySource::PlatformIo => 1,
    };

    for pkg in curated_hits {
        let (src, n, o) = pkg.dedup_key();
        let key = (source_ord(src), n, o);
        order.push(key.clone());
        by_key.insert(key, pkg);
    }

    let mut outcome = SearchOutcome::default();
    if !query.online {
        outcome.packages = order
            .into_iter()
            .filter_map(|k| by_key.remove(&k))
            .collect();
        return outcome;
    }

    let limit = query.limit.clamp(1, 50);
    let trimmed = query.text.trim();
    let effective = if trimmed.is_empty() {
        query.category.query_terms()
    } else {
        trimmed
    };

    if matches!(
        query.source,
        SearchSourceFilter::All | SearchSourceFilter::CratesIo
    ) {
        let url = if effective.is_empty() {
            format!("{CRATES_IO_API_BASE}?category=embedded&sort=downloads&per_page={limit}")
        } else {
            format!(
                "{CRATES_IO_API_BASE}?q={}&per_page={limit}",
                percent_encode(effective)
            )
        };
        match curl_get(&url) {
            Ok(body) => match parse_crates_io_response(&body) {
                Ok((crates, total)) => {
                    outcome.remote_total = outcome.remote_total.saturating_add(total);
                    merge_remote_records(crates, source_ord, &mut by_key, &mut order);
                }
                Err(err) => outcome
                    .warnings
                    .push(format!("crates.io parse error: {err}")),
            },
            Err(err) => outcome
                .warnings
                .push(format!("crates.io offline/unavailable: {err}")),
        }
    }

    if matches!(
        query.source,
        SearchSourceFilter::All | SearchSourceFilter::PlatformIo
    ) {
        let pio_query = if effective.is_empty() {
            "type:library".to_string()
        } else {
            format!("type:library {effective}")
        };
        let url = format!(
            "{PIO_REGISTRY_SEARCH_BASE}?query={}&limit={limit}",
            percent_encode(&pio_query)
        );
        match curl_get(&url) {
            Ok(body) => match parse_pio_registry_response(&body) {
                Ok((mut libs, total)) => {
                    outcome.remote_total = outcome.remote_total.saturating_add(total);
                    sort_pio_remote_results(&mut libs, effective);
                    merge_remote_records(libs, source_ord, &mut by_key, &mut order);
                }
                Err(err) => outcome
                    .warnings
                    .push(format!("PlatformIO registry parse error: {err}")),
            },
            Err(err) => outcome
                .warnings
                .push(format!("PlatformIO registry offline/unavailable: {err}")),
        }
    }

    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for key in order {
        if seen.insert(key.clone()) {
            if let Some(pkg) = by_key.remove(&key) {
                merged.push(pkg);
            }
        }
    }

    if !effective.is_empty() {
        let eff_lc = effective.to_ascii_lowercase();
        merged.sort_by(|a, b| {
            let a_exact = a.name.eq_ignore_ascii_case(&eff_lc);
            let b_exact = b.name.eq_ignore_ascii_case(&eff_lc);
            b_exact
                .cmp(&a_exact)
                .then_with(|| b.curated.cmp(&a.curated))
                .then_with(|| b.popularity.cmp(&a.popularity))
        });
    } else {
        merged.sort_by(|a, b| {
            b.curated
                .cmp(&a.curated)
                .then_with(|| b.popularity.cmp(&a.popularity))
        });
    }

    outcome.packages = merged;
    outcome
}

fn sort_pio_remote_results(libs: &mut [PackageRecord], effective: &str) {
    let eff_lc = effective.to_ascii_lowercase();
    libs.sort_by(|a, b| {
        let a_exact = !eff_lc.is_empty() && a.name.eq_ignore_ascii_case(&eff_lc);
        let b_exact = !eff_lc.is_empty() && b.name.eq_ignore_ascii_case(&eff_lc);
        b_exact
            .cmp(&a_exact)
            .then_with(|| b.popularity.cmp(&a.popularity))
    });
}

fn merge_remote_records(
    remote: Vec<PackageRecord>,
    source_ord: fn(RegistrySource) -> u8,
    by_key: &mut BTreeMap<(u8, String, String), PackageRecord>,
    order: &mut Vec<(u8, String, String)>,
) {
    for mut pkg in remote {
        let (src, n, o) = pkg.dedup_key();
        let key = (source_ord(src), n, o);
        if let Some(existing) = by_key.get_mut(&key) {
            existing.version = pkg.version;
            if pkg.download_url.is_some() {
                existing.download_url = pkg.download_url;
            }
            if pkg.popularity > 0 {
                existing.popularity = pkg.popularity;
            }
            if existing.license.is_none() {
                existing.license = pkg.license;
            }
            if existing.repository.is_none() {
                existing.repository = pkg.repository;
            }
            if existing.docs_url.is_none() {
                existing.docs_url = pkg.docs_url;
            }
            if existing.description.is_empty() {
                existing.description = pkg.description;
            }
            if existing.keywords.is_empty() {
                existing.keywords = pkg.keywords;
            }
        } else {
            pkg.category = classify_category(&pkg.name, &pkg.description, &pkg.keywords);
            order.push(key.clone());
            by_key.insert(key, pkg);
        }
    }
}

fn classify_category(name: &str, description: &str, keywords: &[String]) -> PkgCategory {
    let haystack = format!(
        "{} {} {}",
        name.to_ascii_lowercase(),
        description.to_ascii_lowercase(),
        keywords
            .iter()
            .map(|k| k.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" ")
    );
    if contains_any(
        &haystack,
        &["button", "debounce", "rotary", "encoder", "keypad", "touch"],
    ) {
        PkgCategory::ButtonsInput
    } else if contains_any(
        &haystack,
        &[
            "display", "oled", "ssd1306", "st7789", "ili9341", "lvgl", "tft", "lcd", "u8g2", "gui",
        ],
    ) {
        PkgCategory::DisplayGui
    } else if contains_any(
        &haystack,
        &[
            "sensor", "imu", "mpu6050", "bme280", "bmp280", "dht", "ina219", "gyro",
        ],
    ) {
        PkgCategory::SensorsImu
    } else if contains_any(
        &haystack,
        &[
            "motor", "pwm", "servo", "stepper", "pid", "ws2812", "led", "foc",
        ],
    ) {
        PkgCategory::MotorPwm
    } else if contains_any(
        &haystack,
        &[
            "wifi",
            "ble",
            "bluetooth",
            "mqtt",
            "lora",
            "http",
            "json",
            "net",
        ],
    ) {
        PkgCategory::WirelessIot
    } else if contains_any(
        &haystack,
        &[
            "sd", "fat", "littlefs", "flash", "eeprom", "i2c", "spi", "hal",
        ],
    ) {
        PkgCategory::StorageBus
    } else {
        PkgCategory::All
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn curl_get(url: &str) -> Result<String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "8", "-A", USER_AGENT, url])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CoreError::ToolNotFound("curl".to_string())
            } else {
                CoreError::Package(format!("curl failed to launch: {e}"))
            }
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(CoreError::Package(if stderr.is_empty() {
            format!("HTTP request failed with status {:?}", out.status.code())
        } else {
            stderr
        }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[derive(Deserialize)]
struct CratesIoResponse {
    #[serde(default)]
    crates: Vec<CratesIoItem>,
    #[serde(default)]
    meta: CratesIoMeta,
}

#[derive(Default, Deserialize)]
struct CratesIoMeta {
    #[serde(default)]
    total: usize,
}

#[derive(Deserialize)]
struct CratesIoItem {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    default_version: Option<String>,
    #[serde(default)]
    max_version: Option<String>,
    #[serde(default)]
    newest_version: Option<String>,
    #[serde(default)]
    max_stable_version: Option<String>,
    #[serde(default)]
    documentation: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    keywords: Option<Vec<String>>,
}

/// Parse raw JSON from `https://crates.io/api/v1/crates` into [`PackageRecord`]s.
pub fn parse_crates_io_response(json: &str) -> Result<(Vec<PackageRecord>, usize)> {
    let parsed: CratesIoResponse = serde_json::from_str(json)?;
    let records = parsed
        .crates
        .into_iter()
        .map(|c| {
            let version = c
                .max_stable_version
                .or(c.default_version)
                .or(c.max_version)
                .or(c.newest_version)
                .unwrap_or_else(|| "0.1.0".to_string());
            let keywords = c.keywords.unwrap_or_default();
            let description = c.description.unwrap_or_default().trim().to_string();
            let category = classify_category(&c.name, &description, &keywords);
            let docs_url = c
                .documentation
                .or_else(|| Some(format!("https://docs.rs/{}", c.name)));
            PackageRecord {
                source: RegistrySource::CratesIo,
                owner: None,
                name: c.name,
                version,
                description,
                keywords,
                category,
                license: None,
                repository: c.repository,
                docs_url,
                download_url: None,
                popularity: c.downloads,
                curated: false,
            }
        })
        .collect();
    Ok((records, parsed.meta.total))
}

#[derive(Deserialize)]
struct PioSearchResponse {
    #[serde(default)]
    total: usize,
    #[serde(default)]
    items: Vec<PioSearchItem>,
}

#[derive(Deserialize)]
struct PioSearchItem {
    #[serde(rename = "type", default)]
    item_type: Option<String>,
    #[serde(default)]
    owner: Option<PioOwner>,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    keywords: Option<Vec<String>>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    stars_count: u64,
    #[serde(default)]
    version: Option<PioVersion>,
}

#[derive(Deserialize)]
struct PioOwner {
    username: String,
}

#[derive(Deserialize)]
struct PioVersion {
    name: String,
    #[serde(default)]
    files: Vec<PioFile>,
}

#[derive(Deserialize)]
struct PioFile {
    #[serde(default)]
    download_url: Option<String>,
}

/// Parse raw JSON from `https://api.registry.platformio.org/v3/search` into [`PackageRecord`]s.
pub fn parse_pio_registry_response(json: &str) -> Result<(Vec<PackageRecord>, usize)> {
    let parsed: PioSearchResponse = serde_json::from_str(json)?;
    let records = parsed
        .items
        .into_iter()
        .filter(|it| it.item_type.as_deref().unwrap_or("library") == "library")
        .map(|it| {
            let owner = it.owner.map(|o| o.username);
            let (version, download_url) = match it.version {
                Some(v) => {
                    let dl = v.files.into_iter().find_map(|f| f.download_url);
                    (v.name, dl)
                }
                None => ("latest".to_string(), None),
            };
            let keywords = it.keywords.unwrap_or_default();
            let description = it.description.unwrap_or_default().trim().to_string();
            let category = classify_category(&it.name, &description, &keywords);
            let docs_url = owner.as_ref().map(|own| {
                format!(
                    "https://registry.platformio.org/libraries/{own}/{}",
                    percent_encode(&it.name)
                )
            });
            PackageRecord {
                source: RegistrySource::PlatformIo,
                owner,
                name: it.name,
                version,
                description,
                keywords,
                category,
                license: it.license,
                repository: None,
                docs_url,
                download_url,
                popularity: it.stars_count,
                curated: false,
            }
        })
        .collect();
    Ok((records, parsed.total))
}

/// Percent-encode arbitrary UTF-8 query text for safe HTTPS query parameters.
pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if is_unreserved {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Parse a `ravom.toml` `lib_deps` string (`crates:name@^ver`, `owner/Name@^ver`, `Name`).
pub fn parse_dep_spec(spec: &str) -> (RegistrySource, Option<String>, String, Option<String>) {
    let trimmed = spec.trim();
    if let Some(rest) = trimmed.strip_prefix("crates:") {
        let (name, ver) = split_at_version(rest);
        return (RegistrySource::CratesIo, None, name.to_string(), ver);
    }

    let (base, ver) = split_at_version(trimmed);
    if let Some((owner, name)) = base.split_once('/') {
        return (
            RegistrySource::PlatformIo,
            Some(owner.trim().to_string()),
            name.trim().to_string(),
            ver,
        );
    }

    let looks_like_pio = base.chars().any(|c| c.is_ascii_uppercase() || c == ' ')
        || curated_catalog()
            .iter()
            .any(|p| p.source == RegistrySource::PlatformIo && p.name.eq_ignore_ascii_case(base));
    if looks_like_pio {
        (RegistrySource::PlatformIo, None, base.to_string(), ver)
    } else {
        (RegistrySource::CratesIo, None, base.to_string(), ver)
    }
}

fn split_at_version(input: &str) -> (&str, Option<String>) {
    if let Some((lhs, rhs)) = input.rsplit_once('@') {
        if !rhs.contains('/') {
            let v = rhs.trim();
            let clean = if v.is_empty() {
                None
            } else {
                Some(v.to_string())
            };
            return (lhs.trim(), clean);
        }
    }
    (input.trim(), None)
}

/// Verify archive member paths from `tar -tzf` to prevent path-traversal extraction.
pub fn validate_archive_entries(listing: &str) -> Result<()> {
    for raw_line in listing.lines() {
        let entry = raw_line.trim();
        if entry.is_empty() {
            continue;
        }
        if entry.starts_with('/') || entry.starts_with('\\') {
            return Err(CoreError::Package(format!(
                "refusing archive with absolute path: {entry}"
            )));
        }
        let bytes = entry.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return Err(CoreError::Package(format!(
                "refusing archive with Windows drive path: {entry}"
            )));
        }
        for comp in Path::new(entry).components() {
            if matches!(
                comp,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) {
                return Err(CoreError::Package(format!(
                    "refusing archive with unsafe path component: {entry}"
                )));
            }
        }
    }
    Ok(())
}

/// Sanitize a library name into a single safe directory name under `.ravom/libdeps/<env>/`.
pub fn sanitize_dir_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            _ => c,
        })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        "lib".to_string()
    } else {
        cleaned
    }
}

/// Directory storing unpacked PlatformIO libraries for a specific environment.
pub fn libdeps_dir(project_dir: &Path, env_name: &str) -> PathBuf {
    project_dir
        .join(".ravom")
        .join("libdeps")
        .join(sanitize_dir_name(env_name))
}

/// List dependencies configured in `ravom.toml` (`lib_deps`) and `Cargo.toml` (`[dependencies]`).
pub fn list_installed(project_dir: &Path, env_name: Option<&str>) -> Result<Vec<InstalledPackage>> {
    let mut result: Vec<InstalledPackage> = Vec::new();
    let mut resolved_env = env_name.map(str::to_string);

    match RavomConfig::load_from_dir(project_dir) {
        Ok(cfg) => {
            if let Ok((name, env)) = cfg.resolve_env(env_name) {
                resolved_env = Some(name.to_string());
                let base_dir = libdeps_dir(project_dir, name);
                for spec in &env.lib_deps {
                    let (source, owner, pkg_name, version) = parse_dep_spec(spec);
                    let candidate_dir = base_dir.join(sanitize_dir_name(&pkg_name));
                    let local_dir = candidate_dir.is_dir().then_some(candidate_dir);
                    result.push(InstalledPackage {
                        spec: spec.clone(),
                        name: pkg_name,
                        owner,
                        version,
                        source,
                        origin: InstalledOrigin::RavomManifest,
                        local_dir,
                    });
                }
            }
        }
        Err(CoreError::ManifestNotFound(_)) => {}
        Err(err) => return Err(err),
    }

    let cargo_path = project_dir.join("Cargo.toml");
    if cargo_path.is_file() {
        let raw = fs::read_to_string(&cargo_path)?;
        if let Ok(doc) = toml::from_str::<toml::Value>(&raw) {
            if let Some(deps) = doc.get("dependencies").and_then(|v| v.as_table()) {
                for (dep_name, dep_val) in deps {
                    let version = match dep_val {
                        toml::Value::String(s) => Some(s.clone()),
                        toml::Value::Table(t) => t
                            .get("version")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        _ => None,
                    };
                    if let Some(existing) = result.iter_mut().find(|p| {
                        p.source == RegistrySource::CratesIo
                            && p.name.eq_ignore_ascii_case(dep_name)
                    }) {
                        existing.origin = InstalledOrigin::Both;
                        if existing.version.is_none() {
                            existing.version = version;
                        }
                    } else {
                        let spec = match &version {
                            Some(v) => format!("crates:{dep_name}@{v}"),
                            None => format!("crates:{dep_name}"),
                        };
                        result.push(InstalledPackage {
                            spec,
                            name: dep_name.clone(),
                            owner: None,
                            version,
                            source: RegistrySource::CratesIo,
                            origin: InstalledOrigin::CargoToml,
                            local_dir: None,
                        });
                    }
                }
            }
        }
    }

    let _ = resolved_env;
    Ok(result)
}

/// Add `pkg` to `ravom.toml` (`lib_deps`) and install it into Cargo or `.ravom/libdeps/<env>/`.
pub fn install_package(
    project_dir: &Path,
    env_name: Option<&str>,
    pkg: &PackageRecord,
    log: &mut dyn FnMut(String),
) -> Result<InstalledPackage> {
    let mut cfg = RavomConfig::load_from_dir(project_dir)?;
    let (active_env_name, _) = cfg.resolve_env(env_name)?;
    let active_env_name = active_env_name.to_string();

    let dep_spec = pkg.dep_spec();
    if let Some(env_cfg) = cfg.env.get_mut(&active_env_name) {
        env_cfg
            .lib_deps
            .retain(|existing| !pkg.matches_spec(existing));
        env_cfg.lib_deps.push(dep_spec.clone());
    }
    fs::write(project_dir.join("ravom.toml"), cfg.to_toml_string()?)?;
    log(format!(
        "Updated ravom.toml [env.{active_env_name}]: + {dep_spec}"
    ));

    let mut local_dir = None;
    let mut origin = InstalledOrigin::RavomManifest;

    match pkg.source {
        RegistrySource::CratesIo => {
            if project_dir.join("Cargo.toml").is_file() {
                let target_arg = if pkg.version.trim().is_empty() {
                    pkg.name.clone()
                } else {
                    format!("{}@{}", pkg.name, pkg.version.trim())
                };
                log(format!("$ cargo add {target_arg}"));
                match Command::new("cargo")
                    .args(["add", &target_arg])
                    .current_dir(project_dir)
                    .output()
                {
                    Ok(out) => {
                        emit_process_output(&out, log);
                        if out.status.success() {
                            origin = InstalledOrigin::Both;
                        } else {
                            log(format!(
                                "Note: `cargo add {target_arg}` failed; kept {dep_spec} in ravom.toml"
                            ));
                        }
                    }
                    Err(err) => {
                        log(format!(
                            "Note: could not invoke `cargo add`: {err}; kept {dep_spec} in ravom.toml"
                        ));
                    }
                }
            }
        }
        RegistrySource::PlatformIo => {
            let unpacked = download_and_extract_pio(project_dir, &active_env_name, pkg, log)?;
            local_dir = Some(unpacked);
        }
    }

    Ok(InstalledPackage {
        spec: dep_spec,
        name: pkg.name.clone(),
        owner: pkg.owner.clone(),
        version: (!pkg.version.is_empty()).then(|| pkg.version.clone()),
        source: pkg.source,
        origin,
        local_dir,
    })
}

/// Resolve an arbitrary dependency spec string and install it into `project_dir`.
pub fn install_spec(
    project_dir: &Path,
    env_name: Option<&str>,
    spec: &str,
    log: &mut dyn FnMut(String),
) -> Result<InstalledPackage> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Package(
            "dependency spec cannot be empty".to_string(),
        ));
    }
    let (source, owner, name, version) = parse_dep_spec(trimmed);
    let query = SearchQuery {
        text: name.clone(),
        source: match source {
            RegistrySource::CratesIo => SearchSourceFilter::CratesIo,
            RegistrySource::PlatformIo => SearchSourceFilter::PlatformIo,
        },
        category: PkgCategory::All,
        limit: 10,
        online: true,
    };
    let outcome = search_packages(&query);
    for w in outcome.warnings {
        log(format!("Search note: {w}"));
    }

    let mut pkg = outcome
        .packages
        .into_iter()
        .find(|p| {
            p.source == source
                && p.name.eq_ignore_ascii_case(&name)
                && owner.as_ref().is_none_or(|o| {
                    p.owner
                        .as_deref()
                        .is_some_and(|po| po.eq_ignore_ascii_case(o))
                })
        })
        .unwrap_or_else(|| PackageRecord {
            source,
            owner: owner.clone(),
            name: name.clone(),
            version: version.clone().unwrap_or_default(),
            description: String::new(),
            keywords: Vec::new(),
            category: PkgCategory::All,
            license: None,
            repository: None,
            docs_url: None,
            download_url: None,
            popularity: 0,
            curated: false,
        });

    if let Some(explicit_ver) = version {
        pkg.version = explicit_ver;
    }

    install_package(project_dir, env_name, &pkg, log)
}

/// Remove a matching package from `ravom.toml`, `.ravom/libdeps/<env>/`, and `Cargo.toml`.
pub fn remove_package(
    project_dir: &Path,
    env_name: Option<&str>,
    name_or_spec: &str,
    log: &mut dyn FnMut(String),
) -> Result<bool> {
    let target = name_or_spec.trim();
    if target.is_empty() {
        return Ok(false);
    }
    let (parsed_src, parsed_owner, parsed_name, _) = parse_dep_spec(target);
    let mut cfg = RavomConfig::load_from_dir(project_dir)?;
    let (active_env_name, _) = cfg.resolve_env(env_name)?;
    let active_env_name = active_env_name.to_string();
    let mut changed = false;

    if let Some(env_cfg) = cfg.env.get_mut(&active_env_name) {
        let before = env_cfg.lib_deps.len();
        env_cfg.lib_deps.retain(|existing| {
            if existing.eq_ignore_ascii_case(target) {
                return false;
            }
            let (e_src, e_owner, e_name, _) = parse_dep_spec(existing);
            let same_name = e_name.eq_ignore_ascii_case(&parsed_name);
            let same_owner = parsed_owner.as_ref().is_none_or(|o| {
                e_owner
                    .as_deref()
                    .is_some_and(|eo| eo.eq_ignore_ascii_case(o))
            });
            let same_src = !target.contains(':') || e_src == parsed_src;
            !(same_name && same_owner && same_src)
        });
        changed = env_cfg.lib_deps.len() != before;
    }

    if changed {
        fs::write(project_dir.join("ravom.toml"), cfg.to_toml_string()?)?;
        log(format!(
            "Updated ravom.toml [env.{active_env_name}]: - {target}"
        ));
    }

    let dir_path = libdeps_dir(project_dir, &active_env_name).join(sanitize_dir_name(&parsed_name));
    if dir_path.is_dir() {
        fs::remove_dir_all(&dir_path)?;
        log(format!("Removed {}", dir_path.display()));
        changed = true;
    }

    if project_dir.join("Cargo.toml").is_file() && parsed_src == RegistrySource::CratesIo {
        log(format!("$ cargo remove {parsed_name}"));
        if let Ok(out) = Command::new("cargo")
            .args(["remove", &parsed_name])
            .current_dir(project_dir)
            .output()
        {
            emit_process_output(&out, log);
            if out.status.success() {
                changed = true;
            }
        }
    }

    Ok(changed)
}

/// Ensure every dependency listed in `[env.<active>].lib_deps` is unpacked locally.
pub fn sync_lib_deps(
    project_dir: &Path,
    env_name: Option<&str>,
    log: &mut dyn FnMut(String),
) -> Result<usize> {
    let cfg = RavomConfig::load_from_dir(project_dir)?;
    let (active_env_name, env) = cfg.resolve_env(env_name)?;
    let active_env_name = active_env_name.to_string();
    let specs = env.lib_deps.clone();
    let mut synced = 0usize;

    for spec in &specs {
        let (source, owner, name, _) = parse_dep_spec(spec);
        if source == RegistrySource::PlatformIo {
            let unpacked =
                libdeps_dir(project_dir, &active_env_name).join(sanitize_dir_name(&name));
            if unpacked.is_dir() {
                synced += 1;
                continue;
            }
            let rec_query = SearchQuery {
                text: name.clone(),
                source: SearchSourceFilter::PlatformIo,
                category: PkgCategory::All,
                limit: 10,
                online: true,
            };
            let outcome = search_packages(&rec_query);
            if let Some(pkg) = outcome.packages.into_iter().find(|p| {
                p.name.eq_ignore_ascii_case(&name)
                    && owner.as_ref().is_none_or(|o| {
                        p.owner
                            .as_deref()
                            .is_some_and(|po| po.eq_ignore_ascii_case(o))
                    })
            }) {
                download_and_extract_pio(project_dir, &active_env_name, &pkg, log)?;
                synced += 1;
            } else {
                log(format!(
                    "Warning: could not resolve download URL for {spec}"
                ));
            }
        } else {
            synced += 1;
        }
    }

    log(format!(
        "Synced {synced} library entry(s) for [env.{active_env_name}]"
    ));
    Ok(synced)
}

fn download_and_extract_pio(
    project_dir: &Path,
    env_name: &str,
    pkg: &PackageRecord,
    log: &mut dyn FnMut(String),
) -> Result<PathBuf> {
    let dl_url = pkg
        .download_url
        .clone()
        .or_else(|| resolve_pio_download_url(pkg.owner.as_deref(), &pkg.name))
        .ok_or_else(|| {
            CoreError::Package(format!(
                "no download URL available for PlatformIO library '{}'",
                pkg.qualified_id()
            ))
        })?;

    let base_dir = libdeps_dir(project_dir, env_name);
    fs::create_dir_all(&base_dir)?;
    let safe_name = sanitize_dir_name(&pkg.name);
    let target_dir = base_dir.join(&safe_name);
    let tmp_archive = base_dir.join(format!(".{safe_name}.tar.gz.part"));

    log(format!("Downloading {dl_url}"));
    let dl_out = Command::new("curl")
        .args(["-fsSL", "--max-time", "20", "-A", USER_AGENT, "-o"])
        .arg(&tmp_archive)
        .arg(&dl_url)
        .output()
        .map_err(|e| {
            let _ = fs::remove_file(&tmp_archive);
            CoreError::Package(format!("curl failed: {e}"))
        })?;
    if !dl_out.status.success() {
        let _ = fs::remove_file(&tmp_archive);
        return Err(CoreError::Package(format!(
            "download failed for {}: {}",
            pkg.qualified_id(),
            String::from_utf8_lossy(&dl_out.stderr).trim()
        )));
    }

    let list_out = Command::new("tar")
        .arg("-tzf")
        .arg(&tmp_archive)
        .output()
        .map_err(|e| {
            let _ = fs::remove_file(&tmp_archive);
            CoreError::Package(format!("tar list failed: {e}"))
        })?;
    if !list_out.status.success() {
        let _ = fs::remove_file(&tmp_archive);
        return Err(CoreError::Package(format!(
            "invalid archive for {}: {}",
            pkg.qualified_id(),
            String::from_utf8_lossy(&list_out.stderr).trim()
        )));
    }
    let listing = String::from_utf8_lossy(&list_out.stdout);
    if let Err(err) = validate_archive_entries(&listing) {
        let _ = fs::remove_file(&tmp_archive);
        return Err(err);
    }

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    let extract_out = Command::new("tar")
        .arg("-xzf")
        .arg(&tmp_archive)
        .arg("-C")
        .arg(&target_dir)
        .output()
        .map_err(|e| {
            let _ = fs::remove_file(&tmp_archive);
            CoreError::Package(format!("tar extract failed: {e}"))
        })?;
    let _ = fs::remove_file(&tmp_archive);

    if !extract_out.status.success() {
        return Err(CoreError::Package(format!(
            "extract failed for {}: {}",
            pkg.qualified_id(),
            String::from_utf8_lossy(&extract_out.stderr).trim()
        )));
    }

    log(format!(
        "Installed {} -> {}",
        pkg.name,
        target_dir.display()
    ));
    Ok(target_dir)
}

fn resolve_pio_download_url(owner: Option<&str>, name: &str) -> Option<String> {
    let q = match owner {
        Some(o) if !o.is_empty() => format!("type:library owner:{o} {name}"),
        _ => format!("type:library {name}"),
    };
    let url = format!(
        "{PIO_REGISTRY_SEARCH_BASE}?query={}&limit=5",
        percent_encode(&q)
    );
    let body = curl_get(&url).ok()?;
    let (libs, _) = parse_pio_registry_response(&body).ok()?;
    libs.into_iter()
        .find(|l| {
            l.name.eq_ignore_ascii_case(name)
                && owner.is_none_or(|o| {
                    l.owner
                        .as_deref()
                        .is_some_and(|lo| lo.eq_ignore_ascii_case(o))
                })
        })
        .and_then(|l| l.download_url)
}

fn emit_process_output(out: &std::process::Output, log: &mut dyn FnMut(String)) {
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let t = line.trim_end();
        if !t.is_empty() {
            log(t.to_string());
        }
    }
    for line in String::from_utf8_lossy(&out.stderr).lines() {
        let t = line.trim_end();
        if !t.is_empty() {
            log(t.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::init_project;

    #[test]
    fn percent_encode_handles_spaces_colons_and_slashes() {
        assert_eq!(
            percent_encode("type:library button/gui"),
            "type%3Alibrary%20button%2Fgui"
        );
    }

    #[test]
    fn parses_crates_io_search_json() {
        let sample = r#"{
            "crates": [{
                "name": "async-button",
                "description": "Async button handling crate",
                "downloads": 6209,
                "max_stable_version": "0.2.0",
                "documentation": null,
                "repository": "https://github.com/avsaase/async-button",
                "keywords": ["button", "no_std"]
            }],
            "meta": { "total": 1424 }
        }"#;
        let (list, total) = parse_crates_io_response(sample).expect("parse crates.io JSON");
        assert_eq!(total, 1424);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].source, RegistrySource::CratesIo);
        assert_eq!(list[0].name, "async-button");
        assert_eq!(list[0].version, "0.2.0");
        assert_eq!(list[0].popularity, 6209);
        assert_eq!(list[0].category, PkgCategory::ButtonsInput);
        assert_eq!(list[0].dep_spec(), "crates:async-button@^0.2.0");
    }

    #[test]
    fn parses_platformio_registry_search_json() {
        let sample = r#"{
            "total": 20,
            "items": [{
                "type": "library",
                "owner": { "username": "mathertel" },
                "name": "OneButton",
                "description": "Pushbutton single, double and long press library",
                "keywords": ["arduino", "button"],
                "license": "BSD-3-Clause",
                "stars_count": 720,
                "version": {
                    "name": "2.0.3",
                    "files": [{
                        "download_url": "https://dl.registry.platformio.org/download/mathertel/library/OneButton/2.0.3/OneButton-2.0.3.tar.gz"
                    }]
                }
            }]
        }"#;
        let (list, total) = parse_pio_registry_response(sample).expect("parse PIO JSON");
        assert_eq!(total, 20);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].source, RegistrySource::PlatformIo);
        assert_eq!(list[0].owner.as_deref(), Some("mathertel"));
        assert_eq!(list[0].version, "2.0.3");
        assert_eq!(list[0].popularity, 720);
        assert!(list[0].download_url.is_some());
        assert_eq!(list[0].dep_spec(), "mathertel/OneButton@^2.0.3");
    }

    #[test]
    fn curated_catalog_searches_buttons_and_display_drivers_offline() {
        let btn = search_packages(&SearchQuery {
            text: "button".to_string(),
            source: SearchSourceFilter::All,
            category: PkgCategory::All,
            limit: 20,
            online: false,
        });
        assert!(btn.packages.iter().any(|p| p.name == "OneButton"));
        assert!(btn.packages.iter().any(|p| p.name == "async-button"));

        let disp = search_packages(&SearchQuery {
            text: "st7789".to_string(),
            source: SearchSourceFilter::All,
            category: PkgCategory::DisplayGui,
            limit: 20,
            online: false,
        });
        assert!(disp.packages.iter().any(|p| p.name == "mipidsi"));
        assert!(disp.packages.iter().any(|p| p.name == "TFT_eSPI"));
    }

    #[test]
    fn parse_dep_spec_handles_crates_pio_and_bare_names() {
        assert_eq!(
            parse_dep_spec("crates:debouncr@^0.2.2"),
            (
                RegistrySource::CratesIo,
                None,
                "debouncr".to_string(),
                Some("^0.2.2".to_string())
            )
        );
        assert_eq!(
            parse_dep_spec("mathertel/OneButton@^2.0.3"),
            (
                RegistrySource::PlatformIo,
                Some("mathertel".to_string()),
                "OneButton".to_string(),
                Some("^2.0.3".to_string())
            )
        );
        assert_eq!(
            parse_dep_spec("OneButton"),
            (
                RegistrySource::PlatformIo,
                None,
                "OneButton".to_string(),
                None
            )
        );
    }

    #[test]
    fn add_list_and_remove_lib_deps_roundtrip_in_ravom_toml() {
        let tmp = tempfile::tempdir().expect("tempdir");
        init_project(tmp.path(), "atmelavr/uno", Some("arduino")).expect("init arduino");
        let pkg = curated_catalog()
            .into_iter()
            .find(|p| p.name == "async-button")
            .expect("async-button in catalog");
        let mut logs = Vec::new();
        install_package(tmp.path(), None, &pkg, &mut |l| logs.push(l)).expect("install");

        let installed = list_installed(tmp.path(), None).expect("list");
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].name, "async-button");
        assert!(pkg.matches_spec(&installed[0].spec));

        let removed = remove_package(tmp.path(), None, &installed[0].spec, &mut |l| logs.push(l))
            .expect("remove");
        assert!(removed);
        let after = list_installed(tmp.path(), None).expect("list after remove");
        assert!(after.is_empty());
    }

    #[test]
    fn validate_archive_entries_rejects_path_traversal() {
        assert!(validate_archive_entries("OneButton.h\nsrc/OneButton.cpp\n").is_ok());
        assert!(validate_archive_entries("../etc/passwd\n").is_err());
        assert!(validate_archive_entries("/etc/passwd\n").is_err());
        assert!(validate_archive_entries("C:\\evil.txt\n").is_err());
    }
}
