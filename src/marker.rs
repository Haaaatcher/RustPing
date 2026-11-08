//! 标记实体与动画曲线（纯逻辑，可单测）。
//! 阶段 4 实现动画（圆环扩散 + 淡出，约 1.8s）、并发上限与自动清理。

/// 标记最大存活时长（秒）。
pub const MARKER_LIFETIME_SECS: f64 = 1.8;

/// 一个存活的 ping 标记（阶段 4 充实动画参数与类型）。
#[derive(Debug, Clone, Copy)]
pub struct Marker {
    /// 屏幕逻辑坐标。
    pub pos: [f32; 2],
    /// 出生时刻（秒，单调时钟）。
    pub born: f64,
}

#[cfg(test)]
mod tests {
    use super::MARKER_LIFETIME_SECS;

    #[test]
    fn lifetime_is_reasonable() {
        assert!(MARKER_LIFETIME_SECS > 1.0 && MARKER_LIFETIME_SECS < 3.0);
    }
}
