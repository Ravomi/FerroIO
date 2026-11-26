//! RavomIO 插件协议与框架注册表（支持 Embassy / RTIC / ESP-IDF / FreeRTOS / Zephyr / RT-Thread）。

/// High-level project layout and build backend family for a framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameworkKind {
    RustCargo,
    CCpp,
    Zephyr,
    FpgaHdl { uses_cargo: bool },
}

/// Metadata and template generator for an embedded framework or RTOS plugin.
pub trait FrameworkPlugin: Send + Sync {
    /// Short identifier used in `ravom.toml` (e.g. `embassy`, `rtic`, `freertos`, `zephyr`).
    fn id(&self) -> &'static str;

    /// Human-readable framework name.
    fn display_name(&self) -> &'static str;

    /// Whether this framework is a traditional or Rust-native RTOS / async executor.
    fn is_rtos(&self) -> bool;

    /// Project layout and build backend family.
    fn kind(&self) -> FrameworkKind;

    /// Generate starter `src/main.rs` source for a newly initialized project.
    fn starter_main_rs(&self, board_id: &str) -> String;

    /// Generate initial project files (relative path, contents) for `init_project`.
    fn scaffold_files(
        &self,
        project_name: &str,
        board_id: &str,
        rust_target: Option<&str>,
    ) -> Vec<(String, String)>;
}

struct BuiltinFramework {
    id: &'static str,
    display_name: &'static str,
    is_rtos: bool,
    kind: FrameworkKind,
}

impl FrameworkPlugin for BuiltinFramework {
    fn id(&self) -> &'static str {
        self.id
    }

    fn display_name(&self) -> &'static str {
        self.display_name
    }

    fn is_rtos(&self) -> bool {
        self.is_rtos
    }

    fn kind(&self) -> FrameworkKind {
        self.kind
    }

    fn starter_main_rs(&self, board_id: &str) -> String {
        match self.id {
            "embassy" => format!(
                "// RavomIO Embassy async starter for `{board_id}`\n\
                 #![no_std]\n\
                 #![no_main]\n\n\
                 use core::panic::PanicInfo;\n\n\
                 #[panic_handler]\n\
                 fn panic(_info: &PanicInfo) -> ! {{\n\
                     loop {{}}\n\
                 }}\n\n\
                 #[no_mangle]\n\
                 pub extern \"C\" fn _start() -> ! {{\n\
                     loop {{}}\n\
                 }}\n"
            ),
            "rtic" => format!(
                "// RavomIO RTIC hard real-time starter for `{board_id}`\n\
                 #![no_std]\n\
                 #![no_main]\n\n\
                 use core::panic::PanicInfo;\n\n\
                 #[panic_handler]\n\
                 fn panic(_info: &PanicInfo) -> ! {{\n\
                     loop {{}}\n\
                 }}\n\n\
                 #[no_mangle]\n\
                 pub extern \"C\" fn _start() -> ! {{\n\
                     loop {{}}\n\
                 }}\n"
            ),
            "zephyr-rust" => format!(
                "// RavomIO Zephyr Rust starter for `{board_id}`\n\
                 #![no_std]\n\n\
                 use zephyr::time::{{sleep, Duration}};\n\n\
                 #[no_mangle]\n\
                 pub extern \"C\" fn rust_main() {{\n\
                     zephyr::printkln!(\"RavomIO Zephyr Rust booted on {board_id}\");\n\
                     loop {{\n\
                         sleep(Duration::millis_at_least(1000));\n\
                     }}\n\
                 }}\n"
            ),
            _ => format!(
                "// RavomIO starter for `{board_id}` (framework: {})\n\
                 #![no_std]\n\
                 #![no_main]\n\n\
                 use core::panic::PanicInfo;\n\n\
                 #[panic_handler]\n\
                 fn panic(_info: &PanicInfo) -> ! {{\n\
                     loop {{}}\n\
                 }}\n\n\
                 #[no_mangle]\n\
                 pub extern \"C\" fn _start() -> ! {{\n\
                     loop {{}}\n\
                 }}\n",
                self.id
            ),
        }
    }

    fn scaffold_files(
        &self,
        project_name: &str,
        board_id: &str,
        rust_target: Option<&str>,
    ) -> Vec<(String, String)> {
        let pkg_name = project_name.replace(' ', "-");
        match self.kind {
            FrameworkKind::RustCargo => {
                let mut files = vec![
                    (
                        "Cargo.toml".to_string(),
                        format!(
                            "[package]\nname = \"{pkg_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
                        ),
                    ),
                    ("src/main.rs".to_string(), self.starter_main_rs(board_id)),
                    (".gitignore".to_string(), "target/\n.ravom/\n".to_string()),
                ];
                if let Some(target) = rust_target {
                    files.push((
                        ".cargo/config.toml".to_string(),
                        format!("[build]\ntarget = \"{target}\"\n"),
                    ));
                }
                files
            }
            FrameworkKind::CCpp => {
                let (entry_path, entry_src) = if self.id == "arduino" {
                    (
                        "src/main.cpp".to_string(),
                        format!(
                            "// RavomIO Arduino starter for `{board_id}`\n\
                             #include <Arduino.h>\n\n\
                             void setup() {{}}\n\n\
                             void loop() {{}}\n"
                        ),
                    )
                } else {
                    (
                        "src/main.c".to_string(),
                        format!(
                            "// RavomIO C starter for `{board_id}` (framework: {})\n\
                             int main(void) {{\n\
                                 while (1) {{}}\n\
                                 return 0;\n\
                             }}\n",
                            self.id
                        ),
                    )
                };
                vec![
                    (entry_path, entry_src),
                    (".gitignore".to_string(), "build/\n.ravom/\n".to_string()),
                ]
            }
            FrameworkKind::Zephyr => Vec::new(),
            FrameworkKind::FpgaHdl { uses_cargo } => {
                let mut files = vec![
                    (
                        "top.v".to_string(),
                        format!(
                            "// RavomIO FPGA top module for `{board_id}` (framework: {})\n\
                             module top(\n\
                                 input wire clk,\n\
                                 output wire led\n\
                             );\n\
                                 assign led = clk;\n\
                             endmodule\n",
                            self.id
                        ),
                    ),
                    (
                        ".gitignore".to_string(),
                        "build/\ntarget/\n.ravom/\n".to_string(),
                    ),
                ];
                if uses_cargo {
                    files.push((
                        "Cargo.toml".to_string(),
                        format!(
                            "[package]\nname = \"{pkg_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
                        ),
                    ));
                    files.push(("src/main.rs".to_string(), self.starter_main_rs(board_id)));
                }
                files
            }
        }
    }
}

static BUILTIN_FRAMEWORKS: &[BuiltinFramework] = &[
    BuiltinFramework {
        id: "embassy",
        display_name: "Embassy (Rust Async Executor)",
        is_rtos: true,
        kind: FrameworkKind::RustCargo,
    },
    BuiltinFramework {
        id: "rtic",
        display_name: "RTIC (Real-Time Interrupt-driven Concurrency)",
        is_rtos: true,
        kind: FrameworkKind::RustCargo,
    },
    BuiltinFramework {
        id: "esp-idf",
        display_name: "ESP-IDF (FreeRTOS-based)",
        is_rtos: true,
        kind: FrameworkKind::CCpp,
    },
    BuiltinFramework {
        id: "freertos",
        display_name: "FreeRTOS",
        is_rtos: true,
        kind: FrameworkKind::CCpp,
    },
    BuiltinFramework {
        id: "arduino",
        display_name: "Arduino",
        is_rtos: false,
        kind: FrameworkKind::CCpp,
    },
    BuiltinFramework {
        id: "stm32cube",
        display_name: "STM32Cube HAL",
        is_rtos: false,
        kind: FrameworkKind::CCpp,
    },
    BuiltinFramework {
        id: "cmsis",
        display_name: "CMSIS",
        is_rtos: false,
        kind: FrameworkKind::CCpp,
    },
    BuiltinFramework {
        id: "zephyr",
        display_name: "Zephyr RTOS (C / CMake / west)",
        is_rtos: true,
        kind: FrameworkKind::Zephyr,
    },
    BuiltinFramework {
        id: "zephyr-rust",
        display_name: "Zephyr RTOS (Rust via zephyr-lang-rust)",
        is_rtos: true,
        kind: FrameworkKind::Zephyr,
    },
    BuiltinFramework {
        id: "rt-thread",
        display_name: "RT-Thread RTOS",
        is_rtos: true,
        kind: FrameworkKind::CCpp,
    },
    BuiltinFramework {
        id: "bare-metal",
        display_name: "Bare-Metal (#![no_std])",
        is_rtos: false,
        kind: FrameworkKind::RustCargo,
    },
    BuiltinFramework {
        id: "rust-hdl",
        display_name: "RustHDL (FPGA: Rust -> Verilog -> yosys/nextpnr)",
        is_rtos: false,
        kind: FrameworkKind::FpgaHdl { uses_cargo: true },
    },
    BuiltinFramework {
        id: "spade",
        display_name: "Spade HDL (Rust-inspired FPGA HDL)",
        is_rtos: false,
        kind: FrameworkKind::FpgaHdl { uses_cargo: false },
    },
    BuiltinFramework {
        id: "veryl",
        display_name: "Veryl HDL (Modern SystemVerilog alternative)",
        is_rtos: false,
        kind: FrameworkKind::FpgaHdl { uses_cargo: false },
    },
    BuiltinFramework {
        id: "verilog",
        display_name: "Verilog HDL",
        is_rtos: false,
        kind: FrameworkKind::FpgaHdl { uses_cargo: false },
    },
    BuiltinFramework {
        id: "mirui",
        display_name: "Mirui (no_std ECS Embedded & Desktop UI Framework)",
        is_rtos: false,
        kind: FrameworkKind::RustCargo,
    },
];

/// Return all built-in framework/RTOS plugins.
pub fn builtin_framework_plugins() -> Vec<&'static dyn FrameworkPlugin> {
    BUILTIN_FRAMEWORKS
        .iter()
        .map(|f| f as &'static dyn FrameworkPlugin)
        .collect()
}

/// Look up a built-in framework plugin by ID (case-insensitive).
pub fn find_framework_plugin(id: &str) -> Option<&'static dyn FrameworkPlugin> {
    BUILTIN_FRAMEWORKS
        .iter()
        .find(|f| f.id.eq_ignore_ascii_case(id))
        .map(|f| f as &'static dyn FrameworkPlugin)
}

#[cfg(test)]
mod tests {
    use super::{builtin_framework_plugins, find_framework_plugin};

    #[test]
    fn finds_rtos_and_async_framework_plugins() {
        assert!(builtin_framework_plugins().len() >= 12);
        let embassy = find_framework_plugin("embassy").expect("embassy plugin should exist");
        assert!(embassy.is_rtos());
        assert_eq!(embassy.kind(), super::FrameworkKind::RustCargo);
        assert!(embassy
            .starter_main_rs("bluepill_f103c8")
            .contains("#![no_std]"));

        let zephyr = find_framework_plugin("zephyr").expect("zephyr plugin should exist");
        assert!(zephyr.is_rtos());
        assert_eq!(zephyr.kind(), super::FrameworkKind::Zephyr);
    }

    #[test]
    fn scaffold_files_match_framework_kind() {
        let arduino = find_framework_plugin("arduino").expect("arduino plugin");
        let files = arduino.scaffold_files("blink", "uno", None);
        assert!(files.iter().any(|(p, _)| p == "src/main.cpp"));
        assert!(!files.iter().any(|(p, _)| p == "Cargo.toml"));

        let veryl = find_framework_plugin("veryl").expect("veryl plugin");
        let vfiles = veryl.scaffold_files("fpga-demo", "icestick", None);
        assert!(vfiles.iter().any(|(p, _)| p == "top.v"));
        assert!(!vfiles.iter().any(|(p, _)| p == "Cargo.toml"));
    }
}
