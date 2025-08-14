//! Minimal, ergonomic overlay window API built on top of egui.
//!
//! The API centers around `OverlayBuilder` and the `AppUi` trait. Provide a type
//! that implements `AppUi`, then launch a transparent, topmost window:
//!
//! ```no_run
//! use restident_evil_5::{egui, AppUi, OverlayBuilder};
//!
//! struct MyUi;
//!
//! impl AppUi for MyUi {
//!     fn ui(&mut self, ctx: &egui::Context) {
//!         egui::Window::new("Overlay").title_bar(false).resizable(false).show(ctx, |ui| {
//!             ui.label("Hello from overlay");
//!         });
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! OverlayBuilder::new().run(MyUi)?;
//! # Ok(())
//! # }
//! ```
pub mod analysis;
pub mod config;
pub mod errors;
pub mod hooks;
pub mod memory;
pub mod overlay;
pub mod pattern;
pub mod vtable;
pub mod winapi;

pub use crate::errors::{Error, Result};
pub use overlay::{AppUi, OverlayBuilder, egui, run_overlay};

use core::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub(crate) static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);

fn start_hooks() {
    crate::hooks::init_global_manager::<crate::config::Config>(crate::config::Config::default());
    let _ = crate::hooks::start::<crate::config::Config>();
}

fn stop_hooks() {
    crate::hooks::stop::<crate::config::Config>();
}

fn start_runtime() {
    if SHUTDOWN.load(Ordering::SeqCst) {
        SHUTDOWN.store(false, Ordering::SeqCst);
    }

    thread::spawn(|| {
        let cfg = crate::config::Config::default();
        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }
            if winapi::is_vk_pressed(cfg.exit_vk) {
                stop_system();
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    thread::spawn(|| {
        let cfg = crate::config::Config::default();
        struct Starter;
        impl AppUi for Starter {
            fn ui(&mut self, ctx: &egui::Context) {
                if SHUTDOWN.load(Ordering::SeqCst) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }

                egui::Window::new(crate::config::Config::default().project_name)
                    .title_bar(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Injected overlay running");
                        ui.label("Press F10 to quit");
                    });
            }
        }
        let _ = cfg.overlay_builder().run(Starter);
    });
}

fn stop_runtime() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

fn try_start_system(hinst_dll: isize) -> bool {
    match RUNNING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => {
            winapi::disable_thread_library_calls(hinst_dll as _);
            start_hooks();
            start_runtime();
            true
        }
        Err(_) => false,
    }
}

fn stop_system() {
    if RUNNING.swap(false, Ordering::SeqCst) {
        stop_runtime();
        stop_hooks();
    }
}

pub fn on_process_attach(hinst_dll: isize) {
    let _ = try_start_system(hinst_dll);
}

pub fn on_process_detach() {
    stop_system();
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    hinst_dll: isize,
    fdw_reason: u32,
    _lpv_reserved: *mut core::ffi::c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_PROCESS_DETACH: u32 = 0;
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            on_process_attach(hinst_dll);
            1
        }
        DLL_PROCESS_DETACH => {
            on_process_detach();
            1
        }
        _ => 1,
    }
}
