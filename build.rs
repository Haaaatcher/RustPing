//! 构建脚本：Windows 下嵌入应用清单（PerMonitorV2 DPI + asInvoker）。
//! 应用图标与版本资源在阶段 7（M4 发布）加入。

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=rustping.manifest");
        let mut res = winresource::WindowsResource::new();
        res.set_manifest(include_str!("rustping.manifest"));
        res.compile().expect("嵌入 Windows 清单失败");
    }
}
