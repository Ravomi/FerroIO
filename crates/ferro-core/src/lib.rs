//! FerroIO 核心引擎：工程解析、工具链管理、构建、probe-rs 烧录、串口。

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
