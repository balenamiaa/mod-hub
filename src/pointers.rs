use crate::memory::MemoryManager;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
        // This is a placeholder - in the full implementation, this would get the memory manager
        // from the global Universe core. For now, we'll create a temporary one.
        let memory_manager = Arc::new(Mutex::new(MemoryManager::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create memory manager: {}",
                e
            ))
        })?));

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
            Ok(8)
        }
        FieldType::Array(element_type, count) => {
            let element_size = calculate_field_size(element_type)?;
            Ok(element_size * count)
        }
    }
}

/// Python wrapper for structure pointers with dynamic field access
#[pyclass(name = "StructurePointer")]
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
        
        // This is a placeholder - in the full implementation, this would get the memory manager
        // from the global Universe core. For now, we'll create a temporary one.
        let memory_manager = Arc::new(Mutex::new(MemoryManager::new().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create memory manager: {}",
                e
            ))
        })?));

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
            FieldType::Structure(_) => {
                // For structure fields, we expect the value to be another structure pointer
                // or we could copy the entire structure data
                return Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
                    "Writing structure fields not yet implemented"
                ));
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
}

/// Enhanced Pointer creation function that handles TypeSpec enums, basic types and structures
#[pyfunction]
pub fn create_pointer(address: usize, type_spec: &Bound<'_, PyAny>) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        // Check if it's a TypeSpec enum (preferred method)
        if let Ok(type_spec_enum) = type_spec.extract::<TypeSpec>() {
            let basic_pointer = PyBasicPointer::new(address, type_spec)?;
            return Ok(basic_pointer.into_pyobject(py)?.into());
        }

        // Check if it's a string (basic type - backward compatibility)
        if let Ok(type_name) = type_spec.extract::<String>() {
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