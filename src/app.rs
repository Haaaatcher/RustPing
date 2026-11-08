//! 全局状态编排：接收手势事件、驱动覆盖层绘制。
//! 阶段 2 起接入手势事件 channel；阶段 0 为空窗口。

use eframe::egui;

/// eframe 主应用。
#[derive(Default)]
pub struct RustPingApp {
    // 阶段 2+：手势状态、存活标记、当前配置。
}

impl eframe::App for RustPingApp {
    // eframe 0.37：根 viewport 即中央面板，直接拿到 &mut Ui；
    // 需要 Context 时用 ui.ctx()。
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 临时占位：确认 egui 文字渲染与默认字体可用；阶段 1 起移除。
        ui.label(format!(
            "RustPing v{} — 阶段 0 骨架（wgpu 后端）",
            env!("CARGO_PKG_VERSION")
        ));
        // 无动画时 egui 自动 idle，不做持续重绘（按需渲染原则）。
    }
}
