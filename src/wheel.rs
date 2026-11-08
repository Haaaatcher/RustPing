//! 轮盘几何与命中测试（纯逻辑，可单测）。
//! 阶段 3 实现扇形剖分、角度命中与 egui 绘制。

/// 轮盘布局参数（阶段 3 充实）。
#[derive(Debug, Clone, Copy)]
pub struct WheelLayout {
    /// 扇形数量。
    pub sector_count: usize,
    /// 轮盘半径（逻辑像素）。
    pub radius: f32,
    /// 中心死区半径（小于此距离松开 = 取消）。
    pub dead_zone: f32,
}

impl Default for WheelLayout {
    fn default() -> Self {
        Self {
            sector_count: 8,
            radius: 120.0,
            dead_zone: 28.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WheelLayout;

    #[test]
    fn default_layout_is_sane() {
        let l = WheelLayout::default();
        assert!(l.sector_count > 0);
        assert!(l.dead_zone > 0.0 && l.dead_zone < l.radius);
    }
}
