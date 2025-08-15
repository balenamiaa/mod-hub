//! Minimal, ergonomic overlay window API built on top of egui.
//!
//! The API centers around `OverlayBuilder` and the `AppUi` trait. Provide a type
//! that implements `AppUi`, then launch a transparent, topmost window:
//!
//! ```no_run
//! use mod_template::{egui, AppUi, OverlayBuilder};
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

use crate::hooks::{HookModule, register};
use crate::winapi::IntoHinstance;

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
pub use crate::overlay::{AppUi, OverlayBuilder};
pub use egui;

pub use ilhook::x64::Registers;

use core::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub(crate) static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);

fn init_logging() {
    use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let cfg = ConfigBuilder::new()
        .set_time_offset_to_local()
        .expect("Failed to set time offset to local")
        .set_time_format_rfc3339()
        .build();
    match std::fs::File::create("universe.log") {
        Ok(file) => {
            let _ = WriteLogger::init(level, cfg, file);
            log::info!("logger initialized at level: {:?}", level);
        }
        Err(e) => {
            // As a fallback, still try to initialize logging to stderr.
            let _ = WriteLogger::init(level, ConfigBuilder::new().build(), std::io::stderr());
            log::error!("failed to create universe.log: {e}");
        }
    }
}

fn start_hooks() {
    log::info!("starting hooks manager");
    crate::hooks::init_global_manager::<crate::config::Config>(crate::config::Config::default());
    if let Err(e) = crate::hooks::start::<crate::config::Config>() {
        log::error!("failed to start hooks: {e}");
    } else {
        log::info!("hooks started");
    }
}

fn stop_hooks() {
    log::info!("stopping hooks");
    crate::hooks::stop::<crate::config::Config>();
}

fn start_runtime() {
    if SHUTDOWN.load(Ordering::SeqCst) {
        SHUTDOWN.store(false, Ordering::SeqCst);
    }

    thread::spawn(|| {
        let cfg = crate::config::Config::default();
        log::debug!("runtime watcher thread started");
        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                log::debug!("runtime watcher exiting due to shutdown flag");
                break;
            }
            if winapi::is_vk_pressed(cfg.exit_vk) {
                log::info!("exit key pressed; stopping system");
                stop_system();
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    thread::spawn(|| {
        let cfg = crate::config::Config::default();
        log::debug!("overlay thread starting");
        struct Starter;
        impl AppUi for Starter {
            fn ui(&mut self, ctx: &egui::Context) {
                if SHUTDOWN.load(Ordering::SeqCst) {
                    log::debug!("ui notified of shutdown; closing viewport");
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
        if let Err(e) = cfg.overlay_builder().run(Starter) {
            log::error!("overlay error: {e}");
        }
    });
}

fn stop_runtime() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

fn install_hooks() {
    log::info!("installing hooks");

    {
        struct ExampleModule;

        unsafe extern "win64" fn example_callback(
            registers: *mut Registers,
            ori_func_ptr: usize,
            _user_data: usize,
        ) -> usize {
            log::info!("example_callback called");
            log::info!(
                "ori parameters: {:#x}, {:#x}",
                unsafe { (*registers).rcx },
                unsafe { (*registers).rdx }
            );

            let ori_func = unsafe {
                std::mem::transmute::<usize, unsafe extern "win64" fn(usize, usize) -> usize>(
                    ori_func_ptr,
                )
            };
            let result = unsafe { ori_func(1, 2) };
            log::info!("ori result: {:#x}", result);
            result
        }

        unsafe extern "win64" fn example_original_function(a: usize, b: usize) -> usize {
            a + b
        }

        impl HookModule<crate::config::Config> for ExampleModule {
            fn name(&self) -> &'static str {
                "ExampleModule"
            }

            fn init(
                &mut self,
                ctx: &hooks::HookContext<crate::config::Config>,
            ) -> Result<Vec<hooks::HookGuard>> {
                let example_hook_0 = unsafe {
                    ctx.install_retn(example_original_function as usize, example_callback, 0)?
                };

                Ok(vec![example_hook_0])
            }
        }
        register::<crate::config::Config, ExampleModule>(ExampleModule);

        unsafe {
            let _ = example_original_function(1, 2);
        }
    }
}

fn try_start_system(hinst_dll: isize) -> bool {
    match RUNNING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => {
            winapi::disable_thread_library_calls(hinst_dll.into_hinstance());
            init_logging();
            start_hooks();
            start_runtime();

            install_hooks();

            true
        }
        Err(_) => false,
    }
}

fn stop_system() {
    if RUNNING.swap(false, Ordering::SeqCst) {
        log::info!("stopping system");
        stop_runtime();
        stop_hooks();
    }
}

pub fn on_process_attach(hinst_dll: isize) {
    log::info!("process attach hinst={:#x}", hinst_dll);
    let _ = try_start_system(hinst_dll);
}

pub fn on_process_detach() {
    log::info!("process detach");
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
            log::debug!("DllMain: PROCESS_ATTACH");
            on_process_attach(hinst_dll);
            1
        }
        DLL_PROCESS_DETACH => {
            log::debug!("DllMain: PROCESS_DETACH");
            on_process_detach();
            1
        }
        _ => 1,
    }
}
