//! FerroIO 插件协议与加载器。

pub trait Plugin {
    fn name(&self) -> &str;
}
