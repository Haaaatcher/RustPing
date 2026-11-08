//! 手势状态机（纯逻辑，可单测）。
//!
//! 迁移图（阶段 2 实现）：
//! `Idle → Armed(热键按下) → WheelActive(拖动超阈值) → Selected(松开)`
//! `→ Cancelled(Esc、未过阈值松开)`。
//! 钩子线程只产出事件入 channel，状态迁移全部在纯逻辑层完成。

/// 手势状态（阶段 2 充实迁移逻辑与事件类型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GestureState {
    /// 空闲：钩子只监听热键。
    #[default]
    Idle,
    /// 热键已按下，等待拖动越过阈值。
    Armed,
    /// 轮盘已唤出，跟随光标。
    WheelActive,
}

#[cfg(test)]
mod tests {
    use super::GestureState;

    #[test]
    fn default_state_is_idle() {
        assert_eq!(GestureState::default(), GestureState::Idle);
    }
}
