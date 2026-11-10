//! 平台抽象层：覆盖层窗口扩展行为（click-through 等）。
//!
//! Windows 实现在 [`win32`]（对 `HasWindowHandle` 的 blanket impl，
//! winit::Window 天然满足）；macOS 为后续独立阶段（届时补 platform/macos）。
//! 业务代码只依赖本 trait：`use rustping::platform::OverlayWindowExt;`

pub mod win32;

use raw_window_handle::HasWindowHandle;

/// 覆盖层窗口的平台扩展能力。
pub trait OverlayWindowExt {
    /// 应用覆盖窗口标准样式：不进任务栏/Alt-Tab（TOOLWINDOW）、不抢焦点（NOACTIVATE）、
    /// 分层合成（LAYERED）。幂等，返回是否成功。
    fn apply_overlay_style(&self) -> bool;

    /// click-through 开关（WS_EX_TRANSPARENT）。开启后鼠标事件穿透到下层窗口。
    /// 返回是否成功（写入后回读校验）。
    fn set_click_through(&self, on: bool) -> bool;

    /// 读取当前平台扩展样式原始值（调试/冒烟用）。
    fn ex_style(&self) -> Option<i64>;
}

/// 平台窗口句柄 → HWND（仅 Windows 后端有意义）。
pub(crate) fn hwnd_of(window: &dyn HasWindowHandle) -> Option<win32::HwndPtr> {
    match window.window_handle().ok()?.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
        _ => None,
    }
}
