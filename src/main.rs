//! 启动与线程编排：tracing 初始化 → eframe(wgpu) 事件循环。
//! 阶段 2 起在此增加钩子线程与手势事件 channel 的编排。

use eframe::egui;
use tracing_subscriber::{
    EnvFilter,
    layer::{Layer as _, SubscriberExt},
    util::SubscriberInitExt,
};

use rustping::app::RustPingApp;

fn main() -> eframe::Result<()> {
    let _log_guard = init_tracing();
    install_panic_hook();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "RustPing 启动");

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

/// console（RUST_LOG 可调，默认 info）+ `%LOCALAPPDATA%\RustPing\logs` 按天滚动文件双输出。
/// 返回的 `WorkerGuard` 须在 main 生命周期内持有，退出时才保证日志刷盘。
fn init_tracing() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let console_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    match file_log_writer() {
        Some((file_writer, guard)) => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_filter(console_filter))
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(false)
                        .with_writer(file_writer)
                        .with_filter(EnvFilter::new("info")),
                )
                .init();
            Some(guard)
        }
        None => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_filter(console_filter))
                .init();
            None
        }
    }
}

/// 文件日志 writer；目录不可建时返回 None，退化为仅 console。
fn file_log_writer() -> Option<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let dir = std::path::PathBuf::from(base).join("RustPing").join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let file = tracing_appender::rolling::daily(&dir, "rustping.log");
    Some(tracing_appender::non_blocking(file))
}

/// panic 信息 + backtrace 写日志（含文件层）后再走默认退出流程，保证崩溃可事后分析。
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!("panic: {info}\n{backtrace}");
    }));
}
