use crate::core::UniverseError;
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;
use std::collections::HashMap;
use std::ffi::CString;
use std::mem;

/// Supported native types for FFI
#[derive(Debug, Clone)]
pub enum NativeType {
    Int32,
    Int64,
    Float32,
    Float64,
    Pointer,
    CString,
    Void,
}

/// Supported calling conventions
#[derive(Debug, Clone)]
pub enum CallingConvention {
    Cdecl,
    Stdcall,
    Fastcall,
}

/// Information about a callable function
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub address: usize,
    pub arg_types: Vec<NativeType>,
    pub return_type: NativeType,
    pub calling_convention: CallingConvention,
}

/// A callable Python object that wraps a native function
#[pyclass]
pub struct PyCallableFunction {
    info: FunctionInfo,
}

#[pymethods]
impl PyCallableFunction {
    /// Call the native function with the provided arguments
    fn __call__(&self, args: &Bound<'_, pyo3::types::PyTuple>) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            // Marshal arguments from Python to native types
            let native_args = self.marshal_args_to_native(args)?;
            
            // Call the native function
            let result = self.call_native_function(&native_args)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("FFI call failed: {}", e)))?;
            
            // Marshal return value from native to Python
            self.marshal_return_to_python(py, &result)
        })
    }
}

impl PyCallableFunction {
    /// Create a new callable function wrapper
    pub fn new(info: FunctionInfo) -> Self {
        PyCallableFunction { info }
    }

    /// Marshal Python arguments to native types
    fn marshal_args_to_native(&self, args: &Bound<'_, pyo3::types::PyTuple>) -> PyResult<Vec<u64>> {
        let mut native_args = Vec::new();
        
        if args.len() != self.info.arg_types.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Expected {} arguments, got {}", self.info.arg_types.len(), args.len())
            ));
        }

        for (i, arg_type) in self.info.arg_types.iter().enumerate() {
            let arg = args.get_item(i)?;
            let native_value = match arg_type {
                NativeType::Int32 => {
                    let val: i32 = arg.extract()?;
                    val as u64
                }
                NativeType::Int64 => {
                    let val: i64 = arg.extract()?;
                    val as u64
                }
                NativeType::Float32 => {
                    let val: f32 = arg.extract()?;
                    unsafe { mem::transmute::<f32, u32>(val) as u64 }
                }
                NativeType::Float64 => {
                    let val: f64 = arg.extract()?;
                    unsafe { mem::transmute::<f64, u64>(val) }
                }
                NativeType::Pointer => {
                    let val: usize = arg.extract()?;
                    val as u64
                }
                NativeType::CString => {
                    let val: String = arg.extract()?;
                    let c_string = CString::new(val)
                        .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid C string"))?;
                    let ptr = c_string.as_ptr() as usize;
                    // Note: This leaks memory - in a real implementation we'd need proper lifetime management
                    std::mem::forget(c_string);
                    ptr as u64
                }
                NativeType::Void => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Cannot pass void as argument"));
                }
            };
            native_args.push(native_value);
        }

        Ok(native_args)
    }

    /// Call the native function with marshalled arguments
    fn call_native_function(&self, args: &[u64]) -> Result<u64, UniverseError> {
        // This is a simplified implementation - in a real scenario we'd need proper
        // assembly code generation for different calling conventions
        match self.info.calling_convention {
            CallingConvention::Cdecl => self.call_cdecl(args),
            CallingConvention::Stdcall => self.call_stdcall(args),
            CallingConvention::Fastcall => self.call_fastcall(args),
        }
    }

    /// Call function using cdecl calling convention
    fn call_cdecl(&self, args: &[u64]) -> Result<u64, UniverseError> {
        // Simplified implementation - in reality this would require inline assembly
        // or dynamic code generation to properly handle calling conventions
        unsafe {
            match args.len() {
                0 => {
                    let func: extern "C" fn() -> u64 = mem::transmute(self.info.address);
                    Ok(func())
                }
                1 => {
                    let func: extern "C" fn(u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0]))
                }
                2 => {
                    let func: extern "C" fn(u64, u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0], args[1]))
                }
                3 => {
                    let func: extern "C" fn(u64, u64, u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0], args[1], args[2]))
                }
                4 => {
                    let func: extern "C" fn(u64, u64, u64, u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0], args[1], args[2], args[3]))
                }
                _ => Err(UniverseError::SystemError("Too many arguments for cdecl call".to_string()))
            }
        }
    }

    /// Call function using stdcall calling convention
    fn call_stdcall(&self, args: &[u64]) -> Result<u64, UniverseError> {
        // Simplified implementation - stdcall is similar to cdecl but callee cleans stack
        // Using "system" ABI which maps to stdcall on Windows
        unsafe {
            match args.len() {
                0 => {
                    let func: extern "system" fn() -> u64 = mem::transmute(self.info.address);
                    Ok(func())
                }
                1 => {
                    let func: extern "system" fn(u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0]))
                }
                2 => {
                    let func: extern "system" fn(u64, u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0], args[1]))
                }
                3 => {
                    let func: extern "system" fn(u64, u64, u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0], args[1], args[2]))
                }
                4 => {
                    let func: extern "system" fn(u64, u64, u64, u64) -> u64 = mem::transmute(self.info.address);
                    Ok(func(args[0], args[1], args[2], args[3]))
                }
                _ => Err(UniverseError::SystemError("Too many arguments for stdcall call".to_string()))
            }
        }
    }

    /// Call function using fastcall calling convention
    fn call_fastcall(&self, args: &[u64]) -> Result<u64, UniverseError> {
        // Simplified implementation - fastcall passes first two args in registers
        // For now, we'll use cdecl as a fallback since fastcall isn't supported on all targets
        // In a production implementation, this would require platform-specific assembly
        self.call_cdecl(args)
    }

    /// Marshal native return value to Python object
    fn marshal_return_to_python(&self, py: Python, result: &u64) -> PyResult<PyObject> {
        match self.info.return_type {
            NativeType::Int32 => {
                let val = *result as i32;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::Int64 => {
                let val = *result as i64;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::Float32 => {
                let val = unsafe { mem::transmute::<u32, f32>(*result as u32) };
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::Float64 => {
                let val = unsafe { mem::transmute::<u64, f64>(*result) };
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::Pointer => {
                let val = *result as usize;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::CString => {
                if *result == 0 {
                    Ok(py.None())
                } else {
                    unsafe {
                        let c_str = std::ffi::CStr::from_ptr(*result as *const i8);
                        let rust_str = c_str.to_str()
                            .map_err(|_| PyErr::new::<pyo3::exceptions::PyUnicodeDecodeError, _>("Invalid UTF-8 in C string"))?;
                        Ok(rust_str.into_pyobject(py)?.into_any().unbind())
                    }
                }
            }
            NativeType::Void => {
                Ok(py.None())
            }
        }
    }
}

/// FFI bridge for calling native functions from Python
pub struct FFIBridge {
    function_cache: HashMap<usize, FunctionInfo>,
}

impl FFIBridge {
    /// Create a new FFI bridge instance
    pub fn new() -> Result<Self, UniverseError> {
        Ok(FFIBridge {
            function_cache: HashMap::new(),
        })
    }

    /// Create a callable Python object from a memory address with type information
    pub fn create_function(&mut self, info: FunctionInfo) -> Result<PyObject, UniverseError> {
        // Cache the function info
        self.function_cache.insert(info.address, info.clone());
        
        // Create the callable Python object
        Python::with_gil(|py| {
            let callable = PyCallableFunction::new(info);
            let py_callable = Py::new(py, callable)
                .map_err(|e| UniverseError::PythonError(format!("Failed to create callable: {}", e)))?;
            Ok(py_callable.into())
        })
    }

    /// Get function info for a cached function
    pub fn get_function_info(&self, address: usize) -> Option<&FunctionInfo> {
        self.function_cache.get(&address)
    }

    /// Register a function for calling from Python (legacy method)
    pub fn register_function(&mut self, info: FunctionInfo) -> Result<(), UniverseError> {
        self.function_cache.insert(info.address, info);
        Ok(())
    }
}