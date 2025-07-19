use crate::core::UniverseError;
use std::collections::HashMap;
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, HMODULE};
use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows_sys::Win32::System::Memory::{VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS};
use windows_sys::Win32::System::Threading::GetCurrentProcess;
use windows_sys::Win32::System::ProcessStatus::{EnumProcessModules, GetModuleInformation, GetModuleBaseNameA, MODULEINFO};

/// Information about a loaded module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub base_address: usize,
    pub size: usize,
    pub name: String,
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
                "Failed to get current process handle".to_string()
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
            return Err(UniverseError::MemoryError(
                format!("Failed to enumerate process modules: Windows error {}", error_code)
            ));
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
                name: module_name.clone(),
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
            return Err(UniverseError::MemoryError("Size cannot be zero".to_string()));
        }

        // Check for potential overflow
        if address.checked_add(size).is_none() {
            return Err(UniverseError::MemoryError("Address range overflow".to_string()));
        }

        // Validate the starting address
        if !self.is_valid_address(address) {
            return Err(UniverseError::MemoryError(
                format!("Invalid memory address: 0x{:X}", address)
            ));
        }

        // For larger ranges, also check the end address
        if size > 4096 {  // Only check end for larger ranges to avoid performance impact
            let end_address = address + size - 1;
            if !self.is_valid_address(end_address) {
                return Err(UniverseError::MemoryError(
                    format!("Invalid memory range end: 0x{:X}", end_address)
                ));
            }
        }

        Ok(())
    }

    /// Read memory from the specified address with safety checks
    pub fn read_memory(&self, address: usize, size: usize) -> Result<Vec<u8>, UniverseError> {
        // Validate the memory range first
        self.validate_memory_range(address, size)?;

        let mut buffer = vec![0u8; size];
        let mut bytes_read: usize = 0;

        let success = unsafe {
            ReadProcessMemory(
                self.process_handle,
                address as *const std::ffi::c_void,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
                &mut bytes_read,
            )
        };

        if success == 0 {
            let error_code = unsafe { GetLastError() };
            return Err(UniverseError::MemoryError(
                format!("Failed to read memory at 0x{:X}: Windows error {}", address, error_code)
            ));
        }

        if bytes_read != size {
            return Err(UniverseError::MemoryError(
                format!("Partial read: expected {} bytes, got {} bytes", size, bytes_read)
            ));
        }

        Ok(buffer)
    }

    /// Write data to the specified memory address with safety checks
    pub fn write_memory(&self, address: usize, data: &[u8]) -> Result<(), UniverseError> {
        if data.is_empty() {
            return Err(UniverseError::MemoryError("Cannot write empty data".to_string()));
        }

        // Validate the memory range first
        self.validate_memory_range(address, data.len())?;

        let mut bytes_written: usize = 0;

        let success = unsafe {
            WriteProcessMemory(
                self.process_handle,
                address as *mut std::ffi::c_void,
                data.as_ptr() as *const std::ffi::c_void,
                data.len(),
                &mut bytes_written,
            )
        };

        if success == 0 {
            let error_code = unsafe { GetLastError() };
            return Err(UniverseError::MemoryError(
                format!("Failed to write memory at 0x{:X}: Windows error {}", address, error_code)
            ));
        }

        if bytes_written != data.len() {
            return Err(UniverseError::MemoryError(
                format!("Partial write: expected {} bytes, wrote {} bytes", data.len(), bytes_written)
            ));
        }

        Ok(())
    }

    /// Scan for a byte pattern within a specific module
    pub fn pattern_scan(&self, module_name: &str, pattern: &[u8], mask: &str) -> Option<usize> {
        // Get module information
        let module_info = self.loaded_modules.get(module_name)?;
        
        // Validate pattern and mask lengths match
        if pattern.len() != mask.len() {
            return None;
        }
        
        if pattern.is_empty() {
            return None;
        }

        // Read the entire module memory
        let module_data = match self.read_memory(module_info.base_address, module_info.size) {
            Ok(data) => data,
            Err(_) => return None, // Failed to read module memory
        };

        // Perform pattern matching
        self.find_pattern_in_data(&module_data, pattern, mask, module_info.base_address)
    }

    /// Find pattern in data buffer with mask support
    fn find_pattern_in_data(&self, data: &[u8], pattern: &[u8], mask: &str, base_address: usize) -> Option<usize> {
        let mask_bytes = mask.as_bytes();
        
        if data.len() < pattern.len() {
            return None;
        }

        // Iterate through the data buffer
        for i in 0..=(data.len() - pattern.len()) {
            let mut matches = true;
            
            // Check each byte in the pattern
            for j in 0..pattern.len() {
                // If mask character is '?' or 'x', it's a wildcard - skip comparison
                if mask_bytes[j] == b'?' {
                    continue;
                }
                
                // Compare the byte
                if data[i + j] != pattern[j] {
                    matches = false;
                    break;
                }
            }
            
            if matches {
                // Return the absolute address
                return Some(base_address + i);
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

    /// Get list of all loaded module names
    pub fn get_module_names(&self) -> Vec<String> {
        self.loaded_modules.keys().cloned().collect()
    }

    /// Refresh module enumeration (useful for dynamically loaded modules)
    pub fn refresh_modules(&mut self) -> Result<(), UniverseError> {
        self.enumerate_modules()
    }

    /// Get information about loaded modules (placeholder for future implementation)
    pub fn get_module_info(&self, module_name: &str) -> Option<&ModuleInfo> {
        self.loaded_modules.get(module_name)
    }

    /// Add module information (for future use with pattern scanning)
    pub fn add_module(&mut self, name: String, base_address: usize, size: usize) {
        let module_info = ModuleInfo {
            base_address,
            size,
            name: name.clone(),
        };
        self.loaded_modules.insert(name, module_info);
    }
}