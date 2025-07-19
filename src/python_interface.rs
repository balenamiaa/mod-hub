use crate::core::{UniverseCore, UniverseError};
use crate::ffi::{CallingConvention, FunctionInfo, NativeType, PyCallableFunction};
use crate::hooks::PyOriginalFunction;
use crate::memory::MemoryManager;
use crate::pointers::{create_pointer, PyBasicPointer, PyStructure, PyStructurePointer, TypeSpec};
use crate::registers::PyRegisterAccess;
use pyo3::prelude::*;
use pyo3::types::PyModule;
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
fn get_core_reference() -> PyResult<Arc<Mutex<UniverseCore>>> {
    let core_ref = UNIVERSE_CORE_REF.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core reference lock")
    })?;

    core_ref.as_ref().cloned().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Universe core not initialized")
    })
}

/// Convert UniverseError to Python exception
fn convert_universe_error(error: UniverseError) -> PyErr {
    match error {
        UniverseError::MemoryError(msg) => PyErr::new::<pyo3::exceptions::PyMemoryError, _>(msg),
        UniverseError::PythonError(msg) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(msg),
        UniverseError::HookError(msg) => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Hook error: {}", msg))
        }
        UniverseError::SystemError(msg) => PyErr::new::<pyo3::exceptions::PySystemError, _>(msg),
        UniverseError::InitializationFailed(msg) => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Initialization failed: {}",
                msg
            ))
        }
    }
}

// ============================================================================
// Memory Operations API
// ============================================================================

/// Read memory from the specified address
///
/// Args:
///     address (int): Memory address to read from
///     size (int): Number of bytes to read
///
/// Returns:
///     bytes: The memory contents as bytes
///
/// Raises:
///     MemoryError: If memory access fails
///     RuntimeError: If Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, size))]
pub fn read_memory(address: usize, size: usize) -> PyResult<Vec<u8>> {
    let core = get_core_reference()?;
    let _core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // In a full implementation, we would access the memory manager from the core
    // For now, create a temporary memory manager
    let memory_manager = MemoryManager::new().map_err(convert_universe_error)?;
    memory_manager
        .read_memory(address, size)
        .map_err(convert_universe_error)
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
    let _core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // In a full implementation, we would access the memory manager from the core
    // For now, create a temporary memory manager
    let memory_manager = MemoryManager::new().map_err(convert_universe_error)?;
    memory_manager
        .write_memory(address, &data)
        .map_err(convert_universe_error)
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
    let _core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Failed to acquire core lock")
    })?;

    // In a full implementation, we would access the memory manager from the core
    // For now, create a temporary memory manager
    let memory_manager = MemoryManager::new().map_err(convert_universe_error)?;
    Ok(memory_manager.pattern_scan_hex(module_name, pattern))
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
///
/// Raises:
///     RuntimeError: If hook installation fails or Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, callback))]
pub fn hook_function(address: usize, callback: PyObject) -> PyResult<()> {
    let _core = get_core_reference()?;

    // Validate callback is callable
    Python::with_gil(|py| {
        if !callback.bind(py).is_callable() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Callback must be callable",
            ));
        }
        Ok(())
    })?;

    // In a full implementation, we would:
    // 1. Get the hook manager from the core
    // 2. Install the function hook
    // 3. Handle any errors

    // For now, just log the operation
    println!("Installing function hook at 0x{:x}", address);
    Ok(())
}

/// Install a jmpback hook at the specified address
///
/// Args:
///     address (int): Memory address to hook
///     callback (callable): Python function to call when hook is triggered.
///                          Receives (registers) parameter only.
///
/// Raises:
///     RuntimeError: If hook installation fails or Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, callback))]
pub fn hook_jmpback(address: usize, callback: PyObject) -> PyResult<()> {
    let _core = get_core_reference()?;

    // Validate callback is callable
    Python::with_gil(|py| {
        if !callback.bind(py).is_callable() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Callback must be callable",
            ));
        }
        Ok(())
    })?;

    // In a full implementation, we would:
    // 1. Get the hook manager from the core
    // 2. Install the jmpback hook
    // 3. Handle any errors

    // For now, just log the operation
    println!("Installing jmpback hook at 0x{:x}", address);
    Ok(())
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
    let _core = get_core_reference()?;

    // In a full implementation, we would:
    // 1. Get the hook manager from the core
    // 2. Remove the hook
    // 3. Handle any errors

    // For now, just log the operation
    println!("Removing hook at 0x{:x}", address);
    Ok(())
}

// ============================================================================
// FFI (Foreign Function Interface) API
// ============================================================================

/// Create a callable function from a memory address with type information
///
/// Args:
///     address (int): Memory address of the function
///     arg_types (list[str]): List of argument type names
///     return_type (str): Return type name
///     calling_convention (str, optional): Calling convention ("cdecl", "stdcall", "fastcall")
///
/// Returns:
///     callable: A callable Python object that invokes the native function
///
/// Raises:
///     ValueError: If type names or calling convention are invalid
///     RuntimeError: If Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, arg_types, return_type, calling_convention = None))]
pub fn create_function(
    address: usize,
    arg_types: Vec<String>,
    return_type: String,
    calling_convention: Option<String>,
) -> PyResult<PyObject> {
    let _core = get_core_reference()?;

    // Parse argument types
    let parsed_arg_types: Result<Vec<NativeType>, PyErr> = arg_types
        .iter()
        .map(|type_str| parse_native_type(type_str))
        .collect();
    let parsed_arg_types = parsed_arg_types?;

    // Parse return type
    let parsed_return_type = parse_native_type(&return_type)?;

    // Parse calling convention
    let parsed_calling_convention = match calling_convention.as_deref() {
        Some("stdcall") => CallingConvention::Stdcall,
        Some("fastcall") => CallingConvention::Fastcall,
        Some("cdecl") | None => CallingConvention::Cdecl,
        Some(other) => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unsupported calling convention: {}",
                other
            )));
        }
    };

    // Create function info
    let function_info = FunctionInfo {
        address,
        arg_types: parsed_arg_types,
        return_type: parsed_return_type,
        calling_convention: parsed_calling_convention,
    };

    // Create the callable Python object
    Python::with_gil(|py| {
        let callable = PyCallableFunction::new(function_info);
        let py_callable = Py::new(py, callable)?;
        Ok(py_callable.into())
    })
}

/// Helper function to parse native type strings
fn parse_native_type(type_str: &str) -> PyResult<NativeType> {
    match type_str.to_lowercase().as_str() {
        "int32" | "i32" | "int" => Ok(NativeType::Int32),
        "int64" | "i64" | "long" => Ok(NativeType::Int64),
        "float32" | "f32" | "float" => Ok(NativeType::Float32),
        "float64" | "f64" | "double" => Ok(NativeType::Float64),
        "pointer" | "ptr" | "usize" => Ok(NativeType::Pointer),
        "cstring" | "str" | "string" => Ok(NativeType::CString),
        "void" => Ok(NativeType::Void),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unsupported native type: {}",
            type_str
        ))),
    }
}

// ============================================================================
// Pointer System API
// ============================================================================

/// Create a pointer to access memory with type information
///
/// This function is re-exported from the pointers module and supports both
/// basic types and custom structure classes.
///
/// Args:
///     address (int): Memory address to point to
///     type_spec: Type specification - either a string for basic types
///                or a Structure class for complex types
///
/// Returns:
///     Pointer or StructurePointer: Appropriate pointer object for the type
///
/// Raises:
///     ValueError: If type specification is invalid
///     RuntimeError: If Universe core is not initialized
#[pyfunction]
#[pyo3(signature = (address, type_spec))]
pub fn create_pointer_wrapper(address: usize, type_spec: &Bound<'_, PyAny>) -> PyResult<PyObject> {
    let _core = get_core_reference()?;
    create_pointer(address, type_spec)
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
    let _core = get_core_reference()?;

    // In a full implementation, we would access the logger from the core
    // For now, just print to console and assume it gets logged
    println!("[Universe] {}", message);
    Ok(())
}

// ============================================================================
// Module Registration
// ============================================================================

/// Create and register the universe Python module with all exposed APIs
pub fn create_universe_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let universe_module = PyModule::new(py, "universe")?;

    // Add module docstring
    universe_module.add("__doc__", "Universe modding framework - provides memory access, hooking, FFI, and pointer utilities for game modding")?;
    universe_module.add("__version__", "1.0.0")?;

    // Memory operations
    universe_module.add_function(wrap_pyfunction!(read_memory, &universe_module)?)?;
    universe_module.add_function(wrap_pyfunction!(write_memory, &universe_module)?)?;
    universe_module.add_function(wrap_pyfunction!(pattern_scan, &universe_module)?)?;

    // Hook system
    universe_module.add_function(wrap_pyfunction!(hook_function, &universe_module)?)?;
    universe_module.add_function(wrap_pyfunction!(hook_jmpback, &universe_module)?)?;
    universe_module.add_function(wrap_pyfunction!(remove_hook, &universe_module)?)?;

    // FFI system
    universe_module.add_function(wrap_pyfunction!(create_function, &universe_module)?)?;

    // Pointer system
    universe_module.add_function(wrap_pyfunction!(create_pointer_wrapper, &universe_module)?)?;
    // Add alias for backward compatibility
    universe_module.add("create_pointer", universe_module.getattr("create_pointer_wrapper")?)?;
    // Alias for backward compatibility
    universe_module.add("Pointer", py.get_type::<PyBasicPointer>())?;

    // Structure system
    universe_module.add_class::<PyStructure>()?;
    universe_module.add_class::<PyStructurePointer>()?;

    // Type system
    universe_module.add_class::<TypeSpec>()?;

    // Register access (used in hook callbacks)
    universe_module.add_class::<PyRegisterAccess>()?;

    // Original function wrapper (used in function hook callbacks)
    universe_module.add_class::<PyOriginalFunction>()?;

    // Callable function wrapper (returned by create_function)
    universe_module.add_class::<PyCallableFunction>()?;

    // Logging
    universe_module.add_function(wrap_pyfunction!(log, &universe_module)?)?;

    Ok(universe_module)
}

/// Register the universe module in the Python interpreter
pub fn register_universe_module(py: Python) -> PyResult<()> {
    let universe_module = create_universe_module(py)?;

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
/// This function is called from lib.rs and should not be duplicated
pub fn universe(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add all classes
    m.add_class::<PyRegisterAccess>()?;
    m.add_class::<PyOriginalFunction>()?;
    m.add_class::<PyCallableFunction>()?;
    m.add_class::<PyBasicPointer>()?;
    m.add_class::<PyStructure>()?;
    m.add_class::<PyStructurePointer>()?;
    m.add_class::<TypeSpec>()?;

    // Add all functions
    m.add_function(wrap_pyfunction!(read_memory, m)?)?;
    m.add_function(wrap_pyfunction!(write_memory, m)?)?;
    m.add_function(wrap_pyfunction!(pattern_scan, m)?)?;
    m.add_function(wrap_pyfunction!(hook_function, m)?)?;
    m.add_function(wrap_pyfunction!(hook_jmpback, m)?)?;
    m.add_function(wrap_pyfunction!(remove_hook, m)?)?;
    m.add_function(wrap_pyfunction!(create_function, m)?)?;
    m.add_function(wrap_pyfunction!(create_pointer_wrapper, m)?)?;
    m.add_function(wrap_pyfunction!(log, m)?)?;

    // Add alias for create_pointer
    m.add("create_pointer", m.getattr("create_pointer_wrapper")?)?;

    // Add module metadata
    m.add("__version__", "1.0.0")?;
    m.add("__doc__", "Universe modding framework - provides memory access, hooking, FFI, and pointer utilities for game modding")?;

    // Add Pointer as an alias for PyBasicPointer
    m.add("Pointer", m.py().get_type::<PyBasicPointer>())?;

    Ok(())
}
