#![feature(never_type)]

use pyo3::prelude::*;
use std::ffi::c_void;
use std::io::Write;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::{BOOL, HINSTANCE};
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

// Core modules for the Universe framework
mod core;
mod ffi;
mod ffi_asm;
mod hooks;
mod hook_handlers;
mod logging;
mod memory;
mod pointers;
mod python_interface;
mod python_runtime;
mod registers;

use core::UniverseCore;

/// Global instance of the Universe main thread
static UNIVERSE_MAIN_THREAD: Mutex<Option<std::thread::JoinHandle<!>>> = Mutex::new(None);

/// Global instance of the Universe core - using Mutex for thread safety
static UNIVERSE_CORE: Mutex<Option<UniverseCore>> = Mutex::new(None);

/// Main thread for the universe framework
fn main_thread() -> ! {
    // Initialize the Universe framework
    match UniverseCore::initialize() {
        Ok(core) => {
            // Create shared reference for the python interface
            let core_arc = Arc::new(Mutex::new(core));

            // Initialize the global core reference in python_interface
            if let Err(e) = python_interface::initialize_core_reference(Arc::clone(&core_arc)) {
                if let Ok(guard) = core_arc.lock() {
                    guard.logger().log_error_with_context(&e, "Failed to initialize python interface core reference");
                }
            }

            // Store the core instance in the global mutex
            match UNIVERSE_CORE.lock() {
                Ok(mut guard) => {
                    // Extract the core from the Arc<Mutex<>> for storage in the old format
                    match Arc::try_unwrap(core_arc) {
                        Ok(mutex) => match mutex.into_inner() {
                            Ok(core) => {
                                *guard = Some(core);
                            }
                            Err(_) => {
                                // Can't log to universe.log since we can't access the core
                                // Fall back to stderr for this critical error
                                let _ = std::io::Write::write_all(&mut std::io::stderr(), b"CRITICAL: Failed to extract core from mutex during initialization\n");
                            }
                        },
                        Err(_arc) => {
                            // If we can't unwrap (because there are other references),
                            // we need to clone the core or handle this differently
                            // For now, let's create a new core instance for the global storage
                            // This is not ideal but works for the current implementation
                            match UniverseCore::initialize() {
                                Ok(new_core) => {
                                    *guard = Some(new_core);
                                }
                                Err(e) => {
                                    // Can't log to universe.log since we can't access the core
                                    // Fall back to stderr for this critical error
                                    let error_msg = format!("CRITICAL: Failed to create second core instance: {:?}\n", e);
                                    let _ = std::io::stderr().write_all(error_msg.as_bytes());
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Can't log to universe.log since we can't access the core
                    // Fall back to stderr for this critical error
                    let _ = std::io::stderr().write_all(b"CRITICAL: Failed to acquire Universe core mutex during initialization\n");
                }
            }
        }
        Err(e) => {
            // Can't log to universe.log since core initialization failed
            // Fall back to stderr for this critical error
            let error_msg = format!("CRITICAL: Failed to initialize Universe core: {:?}\n", e);
            let _ = std::io::stderr().write_all(error_msg.as_bytes());
        }
    }

    loop {
        if let Ok(mut guard) = UNIVERSE_CORE.lock() {
            if let Some(core) = guard.as_mut() {
                if let Err(e) = core.tick() {
                    if e.is_recoverable() {
                        core.logger().log_recoverable_error(&e, "Continuing operation");
                    } else {
                        core.logger().log_critical(&format!("Non-recoverable error during tick: {}", e));
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// DLL entry point - called when the DLL is loaded/unloaded
#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> BOOL {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            let thread = std::thread::spawn(main_thread);
            *UNIVERSE_MAIN_THREAD.lock().unwrap() = Some(thread);
            1
        }
        DLL_PROCESS_DETACH => {
            // Cleanup the Universe framework
            match UNIVERSE_CORE.lock() {
                Ok(mut guard) => {
                    if let Some(mut core) = guard.take() {
                        if let Err(e) = core.shutdown() {
                            core.logger().log_error_with_context(&e, "Error during Universe shutdown");
                        }
                    }
                }
                Err(_) => {
                    // Can't log to universe.log since we can't access the core
                    // Fall back to stderr for this critical error
                    let _ = std::io::stderr().write_all(b"CRITICAL: Failed to acquire Universe core mutex during shutdown\n");
                }
            }

            if let Some(thread) = UNIVERSE_MAIN_THREAD.lock().unwrap().take() {
                thread.join().unwrap();
            }

            1
        }
        _ => 1, // TRUE for other reasons
    }
}

/// PyO3 module definition for Python integration
/// This is the main entry point for the universe Python module
#[pymodule]
fn universe(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Delegate to the python_interface module for complete API registration
    python_interface::universe(m)
}
