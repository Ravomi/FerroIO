//! RavomIO 插件协议与框架注册表（支持 Embassy / RTIC / ESP-IDF / FreeRTOS / Zephyr / RT-Thread）。

/// Metadata and template generator for an embedded framework or RTOS plugin.
pub trait FrameworkPlugin: Send + Sync {
    /// Short identifier used in `ravom.toml` (e.g. `embassy`, `rtic`, `freertos`, `zephyr`).
    fn id(&self) -> &'static str;

    /// Human-readable framework name.
    fn display_name(&self) -> &'static str;

    /// Whether this framework is a traditional or Rust-native RTOS / async executor.
    fn is_rtos(&self) -> bool;

    /// Generate starter `src/main.rs` source for a newly initialized project.
    fn starter_main_rs(&self, board_id: &str) -> String;
}

struct BuiltinFramework {
    id: &'static str,
    display_name: &'static str,
    is_rtos: bool,
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
}

static BUILTIN_FRAMEWORKS: &[BuiltinFramework] = &[
    BuiltinFramework {
        id: "embassy",
        display_name: "Embassy (Rust Async Executor)",
        is_rtos: true,
    },
    BuiltinFramework {
        id: "rtic",
        display_name: "RTIC (Real-Time Interrupt-driven Concurrency)",
        is_rtos: true,
    },
    BuiltinFramework {
        id: "esp-idf",
        display_name: "ESP-IDF (FreeRTOS-based)",
        is_rtos: true,
    },
    BuiltinFramework {
        id: "freertos",
        display_name: "FreeRTOS",
        is_rtos: true,
    },
    BuiltinFramework {
        id: "zephyr",
        display_name: "Zephyr RTOS",
        is_rtos: true,
    },
    BuiltinFramework {
        id: "rt-thread",
        display_name: "RT-Thread RTOS",
        is_rtos: true,
    },
    BuiltinFramework {
        id: "bare-metal",
        display_name: "Bare-Metal (#![no_std])",
        is_rtos: false,
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
        assert!(builtin_framework_plugins().len() >= 6);
        let embassy = find_framework_plugin("embassy").expect("embassy plugin should exist");
        assert!(embassy.is_rtos());
        assert!(embassy
            .starter_main_rs("bluepill_f103c8")
            .contains("#![no_std]"));

        let zephyr = find_framework_plugin("zephyr").expect("zephyr plugin should exist");
        assert!(zephyr.is_rtos());
    }
}
