//! 全局状态编排 + 覆盖层 UI 绘制。
//! 阶段 1：每显示器一个覆盖窗口，绘制验收图形；debug 模式可见可点。
//! 阶段 2 起接入手势事件；阶段 4 起绘制标记。

/// 覆盖层 UI。
pub struct OverlayUi {
    /// debug 模式：半透明底 + 描边 + 状态文字（可见可点）；发布模式全透明 + 穿透。
    pub debug: bool,
}

impl OverlayUi {
    /// 每个覆盖窗口的绘制内容。`monitor_index`/`monitor_count` 用于 debug 标注。
    pub fn draw(&self, ui: &mut egui::Ui, monitor_index: usize, monitor_count: usize) {
        let rect = ui.max_rect();

        if self.debug {
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(16, 48, 84, 110),
            );
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 180)),
                egui::StrokeKind::Inside,
            );
        }

        let c = rect.center();
        // 验收图形：绿色圆环 + 圆心
        ui.painter().circle_stroke(
            c,
            80.0,
            egui::Stroke::new(10.0, egui::Color32::from_rgb(0, 210, 130)),
        );
        ui.painter()
            .circle_filled(c, 6.0, egui::Color32::from_rgb(0, 210, 130));

        if self.debug {
            ui.painter().text(
                rect.left_top() + egui::vec2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                format!(
                    "RustPing debug 覆盖层 {}/{} · Esc 退出 · 穿透=关",
                    monitor_index + 1,
                    monitor_count
                ),
                egui::FontId::proportional(16.0),
                egui::Color32::from_rgb(180, 255, 220),
            );
        }
    }
}
