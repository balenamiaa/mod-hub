#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]

use core::ffi::c_void;
use core::ptr::null_mut;
use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
use windows_sys::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows_sys::Win32::System::Threading::{CreateThread, INFINITE, WaitForSingleObject};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F10, VK_INSERT};
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_OK, MessageBoxA};

pub type ThreadFunc = unsafe extern "system" fn(lp_parameter: *mut c_void) -> u32;

pub fn disable_thread_library_calls(module: HINSTANCE) -> bool {
    unsafe { DisableThreadLibraryCalls(module) != 0 }
}

pub fn spawn_thread(func: ThreadFunc, param: *mut c_void) -> Option<*mut c_void> {
    let handle = unsafe { CreateThread(null_mut(), 0, Some(func), param, 0, null_mut()) };
    if handle == null_mut() {
        None
    } else {
        Some(handle)
    }
}

pub fn join_thread(handle: *mut c_void) {
    unsafe {
        WaitForSingleObject(handle, INFINITE);
    }
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
        MessageBoxA(0 as HWND, text.as_ptr(), title.as_ptr(), MB_OK);
    }
}

pub trait IntoHinstance {
    fn into_hinstance(self) -> HINSTANCE;
}

impl IntoHinstance for isize {
    fn into_hinstance(self) -> HINSTANCE {
        self as HINSTANCE
    }
}

pub fn is_f10_pressed() -> bool {
    unsafe { (GetAsyncKeyState(VK_F10 as i32) as u32 & 0x8000) as i32 != 0 }
}

pub fn is_insert_pressed() -> bool {
    unsafe { (GetAsyncKeyState(VK_INSERT as i32) as u32 & 0x8000) as i32 != 0 }
}

pub fn is_vk_pressed(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u32 & 0x8000) as i32 != 0 }
}
