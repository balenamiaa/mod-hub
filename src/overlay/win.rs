use crate::errors::{Error, Result};
use crate::overlay::util::wide_null;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Input::KeyboardAndMouse::{SetActiveWindow, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

/// Registers a minimal window class with a default window procedure.
pub fn register_window_class(name: &str) -> Result<PCWSTR> {
    let wname = wide_null(name);
    let cls = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(def_wndproc),
        hInstance: unsafe { GetModuleHandleW(None).unwrap_or_default().into() },
        lpszClassName: PCWSTR(wname.as_ptr()),
        ..Default::default()
    };
    let atom = unsafe { RegisterClassExW(&cls) };
    if atom == 0 {
        return Err(Error::RegisterClassFailed);
    }
    Ok(PCWSTR(wname.as_ptr()))
}

/// Creates a hidden owner window used to hide the overlay from Alt‑Tab/taskbar.
pub fn create_owner_window(class: PCWSTR, title: &str) -> Result<HWND> {
    let wtitle = wide_null(title);
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class,
            PCWSTR(wtitle.as_ptr()),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            GetModuleHandleW(None)
                .ok()
                .map(|m| windows::Win32::Foundation::HINSTANCE(m.0)),
            None,
        )
    }
    .map_err(Error::CreateOwnerWindow)?;
    Ok(hwnd)
}

/// Creates the overlay popup window with layered transparency and topmost state.
pub fn create_overlay_window(
    class: PCWSTR,
    owner: HWND,
    title: &str,
    width: i32,
    height: i32,
    hide_alt_tab: bool,
) -> Result<HWND> {
    let wtitle = wide_null(title);
    let mut ex = WS_EX_NOREDIRECTIONBITMAP | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST;
    if !hide_alt_tab {
        ex |= WS_EX_APPWINDOW;
    }
    let parent = if hide_alt_tab { Some(owner) } else { None };
    let hwnd = unsafe {
        CreateWindowExW(
            ex,
            class,
            PCWSTR(wtitle.as_ptr()),
            WS_POPUP,
            0,
            0,
            width,
            height,
            parent,
            None,
            GetModuleHandleW(None)
                .ok()
                .map(|m| windows::Win32::Foundation::HINSTANCE(m.0)),
            None,
        )
    }
    .map_err(Error::CreateOverlayWindow)?;

    unsafe {
        if hide_alt_tab {
            let exs = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, exs);
        } else {
            let mut exs = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            exs &= !(WS_EX_TOOLWINDOW.0 as isize);
            exs |= WS_EX_APPWINDOW.0 as isize;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, exs);
        }
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_FRAMECHANGED,
        );
    }

    unsafe {
        let _ = SetLayeredWindowAttributes(
            hwnd,
            windows::Win32::Foundation::COLORREF(0),
            255,
            LWA_ALPHA,
        );
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }

    Ok(hwnd)
}

/// Shows the overlay without activating it and keeps it topmost.
pub fn show_no_activate(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

/// Ensures the overlay stays topmost.
pub fn set_topmost(hwnd: HWND) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        );
    }
}

/// Toggles click‑through mode by setting/removing WS_EX_TRANSPARENT and activation.
pub fn set_click_through(hwnd: HWND, enabled: bool) {
    unsafe {
        let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if enabled {
            ex |= (WS_EX_TRANSPARENT | WS_EX_NOACTIVATE).0 as isize;
        } else {
            ex &= !((WS_EX_TRANSPARENT | WS_EX_NOACTIVATE).0 as isize);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
        if !enabled {
            let _ = SetForegroundWindow(hwnd);
            let _ = SetActiveWindow(hwnd);
            let _ = SetFocus(Some(hwnd));
        }
    }
}

/// Applies per‑pixel transparency and extends the DWM frame into the client area.
pub fn apply_transparency(hwnd: HWND) {
    unsafe {
        let _ = SetLayeredWindowAttributes(
            hwnd,
            windows::Win32::Foundation::COLORREF(0),
            255,
            LWA_ALPHA,
        );
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }
}

extern "system" fn def_wndproc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_SETCURSOR => unsafe {
            SetCursor(LoadCursorW(None, IDC_ARROW).ok());
            LRESULT(1)
        },
        WM_DESTROY => unsafe {
            PostQuitMessage(0);
            LRESULT(0)
        },
        _ => unsafe { DefWindowProcW(hwnd, msg, w, l) },
    }
}
