use crate::core::{UniverseCore, UniverseError};
use crate::hook_handlers::PyRegisterAccess;
use crate::memory::ModuleInfo;

use pyo3::prelude::*;

use pyo3::types::PyModule;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Global reference to the Universe core for accessing subsystems
static UNIVERSE_CORE_REF: Mutex<Option<Arc<Mutex<UniverseCore>>>> = Mutex::new(None);

/// Initialize the global Universe core reference
pub fn initialize_core_reference(core: Arc<Mutex<UniverseCore>>) -> Result<(), UniverseError> {
    let mut core_ref = UNIVERSE_CORE_REF.lock().map_err(|_| {
        UniverseError::SystemError("Failed to acquire core reference lock".to_string())
    })?;
    *core_ref = Some(core);
    Ok(())
}

/// Get a reference to the global Universe core
pub fn get_core_reference() -> PyResult<Arc<Mutex<UniverseCore>>> {
    let core_ref = UNIVERSE_CORE_REF.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core reference lock")
    })?;

    core_ref.as_ref().cloned().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Universe core not initialized")
    })
}

/// Convert UniverseError to Python exception
fn convert_universe_error(error: UniverseError) -> PyErr {
    error.to_python_exception()
}

#[pyfunction]
#[pyo3(signature = (address, size, validate))]
pub fn read_memory(address: usize, size: usize, validate: bool) -> PyResult<Vec<u8>> {
    let core = get_core_reference()?;
    let core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // Access the memory manager from the core
    let memory_manager = core_guard.memory_manager().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Memory manager not initialized")
    })?;

    // Log the memory read operation
    core_guard.logger().log_debug(&format!(
        "Python API: Reading {} bytes from address 0x{:x}",
        size, address
    ));

    let memory_guard = memory_manager.lock().map_err(|_| {
        let error = UniverseError::SystemError("Failed to acquire memory manager lock".to_string());
        core_guard
            .logger()
            .log_error_with_context(&error, "Python API read_memory call");
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire memory manager lock")
    })?;

    match memory_guard.read_memory(address, size, validate) {
        Ok(data) => {
            core_guard.logger().log_debug(&format!(
                "Python API: Successfully read {} bytes from address 0x{:x}",
                data.len(),
                address
            ));
            Ok(data)
        }
        Err(e) => {
            core_guard.logger().log_error_with_context(
                &e,
                &format!("Python API read_memory failed for address 0x{:x}", address),
            );
            Err(convert_universe_error(e))
        }
    }
}

/// Write data to the specified memory address
///
/// Args:
///     address (int): Memory address to write to
///     data (bytes): Data to write
///
/// Raises:
///     MemoryError: If memory access fails
///     RuntimeError: If Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, data))]
pub fn write_memory(address: usize, data: Vec<u8>) -> PyResult<()> {
    let core = get_core_reference()?;
    let core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // Access the memory manager from the core
    let memory_manager = core_guard.memory_manager().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Memory manager not initialized")
    })?;

    // Log the memory write operation
    core_guard.logger().log(&format!(
        "Writing {} bytes to address 0x{:x}",
        data.len(),
        address
    ));

    let memory_guard = memory_manager.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire memory manager lock")
    })?;

    memory_guard
        .write_memory(address, &data)
        .map_err(convert_universe_error)
}

#[pyfunction]
pub fn list_modules() -> PyResult<HashMap<String, ModuleInfo>> {
    let core = get_core_reference()?;
    let core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    let memory_manager = core_guard.memory_manager().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Memory manager not initialized")
    })?;

    let memory_guard = memory_manager.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire memory manager lock")
    })?;

    Ok(memory_guard.list_modules().clone())
}

/// Scan for a byte pattern within a specific module
///
/// Args:
///     module_name (str): Name of the module to scan
///     pattern (str): Hex pattern string (e.g., "48 8B ? ? 89 45")
///
/// Returns:
///     int or None: Memory address of the first match, or None if not found
///
/// Raises:
///     RuntimeError: If Universe core is not initialized or scan fails
#[pyfunction]
#[pyo3(signature = (module_name, pattern))]
pub fn pattern_scan(module_name: &str, pattern: &str) -> PyResult<Option<usize>> {
    let core = get_core_reference()?;
    let core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // Access the memory manager from the core
    let memory_manager = core_guard.memory_manager().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Memory manager not initialized")
    })?;

    // Log the pattern scan operation
    core_guard.logger().log(&format!(
        "Scanning for pattern '{}' in module '{}'",
        pattern, module_name
    ));

    let memory_guard = memory_manager.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire memory manager lock")
    })?;

    let result = memory_guard.pattern_scan_hex(module_name, pattern);

    // Log the result
    match result {
        Some(address) => core_guard
            .logger()
            .log(&format!("Pattern found at address 0x{:x}", address)),
        None => core_guard.logger().log("Pattern not found"),
    }

    Ok(result)
}

// ============================================================================
// Hook System API
// ============================================================================

/// Install a function hook at the specified address
///
/// Args:
///     address (int): Memory address to hook
///     callback (callable): Python function to call when hook is triggered.
///                          Receives (registers, original_function) parameters.
///                          The registers parameter is a PyRegisterAccess object.
///                          The original_function parameter is a usize that is the address of the original function.
///
/// Raises:
///     RuntimeError: If hook installation fails or Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, callback))]
pub fn hook_function(address: usize, callback: PyObject) -> PyResult<()> {
    let core = get_core_reference()?;
    let mut core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // Validate callback is callable
    Python::with_gil(|py| {
        if !callback.bind(py).is_callable() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Callback must be callable",
            ));
        }
        Ok(())
    })?;

    // Log the hook installation
    core_guard.logger().log(&format!(
        "Installing function hook at address 0x{:x}",
        address
    ));

    // Access the hook manager from the core and install the hook
    let result = {
        let hook_manager = core_guard.hook_manager_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Hook manager not initialized")
        })?;

        // Install the function hook
        hook_manager
            .install_function_hook(address, callback)
            .map_err(convert_universe_error)
    };

    // Log the result
    match &result {
        Ok(()) => {
            core_guard
                .logger()
                .log("Function hook installed successfully");
        }
        Err(e) => {
            core_guard
                .logger()
                .log(&format!("Function hook installation failed: {}", e));
        }
    }

    result
}

/// Install a jmpback hook at the specified address
///
/// Args:
///     address (int): Memory address to hook
///     callback (callable): Python function to call when hook is triggered.
///                          Receives (registers) parameter only. The registers parameter is a PyRegisterAccess object.
///
/// Raises:
///     RuntimeError: If hook installation fails or Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, callback))]
pub fn hook_jmpback(address: usize, callback: PyObject) -> PyResult<()> {
    let core = get_core_reference()?;
    let mut core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // Validate callback is callable
    Python::with_gil(|py| {
        if !callback.bind(py).is_callable() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Callback must be callable",
            ));
        }
        Ok(())
    })?;

    // Log the hook installation
    core_guard.logger().log(&format!(
        "Installing jmpback hook at address 0x{:x}",
        address
    ));

    // Access the hook manager from the core and install the hook
    let result = {
        let hook_manager = core_guard.hook_manager_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Hook manager not initialized")
        })?;

        // Install the jmpback hook
        hook_manager
            .install_jmpback_hook(address, callback)
            .map_err(convert_universe_error)
    };

    // Log the result
    match &result {
        Ok(()) => {
            core_guard
                .logger()
                .log("Jmpback hook installed successfully");
        }
        Err(e) => {
            core_guard
                .logger()
                .log(&format!("Jmpback hook installation failed: {}", e));
        }
    }

    result
}

/// Remove a hook at the specified address
///
/// Args:
///     address (int): Memory address of the hook to remove
///
/// Raises:
///     RuntimeError: If hook removal fails or Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address,))]
pub fn remove_hook(address: usize) -> PyResult<()> {
    let core = get_core_reference()?;
    let mut core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // Log the hook removal
    core_guard
        .logger()
        .log(&format!("Removing hook at address 0x{:x}", address));

    // Access the hook manager from the core and remove the hook
    let result = {
        let hook_manager = core_guard.hook_manager_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Hook manager not initialized")
        })?;

        // Remove the hook
        hook_manager
            .remove_hook(address)
            .map_err(convert_universe_error)
    };

    // Log the result
    match &result {
        Ok(()) => {
            core_guard.logger().log("Hook removed successfully");
        }
        Err(e) => {
            core_guard
                .logger()
                .log(&format!("Hook removal failed: {}", e));
        }
    }

    result
}

// ============================================================================
// Logging API
// ============================================================================

/// Log a message to the universe.log file
///
/// Args:
///     message (str): Message to log
///
/// Raises:
///     RuntimeError: If Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (message,))]
pub fn log(message: &str) -> PyResult<()> {
    let core = get_core_reference()?;
    let core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // Access the logger from the core and log the message
    core_guard.logger().log(&format!("[Python] {}", message));
    Ok(())
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register the universe module in the Python interpreter
///
/// This function creates the universe module and registers it in sys.modules
/// so it can be imported by Python scripts.
pub fn register_universe_module(py: Python) -> PyResult<()> {
    // Create the universe module
    let universe_module = PyModule::new(py, "universe")?;

    // Register all classes and functions in the module
    register_module_contents(&universe_module)?;

    // Add the universe module to sys.modules so it can be imported
    let sys = py.import("sys")?;
    let sys_modules = sys.getattr("modules")?;
    sys_modules.set_item("universe", universe_module)?;

    Ok(())
}

// ============================================================================
// Module Initialization for PyO3
// ============================================================================

/// PyO3 module definition - this is the main entry point for the Python extension
/// This function is called from lib.rs when the module is imported directly
pub fn register_module_contents(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add all classes
    m.add_class::<PyRegisterAccess>()?;
    // Add all functions
    m.add_function(wrap_pyfunction!(read_memory, m)?)?;
    m.add_function(wrap_pyfunction!(write_memory, m)?)?;
    m.add_function(wrap_pyfunction!(list_modules, m)?)?;
    m.add_function(wrap_pyfunction!(pattern_scan, m)?)?;
    m.add_function(wrap_pyfunction!(hook_function, m)?)?;
    m.add_function(wrap_pyfunction!(hook_jmpback, m)?)?;
    m.add_function(wrap_pyfunction!(remove_hook, m)?)?;
    m.add_function(wrap_pyfunction!(log, m)?)?;

    Ok(())
}
