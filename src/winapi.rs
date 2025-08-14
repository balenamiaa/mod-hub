#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]

use core::ffi::c_void;
use std::ptr::null_mut;
use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND};
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F10, VK_INSERT};
use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, GetWindowLongPtrA};
use windows::Win32::UI::WindowsAndMessaging::{MB_OK, MessageBoxA};

pub type ThreadFunc = unsafe extern "system" fn(lp_parameter: *mut c_void) -> u32;

pub fn disable_thread_library_calls(module: HINSTANCE) -> bool {
    unsafe { DisableThreadLibraryCalls(HMODULE(module.0)).is_ok() }
}

pub fn spawn_thread(_func: ThreadFunc, _param: *mut c_void) -> Option<*mut c_void> {
    // Thread spawning disabled due to safety constraints
    // Would need proper Windows API bindings for CreateThread
    None
}

pub fn join_thread(_handle: *mut c_void) {
    // Since we're using std::thread, we can't wait on the handle
    // This is a limitation of the current implementation
}

pub fn message_box(title: &str, text: &str) {
    let to_c = |s: &str| {
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0);
        bytes
    };
    let title = to_c(title);
    let text = to_c(text);
    unsafe {
        use windows::core::PCSTR;
        MessageBoxA(
            Some(HWND(null_mut())),
            PCSTR(text.as_ptr()),
            PCSTR(title.as_ptr()),
            MB_OK,
        );
    }
}

pub trait IntoHinstance {
    fn into_hinstance(self) -> HINSTANCE;
}

impl IntoHinstance for isize {
    fn into_hinstance(self) -> HINSTANCE {
        HINSTANCE(self as _)
    }
}

pub fn is_f10_pressed() -> bool {
    unsafe { (GetAsyncKeyState(VK_F10.0 as i32) as u32 & 0x8000) as i32 != 0 }
}

pub fn debug_log(msg: &str) {
    eprintln!("{}", msg);
}

#[cfg(target_os = "windows")]
pub fn hwnd_exstyle_hex(hwnd: isize) -> String {
    unsafe { format!("0x{:016x}", GetWindowLongPtrA(HWND(hwnd as *mut _), GWL_EXSTYLE)) }
}

#[cfg(not(target_os = "windows"))]
pub fn hwnd_exstyle_hex(_hwnd: isize) -> String {
    String::from("n/a")
}

pub fn is_insert_pressed() -> bool {
    unsafe { (GetAsyncKeyState(VK_INSERT.0 as i32) as u32 & 0x8000) as i32 != 0 }
}

pub fn is_vk_pressed(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u32 & 0x8000) as i32 != 0 }
}
