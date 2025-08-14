//! Memory scanning and analysis utilities for reverse engineering.
//!
//! This module provides comprehensive memory scanning capabilities including
//! process memory access, region enumeration, protection analysis, and
//! integration with pattern matching for signature scanning.

use std::fmt;
use std::ptr::null_mut;

use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Memory::{
    MEM_COMMIT, MEM_FREE, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE, PAGE_EXECUTE_READ,
    PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_NOACCESS, PAGE_PROTECTION_FLAGS,
    PAGE_READONLY, PAGE_READWRITE, PAGE_TYPE, PAGE_WRITECOPY, VIRTUAL_ALLOCATION_TYPE,
    VirtualQueryEx,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_ALL_ACCESS};

use crate::pattern::{Pattern, PatternError, PatternScanner};
use crate::vtable::{VTable, VTableScanner};

/// Memory region information.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base_address: usize,
    pub size: usize,
    pub protection: MemoryProtection,
    pub state: MemoryState,
    pub region_type: MemoryType,
}

impl MemoryRegion {
    /// Checks if this region is readable.
    pub fn is_readable(&self) -> bool {
        matches!(
            self.protection,
            MemoryProtection::ReadOnly
                | MemoryProtection::ReadWrite
                | MemoryProtection::ExecuteRead
                | MemoryProtection::ExecuteReadWrite
                | MemoryProtection::WriteCopy
                | MemoryProtection::ExecuteWriteCopy
        )
    }

    /// Checks if this region is executable.
    pub fn is_executable(&self) -> bool {
        matches!(
            self.protection,
            MemoryProtection::Execute
                | MemoryProtection::ExecuteRead
                | MemoryProtection::ExecuteReadWrite
                | MemoryProtection::ExecuteWriteCopy
        )
    }

    /// Checks if this region is writable.
    pub fn is_writable(&self) -> bool {
        matches!(
            self.protection,
            MemoryProtection::ReadWrite
                | MemoryProtection::ExecuteReadWrite
                | MemoryProtection::WriteCopy
                | MemoryProtection::ExecuteWriteCopy
        )
    }

    /// Returns the end address of this region.
    pub fn end_address(&self) -> usize {
        self.base_address + self.size
    }

    /// Checks if an address falls within this region.
    pub fn contains_address(&self, address: usize) -> bool {
        address >= self.base_address && address < self.end_address()
    }
}

/// Memory protection flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProtection {
    NoAccess,
    ReadOnly,
    ReadWrite,
    WriteCopy,
    Execute,
    ExecuteRead,
    ExecuteReadWrite,
    ExecuteWriteCopy,
}

impl From<u32> for MemoryProtection {
    fn from(protection: u32) -> Self {
        match protection {
            x if x == PAGE_NOACCESS.0 as u32 => MemoryProtection::NoAccess,
            x if x == PAGE_READONLY.0 as u32 => MemoryProtection::ReadOnly,
            x if x == PAGE_READWRITE.0 as u32 => MemoryProtection::ReadWrite,
            x if x == PAGE_WRITECOPY.0 as u32 => MemoryProtection::WriteCopy,
            x if x == PAGE_EXECUTE.0 as u32 => MemoryProtection::Execute,
            x if x == PAGE_EXECUTE_READ.0 as u32 => MemoryProtection::ExecuteRead,
            x if x == PAGE_EXECUTE_READWRITE.0 as u32 => MemoryProtection::ExecuteReadWrite,
            x if x == PAGE_EXECUTE_WRITECOPY.0 as u32 => MemoryProtection::ExecuteWriteCopy,
            _ => MemoryProtection::NoAccess,
        }
    }
}

impl From<PAGE_PROTECTION_FLAGS> for MemoryProtection {
    fn from(protection: PAGE_PROTECTION_FLAGS) -> Self {
        match protection {
            x if x.0 == PAGE_NOACCESS.0 as u32 => MemoryProtection::NoAccess,
            x if x.0 == PAGE_READONLY.0 as u32 => MemoryProtection::ReadOnly,
            x if x.0 == PAGE_READWRITE.0 as u32 => MemoryProtection::ReadWrite,
            x if x.0 == PAGE_WRITECOPY.0 as u32 => MemoryProtection::WriteCopy,
            x if x.0 == PAGE_EXECUTE.0 as u32 => MemoryProtection::Execute,
            x if x.0 == PAGE_EXECUTE_READ.0 as u32 => MemoryProtection::ExecuteRead,
            x if x.0 == PAGE_EXECUTE_READWRITE.0 as u32 => MemoryProtection::ExecuteReadWrite,
            x if x.0 == PAGE_EXECUTE_WRITECOPY.0 as u32 => MemoryProtection::ExecuteWriteCopy,
            _ => MemoryProtection::NoAccess,
        }
    }
}

impl fmt::Display for MemoryProtection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MemoryProtection::NoAccess => "---",
            MemoryProtection::ReadOnly => "R--",
            MemoryProtection::ReadWrite => "RW-",
            MemoryProtection::WriteCopy => "RC-",
            MemoryProtection::Execute => "--X",
            MemoryProtection::ExecuteRead => "R-X",
            MemoryProtection::ExecuteReadWrite => "RWX",
            MemoryProtection::ExecuteWriteCopy => "RCX",
        };
        write!(f, "{}", s)
    }
}

/// Memory state flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryState {
    Commit,
    Free,
    Reserve,
}

impl From<u32> for MemoryState {
    fn from(state: u32) -> Self {
        match state {
            x if x == MEM_COMMIT.0 as u32 => MemoryState::Commit,
            x if x == MEM_FREE.0 as u32 => MemoryState::Free,
            x if x == MEM_RESERVE.0 as u32 => MemoryState::Reserve,
            _ => MemoryState::Free,
        }
    }
}

/// Memory type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Image,
    Mapped,
    Private,
}

/// Errors that can occur during memory operations.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Failed to open process")]
    ProcessAccessFailed,
    #[error("Failed to read memory at 0x{address:X}: {reason}")]
    ReadFailed { address: usize, reason: String },
    #[error("Failed to write memory at 0x{address:X}: {reason}")]
    WriteFailed { address: usize, reason: String },
    #[error("Failed to query memory information: {reason}")]
    QueryFailed { reason: String },
    #[error("Invalid address: 0x{address:X}")]
    InvalidAddress { address: usize },
    #[error("Pattern error: {0}")]
    PatternError(#[from] PatternError),
}

impl From<windows::core::Error> for MemoryError {
    fn from(err: windows::core::Error) -> Self {
        MemoryError::QueryFailed {
            reason: err.to_string(),
        }
    }
}

/// Configuration for memory scanning operations.
#[derive(Debug, Clone)]
pub struct MemoryScanConfig {
    /// Whether to scan executable regions.
    pub scan_executable: bool,
    /// Whether to scan readable regions.
    pub scan_readable: bool,
    /// Whether to scan writable regions.
    pub scan_writable: bool,
    /// Maximum size of a single read operation.
    pub max_read_size: usize,
    /// Minimum region size to consider scanning.
    pub min_region_size: usize,
    /// Alignment for scanning operations.
    pub scan_alignment: usize,
}

impl Default for MemoryScanConfig {
    fn default() -> Self {
        Self {
            scan_executable: true,
            scan_readable: true,
            scan_writable: false,
            max_read_size: 1024 * 1024, // 1MB
            min_region_size: 4096,      // 4KB
            scan_alignment: 1,
        }
    }
}

/// High-level memory scanner for process analysis.
pub struct MemoryScanner {
    process_handle: HANDLE,
    pattern_scanner: PatternScanner,
    vtable_scanner: VTableScanner,
    config: MemoryScanConfig,
}

impl MemoryScanner {
    /// Creates a new scanner for the current process.
    pub fn new() -> Result<Self, MemoryError> {
        Self::for_process(unsafe { GetCurrentProcess() })
    }

    /// Creates a new scanner for a specific process.
    pub fn for_process(process_handle: HANDLE) -> Result<Self, MemoryError> {
        if process_handle == INVALID_HANDLE_VALUE || process_handle.0.is_null() {
            return Err(MemoryError::ProcessAccessFailed);
        }

        Ok(Self {
            process_handle,
            pattern_scanner: PatternScanner::new(),
            vtable_scanner: VTableScanner::new(),
            config: MemoryScanConfig::default(),
        })
    }

    /// Creates a scanner for a process ID.
    pub fn for_process_id(process_id: u32) -> Result<Self, MemoryError> {
        let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, process_id) }?;
        Self::for_process(handle)
    }

    /// Sets the scanning configuration.
    pub fn with_config(mut self, config: MemoryScanConfig) -> Self {
        self.config = config;
        self
    }

    /// Enumerates all memory regions in the process.
    pub fn enumerate_regions(&self) -> Result<Vec<MemoryRegion>, MemoryError> {
        let mut regions = Vec::new();
        let mut address = 0;

        loop {
            let mut mbi = MEMORY_BASIC_INFORMATION {
                BaseAddress: null_mut(),
                AllocationBase: null_mut(),
                AllocationProtect: PAGE_PROTECTION_FLAGS(0),
                PartitionId: 0,
                RegionSize: 0,
                State: VIRTUAL_ALLOCATION_TYPE(0),
                Protect: PAGE_PROTECTION_FLAGS(0),
                Type: PAGE_TYPE(0),
            };

            let result = unsafe {
                VirtualQueryEx(
                    self.process_handle,
                    Some(address as *const _),
                    &mut mbi,
                    std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };

            if result == 0 {
                break;
            }

            if mbi.State == MEM_COMMIT {
                regions.push(MemoryRegion {
                    base_address: mbi.BaseAddress as usize,
                    size: mbi.RegionSize,
                    protection: MemoryProtection::from(mbi.Protect.0),
                    state: MemoryState::from(mbi.State.0),
                    region_type: MemoryType::Private, // Simplified
                });
            }

            address = (mbi.BaseAddress as usize) + mbi.RegionSize;
        }

        Ok(regions)
    }

    /// Reads memory from the target process.
    pub fn read_memory(&self, address: usize, size: usize) -> Result<Vec<u8>, MemoryError> {
        let mut buffer = vec![0u8; size];
        let mut bytes_read = 0;

        let success = unsafe {
            ReadProcessMemory(
                self.process_handle,
                address as *const _,
                buffer.as_mut_ptr() as *mut _,
                size,
                Some(&mut bytes_read),
            )
        };

        if success.is_err() || bytes_read != size {
            return Err(MemoryError::ReadFailed {
                address,
                reason: "ReadProcessMemory failed".to_string(),
            });
        }

        Ok(buffer)
    }

    /// Writes memory to the target process.
    pub fn write_memory(&self, address: usize, data: &[u8]) -> Result<(), MemoryError> {
        let mut bytes_written = 0;

        let success = unsafe {
            WriteProcessMemory(
                self.process_handle,
                address as *mut _,
                data.as_ptr() as *const _,
                data.len(),
                Some(&mut bytes_written),
            )
        };

        if success.is_err() || bytes_written != data.len() {
            return Err(MemoryError::WriteFailed {
                address,
                reason: "WriteProcessMemory failed".to_string(),
            });
        }

        Ok(())
    }

    /// Scans all suitable memory regions for a pattern.
    pub fn scan_pattern(&self, pattern_str: &str) -> Result<Vec<ScanResult>, MemoryError> {
        let regions = self.enumerate_regions()?;
        let pattern = Pattern::new(pattern_str)?;
        let mut results = Vec::new();

        for region in regions.iter().filter(|r| self.should_scan_region(r)) {
            if let Ok(data) = self.read_memory(region.base_address, region.size) {
                let matches = self.pattern_scanner.scan_pattern(&pattern, &data);

                for pattern_match in matches {
                    results.push(ScanResult {
                        address: region.base_address + pattern_match.offset,
                        size: pattern_match.size,
                        region: region.clone(),
                        result_type: ScanResultType::Pattern,
                        data: data[pattern_match.offset..pattern_match.offset + pattern_match.size]
                            .to_vec(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Scans for VTables in memory.
    pub fn scan_vtables(&self) -> Result<Vec<VTable>, MemoryError> {
        let regions = self.enumerate_regions()?;
        let mut vtables = Vec::new();

        for region in regions.iter().filter(|r| self.should_scan_region(r)) {
            if let Ok(data) = self.read_memory(region.base_address, region.size) {
                let region_vtables = self.vtable_scanner.scan_vtables(&data, region.base_address);
                vtables.extend(region_vtables);
            }
        }

        Ok(vtables)
    }

    /// Scans for specific byte sequences.
    pub fn scan_bytes(&self, bytes: &[u8]) -> Result<Vec<ScanResult>, MemoryError> {
        let regions = self.enumerate_regions()?;
        let mut results = Vec::new();

        for region in regions.iter().filter(|r| self.should_scan_region(r)) {
            if let Ok(data) = self.read_memory(region.base_address, region.size) {
                let matches = self.find_byte_sequences(&data, bytes);

                for offset in matches {
                    results.push(ScanResult {
                        address: region.base_address + offset,
                        size: bytes.len(),
                        region: region.clone(),
                        result_type: ScanResultType::Bytes,
                        data: bytes.to_vec(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Performs a comprehensive scan including patterns and VTables.
    pub fn comprehensive_scan(
        &self,
        patterns: &[&str],
    ) -> Result<ComprehensiveScanResult, MemoryError> {
        let mut pattern_results = Vec::new();

        for pattern_str in patterns {
            let matches = self.scan_pattern(pattern_str)?;
            pattern_results.extend(matches);
        }

        let vtables = self.scan_vtables()?;
        let regions = self.enumerate_regions()?;

        Ok(ComprehensiveScanResult {
            pattern_matches: pattern_results,
            vtables,
            memory_regions: regions,
        })
    }

    /// Checks if a region should be scanned based on configuration.
    fn should_scan_region(&self, region: &MemoryRegion) -> bool {
        if region.size < self.config.min_region_size {
            return false;
        }

        let has_permission = (self.config.scan_executable && region.is_executable())
            || (self.config.scan_readable && region.is_readable())
            || (self.config.scan_writable && region.is_writable());

        has_permission && region.state == MemoryState::Commit
    }

    /// Finds byte sequences in data using naive search.
    fn find_byte_sequences(&self, data: &[u8], pattern: &[u8]) -> Vec<usize> {
        let mut matches = Vec::new();

        if pattern.is_empty() || data.len() < pattern.len() {
            return matches;
        }

        for i in 0..=data.len() - pattern.len() {
            if data[i..i + pattern.len()] == *pattern {
                matches.push(i);
            }
        }

        matches
    }
}

impl Default for MemoryScanner {
    fn default() -> Self {
        Self::new().expect("Failed to create default memory scanner")
    }
}

/// Result of a memory scan operation.
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub address: usize,
    pub size: usize,
    pub region: MemoryRegion,
    pub result_type: ScanResultType,
    pub data: Vec<u8>,
}

impl ScanResult {
    /// Returns a hexdump of the found data.
    pub fn hexdump(&self) -> String {
        self.data
            .chunks(16)
            .enumerate()
            .map(|(i, chunk)| {
                let hex = chunk
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                let ascii = chunk
                    .iter()
                    .map(|b| {
                        if b.is_ascii_graphic() {
                            *b as char
                        } else {
                            '.'
                        }
                    })
                    .collect::<String>();
                format!("{:08X}  {:<47} |{}|", self.address + i * 16, hex, ascii)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Type of scan result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanResultType {
    Pattern,
    Bytes,
    VTable,
}

/// Result of a comprehensive scan.
#[derive(Debug)]
pub struct ComprehensiveScanResult {
    pub pattern_matches: Vec<ScanResult>,
    pub vtables: Vec<VTable>,
    pub memory_regions: Vec<MemoryRegion>,
}

impl ComprehensiveScanResult {
    /// Returns statistics about the scan results.
    pub fn statistics(&self) -> ScanStatistics {
        let total_regions = self.memory_regions.len();
        let executable_regions = self
            .memory_regions
            .iter()
            .filter(|r| r.is_executable())
            .count();
        let readable_regions = self
            .memory_regions
            .iter()
            .filter(|r| r.is_readable())
            .count();
        let writable_regions = self
            .memory_regions
            .iter()
            .filter(|r| r.is_writable())
            .count();

        ScanStatistics {
            total_regions,
            executable_regions,
            readable_regions,
            writable_regions,
            pattern_matches: self.pattern_matches.len(),
            vtables_found: self.vtables.len(),
            total_virtual_functions: self.vtables.iter().map(|v| v.function_count()).sum(),
        }
    }
}

/// Statistics from a scan operation.
#[derive(Debug)]
pub struct ScanStatistics {
    pub total_regions: usize,
    pub executable_regions: usize,
    pub readable_regions: usize,
    pub writable_regions: usize,
    pub pattern_matches: usize,
    pub vtables_found: usize,
    pub total_virtual_functions: usize,
}

impl fmt::Display for ScanStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Scan Statistics:")?;
        writeln!(f, "  Memory Regions: {}", self.total_regions)?;
        writeln!(f, "    Executable: {}", self.executable_regions)?;
        writeln!(f, "    Readable: {}", self.readable_regions)?;
        writeln!(f, "    Writable: {}", self.writable_regions)?;
        writeln!(f, "  Pattern Matches: {}", self.pattern_matches)?;
        writeln!(f, "  VTables Found: {}", self.vtables_found)?;
        writeln!(f, "  Virtual Functions: {}", self.total_virtual_functions)?;
        Ok(())
    }
}

/// Memory region filter for targeted scanning.
pub struct RegionFilter {
    criteria: Vec<Box<dyn Fn(&MemoryRegion) -> bool>>,
}

impl RegionFilter {
    pub fn new() -> Self {
        Self {
            criteria: Vec::new(),
        }
    }

    /// Adds a filter for executable regions.
    pub fn executable(mut self) -> Self {
        self.criteria.push(Box::new(|r| r.is_executable()));
        self
    }

    /// Adds a filter for readable regions.
    pub fn readable(mut self) -> Self {
        self.criteria.push(Box::new(|r| r.is_readable()));
        self
    }

    /// Adds a filter for writable regions.
    pub fn writable(mut self) -> Self {
        self.criteria.push(Box::new(|r| r.is_writable()));
        self
    }

    /// Adds a filter for minimum region size.
    pub fn min_size(mut self, size: usize) -> Self {
        self.criteria.push(Box::new(move |r| r.size >= size));
        self
    }

    /// Adds a filter for address range.
    pub fn address_range(mut self, start: usize, end: usize) -> Self {
        self.criteria.push(Box::new(move |r| {
            r.base_address >= start && r.end_address() <= end
        }));
        self
    }

    /// Applies all filters to a region.
    pub fn matches(&self, region: &MemoryRegion) -> bool {
        self.criteria.iter().all(|criterion| criterion(region))
    }

    /// Filters a list of regions.
    pub fn filter_regions<'a>(&self, regions: &'a [MemoryRegion]) -> Vec<&'a MemoryRegion> {
        regions.iter().filter(|r| self.matches(r)).collect()
    }
}

impl Default for RegionFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_protection() {
        let protection = MemoryProtection::from(PAGE_EXECUTE_READ);
        assert_eq!(protection, MemoryProtection::ExecuteRead);
        assert_eq!(format!("{}", protection), "R-X");
    }

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion {
            base_address: 0x1000,
            size: 0x1000,
            protection: MemoryProtection::ReadWrite,
            state: MemoryState::Commit,
            region_type: MemoryType::Private,
        };

        assert!(region.is_readable());
        assert!(region.is_writable());
        assert!(!region.is_executable());
        assert!(region.contains_address(0x1500));
        assert!(!region.contains_address(0x2000));
        assert_eq!(region.end_address(), 0x2000);
    }

    #[test]
    fn test_region_filter() {
        let region = MemoryRegion {
            base_address: 0x1000,
            size: 0x1000,
            protection: MemoryProtection::ExecuteRead,
            state: MemoryState::Commit,
            region_type: MemoryType::Private,
        };

        let filter = RegionFilter::new().executable().readable().min_size(0x800);

        assert!(filter.matches(&region));

        let filter2 = RegionFilter::new().writable();

        assert!(!filter2.matches(&region));
    }
}
