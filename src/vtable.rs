//! Advanced VTable scanning and analysis utilities for reverse engineering.
//!
//! This module provides comprehensive tools for finding, analyzing, and reconstructing
//! C++ virtual function tables in memory. It includes heuristics for detecting VTables,
//! analyzing virtual function layouts, and extracting class hierarchies.

use crate::errors::Result;
use crate::pattern::{PatternMatch, PatternScanner};
use std::collections::HashMap;
use std::fmt;

/// Represents a virtual function table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualFunction {
    pub address: usize,
    pub index: usize,
}

/// Represents a complete virtual table.
#[derive(Debug, Clone)]
pub struct VTable {
    pub base_address: usize,
    pub functions: Vec<VirtualFunction>,
    pub type_info_ptr: Option<usize>,
    pub size: usize,
}

impl VTable {
    /// Creates a new VTable.
    pub fn new(base_address: usize) -> Self {
        Self {
            base_address,
            functions: Vec::new(),
            type_info_ptr: None,
            size: 0,
        }
    }

    /// Adds a virtual function to the table.
    pub fn add_function(&mut self, address: usize, index: usize) {
        self.functions.push(VirtualFunction { address, index });
        self.size =
            (self.functions.len() * std::mem::size_of::<usize>()) + std::mem::size_of::<usize>(); // +1 for RTTI pointer
    }

    /// Returns the number of virtual functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Returns a function at the specified index.
    pub fn get_function(&self, index: usize) -> Option<VirtualFunction> {
        self.functions.get(index).copied()
    }

    /// Checks if this VTable contains a function at the given address.
    pub fn contains_function(&self, address: usize) -> bool {
        self.functions.iter().any(|f| f.address == address)
    }

    /// Returns the estimated class name based on heuristics.
    pub fn estimated_class_name(&self) -> Option<String> {
        // This would typically involve RTTI analysis
        // For now, return a placeholder based on the base address
        Some(format!("Class_{:X}", self.base_address))
    }
}

impl fmt::Display for VTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "VTable @ 0x{:X}:", self.base_address)?;
        if let Some(type_info) = self.type_info_ptr {
            writeln!(f, "  Type Info: 0x{:X}", type_info)?;
        }
        writeln!(f, "  Functions ({}):", self.functions.len())?;
        for func in &self.functions {
            writeln!(f, "    [{}]: 0x{:X}", func.index, func.address)?;
        }
        Ok(())
    }
}

/// Configuration for VTable scanning.
#[derive(Debug, Clone)]
pub struct VTableScanConfig {
    /// Minimum number of consecutive valid function pointers to consider a VTable.
    pub min_functions: usize,
    /// Maximum number of functions to scan in a single VTable.
    pub max_functions: usize,
    /// Whether to include RTTI information in the scan.
    pub include_rtti: bool,
    /// Minimum alignment for VTable addresses.
    pub alignment: usize,
    /// Address ranges to exclude from scanning.
    pub excluded_ranges: Vec<(usize, usize)>,
}

impl Default for VTableScanConfig {
    fn default() -> Self {
        Self {
            min_functions: 2,
            max_functions: 256,
            include_rtti: true,
            alignment: std::mem::size_of::<usize>(),
            excluded_ranges: Vec::new(),
        }
    }
}

/// Heuristics for determining if a memory location contains valid code.
pub struct CodeHeuristics;

impl CodeHeuristics {
    /// Checks if an address looks like a valid function pointer.
    pub fn is_valid_function_ptr(address: usize, data: &[u8], base_addr: usize) -> bool {
        if address == 0 {
            return false;
        }

        // Check if address is within reasonable bounds
        if address < base_addr || address > base_addr + data.len() {
            return false;
        }

        // Calculate offset into our data
        let offset = address - base_addr;
        if offset >= data.len() {
            return false;
        }

        // Look for common x64 function prologues
        Self::has_function_prologue(data, offset)
    }

    /// Checks for common x64 function prologues.
    fn has_function_prologue(data: &[u8], offset: usize) -> bool {
        if offset + 4 > data.len() {
            return false;
        }

        let bytes = &data[offset..offset + 4];

        // Common x64 prologues:
        matches!(
            bytes,
            // push rbp; mov rbp, rsp
            [0x55, 0x48, 0x89, 0xE5] |
            // push rbp; mov rbp, rsp (alternative)
            [0x55, 0x48, 0x8B, 0xEC] |
            // sub rsp, imm8
            [0x48, 0x83, 0xEC, _] |
            // sub rsp, imm32
            [0x48, 0x81, 0xEC, _] |
            // push rbx
            [0x53, _, _, _] |
            // mov [rsp+8], rcx (fastcall)
            [0x48, 0x89, 0x4C, 0x24] |
            // int 3 (breakpoint - sometimes at function start)
            [0xCC, _, _, _]
        )
    }

    /// Checks if an address looks like RTTI type info.
    pub fn is_rtti_type_info(address: usize, data: &[u8], base_addr: usize) -> bool {
        if address < base_addr || address > base_addr + data.len() {
            return false;
        }

        let offset = address - base_addr;
        if offset + 16 > data.len() {
            return false;
        }

        // RTTI typically has specific patterns
        // This is a simplified check - real RTTI analysis is more complex
        let bytes = &data[offset..offset + 8];

        // Check for non-zero values that could be vtable pointers or name pointers
        let ptr_val = usize::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));

        ptr_val != 0 && ptr_val >= base_addr && ptr_val <= base_addr + data.len()
    }
}

/// High-level VTable scanner.
pub struct VTableScanner {
    config: VTableScanConfig,
    pattern_scanner: PatternScanner,
}

impl Default for VTableScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl VTableScanner {
    /// Creates a new VTable scanner with default configuration.
    pub fn new() -> Self {
        Self {
            config: VTableScanConfig::default(),
            pattern_scanner: PatternScanner::new(),
        }
    }

    /// Creates a scanner with custom configuration.
    pub fn with_config(config: VTableScanConfig) -> Self {
        Self {
            config,
            pattern_scanner: PatternScanner::new(),
        }
    }

    /// Scans memory for VTables.
    pub fn scan_vtables(&self, data: &[u8], base_address: usize) -> Vec<VTable> {
        let mut vtables = Vec::new();
        let ptr_size = std::mem::size_of::<usize>();

        // Align scanning to pointer boundaries
        for i in (0..data.len()).step_by(self.config.alignment) {
            if i + ptr_size > data.len() {
                break;
            }

            // Check if this location is in an excluded range
            let current_addr = base_address + i;
            if self.is_address_excluded(current_addr) {
                continue;
            }

            if let Some(vtable) = self.analyze_potential_vtable(data, base_address, i) {
                vtables.push(vtable);
            }
        }

        vtables
    }

    /// Analyzes a potential VTable location.
    fn analyze_potential_vtable(
        &self,
        data: &[u8],
        base_addr: usize,
        offset: usize,
    ) -> Option<VTable> {
        let mut vtable = VTable::new(base_addr + offset);
        let ptr_size = std::mem::size_of::<usize>();
        let mut current_offset = offset;

        // Skip RTTI pointer if configured
        if self.config.include_rtti {
            if current_offset + ptr_size <= data.len() {
                let rtti_ptr = self.read_pointer(data, current_offset);
                if CodeHeuristics::is_rtti_type_info(rtti_ptr, data, base_addr) {
                    vtable.type_info_ptr = Some(rtti_ptr);
                    current_offset += ptr_size;
                }
            }
        }

        // Scan for virtual functions
        let mut function_index = 0;
        while function_index < self.config.max_functions && current_offset + ptr_size <= data.len()
        {
            let func_ptr = self.read_pointer(data, current_offset);

            if !CodeHeuristics::is_valid_function_ptr(func_ptr, data, base_addr) {
                break;
            }

            vtable.add_function(func_ptr, function_index);
            function_index += 1;
            current_offset += ptr_size;
        }

        // Only return VTables with sufficient functions
        if vtable.function_count() >= self.config.min_functions {
            Some(vtable)
        } else {
            None
        }
    }

    /// Reads a pointer from the data at the given offset.
    fn read_pointer(&self, data: &[u8], offset: usize) -> usize {
        let ptr_size = std::mem::size_of::<usize>();
        if offset + ptr_size > data.len() {
            return 0;
        }

        let bytes = &data[offset..offset + ptr_size];
        match bytes.try_into() {
            Ok(bytes_array) => usize::from_le_bytes(bytes_array),
            Err(_) => 0,
        }
    }

    /// Checks if an address is in an excluded range.
    fn is_address_excluded(&self, address: usize) -> bool {
        self.config
            .excluded_ranges
            .iter()
            .any(|(start, end)| address >= *start && address <= *end)
    }

    /// Finds VTables by scanning for common VTable patterns.
    pub fn find_vtables_by_patterns(&self, data: &[u8]) -> Result<Vec<PatternMatch>> {
        // Common VTable patterns (this is highly specific to compiler and architecture)
        let patterns = vec![
            // Pattern for typical VTable with RTTI
            "?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ?? ??", // 16 bytes pattern
            // Pattern for functions starting with common prologues
            "55 48 89 E5", // push rbp; mov rbp, rsp
            "48 83 EC ??", // sub rsp, imm8
        ];

        let mut all_matches = Vec::new();
        for pattern_str in patterns {
            let matches = self.pattern_scanner.scan(pattern_str, data)?;
            all_matches.extend(matches);
        }

        Ok(all_matches)
    }

    /// Analyzes inheritance relationships between VTables.
    pub fn analyze_inheritance(&self, vtables: &[VTable]) -> HashMap<usize, Vec<usize>> {
        let mut inheritance_map = HashMap::new();

        for base_vtable in vtables {
            let mut derived_vtables = Vec::new();

            for derived_vtable in vtables {
                if base_vtable.base_address == derived_vtable.base_address {
                    continue;
                }

                // Check if derived_vtable extends base_vtable
                if self.is_derived_vtable(base_vtable, derived_vtable) {
                    derived_vtables.push(derived_vtable.base_address);
                }
            }

            if !derived_vtables.is_empty() {
                inheritance_map.insert(base_vtable.base_address, derived_vtables);
            }
        }

        inheritance_map
    }

    /// Heuristic to determine if one VTable is derived from another.
    fn is_derived_vtable(&self, base: &VTable, derived: &VTable) -> bool {
        if derived.function_count() < base.function_count() {
            return false;
        }

        // Check if the first N functions match (where N = base function count)
        let base_functions = base.functions.len();
        for i in 0..base_functions {
            if let (Some(base_func), Some(derived_func)) =
                (base.get_function(i), derived.get_function(i))
            {
                if base_func.address != derived_func.address {
                    return false;
                }
            }
        }

        true
    }
}

/// Utilities for VTable reconstruction and analysis.
pub struct VTableAnalyzer;

impl VTableAnalyzer {
    /// Reconstructs class hierarchies from VTables.
    pub fn reconstruct_hierarchy(vtables: &[VTable]) -> ClassHierarchy {
        let scanner = VTableScanner::new();
        let inheritance_map = scanner.analyze_inheritance(vtables);

        let mut hierarchy = ClassHierarchy::new();

        for vtable in vtables {
            let class_info = ClassInfo {
                vtable_address: vtable.base_address,
                name: vtable
                    .estimated_class_name()
                    .unwrap_or_else(|| format!("UnknownClass_{:X}", vtable.base_address)),
                functions: vtable.functions.clone(),
                base_classes: Vec::new(),
                derived_classes: inheritance_map
                    .get(&vtable.base_address)
                    .cloned()
                    .unwrap_or_default(),
            };

            hierarchy.add_class(class_info);
        }

        hierarchy
    }

    /// Finds function duplicates across VTables.
    pub fn find_shared_functions(vtables: &[VTable]) -> HashMap<usize, Vec<usize>> {
        let mut function_to_vtables = HashMap::new();

        for vtable in vtables {
            for function in &vtable.functions {
                function_to_vtables
                    .entry(function.address)
                    .or_insert_with(Vec::new)
                    .push(vtable.base_address);
            }
        }

        // Filter to only functions shared between multiple VTables
        function_to_vtables
            .into_iter()
            .filter(|(_, vtables)| vtables.len() > 1)
            .collect()
    }

    /// Estimates the size of objects based on VTable analysis.
    pub fn estimate_object_sizes(vtables: &[VTable]) -> HashMap<usize, usize> {
        vtables
            .iter()
            .map(|vtable| {
                // Basic estimation based on function count and known patterns
                let base_size = std::mem::size_of::<usize>(); // vtable pointer
                let estimated_size = base_size + (vtable.function_count() * 8); // rough estimate
                (vtable.base_address, estimated_size)
            })
            .collect()
    }
}

/// Represents class information extracted from VTable analysis.
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub vtable_address: usize,
    pub name: String,
    pub functions: Vec<VirtualFunction>,
    pub base_classes: Vec<usize>,
    pub derived_classes: Vec<usize>,
}

/// Represents a complete class hierarchy.
#[derive(Debug, Default)]
pub struct ClassHierarchy {
    classes: HashMap<usize, ClassInfo>,
}

impl ClassHierarchy {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }

    pub fn add_class(&mut self, class_info: ClassInfo) {
        self.classes.insert(class_info.vtable_address, class_info);
    }

    pub fn get_class(&self, vtable_address: usize) -> Option<&ClassInfo> {
        self.classes.get(&vtable_address)
    }

    pub fn get_all_classes(&self) -> impl Iterator<Item = &ClassInfo> {
        self.classes.values()
    }

    pub fn find_root_classes(&self) -> Vec<&ClassInfo> {
        self.classes
            .values()
            .filter(|class| class.base_classes.is_empty())
            .collect()
    }

    pub fn find_leaf_classes(&self) -> Vec<&ClassInfo> {
        self.classes
            .values()
            .filter(|class| class.derived_classes.is_empty())
            .collect()
    }
}

impl fmt::Display for ClassHierarchy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Class Hierarchy:")?;

        for root_class in self.find_root_classes() {
            self.print_class_tree(f, root_class, 0)?;
        }

        Ok(())
    }
}

impl ClassHierarchy {
    fn print_class_tree(
        &self,
        f: &mut fmt::Formatter<'_>,
        class: &ClassInfo,
        depth: usize,
    ) -> fmt::Result {
        let indent = "  ".repeat(depth);
        writeln!(f, "{}{} (0x{:X})", indent, class.name, class.vtable_address)?;

        for &derived_addr in &class.derived_classes {
            if let Some(derived_class) = self.get_class(derived_addr) {
                self.print_class_tree(f, derived_class, depth + 1)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vtable_creation() {
        let mut vtable = VTable::new(0x1000);
        vtable.add_function(0x2000, 0);
        vtable.add_function(0x2100, 1);

        assert_eq!(vtable.function_count(), 2);
        assert!(vtable.contains_function(0x2000));
        assert!(!vtable.contains_function(0x3000));
    }

    #[test]
    fn test_code_heuristics() {
        let data = vec![
            0x55, 0x48, 0x89, 0xE5, // push rbp; mov rbp, rsp
            0x90, 0x90, 0x90, 0x90, // nops
        ];

        assert!(CodeHeuristics::has_function_prologue(&data, 0));
        assert!(!CodeHeuristics::has_function_prologue(&data, 4));
    }

    #[test]
    fn test_vtable_scanner_config() {
        let config = VTableScanConfig {
            min_functions: 3,
            max_functions: 10,
            ..Default::default()
        };

        let scanner = VTableScanner::with_config(config);
        assert_eq!(scanner.config.min_functions, 3);
        assert_eq!(scanner.config.max_functions, 10);
    }
}
