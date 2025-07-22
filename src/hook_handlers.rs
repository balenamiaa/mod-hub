use crate::core::UniverseError;
use crate::logging::Logger;
use ilhook::x64::Registers;
use lazy_static::lazy_static;
use pyo3::{prelude::*, IntoPyObjectExt};
use std::sync::{Arc, Mutex, RwLock};

// Global registry for hook callbacks
lazy_static! {
    static ref HOOK_REGISTRY: RwLock<Vec<usize>> = RwLock::new(Vec::new()); // A vector of leaked Box<PyObject> raw pointers
    static ref JMPBACK_REGISTRY: RwLock<Vec<usize>> = RwLock::new(Vec::new()); // A vector of leaked Box<PyObject> raw pointers
    static ref LOGGER: Mutex<Option<Arc<Logger>>> = Mutex::new(None);
}

/// Initialize the global logger reference
pub fn initialize_logger(logger: Arc<Logger>) -> Result<(), UniverseError> {
    let mut logger_ref = LOGGER
        .lock()
        .map_err(|_| UniverseError::SystemError("Failed to acquire logger lock".to_string()))?;
    *logger_ref = Some(logger);
    Ok(())
}

/// Get the logger for error reporting
fn get_logger() -> Option<Arc<Logger>> {
    if let Ok(logger_guard) = LOGGER.lock() {
        logger_guard.clone()
    } else {
        None
    }
}

/// Register a function hook callback
pub fn register_hook_callback(callback: *mut PyObject) -> Result<(), UniverseError> {
    let mut registry = HOOK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire hook registry write lock".to_string())
    })?;

    registry.push(callback as usize);
    Ok(())
}

/// Register a jmpback hook callback
pub fn register_jmpback_callback(callback: *mut PyObject) -> Result<(), UniverseError> {
    let mut registry = JMPBACK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire jmpback registry write lock".to_string())
    })?;

    registry.push(callback as usize);
    Ok(())
}

/// Remove a hook callback
pub fn remove_hook_callback(py_callback_ptr: usize) -> Result<(), UniverseError> {
    let mut registry = HOOK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire hook registry write lock".to_string())
    })?;

    if let Some(index) = registry.iter().position(|&x| x == py_callback_ptr) {
        let boxed_callback = unsafe { Box::from_raw(py_callback_ptr as *mut PyObject) };
        Python::with_gil(|py| boxed_callback.drop_ref(py));
        registry.remove(index);
    }

    Ok(())
}

/// Remove a jmpback hook callback
pub fn remove_jmpback_callback(py_callback_ptr: usize) -> Result<(), UniverseError> {
    let mut registry = JMPBACK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire jmpback registry write lock".to_string())
    })?;

    if let Some(index) = registry.iter().position(|&x| x == py_callback_ptr) {
        let boxed_callback = unsafe { Box::from_raw(py_callback_ptr as *mut PyObject) };
        Python::with_gil(|py| boxed_callback.drop_ref(py));
        registry.remove(index);
    }

    Ok(())
}

/// Clear all hook callbacks
pub fn clear_all_hook_callbacks() -> Result<(), UniverseError> {
    let mut registry = HOOK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire hook registry write lock".to_string())
    })?;

    registry.clear();
    Ok(())
}

/// Clear all jmpback hook callbacks
pub fn clear_all_jmpback_callbacks() -> Result<(), UniverseError> {
    let mut registry = JMPBACK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire jmpback registry write lock".to_string())
    })?;

    registry.clear();
    Ok(())
}

/// Execute a function hook callback
///
/// This function is called from the ilhook assembly hook handler
#[no_mangle]
pub extern "win64" fn execute_hook_callback(
    regs: *mut Registers,
    original_function_ptr: usize,
    py_callback: usize,
) -> usize {
    let callback = py_callback as *mut PyObject;

    Python::with_gil(|py| {
        let py_registers_obj = match Py::new(py, PyRegisterAccess::new(unsafe { &*regs })) {
            Ok(py_registers_obj) => py_registers_obj,
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::PythonError(format!(
                        "Failed to convert registers to Python object. Entire program state is likely corrupted. {}",
                        e
                    )));
                }
                return 0;
            }
        };

        let original_function_ptr_obj = match original_function_ptr.into_py_any(py) {
            Ok(original_function_ptr_obj) => original_function_ptr_obj.clone_ref(py),
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::PythonError(format!(
                        "Failed to convert original function pointer to Python object. Entire program state is likely corrupted. {}",
                        e
                    )));
                }
                return 0;
            }
        };

        let args = (
            py_registers_obj.clone_ref(py),
            original_function_ptr_obj.clone_ref(py),
        );

        match (unsafe { &mut *callback }).call1(py, args) {
            Ok(result) => {
                let modified_py_regs: PyRef<PyRegisterAccess> = py_registers_obj.borrow(py);
                modified_py_regs.copy_to_ilhook_registers(unsafe { &mut *regs });

                match result.extract::<usize>(py) {
                    Ok(result) => result,
                    Err(e) => {
                        if let Some(logger) = get_logger() {
                            logger.log_error(&UniverseError::PythonError(format!(
                                "Hook callback returned non-usize value. Entire program state is likely corrupted. {}",
                                e
                            )));
                        }

                        0
                    }
                }
            }
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::PythonError(format!(
                        "Hook callback failed. Entire program state is likely corrupted. {}",
                        e
                    )));
                }

                0
            }
        }
    })
}

/// Execute a jmpback hook callback
///
/// This function is called from the ilhook assembly jmpback hook handler
#[no_mangle]
pub extern "win64" fn execute_jmpback_callback(regs: *mut Registers, py_callback: usize) {
    let callback = py_callback as *mut PyObject;

    Python::with_gil(|py| {
        let py_registers_obj = match Py::new(py, PyRegisterAccess::new(unsafe { &*regs })) {
            Ok(py_registers_obj) => py_registers_obj,
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::PythonError(format!(
                        "Failed to convert registers to Python object. Entire program state is likely corrupted. {}",
                        e
                    )));
                }
                return;
            }
        };

        let args = (py_registers_obj.clone_ref(py),);

        match (unsafe { &mut *callback }).call1(py, args) {
            Ok(_) => {
                let modified_py_regs: PyRef<PyRegisterAccess> = py_registers_obj.borrow(py);
                modified_py_regs.copy_to_ilhook_registers(unsafe { &mut *regs });
            }
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::PythonError(format!(
                        "Jmpback hook callback failed. Entire program state is likely corrupted. {}",
                        e
                    )));
                }
            }
        }
    })
}

/// Python wrapper for register access with read/write capabilities.
/// This struct holds a copy of the register state from ilhook, making it thread-safe.
#[pyclass(name = "Registers")]
#[derive(Clone)]
pub struct PyRegisterAccess {
    // General-purpose registers
    #[pyo3(get, set)]
    pub rax: u64,
    #[pyo3(get, set)]
    pub rbx: u64,
    #[pyo3(get, set)]
    pub rcx: u64,
    #[pyo3(get, set)]
    pub rdx: u64,
    #[pyo3(get, set)]
    pub rsi: u64,
    #[pyo3(get, set)]
    pub rdi: u64,
    #[pyo3(get, set)]
    pub rsp: u64,
    #[pyo3(get, set)]
    pub rbp: u64,
    #[pyo3(get, set)]
    pub r8: u64,
    #[pyo3(get, set)]
    pub r9: u64,
    #[pyo3(get, set)]
    pub r10: u64,
    #[pyo3(get, set)]
    pub r11: u64,
    #[pyo3(get, set)]
    pub r12: u64,
    #[pyo3(get, set)]
    pub r13: u64,
    #[pyo3(get, set)]
    pub r14: u64,
    #[pyo3(get, set)]
    pub r15: u64,
    #[pyo3(get, set)]
    pub rflags: u64,

    // XMM registers
    pub xmm0: u128,
    pub xmm1: u128,
    pub xmm2: u128,
    pub xmm3: u128,
}

impl PyRegisterAccess {
    pub fn new(regs: &Registers) -> Self {
        PyRegisterAccess {
            rax: regs.rax,
            rbx: regs.rbx,
            rcx: regs.rcx,
            rdx: regs.rdx,
            rsi: regs.rsi,
            rdi: regs.rdi,
            rsp: regs.rsp,
            rbp: regs.rbp,
            r8: regs.r8,
            r9: regs.r9,
            r10: regs.r10,
            r11: regs.r11,
            r12: regs.r12,
            r13: regs.r13,
            r14: regs.r14,
            r15: regs.r15,
            rflags: regs.rflags,
            xmm0: regs.xmm0,
            xmm1: regs.xmm1,
            xmm2: regs.xmm2,
            xmm3: regs.xmm3,
        }
    }

    pub fn copy_to_ilhook_registers(&self, regs: &mut Registers) {
        regs.rax = self.rax;
        regs.rbx = self.rbx;
        regs.rcx = self.rcx;
        regs.rdx = self.rdx;
        regs.rsi = self.rsi;
        regs.rdi = self.rdi;
        regs.rsp = self.rsp;
        regs.rbp = self.rbp;
        regs.r8 = self.r8;
        regs.r9 = self.r9;
        regs.r10 = self.r10;
        regs.r11 = self.r11;
        regs.r12 = self.r12;
        regs.r13 = self.r13;
        regs.r14 = self.r14;
        regs.r15 = self.r15;
        regs.rflags = self.rflags;
        regs.xmm0 = self.xmm0;
        regs.xmm1 = self.xmm1;
        regs.xmm2 = self.xmm2;
        regs.xmm3 = self.xmm3;
    }
}

#[pymethods]
impl PyRegisterAccess {
    #[new]
    fn py_new() -> Self {
        // A default constructor for Python side if needed, though it's mainly created from Rust.
        PyRegisterAccess {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rsp: 0,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0,
            xmm0: 0,
            xmm1: 0,
            xmm2: 0,
            xmm3: 0,
        }
    }

    fn get_xmm(&self, index: usize) -> PyResult<u128> {
        match index {
            0 => Ok(self.xmm0),
            1 => Ok(self.xmm1),
            2 => Ok(self.xmm2),
            3 => Ok(self.xmm3),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "XMM register index must be 0-15",
            )),
        }
    }

    fn set_xmm(&mut self, index: usize, value: u128) -> PyResult<()> {
        match index {
            0 => self.xmm0 = value,
            1 => self.xmm1 = value,
            2 => self.xmm2 = value,
            3 => self.xmm3 = value,
            _ => {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "XMM register index must be 0-15",
                ))
            }
        }
        Ok(())
    }

    // Convenience methods for XMM registers as bytes
    fn get_xmm_bytes(&self, index: usize) -> PyResult<Vec<u8>> {
        self.get_xmm(index).map(|val| val.to_le_bytes().to_vec())
    }

    fn set_xmm_bytes(&mut self, index: usize, bytes: Vec<u8>) -> PyResult<()> {
        if bytes.len() != 16 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "XMM register requires exactly 16 bytes",
            ));
        }
        let mut array = [0u8; 16];
        array.copy_from_slice(&bytes);
        self.set_xmm(index, u128::from_le_bytes(array))
    }

    // String representation for debugging
    fn __repr__(&self) -> String {
        format!(
            "Registers(rax=0x{:016x}, rbx=0x{:016x}, rcx=0x{:016x}, rdx=0x{:016x}, rsi=0x{:016x}, rdi=0x{:016x}, rsp=0x{:016x}, rbp=0x{:016x}, r8=0x{:016x}, r9=0x{:016x}, r10=0x{:016x}, r11=0x{:016x}, r12=0x{:016x}, r13=0x{:016x}, r14=0x{:016x}, r15=0x{:016x}, rflags=0x{:016x})",
            self.rax, self.rbx, self.rcx, self.rdx,
            self.rsi, self.rdi, self.rsp, self.rbp,
            self.r8, self.r9, self.r10, self.r11,
            self.r12, self.r13, self.r14, self.r15,
            self.rflags
        )
    }
}
