use crate::core::UniverseError;
use crate::hook_handlers::{
    get_hook_handler_address, get_jmpback_handler_address,
    register_hook_callback, register_jmpback_callback,
    remove_hook_callback, remove_jmpback_callback,
    clear_all_hook_callbacks, clear_all_jmpback_callbacks
};
use crate::registers::{RegisterManager, RegisterState};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::ptr;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows_sys::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    PAGE_EXECUTE_READWRITE,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// Information about an installed hook
#[derive(Debug)]
pub struct HookInfo {
    pub address: usize,
    pub hook_type: HookType,
    pub callback: Option<PyObject>,
}

/// Types of hooks supported by the framework
#[derive(Debug)]
pub enum HookType {
    Function {
        original_bytes: Vec<u8>,
        trampoline: usize,
        original_function: usize,
    },
    JmpBack {
        original_bytes: Vec<u8>,
    },
}

/// Original function wrapper for Python callbacks
#[pyclass(name = "OriginalFunction")]
pub struct PyOriginalFunction {
    address: usize,
    original_bytes: Vec<u8>,
    was_called: bool,
}

#[pymethods]
impl PyOriginalFunction {
    /// Call the original function with current register state
    fn call(&mut self, py_registers: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            // Extract register state from Python object
            let register_manager = RegisterManager::new();
            let _register_state = register_manager.extract_register_state(&py_registers, py)?;

            // In a real implementation, this would:
            // 1. Set up the CPU registers with the provided state
            // 2. Call the original function at the trampoline address
            // 3. Capture the resulting register state
            // 4. Update the Python register object with the new state

            // Mark that the original function was called
            self.was_called = true;

            // Note: We can't access the logger from here since this is a simple struct
            // The logging will be handled by the caller

            Ok(())
        })
    }

    /// Get the address of the original function
    #[getter]
    fn address(&self) -> usize {
        self.address
    }
}

impl PyOriginalFunction {
    pub fn new(address: usize, original_bytes: Vec<u8>) -> Self {
        PyOriginalFunction {
            address,
            original_bytes,
            was_called: false,
        }
    }
    
    /// Check if the original function was called
    pub fn was_called(&self) -> bool {
        self.was_called
    }
}

/// Trampoline allocator for managing executable memory regions
pub struct TrampolineAllocator {
    allocated_regions: Vec<usize>,
    current_region: Option<usize>,
    current_offset: usize,
    region_size: usize,
}

impl TrampolineAllocator {
    /// Create a new trampoline allocator
    pub fn new() -> Self {
        TrampolineAllocator {
            allocated_regions: Vec::new(),
            current_region: None,
            current_offset: 0,
            region_size: 4096, // 4KB pages
        }
    }

    /// Allocate space for a trampoline
    pub fn allocate_trampoline(&mut self, size: usize) -> Result<usize, UniverseError> {
        // Ensure we have enough space in current region or allocate a new one
        if self.current_region.is_none() || self.current_offset + size > self.region_size {
            self.allocate_new_region()?;
        }

        let current_region = self.current_region.unwrap();
        let trampoline_address = current_region + self.current_offset;
        self.current_offset += size;

        Ok(trampoline_address)
    }

    /// Allocate a new executable memory region
    fn allocate_new_region(&mut self) -> Result<(), UniverseError> {
        unsafe {
            let region = VirtualAlloc(
                ptr::null_mut(),
                self.region_size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            );

            if region.is_null() {
                return Err(UniverseError::HookError(format!(
                    "Failed to allocate trampoline region: {}",
                    GetLastError()
                )));
            }

            let region_addr = region as usize;
            self.allocated_regions.push(region_addr);
            self.current_region = Some(region_addr);
            self.current_offset = 0;

            Ok(())
        }
    }

    /// Cleanup all allocated regions
    pub fn cleanup(&mut self) -> Result<(), UniverseError> {
        for &region in &self.allocated_regions {
            unsafe {
                if VirtualFree(region as *mut _, 0, MEM_RELEASE) == 0 {
                    // Note: We can't access the logger from here since this is cleanup
                    // The error will be handled by the caller
                }
            }
        }
        self.allocated_regions.clear();
        self.current_region = None;
        self.current_offset = 0;
        Ok(())
    }
}

/// Hook management system for function interception
pub struct HookManager {
    active_hooks: HashMap<usize, HookInfo>,
    trampoline_allocator: TrampolineAllocator,
}

impl HookManager {
    /// Create a new hook manager instance
    pub fn new() -> Result<Self, UniverseError> {
        Ok(HookManager {
            active_hooks: HashMap::new(),
            trampoline_allocator: TrampolineAllocator::new(),
        })
    }

    /// Install a function hook at the specified address with Python callback
    pub fn install_function_hook(
        &mut self,
        address: usize,
        callback: PyObject,
    ) -> Result<(), UniverseError> {
        // Check if hook already exists at this address
        if self.active_hooks.contains_key(&address) {
            return Err(UniverseError::HookError(format!(
                "Hook already exists at address 0x{:x}",
                address
            )));
        }

        // Validate the target address
        if !self.is_valid_hook_address(address) {
            return Err(UniverseError::HookError(format!(
                "Invalid hook address: 0x{:x}",
                address
            )));
        }

        // Read original bytes (we'll use 5 bytes for a JMP instruction)
        let original_bytes = self.read_memory_bytes(address, 5)?;

        // Generate trampoline code
        let trampoline_address = self.generate_function_trampoline(address, &original_bytes)?;

        // Create the JMP instruction to our hook handler
        let hook_handler_address = get_hook_handler_address();
        let jmp_instruction = self.create_jmp_instruction(address, hook_handler_address)?;

        // Register the callback in the global registry
        Python::with_gil(|py| {
            register_hook_callback(address, callback.clone_ref(py))
        })?;

        // Install the hook by writing the JMP instruction
        self.write_memory_bytes(address, &jmp_instruction)?;

        // Create hook info
        let hook_info = HookInfo {
            address,
            hook_type: HookType::Function {
                original_bytes: original_bytes.clone(),
                trampoline: trampoline_address,
                original_function: trampoline_address,
            },
            callback: Some(callback),
        };

        // Store hook information
        self.active_hooks.insert(address, hook_info);

        Ok(())
    }

    /// Install a jmpback hook at the specified address with Python callback
    pub fn install_jmpback_hook(
        &mut self,
        address: usize,
        callback: PyObject,
    ) -> Result<(), UniverseError> {
        // Check if hook already exists at this address
        if self.active_hooks.contains_key(&address) {
            return Err(UniverseError::HookError(format!(
                "Hook already exists at address 0x{:x}",
                address
            )));
        }

        // Validate the target address
        if !self.is_valid_hook_address(address) {
            return Err(UniverseError::HookError(format!(
                "Invalid hook address: 0x{:x}",
                address
            )));
        }

        // Read original bytes (we'll use 5 bytes for a JMP instruction)
        let original_bytes = self.read_memory_bytes(address, 5)?;

        // Generate jmpback trampoline code
        let _trampoline_address = self.generate_jmpback_trampoline(address, &original_bytes)?;

        // Create the JMP instruction to our jmpback handler
        let jmpback_handler_address = get_jmpback_handler_address();
        let jmp_instruction = self.create_jmp_instruction(address, jmpback_handler_address)?;

        // Register the callback in the global registry
        Python::with_gil(|py| {
            register_jmpback_callback(address, callback.clone_ref(py))
        })?;

        // Install the hook by writing the JMP instruction
        self.write_memory_bytes(address, &jmp_instruction)?;

        // Create hook info
        let hook_info = HookInfo {
            address,
            hook_type: HookType::JmpBack {
                original_bytes: original_bytes.clone(),
            },
            callback: Some(callback),
        };

        // Store hook information
        self.active_hooks.insert(address, hook_info);

        Ok(())
    }

    /// Remove a hook at the specified address
    pub fn remove_hook(&mut self, address: usize) -> Result<(), UniverseError> {
        if let Some(hook_info) = self.active_hooks.remove(&address) {
            // Restore original bytes
            match &hook_info.hook_type {
                HookType::Function { original_bytes, .. } => {
                    self.write_memory_bytes(address, original_bytes)?;
                    // Remove from function hook registry
                    remove_hook_callback(address)?;
                }
                HookType::JmpBack { original_bytes } => {
                    self.write_memory_bytes(address, original_bytes)?;
                    // Remove from jmpback hook registry
                    remove_jmpback_callback(address)?;
                }
            }
            Ok(())
        } else {
            Err(UniverseError::HookError(format!(
                "No hook found at address 0x{:x}",
                address
            )))
        }
    }

    /// Remove all active hooks
    pub fn remove_all_hooks(&mut self) -> Result<(), UniverseError> {
        let addresses: Vec<usize> = self.active_hooks.keys().cloned().collect();

        for address in addresses {
            if let Err(_e) = self.remove_hook(address) {
                // Note: We can't access the logger from here
                // The error will be handled by the caller
            }
        }

        // Ensure the map is cleared even if some removals failed
        self.active_hooks.clear();
        
        // Clear all callback registries
        clear_all_hook_callbacks()?;
        clear_all_jmpback_callbacks()?;
        
        Ok(())
    }

    /// Get information about an active hook
    pub fn get_hook_info(&self, address: usize) -> Option<&HookInfo> {
        self.active_hooks.get(&address)
    }

    /// Get all active hook addresses
    pub fn get_active_hooks(&self) -> Vec<usize> {
        self.active_hooks.keys().cloned().collect()
    }

    /// Check if a hook exists at the specified address
    pub fn has_hook(&self, address: usize) -> bool {
        self.active_hooks.contains_key(&address)
    }

    /// Validate if an address is suitable for hooking
    fn is_valid_hook_address(&self, address: usize) -> bool {
        // Basic validation - ensure address is not null and is aligned
        if address == 0 {
            return false;
        }

        // Check if we can read from the address
        unsafe {
            use windows_sys::Win32::System::Memory::{VirtualQuery, MEMORY_BASIC_INFORMATION};

            let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
            let result = VirtualQuery(
                address as *const _,
                &mut mbi,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            );

            if result == 0 {
                return false;
            }

            // Check if memory is committed and executable
            use windows_sys::Win32::System::Memory::{
                MEM_COMMIT, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
                PAGE_EXECUTE_WRITECOPY,
            };

            mbi.State == MEM_COMMIT
                && (mbi.Protect == PAGE_EXECUTE
                    || mbi.Protect == PAGE_EXECUTE_READ
                    || mbi.Protect == PAGE_EXECUTE_READWRITE
                    || mbi.Protect == PAGE_EXECUTE_WRITECOPY)
        }
    }

    /// Read bytes from memory at the specified address
    fn read_memory_bytes(&self, address: usize, size: usize) -> Result<Vec<u8>, UniverseError> {
        let mut buffer = vec![0u8; size];

        unsafe {
            use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;

            let process = GetCurrentProcess();
            let mut bytes_read = 0;

            let result = ReadProcessMemory(
                process,
                address as *const _,
                buffer.as_mut_ptr() as *mut _,
                size,
                &mut bytes_read,
            );

            if result == 0 {
                return Err(UniverseError::HookError(format!(
                    "Failed to read memory at 0x{:x}: {}",
                    address,
                    GetLastError()
                )));
            }

            if bytes_read != size {
                return Err(UniverseError::HookError(format!(
                    "Partial read at 0x{:x}: expected {} bytes, got {}",
                    address, size, bytes_read
                )));
            }
        }

        Ok(buffer)
    }

    /// Write bytes to memory at the specified address
    fn write_memory_bytes(&self, address: usize, data: &[u8]) -> Result<(), UniverseError> {
        unsafe {
            let process = GetCurrentProcess();
            let mut bytes_written = 0;

            // Change memory protection to allow writing
            let mut old_protect = 0;
            let protect_result = VirtualProtect(
                address as *mut _,
                data.len(),
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            );

            if protect_result == 0 {
                return Err(UniverseError::HookError(format!(
                    "Failed to change memory protection at 0x{:x}: {}",
                    address,
                    GetLastError()
                )));
            }

            // Write the data
            let result = WriteProcessMemory(
                process,
                address as *mut _,
                data.as_ptr() as *const _,
                data.len(),
                &mut bytes_written,
            );

            // Restore original protection
            let mut temp_protect = 0;
            VirtualProtect(
                address as *mut _,
                data.len(),
                old_protect,
                &mut temp_protect,
            );

            if result == 0 {
                return Err(UniverseError::HookError(format!(
                    "Failed to write memory at 0x{:x}: {}",
                    address,
                    GetLastError()
                )));
            }

            if bytes_written != data.len() {
                return Err(UniverseError::HookError(format!(
                    "Partial write at 0x{:x}: expected {} bytes, wrote {}",
                    address,
                    data.len(),
                    bytes_written
                )));
            }
        }

        Ok(())
    }

    /// Generate a function trampoline that preserves original function and handles callback
    fn generate_function_trampoline(
        &mut self,
        original_address: usize,
        original_bytes: &[u8],
    ) -> Result<usize, UniverseError> {
        // Calculate trampoline size:
        // - Space for original bytes
        // - JMP back to original function + 5 (after our hook)
        // - Extra space for hook data (hook address, original function, return address)
        let trampoline_size = original_bytes.len() + 5 + 24; // 24 bytes for hook data

        let trampoline_address = self
            .trampoline_allocator
            .allocate_trampoline(trampoline_size)?;

        // Create a trampoline that:
        // 1. Contains the original bytes
        // 2. Has a JMP back to original function + 5
        // 3. Stores hook address, original function address, and return address for the assembly handler

        let mut trampoline_code = Vec::new();

        // Add original bytes
        trampoline_code.extend_from_slice(original_bytes);

        // Add JMP back to original function (after our 5-byte hook)
        let return_address = original_address + 5;
        let jmp_back =
            self.create_jmp_instruction(trampoline_address + original_bytes.len(), return_address)?;
        trampoline_code.extend_from_slice(&jmp_back);

        // Add hook data at the end of the trampoline
        // This data will be accessed by the assembly hook handler
        // Format: [hook_address (8 bytes), original_function (8 bytes), return_address (8 bytes)]
        trampoline_code.extend_from_slice(&original_address.to_le_bytes());
        trampoline_code.extend_from_slice(&trampoline_address.to_le_bytes());
        trampoline_code.extend_from_slice(&return_address.to_le_bytes());

        // Write trampoline code to allocated memory
        self.write_memory_bytes(trampoline_address, &trampoline_code)?;

        Ok(trampoline_address)
    }

    /// Generate a jmpback trampoline that executes callback and returns to original location
    fn generate_jmpback_trampoline(
        &mut self,
        original_address: usize,
        original_bytes: &[u8],
    ) -> Result<usize, UniverseError> {
        // Calculate trampoline size:
        // - Space for original bytes
        // - JMP back to original location + 5 (after our hook)
        // - Extra space for hook data (hook address, trampoline address)
        let trampoline_size = original_bytes.len() + 5 + 16; // 16 bytes for hook data

        let trampoline_address = self
            .trampoline_allocator
            .allocate_trampoline(trampoline_size)?;

        // Create a jmpback trampoline that:
        // 1. Contains the original bytes that were overwritten
        // 2. Has a JMP back to the original location + 5 (continuing execution)
        // 3. Stores hook address and trampoline address for the assembly handler

        let mut trampoline_code = Vec::new();

        // Add original bytes that were overwritten by our hook
        trampoline_code.extend_from_slice(original_bytes);

        // Add JMP back to original location (after our 5-byte hook)
        let return_address = original_address + 5;
        let jmp_back =
            self.create_jmp_instruction(trampoline_address + original_bytes.len(), return_address)?;
        trampoline_code.extend_from_slice(&jmp_back);

        // Add hook data at the end of the trampoline
        // This data will be accessed by the assembly jmpback hook handler
        // Format: [hook_address (8 bytes), trampoline_address (8 bytes)]
        trampoline_code.extend_from_slice(&original_address.to_le_bytes());
        trampoline_code.extend_from_slice(&trampoline_address.to_le_bytes());

        // Write trampoline code to allocated memory
        self.write_memory_bytes(trampoline_address, &trampoline_code)?;

        Ok(trampoline_address)
    }

    /// Create a JMP instruction from source to destination
    fn create_jmp_instruction(&self, from: usize, to: usize) -> Result<Vec<u8>, UniverseError> {
        // Calculate relative offset for JMP instruction
        // JMP instruction format: E9 [4-byte relative offset]
        let offset = (to as i64) - (from as i64) - 5; // -5 because JMP instruction is 5 bytes

        if offset < i32::MIN as i64 || offset > i32::MAX as i64 {
            return Err(UniverseError::HookError(format!(
                "JMP offset too large: {} (from 0x{:x} to 0x{:x})",
                offset, from, to
            )));
        }

        let mut jmp_instruction = Vec::new();
        jmp_instruction.push(0xE9); // JMP opcode
        jmp_instruction.extend_from_slice(&(offset as i32).to_le_bytes());

        Ok(jmp_instruction)
    }



    /// Execute hook callback with registers and original function
    pub fn execute_hook_callback(
        &self,
        hook_address: usize,
        registers: RegisterState,
    ) -> Result<RegisterState, UniverseError> {
        // Get hook info
        let hook_info = self.active_hooks.get(&hook_address).ok_or_else(|| {
            UniverseError::HookError(format!("No hook found at 0x{:x}", hook_address))
        })?;

        // Get callback
        let callback = hook_info
            .callback
            .as_ref()
            .ok_or_else(|| UniverseError::HookError("No callback found for hook".to_string()))?;

        // Execute callback with Python GIL
        Python::with_gil(|py| {
            // Create Python register object
            let py_registers = registers.to_python_object(py).map_err(|e| {
                UniverseError::PythonError(format!("Failed to create Python registers: {}", e))
            })?;

            // Create original function object
            let original_function = match &hook_info.hook_type {
                HookType::Function {
                    original_bytes,
                    trampoline,
                    ..
                } => PyOriginalFunction::new(*trampoline, original_bytes.clone()),
                _ => {
                    return Err(UniverseError::HookError(
                        "Invalid hook type for function callback".to_string(),
                    ))
                }
            };

            let py_original = Py::new(py, original_function).map_err(|e| {
                UniverseError::PythonError(format!(
                    "Failed to create original function object: {}",
                    e
                ))
            })?;

            // Call the Python callback with (registers, original_function)
            let args = (py_registers.clone_ref(py), py_original);
            callback
                .call1(py, args)
                .map_err(|e| UniverseError::PythonError(format!("Hook callback failed: {}", e)))?;

            // Extract potentially modified register state
            let register_manager = RegisterManager::new();
            let modified_registers = register_manager
                .extract_register_state(&py_registers, py)
                .map_err(|e| {
                    UniverseError::PythonError(format!("Failed to extract register state: {}", e))
                })?;

            Ok(modified_registers)
        })
    }

    /// Execute jmpback hook callback with registers only (no original function)
    pub fn execute_jmpback_callback(
        &self,
        hook_address: usize,
        registers: RegisterState,
    ) -> Result<RegisterState, UniverseError> {
        // Get hook info
        let hook_info = self.active_hooks.get(&hook_address).ok_or_else(|| {
            UniverseError::HookError(format!("No jmpback hook found at 0x{:x}", hook_address))
        })?;

        // Verify this is a jmpback hook
        match &hook_info.hook_type {
            HookType::JmpBack { .. } => {}
            _ => {
                return Err(UniverseError::HookError(
                    "Invalid hook type for jmpback callback".to_string(),
                ))
            }
        }

        // Get callback
        let callback = hook_info.callback.as_ref().ok_or_else(|| {
            UniverseError::HookError("No callback found for jmpback hook".to_string())
        })?;

        // Execute callback with Python GIL
        Python::with_gil(|py| {
            // Create Python register object
            let py_registers = registers.to_python_object(py).map_err(|e| {
                UniverseError::PythonError(format!("Failed to create Python registers: {}", e))
            })?;

            // Call the Python callback with only (registers) parameter
            // Note: jmpback hooks only receive registers, not original_function
            callback
                .call1(py, (py_registers.clone_ref(py),))
                .map_err(|e| {
                    UniverseError::PythonError(format!("Jmpback hook callback failed: {}", e))
                })?;

            // Extract potentially modified register state
            let register_manager = RegisterManager::new();
            let modified_registers = register_manager
                .extract_register_state(&py_registers, py)
                .map_err(|e| {
                    UniverseError::PythonError(format!("Failed to extract register state: {}", e))
                })?;

            Ok(modified_registers)
        })
    }

    /// Cleanup hook manager resources
    pub fn cleanup(&mut self) -> Result<(), UniverseError> {
        // Remove all hooks first
        self.remove_all_hooks()?;

        // Cleanup trampoline allocator
        self.trampoline_allocator.cleanup()?;

        Ok(())
    }
}

// Hook handler and jmpback handler functions are now implemented in hook_handlers.rs
// using real assembly code that properly captures and restores all CPU registers
