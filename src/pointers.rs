use crate::memory::MemoryManager;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Get a reference to the global memory manager from the Universe core
fn get_global_memory_manager() -> PyResult<Arc<Mutex<MemoryManager>>> {
    // Import the function to get core reference from python_interface
    use crate::python_interface::get_core_reference;
    
    let core = get_core_reference()?;
    let core_guard = core.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            "Failed to acquire core lock"
        )
    })?;

    core_guard.memory_manager().ok_or_else(|| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            "Memory manager not initialized in Universe core"
        )
    })
}

/// Type-safe specification for all supported data types
#[derive(Debug, Clone, PartialEq)]
#[pyclass(name = "TypeSpec")]
pub enum TypeSpec {
    Int32,
    Int64,
    Float32,
    Float64,
    String,
    Bool,
    UInt32,
    UInt64,
    Pointer,
}

#[pymethods]
impl TypeSpec {
    /// Create Int32 type specification
    #[classattr]
    const INT32: TypeSpec = TypeSpec::Int32;
    
    /// Create Int64 type specification
    #[classattr]
    const INT64: TypeSpec = TypeSpec::Int64;
    
    /// Create Float32 type specification
    #[classattr]
    const FLOAT32: TypeSpec = TypeSpec::Float32;
    
    /// Create Float64 type specification
    #[classattr]
    const FLOAT64: TypeSpec = TypeSpec::Float64;
    
    /// Create String type specification
    #[classattr]
    const STRING: TypeSpec = TypeSpec::String;
    
    /// Create Bool type specification
    #[classattr]
    const BOOL: TypeSpec = TypeSpec::Bool;
    
    /// Create UInt32 type specification
    #[classattr]
    const UINT32: TypeSpec = TypeSpec::UInt32;
    
    /// Create UInt64 type specification
    #[classattr]
    const UINT64: TypeSpec = TypeSpec::UInt64;
    
    /// Create Pointer type specification
    #[classattr]
    const POINTER: TypeSpec = TypeSpec::Pointer;
    
    /// Get the size in bytes for this type
    pub fn size(&self) -> usize {
        match self {
            TypeSpec::Int32 => 4,
            TypeSpec::Int64 => 8,
            TypeSpec::Float32 => 4,
            TypeSpec::Float64 => 8,
            TypeSpec::String => 0, // Variable size
            TypeSpec::Bool => 1,
            TypeSpec::UInt32 => 4,
            TypeSpec::UInt64 => 8,
            TypeSpec::Pointer => 8, // 64-bit pointer
        }
    }
    
    /// Get the name of this type
    pub fn name(&self) -> &'static str {
        match self {
            TypeSpec::Int32 => "int32",
            TypeSpec::Int64 => "int64",
            TypeSpec::Float32 => "float32",
            TypeSpec::Float64 => "float64",
            TypeSpec::String => "string",
            TypeSpec::Bool => "bool",
            TypeSpec::UInt32 => "uint32",
            TypeSpec::UInt64 => "uint64",
            TypeSpec::Pointer => "pointer",
        }
    }
    
    /// String representation
    fn __repr__(&self) -> String {
        format!("TypeSpec.{}", self.name().to_uppercase())
    }
    
    /// String representation
    fn __str__(&self) -> String {
        self.name().to_string()
    }
}

/// Supported basic data types for pointers (internal representation)
#[derive(Debug, Clone)]
pub enum BasicType {
    Int32,
    Int64,
    Float32,
    Float64,
    String,
    Bool,
    UInt32,
    UInt64,
    Pointer,
}

/// Conversion utilities between TypeSpec and BasicType
impl From<TypeSpec> for BasicType {
    fn from(type_spec: TypeSpec) -> Self {
        match type_spec {
            TypeSpec::Int32 => BasicType::Int32,
            TypeSpec::Int64 => BasicType::Int64,
            TypeSpec::Float32 => BasicType::Float32,
            TypeSpec::Float64 => BasicType::Float64,
            TypeSpec::String => BasicType::String,
            TypeSpec::Bool => BasicType::Bool,
            TypeSpec::UInt32 => BasicType::UInt32,
            TypeSpec::UInt64 => BasicType::UInt64,
            TypeSpec::Pointer => BasicType::Pointer,
        }
    }
}

impl From<BasicType> for TypeSpec {
    fn from(basic_type: BasicType) -> Self {
        match basic_type {
            BasicType::Int32 => TypeSpec::Int32,
            BasicType::Int64 => TypeSpec::Int64,
            BasicType::Float32 => TypeSpec::Float32,
            BasicType::Float64 => TypeSpec::Float64,
            BasicType::String => TypeSpec::String,
            BasicType::Bool => TypeSpec::Bool,
            BasicType::UInt32 => TypeSpec::UInt32,
            BasicType::UInt64 => TypeSpec::UInt64,
            BasicType::Pointer => TypeSpec::Pointer,
        }
    }
}

/// Field definition for structure fields
#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub offset: usize,
    pub size: usize,
}

/// Supported field types in structures
#[derive(Debug, Clone)]
pub enum FieldType {
    Basic(BasicType),
    Structure(String), // Name of another structure type
    Array(Box<FieldType>, usize), // Element type and count
}

/// Structure definition containing field layout
#[derive(Debug, Clone)]
pub struct StructureDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub total_size: usize,
}

/// Pointer manager for handling both basic type and structure pointers
pub struct PointerManager {
    memory_manager: Arc<Mutex<MemoryManager>>,
    structure_registry: Arc<Mutex<HashMap<String, StructureDefinition>>>,
}

impl PointerManager {
    /// Create a new pointer manager
    pub fn new(memory_manager: Arc<Mutex<MemoryManager>>) -> Self {
        PointerManager { 
            memory_manager,
            structure_registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a basic type pointer
    pub fn create_basic_pointer(
        &self,
        address: usize,
        basic_type: BasicType,
    ) -> PyResult<PyBasicPointer> {
        Ok(PyBasicPointer {
            address,
            basic_type,
            memory_manager: Arc::clone(&self.memory_manager),
        })
    }

    /// Create a structure pointer
    pub fn create_structure_pointer(
        &self,
        address: usize,
        structure_def: StructureDefinition,
    ) -> PyResult<PyStructurePointer> {
        Ok(PyStructurePointer {
            address,
            structure_def,
            memory_manager: Arc::clone(&self.memory_manager),
            structure_registry: Arc::clone(&self.structure_registry),
        })
    }

    /// Register a structure definition
    pub fn register_structure(&self, definition: StructureDefinition) -> PyResult<()> {
        let mut registry = self.structure_registry.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire structure registry lock"
            )
        })?;
        registry.insert(definition.name.clone(), definition);
        Ok(())
    }

    /// Get a structure definition by name
    pub fn get_structure_definition(&self, name: &str) -> PyResult<Option<StructureDefinition>> {
        let registry = self.structure_registry.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire structure registry lock"
            )
        })?;
        Ok(registry.get(name).cloned())
    }
}

/// Python wrapper for basic type pointers
#[pyclass(name = "Pointer")]
pub struct PyBasicPointer {
    address: usize,
    basic_type: BasicType,
    memory_manager: Arc<Mutex<MemoryManager>>,
}

#[pymethods]
impl PyBasicPointer {
    /// Create a new basic pointer from Python using TypeSpec
    #[new]
    #[pyo3(signature = (address, type_spec))]
    pub fn new(address: usize, type_spec: &Bound<'_, PyAny>) -> PyResult<Self> {
        // Get the memory manager from the global Universe core
        let memory_manager = get_global_memory_manager()?;

        // Try to extract TypeSpec enum first
        let basic_type = if let Ok(type_spec_enum) = type_spec.extract::<TypeSpec>() {
            BasicType::from(type_spec_enum)
        } else if let Ok(type_name) = type_spec.extract::<String>() {
            // Backward compatibility with string-based types
            match type_name.as_str() {
                "int" | "i32" | "int32" => BasicType::Int32,
                "int64" | "i64" => BasicType::Int64,
                "float" | "f32" | "float32" => BasicType::Float32,
                "float64" | "f64" => BasicType::Float64,
                "str" | "string" => BasicType::String,
                "bool" | "boolean" => BasicType::Bool,
                "uint32" | "u32" => BasicType::UInt32,
                "uint64" | "u64" => BasicType::UInt64,
                "pointer" | "ptr" => BasicType::Pointer,
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unsupported type: {}. Use TypeSpec enum for type-safe access.",
                        type_name
                    )))
                }
            }
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "type_spec must be a TypeSpec enum or string"
            ));
        };

        Ok(PyBasicPointer {
            address,
            basic_type,
            memory_manager,
        })
    }

    /// Read value from memory address
    pub fn read(&self) -> PyResult<PyObject> {
        let memory_manager = self.memory_manager.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire memory manager lock",
            )
        })?;

        match &self.basic_type {
            BasicType::Int32 => {
                let bytes = memory_manager.read_memory(self.address, 4).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
            BasicType::Int64 => {
                let bytes = memory_manager.read_memory(self.address, 8).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = i64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
            BasicType::Float32 => {
                let bytes = memory_manager.read_memory(self.address, 4).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
            BasicType::Float64 => {
                let bytes = memory_manager.read_memory(self.address, 8).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = f64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
            BasicType::String => {
                // For strings, we need to read until null terminator or a reasonable limit
                let max_length = 1024; // Reasonable limit for string reading
                let bytes = memory_manager
                    .read_memory(self.address, max_length)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Memory read error: {}",
                            e
                        ))
                    })?;

                // Find null terminator
                let null_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                let string_bytes = &bytes[..null_pos];

                let value = String::from_utf8_lossy(string_bytes).to_string();
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
            BasicType::Bool => {
                let bytes = memory_manager.read_memory(self.address, 1).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = bytes[0] != 0;
                Python::with_gil(|py| {
                    let py_bool = value.into_pyobject(py)?;
                    Ok(py_bool.as_any().clone().unbind())
                })
            }
            BasicType::UInt32 => {
                let bytes = memory_manager.read_memory(self.address, 4).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
            BasicType::UInt64 => {
                let bytes = memory_manager.read_memory(self.address, 8).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = u64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
            BasicType::Pointer => {
                let bytes = memory_manager.read_memory(self.address, 8).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                let value = usize::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Python::with_gil(|py| Ok(value.into_pyobject(py)?.into()))
            }
        }
    }

    /// Write value to memory address
    pub fn write(&self, value: PyObject) -> PyResult<()> {
        let memory_manager = self.memory_manager.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire memory manager lock",
            )
        })?;

        Python::with_gil(|py| {
            match &self.basic_type {
                BasicType::Int32 => {
                    let int_value: i32 = value.extract(py)?;
                    let bytes = int_value.to_le_bytes();
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::Int64 => {
                    let int_value: i64 = value.extract(py)?;
                    let bytes = int_value.to_le_bytes();
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::Float32 => {
                    let float_value: f32 = value.extract(py)?;
                    let bytes = float_value.to_le_bytes();
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::Float64 => {
                    let float_value: f64 = value.extract(py)?;
                    let bytes = float_value.to_le_bytes();
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::String => {
                    let string_value: String = value.extract(py)?;
                    let mut bytes = string_value.into_bytes();
                    bytes.push(0); // Add null terminator
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::Bool => {
                    let bool_value: bool = value.extract(py)?;
                    let bytes = [if bool_value { 1u8 } else { 0u8 }];
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::UInt32 => {
                    let uint_value: u32 = value.extract(py)?;
                    let bytes = uint_value.to_le_bytes();
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::UInt64 => {
                    let uint_value: u64 = value.extract(py)?;
                    let bytes = uint_value.to_le_bytes();
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
                BasicType::Pointer => {
                    let ptr_value: usize = value.extract(py)?;
                    let bytes = ptr_value.to_le_bytes();
                    memory_manager
                        .write_memory(self.address, &bytes)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Memory write error: {}",
                                e
                            ))
                        })?;
                }
            }
            Ok(())
        })
    }

    /// Get the memory address of this pointer
    #[getter]
    pub fn address(&self) -> usize {
        self.address
    }

    /// Get the type name of this pointer
    #[getter]
    pub fn type_name(&self) -> String {
        match &self.basic_type {
            BasicType::Int32 => "int32".to_string(),
            BasicType::Int64 => "int64".to_string(),
            BasicType::Float32 => "float32".to_string(),
            BasicType::Float64 => "float64".to_string(),
            BasicType::String => "string".to_string(),
            BasicType::Bool => "bool".to_string(),
            BasicType::UInt32 => "uint32".to_string(),
            BasicType::UInt64 => "uint64".to_string(),
            BasicType::Pointer => "pointer".to_string(),
        }
    }

    /// Get the TypeSpec for this pointer
    #[getter]
    pub fn type_spec(&self) -> TypeSpec {
        TypeSpec::from(self.basic_type.clone())
    }

    /// String representation for debugging
    fn __repr__(&self) -> String {
        format!(
            "Pointer(address=0x{:X}, type={})",
            self.address,
            self.type_name()
        )
    }

    /// String representation
    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Serialization utilities for basic types
pub struct TypeSerializer;

impl TypeSerializer {
    /// Serialize a Python object to bytes based on the specified type
    pub fn serialize_to_bytes(value: PyObject, basic_type: &BasicType) -> PyResult<Vec<u8>> {
        Python::with_gil(|py| {
            match basic_type {
                BasicType::Int32 => {
                    let int_value: i32 = value.extract(py)?;
                    Ok(int_value.to_le_bytes().to_vec())
                }
                BasicType::Int64 => {
                    let int_value: i64 = value.extract(py)?;
                    Ok(int_value.to_le_bytes().to_vec())
                }
                BasicType::Float32 => {
                    let float_value: f32 = value.extract(py)?;
                    Ok(float_value.to_le_bytes().to_vec())
                }
                BasicType::Float64 => {
                    let float_value: f64 = value.extract(py)?;
                    Ok(float_value.to_le_bytes().to_vec())
                }
                BasicType::String => {
                    let string_value: String = value.extract(py)?;
                    let mut bytes = string_value.into_bytes();
                    bytes.push(0); // Add null terminator
                    Ok(bytes)
                }
                BasicType::Bool => {
                    let bool_value: bool = value.extract(py)?;
                    Ok(vec![if bool_value { 1u8 } else { 0u8 }])
                }
                BasicType::UInt32 => {
                    let uint_value: u32 = value.extract(py)?;
                    Ok(uint_value.to_le_bytes().to_vec())
                }
                BasicType::UInt64 => {
                    let uint_value: u64 = value.extract(py)?;
                    Ok(uint_value.to_le_bytes().to_vec())
                }
                BasicType::Pointer => {
                    let ptr_value: usize = value.extract(py)?;
                    Ok(ptr_value.to_le_bytes().to_vec())
                }
            }
        })
    }

    /// Deserialize bytes to a Python object based on the specified type
    pub fn deserialize_from_bytes(bytes: &[u8], basic_type: &BasicType) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            match basic_type {
                BasicType::Int32 => {
                    if bytes.len() < 4 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for int32",
                        ));
                    }
                    let value = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    Ok(value.into_pyobject(py)?.into())
                }
                BasicType::Int64 => {
                    if bytes.len() < 8 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for int64",
                        ));
                    }
                    let value = i64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    Ok(value.into_pyobject(py)?.into())
                }
                BasicType::Float32 => {
                    if bytes.len() < 4 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for float32",
                        ));
                    }
                    let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    Ok(value.into_pyobject(py)?.into())
                }
                BasicType::Float64 => {
                    if bytes.len() < 8 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for float64",
                        ));
                    }
                    let value = f64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    Ok(value.into_pyobject(py)?.into())
                }
                BasicType::String => {
                    // Find null terminator
                    let null_pos = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
                    let string_bytes = &bytes[..null_pos];
                    let value = String::from_utf8_lossy(string_bytes).to_string();
                    Ok(value.into_pyobject(py)?.into())
                }
                BasicType::Bool => {
                    if bytes.is_empty() {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for bool",
                        ));
                    }
                    let value = bytes[0] != 0;
                    let py_bool = value.into_pyobject(py)?;
                    Ok(py_bool.as_any().clone().unbind())
                }
                BasicType::UInt32 => {
                    if bytes.len() < 4 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for uint32",
                        ));
                    }
                    let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    Ok(value.into_pyobject(py)?.into())
                }
                BasicType::UInt64 => {
                    if bytes.len() < 8 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for uint64",
                        ));
                    }
                    let value = u64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    Ok(value.into_pyobject(py)?.into())
                }
                BasicType::Pointer => {
                    if bytes.len() < 8 {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Insufficient bytes for pointer",
                        ));
                    }
                    let value = usize::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    Ok(value.into_pyobject(py)?.into())
                }
            }
        })
    }

    /// Get the size in bytes for a basic type
    pub fn get_type_size(basic_type: &BasicType) -> usize {
        match basic_type {
            BasicType::Int32 => 4,
            BasicType::Int64 => 8,
            BasicType::Float32 => 4,
            BasicType::Float64 => 8,
            BasicType::String => 0, // Variable size - handled specially
            BasicType::Bool => 1,
            BasicType::UInt32 => 4,
            BasicType::UInt64 => 8,
            BasicType::Pointer => 8, // 64-bit pointer
        }
    }
}
/// Python base class for structure definitions
#[pyclass(name = "Structure")]
pub struct PyStructure {
    #[pyo3(get)]
    pub name: String,
}

#[pymethods]
impl PyStructure {
    #[new]
    pub fn new() -> Self {
        PyStructure {
            name: "Structure".to_string(),
        }
    }
}

impl PyStructure {
    /// Create a structure definition from a Python class
    pub fn create_definition_from_class(py_class: &Bound<'_, PyType>) -> PyResult<StructureDefinition> {
        let class_name = py_class.name()?.to_string();
        let mut fields = Vec::new();
        let mut current_offset = 0;

        // Get the class dictionary to inspect field annotations
        if let Ok(class_dict) = py_class.getattr("__dict__") {
            let dict = class_dict.downcast::<PyDict>()?;
            
            // Look for __annotations__ which contains type hints
            if let Ok(annotations) = dict.get_item("__annotations__") {
                if let Some(annotations_dict) = annotations {
                    let annotations = annotations_dict.downcast::<PyDict>()?;
                    
                    for (field_name, field_type) in annotations.iter() {
                        let name = field_name.extract::<String>()?;
                        let type_info = parse_field_type(&field_type)?;
                        let size = calculate_field_size(&type_info)?;
                        
                        fields.push(FieldDefinition {
                            name,
                            field_type: type_info,
                            offset: current_offset,
                            size,
                        });
                        
                        current_offset += size;
                    }
                }
            }
        }

        Ok(StructureDefinition {
            name: class_name,
            fields,
            total_size: current_offset,
        })
    }

    /// Calculate the size of a field type
    pub fn calculate_field_size(field_type: &FieldType) -> PyResult<usize> {
        calculate_field_size(field_type)
    }
}

/// Parse a Python type annotation into a FieldType
fn parse_field_type(py_type: &Bound<'_, PyAny>) -> PyResult<FieldType> {
    // Try to extract TypeSpec enum first (preferred method)
    if let Ok(type_spec) = py_type.extract::<TypeSpec>() {
        return Ok(FieldType::Basic(BasicType::from(type_spec)));
    }

    // Try to extract as string (backward compatibility)
    if let Ok(type_str) = py_type.extract::<String>() {
        return match type_str.as_str() {
            "int" | "i32" | "int32" => Ok(FieldType::Basic(BasicType::Int32)),
            "int64" | "i64" => Ok(FieldType::Basic(BasicType::Int64)),
            "float" | "f32" | "float32" => Ok(FieldType::Basic(BasicType::Float32)),
            "float64" | "f64" => Ok(FieldType::Basic(BasicType::Float64)),
            "str" | "string" => Ok(FieldType::Basic(BasicType::String)),
            "bool" | "boolean" => Ok(FieldType::Basic(BasicType::Bool)),
            "uint32" | "u32" => Ok(FieldType::Basic(BasicType::UInt32)),
            "uint64" | "u64" => Ok(FieldType::Basic(BasicType::UInt64)),
            "pointer" | "ptr" => Ok(FieldType::Basic(BasicType::Pointer)),
            _ => Ok(FieldType::Structure(type_str)), // Assume it's a custom structure
        };
    }

    // Try to handle type objects
    if let Ok(type_name) = py_type.getattr("__name__") {
        let name = type_name.extract::<String>()?;
        return match name.as_str() {
            "int" => Ok(FieldType::Basic(BasicType::Int32)),
            "float" => Ok(FieldType::Basic(BasicType::Float64)),
            "str" => Ok(FieldType::Basic(BasicType::String)),
            "bool" => Ok(FieldType::Basic(BasicType::Bool)),
            _ => Ok(FieldType::Structure(name)),
        };
    }

    // Default to treating as structure name
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
        "Unable to parse field type. Use TypeSpec enum for type-safe field definitions."
    ))
}

/// Calculate the size of a field type
fn calculate_field_size(field_type: &FieldType) -> PyResult<usize> {
    match field_type {
        FieldType::Basic(basic_type) => Ok(TypeSerializer::get_type_size(basic_type)),
        FieldType::Structure(_) => {
            // For structures, we'll need to look up the definition
            // For now, assume pointer size (8 bytes on x64)
            // TODO: In a full implementation, we would look up the actual structure size
            Ok(8)
        }
        FieldType::Array(element_type, count) => {
            let element_size = calculate_field_size(element_type)?;
            Ok(element_size * count)
        }
    }
}

/// Calculate the size of a field type with structure registry access
fn calculate_field_size_with_registry(
    field_type: &FieldType, 
    registry: &HashMap<String, StructureDefinition>
) -> PyResult<usize> {
    match field_type {
        FieldType::Basic(basic_type) => Ok(TypeSerializer::get_type_size(basic_type)),
        FieldType::Structure(struct_name) => {
            if let Some(struct_def) = registry.get(struct_name) {
                Ok(struct_def.total_size)
            } else {
                // If structure not found in registry, assume pointer size
                Ok(8)
            }
        }
        FieldType::Array(element_type, count) => {
            let element_size = calculate_field_size_with_registry(element_type, registry)?;
            Ok(element_size * count)
        }
    }
}

/// Python wrapper for structure pointers with dynamic field access
#[pyclass(name = "StructurePointer")]
#[derive(Clone)]
pub struct PyStructurePointer {
    address: usize,
    structure_def: StructureDefinition,
    memory_manager: Arc<Mutex<MemoryManager>>,
    structure_registry: Arc<Mutex<HashMap<String, StructureDefinition>>>,
}

#[pymethods]
impl PyStructurePointer {
    /// Create a new structure pointer
    #[new]
    pub fn new(address: usize, structure_class: &Bound<'_, PyType>) -> PyResult<Self> {
        // Create structure definition from the Python class
        let structure_def = PyStructure::create_definition_from_class(structure_class)?;
        
        // Get the memory manager from the global Universe core
        let memory_manager = get_global_memory_manager()?;

        let structure_registry = Arc::new(Mutex::new(HashMap::new()));

        Ok(PyStructurePointer {
            address,
            structure_def,
            memory_manager,
            structure_registry,
        })
    }

    /// Dynamic attribute getter for structure fields
    fn __getattr__(&self, name: &str) -> PyResult<PyObject> {
        // Find the field definition
        let field = self.structure_def.fields.iter()
            .find(|f| f.name == name)
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyAttributeError, _>(format!(
                    "Structure '{}' has no field '{}'",
                    self.structure_def.name, name
                ))
            })?;

        // Calculate the field address
        let field_address = self.address + field.offset;

        // Read and deserialize the field value
        self.read_field_value(field_address, &field.field_type)
    }

    /// Dynamic attribute setter for structure fields
    fn __setattr__(&self, name: &str, value: PyObject) -> PyResult<()> {
        // Find the field definition
        let field = self.structure_def.fields.iter()
            .find(|f| f.name == name)
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyAttributeError, _>(format!(
                    "Structure '{}' has no field '{}'",
                    self.structure_def.name, name
                ))
            })?;

        // Validate the assignment before writing
        self.validate_structure_assignment(&field.field_type, &value)?;

        // Calculate the field address
        let field_address = self.address + field.offset;

        // Serialize and write the field value
        self.write_field_value(field_address, &field.field_type, value)
    }

    /// Get the memory address of this structure pointer
    #[getter]
    pub fn address(&self) -> usize {
        self.address
    }

    /// Get the structure name
    #[getter]
    pub fn structure_name(&self) -> String {
        self.structure_def.name.clone()
    }

    /// Get the total size of the structure
    #[getter]
    pub fn size(&self) -> usize {
        self.structure_def.total_size
    }

    /// String representation for debugging
    fn __repr__(&self) -> String {
        format!(
            "StructurePointer(address=0x{:X}, type={})",
            self.address,
            self.structure_def.name
        )
    }

    /// String representation
    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Copy this entire structure to another address
    pub fn copy_to(&self, dest_address: usize) -> PyResult<()> {
        let memory_manager = self.memory_manager.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire memory manager lock"
            )
        })?;

        // Read the entire structure data
        let source_data = memory_manager.read_memory(self.address, self.structure_def.total_size)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to read source structure data: {}", e
                ))
            })?;

        // Write the data to the destination
        memory_manager.write_memory(dest_address, &source_data)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to write structure data: {}", e
                ))
            })?;

        Ok(())
    }

    /// Copy another structure to this address
    pub fn copy_from(&self, source: &PyStructurePointer) -> PyResult<()> {
        // Validate that the structure types match
        if source.structure_def.name != self.structure_def.name {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Structure type mismatch: expected '{}', got '{}'",
                self.structure_def.name, source.structure_def.name
            )));
        }

        source.copy_to(self.address)
    }

    /// Get all field values as a dictionary
    pub fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            
            for field in &self.structure_def.fields {
                let field_address = self.address + field.offset;
                let field_value = self.read_field_value(field_address, &field.field_type)?;
                dict.set_item(&field.name, field_value)?;
            }
            
            Ok(dict.into())
        })
    }

    /// Set multiple field values from a dictionary
    pub fn from_dict(&self, dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        self.write_structure_from_dict(self.address, &self.structure_def.name, dict)
    }

    /// Get a list of all field names
    pub fn get_field_names(&self) -> Vec<String> {
        self.structure_def.fields.iter().map(|f| f.name.clone()).collect()
    }

    /// Get field information (name, type, offset, size)
    pub fn get_field_info(&self, field_name: &str) -> PyResult<PyObject> {
        let field = self.structure_def.fields.iter()
            .find(|f| f.name == field_name)
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyAttributeError, _>(format!(
                    "Structure '{}' has no field '{}'",
                    self.structure_def.name, field_name
                ))
            })?;

        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("name", &field.name)?;
            dict.set_item("offset", field.offset)?;
            dict.set_item("size", field.size)?;
            
            let type_name = match &field.field_type {
                FieldType::Basic(basic_type) => {
                    match basic_type {
                        BasicType::Int32 => "int32",
                        BasicType::Int64 => "int64",
                        BasicType::Float32 => "float32",
                        BasicType::Float64 => "float64",
                        BasicType::String => "string",
                        BasicType::Bool => "bool",
                        BasicType::UInt32 => "uint32",
                        BasicType::UInt64 => "uint64",
                        BasicType::Pointer => "pointer",
                    }
                }
                FieldType::Structure(name) => name.as_str(),
                FieldType::Array(element_type, count) => {
                    // For arrays, create a descriptive string
                    let element_name = match element_type.as_ref() {
                        FieldType::Basic(basic_type) => {
                            match basic_type {
                                BasicType::Int32 => "int32",
                                BasicType::Int64 => "int64",
                                BasicType::Float32 => "float32",
                                BasicType::Float64 => "float64",
                                BasicType::String => "string",
                                BasicType::Bool => "bool",
                                BasicType::UInt32 => "uint32",
                                BasicType::UInt64 => "uint64",
                                BasicType::Pointer => "pointer",
                            }
                        }
                        FieldType::Structure(name) => name.as_str(),
                        FieldType::Array(_, _) => "nested_array", // Simplified for nested arrays
                    };
                    dict.set_item("array_element_type", element_name)?;
                    dict.set_item("array_count", *count)?;
                    "array"
                }
            };
            
            dict.set_item("type", type_name)?;
            Ok(dict.into())
        })
    }
}

impl PyStructurePointer {
    /// Read a field value from memory and convert to Python object
    fn read_field_value(&self, address: usize, field_type: &FieldType) -> PyResult<PyObject> {
        let memory_manager = self.memory_manager.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire memory manager lock"
            )
        })?;

        match field_type {
            FieldType::Basic(basic_type) => {
                let size = TypeSerializer::get_type_size(basic_type);
                let bytes = memory_manager.read_memory(address, size).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory read error: {}",
                        e
                    ))
                })?;
                TypeSerializer::deserialize_from_bytes(&bytes, basic_type)
            }
            FieldType::Structure(struct_name) => {
                // For structure fields, return a new structure pointer
                let registry = self.structure_registry.lock().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "Failed to acquire structure registry lock"
                    )
                })?;
                
                if let Some(struct_def) = registry.get(struct_name) {
                    let struct_pointer = PyStructurePointer {
                        address,
                        structure_def: struct_def.clone(),
                        memory_manager: Arc::clone(&self.memory_manager),
                        structure_registry: Arc::clone(&self.structure_registry),
                    };
                    Python::with_gil(|py| Ok(struct_pointer.into_pyobject(py)?.into()))
                } else {
                    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unknown structure type: {}",
                        struct_name
                    )))
                }
            }
            FieldType::Array(element_type, count) => {
                // For arrays, return a list of values
                let mut result = Vec::new();
                let element_size = self.calculate_field_size(element_type)?;
                
                for i in 0..*count {
                    let element_address = address + (i * element_size);
                    let element_value = self.read_field_value(element_address, element_type)?;
                    result.push(element_value);
                }
                
                Python::with_gil(|py| Ok(result.into_pyobject(py)?.into()))
            }
        }
    }

    /// Write a field value to memory after serializing from Python object
    fn write_field_value(&self, address: usize, field_type: &FieldType, value: PyObject) -> PyResult<()> {
        let memory_manager = self.memory_manager.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire memory manager lock"
            )
        })?;

        match field_type {
            FieldType::Basic(basic_type) => {
                let bytes = TypeSerializer::serialize_to_bytes(value, basic_type)?;
                memory_manager.write_memory(address, &bytes).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Memory write error: {}",
                        e
                    ))
                })?;
            }
            FieldType::Structure(struct_name) => {
                // For structure fields, we support multiple input types:
                // 1. Another structure pointer of the same type (copy entire structure)
                // 2. A dictionary with field values
                // 3. A Python object with matching attributes
                self.write_structure_field(address, struct_name, value)?;
            }
            FieldType::Array(element_type, count) => {
                // For arrays, expect a list of values
                Python::with_gil(|py| {
                    let list = value.extract::<Vec<PyObject>>(py)?;
                    if list.len() != *count {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Array length mismatch: expected {}, got {}",
                            count, list.len()
                        )));
                    }

                    let element_size = self.calculate_field_size(element_type)?;
                    for (i, element_value) in list.iter().enumerate() {
                        let element_address = address + (i * element_size);
                        self.write_field_value(element_address, element_type, element_value.clone_ref(py))?;
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    /// Calculate the size of a field type (helper method)
    fn calculate_field_size(&self, field_type: &FieldType) -> PyResult<usize> {
        PyStructure::calculate_field_size(field_type)
    }

    /// Write a structure field value, supporting multiple input types
    fn write_structure_field(&self, address: usize, struct_name: &str, value: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            // Try to extract as another structure pointer first
            if let Ok(other_struct) = value.extract::<PyStructurePointer>(py) {
                // Validate that the structure types match
                if other_struct.structure_def.name != *struct_name {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Structure type mismatch: expected '{}', got '{}'",
                        struct_name, other_struct.structure_def.name
                    )));
                }
                
                // Copy the entire structure data from source to destination
                return self.copy_structure_data(&other_struct, address);
            }

            // Try to extract as a dictionary
            if let Ok(dict) = value.downcast_bound::<pyo3::types::PyDict>(py) {
                return self.write_structure_from_dict(address, struct_name, dict);
            }

            // Try to extract as an object with attributes
            self.write_structure_from_object(address, struct_name, &value.bind(py))
        })
    }

    /// Copy structure data from another structure pointer
    fn copy_structure_data(&self, source: &PyStructurePointer, dest_address: usize) -> PyResult<()> {
        let memory_manager = self.memory_manager.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire memory manager lock"
            )
        })?;

        // Read the entire structure data from source
        let source_data = memory_manager.read_memory(source.address, source.structure_def.total_size)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to read source structure data: {}", e
                ))
            })?;

        // Write the data to the destination
        memory_manager.write_memory(dest_address, &source_data)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to write structure data: {}", e
                ))
            })?;

        Ok(())
    }

    /// Write structure fields from a Python dictionary
    fn write_structure_from_dict(&self, address: usize, struct_name: &str, dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        // Get the structure definition
        let registry = self.structure_registry.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire structure registry lock"
            )
        })?;
        
        let struct_def = registry.get(struct_name).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown structure type: {}", struct_name
            ))
        })?.clone();
        
        drop(registry); // Release the lock

        // Write each field from the dictionary
        for (key, value) in dict.iter() {
            let field_name = key.extract::<String>()?;
            
            // Find the field definition
            if let Some(field) = struct_def.fields.iter().find(|f| f.name == field_name) {
                let field_address = address + field.offset;
                self.write_field_value(field_address, &field.field_type, value.unbind())?;
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(format!(
                    "Structure '{}' has no field '{}'", struct_name, field_name
                )));
            }
        }

        Ok(())
    }

    /// Write structure fields from a Python object with attributes
    fn write_structure_from_object(&self, address: usize, struct_name: &str, obj: &Bound<'_, PyAny>) -> PyResult<()> {
        // Get the structure definition
        let registry = self.structure_registry.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Failed to acquire structure registry lock"
            )
        })?;
        
        let struct_def = registry.get(struct_name).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown structure type: {}", struct_name
            ))
        })?.clone();
        
        drop(registry); // Release the lock

        // Try to read each field from the object
        for field in &struct_def.fields {
            if let Ok(field_value) = obj.getattr(&field.name) {
                let field_address = address + field.offset;
                self.write_field_value(field_address, &field.field_type, field_value.unbind())?;
            }
            // If the field doesn't exist on the object, we skip it (partial assignment)
        }

        Ok(())
    }

    /// Validate structure field assignment
    fn validate_structure_assignment(&self, field_type: &FieldType, value: &PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            match field_type {
                FieldType::Basic(basic_type) => {
                    // Validate that the value can be converted to the expected basic type
                    match basic_type {
                        BasicType::Int32 => {
                            value.extract::<i32>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected int32 value"
                                )
                            })?;
                        }
                        BasicType::Int64 => {
                            value.extract::<i64>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected int64 value"
                                )
                            })?;
                        }
                        BasicType::Float32 => {
                            value.extract::<f32>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected float32 value"
                                )
                            })?;
                        }
                        BasicType::Float64 => {
                            value.extract::<f64>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected float64 value"
                                )
                            })?;
                        }
                        BasicType::String => {
                            value.extract::<String>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected string value"
                                )
                            })?;
                        }
                        BasicType::Bool => {
                            value.extract::<bool>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected bool value"
                                )
                            })?;
                        }
                        BasicType::UInt32 => {
                            value.extract::<u32>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected uint32 value"
                                )
                            })?;
                        }
                        BasicType::UInt64 => {
                            value.extract::<u64>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected uint64 value"
                                )
                            })?;
                        }
                        BasicType::Pointer => {
                            value.extract::<usize>(py).map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Expected pointer/address value"
                                )
                            })?;
                        }
                    }
                }
                FieldType::Structure(_) => {
                    // For structures, we accept structure pointers, dicts, or objects with attributes
                    // The validation is done in write_structure_field
                }
                FieldType::Array(element_type, expected_count) => {
                    // For arrays, validate that we have a list of the correct length
                    let list = value.extract::<Vec<PyObject>>(py).map_err(|_| {
                        PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                            "Expected list for array field"
                        )
                    })?;
                    
                    if list.len() != *expected_count {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Array length mismatch: expected {}, got {}",
                            expected_count, list.len()
                        )));
                    }

                    // Validate each element
                    for element in &list {
                        self.validate_structure_assignment(element_type, element)?;
                    }
                }
            }
            Ok(())
        })
    }
}

/// Comprehensive structure serialization system
pub struct StructureSerializer;

impl StructureSerializer {
    /// Serialize a complete structure to bytes
    pub fn serialize_structure(
        structure_def: &StructureDefinition,
        values: &std::collections::HashMap<String, PyObject>,
        registry: &HashMap<String, StructureDefinition>
    ) -> PyResult<Vec<u8>> {
        let mut buffer = vec![0u8; structure_def.total_size];
        
        for field in &structure_def.fields {
            if let Some(value) = values.get(&field.name) {
                let field_bytes = Self::serialize_field_value(&field.field_type, value, registry)?;
                
                // Ensure we don't write beyond the field boundaries
                let end_offset = field.offset + field.size;
                if end_offset <= buffer.len() {
                    let copy_size = field_bytes.len().min(field.size);
                    buffer[field.offset..field.offset + copy_size].copy_from_slice(&field_bytes[..copy_size]);
                }
            }
        }
        
        Ok(buffer)
    }
    
    /// Serialize a single field value to bytes
    pub fn serialize_field_value(
        field_type: &FieldType,
        value: &PyObject,
        registry: &HashMap<String, StructureDefinition>
    ) -> PyResult<Vec<u8>> {
        match field_type {
            FieldType::Basic(basic_type) => {
                Python::with_gil(|py| {
                    TypeSerializer::serialize_to_bytes(value.clone_ref(py), basic_type)
                })
            }
            FieldType::Structure(struct_name) => {
                // For nested structures, we need to serialize the entire structure
                if let Some(struct_def) = registry.get(struct_name) {
                    Python::with_gil(|py| {
                        // Try to extract as structure pointer
                        if let Ok(struct_ptr) = value.extract::<PyStructurePointer>(py) {
                            // Read the structure data directly
                            let memory_manager = struct_ptr.memory_manager.lock().map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                                    "Failed to acquire memory manager lock"
                                )
                            })?;
                            
                            return memory_manager.read_memory(struct_ptr.address, struct_def.total_size)
                                .map_err(|e| {
                                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                        "Failed to read structure data: {}", e
                                    ))
                                });
                        }
                        
                        // Try to extract as dictionary
                        if let Ok(dict) = value.downcast_bound::<pyo3::types::PyDict>(py) {
                            let mut field_values = std::collections::HashMap::new();
                            for (key, val) in dict.iter() {
                                let field_name = key.extract::<String>()?;
                                field_values.insert(field_name, val.unbind());
                            }
                            return Self::serialize_structure(struct_def, &field_values, registry);
                        }
                        
                        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                            "Structure field value must be a structure pointer or dictionary"
                        ))
                    })
                } else {
                    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unknown structure type: {}", struct_name
                    )))
                }
            }
            FieldType::Array(element_type, count) => {
                Python::with_gil(|py| {
                    let list = value.extract::<Vec<PyObject>>(py)?;
                    if list.len() != *count {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Array length mismatch: expected {}, got {}",
                            count, list.len()
                        )));
                    }
                    
                    let mut result = Vec::new();
                    for element in &list {
                        let element_bytes = Self::serialize_field_value(element_type, element, registry)?;
                        result.extend_from_slice(&element_bytes);
                    }
                    
                    Ok(result)
                })
            }
        }
    }
    
    /// Deserialize a complete structure from bytes
    pub fn deserialize_structure(
        structure_def: &StructureDefinition,
        data: &[u8],
        registry: &HashMap<String, StructureDefinition>
    ) -> PyResult<std::collections::HashMap<String, PyObject>> {
        let mut result = std::collections::HashMap::new();
        
        for field in &structure_def.fields {
            if field.offset + field.size <= data.len() {
                let field_data = &data[field.offset..field.offset + field.size];
                let field_value = Self::deserialize_field_value(&field.field_type, field_data, registry)?;
                result.insert(field.name.clone(), field_value);
            }
        }
        
        Ok(result)
    }
    
    /// Deserialize a single field value from bytes
    pub fn deserialize_field_value(
        field_type: &FieldType,
        data: &[u8],
        registry: &HashMap<String, StructureDefinition>
    ) -> PyResult<PyObject> {
        match field_type {
            FieldType::Basic(basic_type) => {
                TypeSerializer::deserialize_from_bytes(data, basic_type)
            }
            FieldType::Structure(struct_name) => {
                if let Some(struct_def) = registry.get(struct_name) {
                    let field_values = Self::deserialize_structure(struct_def, data, registry)?;
                    
                    Python::with_gil(|py| {
                        let dict = pyo3::types::PyDict::new(py);
                        for (key, value) in field_values {
                            dict.set_item(key, value)?;
                        }
                        Ok(dict.into())
                    })
                } else {
                    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Unknown structure type: {}", struct_name
                    )))
                }
            }
            FieldType::Array(element_type, count) => {
                let element_size = calculate_field_size_with_registry(element_type, registry)?;
                let mut result = Vec::new();
                
                for i in 0..*count {
                    let start = i * element_size;
                    let end = start + element_size;
                    if end <= data.len() {
                        let element_data = &data[start..end];
                        let element_value = Self::deserialize_field_value(element_type, element_data, registry)?;
                        result.push(element_value);
                    }
                }
                
                Python::with_gil(|py| Ok(result.into_pyobject(py)?.into()))
            }
        }
    }
}

/// Enhanced Pointer creation function that handles TypeSpec enums, basic types and structures
#[pyfunction]
pub fn create_pointer(address: usize, type_spec: &Bound<'_, PyAny>) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        // Check if it's a TypeSpec enum (preferred method)
        if let Ok(_type_spec_enum) = type_spec.extract::<TypeSpec>() {
            let basic_pointer = PyBasicPointer::new(address, type_spec)?;
            return Ok(basic_pointer.into_pyobject(py)?.into());
        }

        // Check if it's a string (basic type - backward compatibility)
        if let Ok(_type_name) = type_spec.extract::<String>() {
            let basic_pointer = PyBasicPointer::new(address, type_spec)?;
            return Ok(basic_pointer.into_pyobject(py)?.into());
        }

        // Check if it's a type object (structure class)
        if let Ok(type_obj) = type_spec.downcast::<PyType>() {
            let struct_pointer = PyStructurePointer::new(address, type_obj)?;
            return Ok(struct_pointer.into_pyobject(py)?.into());
        }

        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Type specification must be a TypeSpec enum, string (for basic types), or a class (for structures)"
        ))
    })
}