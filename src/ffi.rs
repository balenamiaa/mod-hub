use crate::core::UniverseError;
use crate::ffi_asm;
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;
use std::collections::HashMap;
use std::ffi::CString;
use std::mem;

/// Supported native types for FFI
#[derive(Debug, Clone, PartialEq)]
pub enum NativeType {
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Pointer,
    CString,
    Void,
    Struct(String),  // Named structure type
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
                NativeType::Int8 => {
                    let val: i8 = arg.extract()?;
                    val as u64
                }
                NativeType::Int16 => {
                    let val: i16 = arg.extract()?;
                    val as u64
                }
                NativeType::Int32 => {
                    let val: i32 = arg.extract()?;
                    val as u64
                }
                NativeType::Int64 => {
                    let val: i64 = arg.extract()?;
                    val as u64
                }
                NativeType::UInt8 => {
                    let val: u8 = arg.extract()?;
                    val as u64
                }
                NativeType::UInt16 => {
                    let val: u16 = arg.extract()?;
                    val as u64
                }
                NativeType::UInt32 => {
                    let val: u32 = arg.extract()?;
                    val as u64
                }
                NativeType::UInt64 => {
                    let val: u64 = arg.extract()?;
                    val
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
                    // Handle different pointer types
                    if arg.is_instance_of::<pyo3::types::PyInt>() {
                        // Raw pointer value
                        let val: usize = arg.extract()?;
                        val as u64
                    } else {
                        // Check if it's a structure pointer
                        if arg.hasattr("_address")? {
                            let addr = arg.getattr("_address")?.extract::<usize>()?;
                            addr as u64
                        } else {
                            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                "Expected pointer object with _address attribute"
                            ));
                        }
                    }
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
                NativeType::Struct(struct_name) => {
                    // Handle structure by reference
                    if arg.hasattr("_address")? {
                        // It's a structure pointer, pass its address
                        let addr = arg.getattr("_address")?.extract::<usize>()?;
                        addr as u64
                    } else {
                        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                            format!("Expected structure pointer for type {}", struct_name)
                        ));
                    }
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
        // Use our assembly implementation to handle different calling conventions
        ffi_asm::call_function(
            self.info.address,
            args,
            &self.info.return_type,
            &self.info.calling_convention,
        )
    }

    /// Marshal native return value to Python object
    fn marshal_return_to_python(&self, py: Python, result: &u64) -> PyResult<PyObject> {
        match &self.info.return_type {
            NativeType::Int8 => {
                let val = *result as i8;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::Int16 => {
                let val = *result as i16;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::Int32 => {
                let val = *result as i32;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::Int64 => {
                let val = *result as i64;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::UInt8 => {
                let val = *result as u8;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::UInt16 => {
                let val = *result as u16;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::UInt32 => {
                let val = *result as u32;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            NativeType::UInt64 => {
                let val = *result;
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
            NativeType::Struct(_struct_name) => {
                // For structure returns, we need to create a structure pointer
                // This requires access to the pointer manager, which we don't have here
                // For now, just return the raw address and let the user create a pointer
                let val = *result as usize;
                
                // Note: We can't access the logger from here easily
                // This debug information will be handled by the caller if needed
                
                Ok(val.into_pyobject(py)?.into_any().unbind())
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