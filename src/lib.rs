use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::{BOOL, HINSTANCE};
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use pyo3::prelude::*;

// Core modules for the Universe framework
mod core;
mod memory;
mod hooks;
mod python_runtime;
mod ffi;
mod logging;
mod registers;
mod pointers;
mod python_interface;

use core::UniverseCore;
// Removed unused imports - these are now handled by python_interface module

// Global instance of the Universe core - using Mutex for thread safety
static UNIVERSE_CORE: Mutex<Option<UniverseCore>> = Mutex::new(None);

/// DLL entry point - called when the DLL is loaded/unloaded
#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> BOOL {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            // Initialize the Universe framework
            match UniverseCore::initialize() {
                Ok(core) => {
                    // Create shared reference for the python interface
                    let core_arc = Arc::new(Mutex::new(core));
                    
                    // Initialize the global core reference in python_interface
                    if let Err(e) = python_interface::initialize_core_reference(Arc::clone(&core_arc)) {
                        eprintln!("Failed to initialize python interface core reference: {:?}", e);
                        return 0; // FALSE
                    }
                    
                    // Store the core instance in the global mutex
                    match UNIVERSE_CORE.lock() {
                        Ok(mut guard) => {
                            // Extract the core from the Arc<Mutex<>> for storage in the old format
                            match Arc::try_unwrap(core_arc) {
                                Ok(mutex) => {
                                    match mutex.into_inner() {
                                        Ok(core) => {
                                            *guard = Some(core);
                                            1 // TRUE
                                        }
                                        Err(_) => {
                                            eprintln!("Failed to extract core from mutex during initialization");
                                            0 // FALSE
                                        }
                                    }
                                }
                                Err(_arc) => {
                                    // If we can't unwrap (because there are other references), 
                                    // we need to clone the core or handle this differently
                                    // For now, let's create a new core instance for the global storage
                                    // This is not ideal but works for the current implementation
                                    match UniverseCore::initialize() {
                                        Ok(new_core) => {
                                            *guard = Some(new_core);
                                            1 // TRUE
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to create second core instance: {:?}", e);
                                            0 // FALSE
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            eprintln!("Failed to acquire Universe core mutex during initialization");
                            0 // FALSE
                        }
                    }
                }
                Err(e) => {
                    // Log error and fail gracefully
                    eprintln!("Failed to initialize Universe core: {:?}", e);
                    0 // FALSE
                }
            }
        }
        DLL_PROCESS_DETACH => {
            // Cleanup the Universe framework
            match UNIVERSE_CORE.lock() {
                Ok(mut guard) => {
                    if let Some(mut core) = guard.take() {
                        if let Err(e) = core.shutdown() {
                            eprintln!("Error during Universe shutdown: {:?}", e);
                        }
                    }
                    1 // TRUE
                }
                Err(_) => {
                    eprintln!("Failed to acquire Universe core mutex during shutdown");
                    1 // TRUE - still return success to avoid blocking DLL unload
                }
            }
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