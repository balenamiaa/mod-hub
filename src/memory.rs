use crate::core::UniverseError;
use pyo3::pyclass;
use std::collections::HashMap;
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, HMODULE};
use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows_sys::Win32::System::Memory::{
    VirtualProtect, VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READWRITE,
    PAGE_GUARD, PAGE_NOACCESS, PAGE_READWRITE,
};
use windows_sys::Win32::System::ProcessStatus::{
    EnumProcessModules, GetModuleBaseNameA, GetModuleInformation, MODULEINFO,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// Information about a loaded module
#[derive(Debug, Clone)]
#[pyclass]
pub struct ModuleInfo {
    pub base_address: usize,
    pub size: usize,
}

/// Memory management system for safe game memory access
pub struct MemoryManager {
    process_handle: HANDLE,
    loaded_modules: HashMap<String, ModuleInfo>,
}

impl MemoryManager {
    /// Create a new memory manager instance
    pub fn new() -> Result<Self, UniverseError> {
        let process_handle = unsafe { GetCurrentProcess() };

        if process_handle == 0 {
            return Err(UniverseError::MemoryError(
                "Failed to get current process handle".to_string(),
            ));
        }

        let mut manager = MemoryManager {
            process_handle,
            loaded_modules: HashMap::new(),
        };

        // Enumerate modules on initialization
        manager.enumerate_modules()?;

        Ok(manager)
    }

    /// Enumerate all loaded modules in the current process
    pub fn enumerate_modules(&mut self) -> Result<(), UniverseError> {
        const MAX_MODULES: usize = 1024;
        let mut modules: [HMODULE; MAX_MODULES] = [0; MAX_MODULES];
        let mut bytes_needed: u32 = 0;

        let success = unsafe {
            EnumProcessModules(
                self.process_handle,
                modules.as_mut_ptr(),
                (MAX_MODULES * std::mem::size_of::<HMODULE>()) as u32,
                &mut bytes_needed,
            )
        };

        if success == 0 {
            let error_code = unsafe { GetLastError() };
            return Err(UniverseError::MemoryError(format!(
                "Failed to enumerate process modules: Windows error {}",
                error_code
            )));
        }

        let module_count = (bytes_needed as usize) / std::mem::size_of::<HMODULE>();

        // Clear existing modules
        self.loaded_modules.clear();

        for i in 0..module_count.min(MAX_MODULES) {
            let module_handle = modules[i];
            if module_handle == 0 {
                continue;
            }

            // Get module information
            let mut module_info: MODULEINFO = unsafe { std::mem::zeroed() };
            let info_success = unsafe {
                GetModuleInformation(
                    self.process_handle,
                    module_handle,
                    &mut module_info,
                    std::mem::size_of::<MODULEINFO>() as u32,
                )
            };

            if info_success == 0 {
                continue; // Skip this module if we can't get info
            }

            // Get module name
            let mut name_buffer = [0u8; 256];
            let name_length = unsafe {
                GetModuleBaseNameA(
                    self.process_handle,
                    module_handle,
                    name_buffer.as_mut_ptr(),
                    name_buffer.len() as u32,
                )
            };

            if name_length == 0 {
                continue; // Skip if we can't get the name
            }

            // Convert name to string
            let module_name = match std::str::from_utf8(&name_buffer[..name_length as usize]) {
                Ok(name) => name.to_string(),
                Err(_) => continue, // Skip if name is not valid UTF-8
            };

            // Store module information
            let info = ModuleInfo {
                base_address: module_info.lpBaseOfDll as usize,
                size: module_info.SizeOfImage as usize,
            };

            self.loaded_modules.insert(module_name, info);
        }

        Ok(())
    }

    /// Check if an address is valid for access using VirtualQuery
    pub fn is_valid_address(&self, address: usize) -> bool {
        let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };

        let result = unsafe {
            VirtualQuery(
                address as *const std::ffi::c_void,
                &mut mbi,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };

        if result == 0 {
            return false;
        }

        // Check if memory is committed and accessible
        if mbi.State != MEM_COMMIT {
            return false;
        }

        // Check if memory is not protected with PAGE_NOACCESS or PAGE_GUARD
        if (mbi.Protect & PAGE_NOACCESS) != 0 || (mbi.Protect & PAGE_GUARD) != 0 {
            return false;
        }

        true
    }

    /// Validate memory range for access
    fn validate_memory_range(&self, address: usize, size: usize) -> Result<(), UniverseError> {
        if size == 0 {
            return Err(UniverseError::MemoryError(
                "Size cannot be zero".to_string(),
            ));
        }

        // Check for potential overflow
        if address.checked_add(size).is_none() {
            return Err(UniverseError::MemoryError(
                "Address range overflow".to_string(),
            ));
        }

        // Validate the starting address
        if !self.is_valid_address(address) {
            return Err(UniverseError::MemoryError(format!(
                "Invalid memory address: 0x{:X}",
                address
            )));
        }

        // For larger ranges, also check the end address
        if size > 4096 {
            // Only check end for larger ranges to avoid performance impact
            let end_address = address + size - 1;
            if !self.is_valid_address(end_address) {
                return Err(UniverseError::MemoryError(format!(
                    "Invalid memory range end: 0x{:X}",
                    end_address
                )));
            }
        }

        Ok(())
    }

    /// Apply write protection to the specified memory range if needed.
    fn apply_write_protection(
        &self,
        address: usize,
        size: usize,
    ) -> Result<Option<u32>, UniverseError> {
        let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };

        let current_protect = unsafe {
            VirtualQuery(
                address as *const std::ffi::c_void,
                &mut mbi,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };

        if current_protect == 0 {
            return Err(UniverseError::MemoryError(
                "Failed to get current memory protection".to_string(),
            ));
        }

        let mut old_protect: u32 = 0;
        if mbi.Protect & PAGE_READWRITE == 0 {
            unsafe {
                VirtualProtect(
                    address as *mut std::ffi::c_void,
                    size,
                    PAGE_READWRITE,
                    &mut old_protect,
                );
            }

            Ok(Some(old_protect))
        } else {
            Ok(None)
        }
    }

    fn restore_write_protection(
        &self,
        address: usize,
        size: usize,
        old_protect: Option<u32>,
    ) -> Result<bool, UniverseError> {
        if let Some(mut old_protect) = old_protect {
            unsafe {
                VirtualProtect(
                    address as *mut std::ffi::c_void,
                    size,
                    old_protect,
                    &mut old_protect,
                );
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Read memory from the specified address with safety checks
    pub fn read_memory(
        &self,
        address: usize,
        size: usize,
        validate: bool,
    ) -> Result<Vec<u8>, UniverseError> {
        // Validate the memory range first
        if validate {
            self.validate_memory_range(address, size)?;
        }

        let mut buffer = vec![0u8; size];
        let mut bytes_read: usize = 0;

        unsafe {
            std::ptr::copy_nonoverlapping(
                address as *const std::ffi::c_void,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
            );
        }

        let success = unsafe {
            ReadProcessMemory(
                self.process_handle,
                address as *const std::ffi::c_void,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
                &mut bytes_read,
            )
        };

        Ok(buffer)
    }

    /// Write data to the specified memory address with safety checks
    pub fn write_memory(&self, address: usize, data: &[u8]) -> Result<(), UniverseError> {
        if data.is_empty() {
            return Err(UniverseError::MemoryError(
                "Cannot write empty data".to_string(),
            ));
        }

        let old_protect = self.apply_write_protection(address, data.len())?;

        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr() as *const std::ffi::c_void,
                address as *mut std::ffi::c_void,
                data.len(),
            );
        }

        let _ = self.restore_write_protection(address, data.len(), old_protect)?;

        Ok(())
    }

    pub fn list_modules(&self) -> &HashMap<String, ModuleInfo> {
        &self.loaded_modules
    }

    /// Scan for a byte pattern within a specific module
    pub fn pattern_scan(&self, module_name: &str, pattern: &[u8], mask: &str) -> Option<usize> {
        // Get module information
        let module_info = self.loaded_modules.get(module_name)?;
        let mask_bytes = mask.as_bytes();
        let module_base = module_info.base_address;
        let module_len = module_info.size;
        let pattern_len = pattern.len();
        let mask_len = mask.len();

        // Validate pattern and mask lengths match
        if pattern_len != mask_len {
            return None;
        }

        if pattern.is_empty() {
            return None;
        }

        // Iterate through the data buffer
        for i in 0..=(module_len - pattern_len) {
            let mut matches = true;

            // Check each byte in the pattern
            for j in 0..pattern.len() {
                // If mask character is '?', it's a wildcard - skip comparison
                if mask_bytes[j] == b'?' {
                    continue;
                }

                let byte = unsafe {
                    std::ptr::read_unaligned(std::ops::Add::add(module_base, i + j) as *const u8)
                };

                // Compare the byte
                if byte != pattern[j] {
                    matches = false;
                    break;
                }
            }

            if matches {
                // Return the absolute address
                return Some(module_base + i);
            }
        }

        None
    }

    /// Convenience method to scan with a hex string pattern
    pub fn pattern_scan_hex(&self, module_name: &str, hex_pattern: &str) -> Option<usize> {
        let (pattern, mask) = match self.parse_hex_pattern(hex_pattern) {
            Some((p, m)) => (p, m),
            None => return None,
        };

        self.pattern_scan(module_name, &pattern, &mask)
    }

    /// Parse hex pattern string like "48 8B ? ? 89 45" into bytes and mask
    fn parse_hex_pattern(&self, hex_pattern: &str) -> Option<(Vec<u8>, String)> {
        let parts: Vec<&str> = hex_pattern.split_whitespace().collect();
        let mut pattern = Vec::new();
        let mut mask = String::new();

        for part in parts {
            if part == "?" || part == "??" {
                // Wildcard
                pattern.push(0x00); // Placeholder byte
                mask.push('?');
            } else if part.len() == 2 {
                // Try to parse as hex
                match u8::from_str_radix(part, 16) {
                    Ok(byte) => {
                        pattern.push(byte);
                        mask.push('x');
                    }
                    Err(_) => return None, // Invalid hex
                }
            } else {
                return None; // Invalid format
            }
        }

        if pattern.is_empty() {
            return None;
        }

        Some((pattern, mask))
    }

    /// Refresh module enumeration (useful for dynamically loaded modules)
    pub fn refresh_modules(&mut self) -> Result<(), UniverseError> {
        self.enumerate_modules()
    }
}
