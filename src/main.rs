//! 启动与线程编排：日志初始化 → eframe(wgpu) 事件循环。
//! 阶段 2 起在此增加钩子线程与手势事件 channel 的编排。

use eframe::egui;

use rustping::{app::RustPingApp, logging};

fn main() -> eframe::Result<()> {
    // guard 持有到 main 结束，保证退出时日志刷盘
    let _log_guard = logging::init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "RustPing 启动");

    // 崩溃管线自测入口：RUSTPING_SELFTEST_PANIC=1 触发一次 panic，
    // 用于验证 crash-*.log 同步落盘（见 DEVELOPMENT.md §6）。
    if std::env::var_os("RUSTPING_SELFTEST_PANIC").is_some() {
        panic!("self-test：验证崩溃日志管线");
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("RustPing v{}", env!("CARGO_PKG_VERSION")))
            .with_inner_size([800.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "RustPing",
        native_options,
        Box::new(|_cc| Ok(Box::new(RustPingApp::default()))),
    )
}
