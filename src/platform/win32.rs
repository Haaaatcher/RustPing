//! Win32 实现：扩展窗口样式（TOOLWINDOW / NOACTIVATE / LAYERED / TRANSPARENT）。
//!
//! 验证依据：Spy++ 检查 ex-style（见 DEVELOPMENT.md §5），不靠猜。

use raw_window_handle::HasWindowHandle;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

use super::{OverlayWindowExt, hwnd_of};

/// HWND 的原始指针值（raw-window-handle 的 `hwnd.get()` 产物）。
pub type HwndPtr = isize;

/// `WS_EX_*` 常量 → isize（Get/SetWindowLongPtrW 的位掩码）。
fn ex(bit: windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE) -> isize {
    bit.0 as isize
}

impl<T: HasWindowHandle> OverlayWindowExt for T {
    fn apply_overlay_style(&self) -> bool {
        let Some(hwnd) = hwnd_of(self) else {
            return false;
        };
        // SAFETY：hwnd 来自 winit 持有的合法窗口句柄
        unsafe {
            let cur = GetWindowLongPtrW(hwnd_from(hwnd), GWL_EXSTYLE);
            let next = cur | ex(WS_EX_TOOLWINDOW) | ex(WS_EX_NOACTIVATE) | ex(WS_EX_LAYERED);
            SetWindowLongPtrW(hwnd_from(hwnd), GWL_EXSTYLE, next);
            GetWindowLongPtrW(hwnd_from(hwnd), GWL_EXSTYLE) == next
        }
    }

    fn set_click_through(&self, on: bool) -> bool {
        let Some(hwnd) = hwnd_of(self) else {
            return false;
        };
        // SAFETY：同上
        unsafe {
            let cur = GetWindowLongPtrW(hwnd_from(hwnd), GWL_EXSTYLE);
            let next = if on {
                cur | ex(WS_EX_TRANSPARENT)
            } else {
                cur & !ex(WS_EX_TRANSPARENT)
            };
            SetWindowLongPtrW(hwnd_from(hwnd), GWL_EXSTYLE, next);
            GetWindowLongPtrW(hwnd_from(hwnd), GWL_EXSTYLE) == next
        }
    }

    fn ex_style(&self) -> Option<i64> {
        let hwnd = hwnd_of(self)?;
        // SAFETY：同上
        Some(unsafe { GetWindowLongPtrW(hwnd_from(hwnd), GWL_EXSTYLE) } as i64)
    }
}

/// isize 句柄值 → `HWND`。
fn hwnd_from(ptr: HwndPtr) -> windows::Win32::Foundation::HWND {
    windows::Win32::Foundation::HWND(ptr as *mut core::ffi::c_void)
}
