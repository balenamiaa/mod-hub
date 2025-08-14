//! Advanced analysis utilities for reverse engineering tasks.
//!
//! This module provides high-level analysis tools that combine pattern matching,
//! VTable analysis, and memory scanning to extract meaningful information about
//! binary structures, class hierarchies, and code patterns.

use std::collections::HashMap;
use std::fmt;

use crate::memory::{MemoryScanner, ComprehensiveScanResult};
use crate::pattern::PatternError;
use crate::vtable::{VTable, VTableScanner, ClassHierarchy, VTableAnalyzer};

/// Configuration for analysis operations.
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// Confidence threshold for analysis results (0.0 to 1.0).
    pub confidence_threshold: f32,
    /// Maximum depth for recursive analysis.
    pub max_analysis_depth: usize,
    /// Whether to include low-confidence results.
    pub include_low_confidence: bool,
    /// Patterns to prioritize during analysis.
    pub priority_patterns: Vec<String>,
    /// Whether to perform cross-reference analysis.
    pub enable_cross_reference: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            max_analysis_depth: 5,
            include_low_confidence: false,
            priority_patterns: Vec::new(),
            enable_cross_reference: true,
        }
    }
}

/// Represents a discovered function in the binary.
#[derive(Debug, Clone)]
pub struct DiscoveredFunction {
    pub address: usize,
    pub size: Option<usize>,
    pub name: Option<String>,
    pub calling_convention: CallingConvention,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<DataType>,
    pub confidence: f32,
    pub references: Vec<usize>,
    pub xrefs_to: Vec<usize>,
    pub xrefs_from: Vec<usize>,
}

/// Calling conventions for functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConvention {
    Unknown,
    Cdecl,
    Stdcall,
    Fastcall,
    Thiscall,
    Vectorcall,
}

/// Function parameter information.
#[derive(Debug, Clone)]
pub struct Parameter {
    pub index: usize,
    pub data_type: DataType,
    pub location: ParameterLocation,
}

/// Parameter location (register or stack).
#[derive(Debug, Clone)]
pub enum ParameterLocation {
    Register(String),
    Stack(i32),
    Unknown,
}

/// Data types discovered during analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Unknown,
    Void,
    Bool,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float32,
    Float64,
    Pointer(Box<DataType>),
    Array(Box<DataType>, usize),
    Struct(String),
    Class(String),
}

/// Represents a discovered structure or class.
#[derive(Debug, Clone)]
pub struct DiscoveredStruct {
    pub name: String,
    pub size: usize,
    pub fields: Vec<StructField>,
    pub vtable_offset: Option<usize>,
    pub base_classes: Vec<String>,
    pub confidence: f32,
}

/// Structure field information.
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub offset: usize,
    pub data_type: DataType,
    pub size: usize,
}

/// Results from comprehensive binary analysis.
#[derive(Debug)]
pub struct AnalysisResult {
    pub functions: Vec<DiscoveredFunction>,
    pub structures: Vec<DiscoveredStruct>,
    pub class_hierarchy: ClassHierarchy,
    pub string_references: Vec<StringReference>,
    pub import_table: Vec<ImportEntry>,
    pub export_table: Vec<ExportEntry>,
    pub code_patterns: Vec<CodePattern>,
    pub statistics: AnalysisStatistics,
}

/// String reference found in the binary.
#[derive(Debug, Clone)]
pub struct StringReference {
    pub address: usize,
    pub value: String,
    pub encoding: StringEncoding,
    pub references: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringEncoding {
    Ascii,
    Unicode,
    Utf8,
}

/// Import table entry.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub module_name: String,
    pub function_name: String,
    pub address: usize,
    pub ordinal: Option<u16>,
}

/// Export table entry.
#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub name: String,
    pub address: usize,
    pub ordinal: u16,
}

/// Discovered code pattern.
#[derive(Debug, Clone)]
pub struct CodePattern {
    pub pattern_type: PatternType,
    pub addresses: Vec<usize>,
    pub description: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatternType {
    FunctionPrologue,
    FunctionEpilogue,
    VirtualCall,
    SystemCall,
    StringReference,
    JumpTable,
    ExceptionHandler,
    Custom(String),
}

/// Analysis statistics.
#[derive(Debug)]
pub struct AnalysisStatistics {
    pub total_functions: usize,
    pub high_confidence_functions: usize,
    pub total_structures: usize,
    pub vtables_analyzed: usize,
    pub string_references: usize,
    pub code_patterns: usize,
    pub analysis_time_ms: u64,
}

impl fmt::Display for AnalysisStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Analysis Statistics:")?;
        writeln!(f, "  Functions: {} ({} high confidence)", 
                self.total_functions, self.high_confidence_functions)?;
        writeln!(f, "  Structures: {}", self.total_structures)?;
        writeln!(f, "  VTables: {}", self.vtables_analyzed)?;
        writeln!(f, "  String References: {}", self.string_references)?;
        writeln!(f, "  Code Patterns: {}", self.code_patterns)?;
        writeln!(f, "  Analysis Time: {}ms", self.analysis_time_ms)?;
        Ok(())
    }
}

/// Main analysis engine that coordinates all analysis tasks.
pub struct AnalysisEngine {
    config: AnalysisConfig,
    memory_scanner: MemoryScanner,
    vtable_scanner: VTableScanner,
    pattern_database: PatternDatabase,
}

impl AnalysisEngine {
    /// Creates a new analysis engine.
    pub fn new(memory_scanner: MemoryScanner) -> Self {
        Self {
            config: AnalysisConfig::default(),
            memory_scanner,
            vtable_scanner: VTableScanner::new(),
            pattern_database: PatternDatabase::new(),
        }
    }

    /// Creates an engine with custom configuration.
    pub fn with_config(memory_scanner: MemoryScanner, config: AnalysisConfig) -> Self {
        Self {
            config,
            memory_scanner,
            vtable_scanner: VTableScanner::new(),
            pattern_database: PatternDatabase::new(),
        }
    }

    /// Performs comprehensive analysis of the target binary.
    pub fn analyze(&self) -> Result<AnalysisResult, AnalysisError> {
        let start_time = std::time::Instant::now();

        // Perform initial scans
        let scan_result = self.comprehensive_scan()?;
        
        // Analyze functions
        let functions = self.analyze_functions(&scan_result)?;
        
        // Analyze structures and classes
        let structures = self.analyze_structures(&scan_result.vtables)?;
        let class_hierarchy = VTableAnalyzer::reconstruct_hierarchy(&scan_result.vtables);
        
        // Find string references
        let string_references = self.find_string_references(&scan_result)?;
        
        // Analyze imports/exports (simplified - would need PE parsing)
        let import_table = self.analyze_imports()?;
        let export_table = self.analyze_exports()?;
        
        // Detect code patterns
        let code_patterns = self.detect_code_patterns(&scan_result)?;

        let analysis_time = start_time.elapsed().as_millis() as u64;
        
        let statistics = AnalysisStatistics {
            total_functions: functions.len(),
            high_confidence_functions: functions.iter()
                .filter(|f| f.confidence >= self.config.confidence_threshold)
                .count(),
            total_structures: structures.len(),
            vtables_analyzed: scan_result.vtables.len(),
            string_references: string_references.len(),
            code_patterns: code_patterns.len(),
            analysis_time_ms: analysis_time,
        };

        Ok(AnalysisResult {
            functions,
            structures,
            class_hierarchy,
            string_references,
            import_table,
            export_table,
            code_patterns,
            statistics,
        })
    }

    /// Performs targeted analysis on specific addresses.
    pub fn analyze_addresses(&self, addresses: &[usize]) -> Result<Vec<AddressAnalysis>, AnalysisError> {
        let mut results = Vec::new();
        
        for &address in addresses {
            let analysis = self.analyze_single_address(address)?;
            results.push(analysis);
        }
        
        Ok(results)
    }

    /// Analyzes a specific function at the given address.
    pub fn analyze_function(&self, address: usize) -> Result<DiscoveredFunction, AnalysisError> {
        // Read function data
        let data = self.memory_scanner.read_memory(address, 512)
            .map_err(|e| AnalysisError::MemoryError(e.to_string()))?;

        let mut function = DiscoveredFunction {
            address,
            size: None,
            name: None,
            calling_convention: self.detect_calling_convention(&data),
            parameters: Vec::new(),
            return_type: None,
            confidence: 0.5,
            references: Vec::new(),
            xrefs_to: Vec::new(),
            xrefs_from: Vec::new(),
        };

        // Analyze function prologue/epilogue for size estimation
        if let Some(size) = self.estimate_function_size(&data) {
            function.size = Some(size);
            function.confidence += 0.2;
        }

        // Detect parameters
        function.parameters = self.analyze_function_parameters(&data);
        if !function.parameters.is_empty() {
            function.confidence += 0.1;
        }

        Ok(function)
    }

    fn comprehensive_scan(&self) -> Result<ComprehensiveScanResult, AnalysisError> {
        let patterns = self.pattern_database.get_common_patterns();
        self.memory_scanner.comprehensive_scan(&patterns)
            .map_err(|e| AnalysisError::ScanError(e.to_string()))
    }

    fn analyze_functions(&self, scan_result: &ComprehensiveScanResult) -> Result<Vec<DiscoveredFunction>, AnalysisError> {
        let mut functions = Vec::new();
        
        // Extract functions from pattern matches
        for result in &scan_result.pattern_matches {
            if let Ok(function) = self.analyze_function(result.address) {
                functions.push(function);
            }
        }

        // Extract functions from VTables
        for vtable in &scan_result.vtables {
            for virtual_func in &vtable.functions {
                if let Ok(mut function) = self.analyze_function(virtual_func.address) {
                    function.calling_convention = CallingConvention::Thiscall;
                    function.confidence += 0.1; // Higher confidence for vtable functions
                    functions.push(function);
                }
            }
        }

        Ok(functions)
    }

    fn analyze_structures(&self, vtables: &[VTable]) -> Result<Vec<DiscoveredStruct>, AnalysisError> {
        let mut structures = Vec::new();
        
        for vtable in vtables {
            let mut structure = DiscoveredStruct {
                name: vtable.estimated_class_name().unwrap_or_else(|| {
                    format!("Struct_{:X}", vtable.base_address)
                }),
                size: std::mem::size_of::<usize>(), // At least vtable pointer
                fields: Vec::new(),
                vtable_offset: Some(0), // Typically at offset 0
                base_classes: Vec::new(),
                confidence: 0.6,
            };

            // Add vtable pointer field
            structure.fields.push(StructField {
                name: "vtable".to_string(),
                offset: 0,
                data_type: DataType::Pointer(Box::new(DataType::Void)),
                size: std::mem::size_of::<usize>(),
            });

            structures.push(structure);
        }
        
        Ok(structures)
    }

    fn find_string_references(&self, scan_result: &ComprehensiveScanResult) -> Result<Vec<StringReference>, AnalysisError> {
        let mut string_refs = Vec::new();
        
        // Look for ASCII strings in readable regions
        for region in &scan_result.memory_regions {
            if region.is_readable() {
                if let Ok(data) = self.memory_scanner.read_memory(region.base_address, region.size) {
                    let strings = self.extract_strings(&data, region.base_address);
                    string_refs.extend(strings);
                }
            }
        }
        
        Ok(string_refs)
    }

    fn analyze_imports(&self) -> Result<Vec<ImportEntry>, AnalysisError> {
        // Simplified - would need proper PE parsing
        Ok(Vec::new())
    }

    fn analyze_exports(&self) -> Result<Vec<ExportEntry>, AnalysisError> {
        // Simplified - would need proper PE parsing
        Ok(Vec::new())
    }

    fn detect_code_patterns(&self, scan_result: &ComprehensiveScanResult) -> Result<Vec<CodePattern>, AnalysisError> {
        let mut patterns = Vec::new();
        
        // Detect function prologues
        for result in &scan_result.pattern_matches {
            if self.pattern_database.is_function_prologue(&result.data) {
                patterns.push(CodePattern {
                    pattern_type: PatternType::FunctionPrologue,
                    addresses: vec![result.address],
                    description: "Standard function prologue".to_string(),
                    confidence: 0.8,
                });
            }
        }

        // Detect virtual calls
        for vtable in &scan_result.vtables {
            patterns.push(CodePattern {
                pattern_type: PatternType::VirtualCall,
                addresses: vtable.functions.iter().map(|f| f.address).collect(),
                description: format!("Virtual function table with {} functions", vtable.function_count()),
                confidence: 0.9,
            });
        }
        
        Ok(patterns)
    }

    fn analyze_single_address(&self, address: usize) -> Result<AddressAnalysis, AnalysisError> {
        let data = self.memory_scanner.read_memory(address, 64)
            .map_err(|e| AnalysisError::MemoryError(e.to_string()))?;

        let analysis_type = if self.pattern_database.is_function_prologue(&data) {
            AddressType::Function
        } else if self.is_data_pointer(&data) {
            AddressType::DataPointer
        } else if self.is_string_data(&data) {
            AddressType::String
        } else {
            AddressType::Unknown
        };

        Ok(AddressAnalysis {
            address,
            analysis_type,
            confidence: 0.5,
            description: self.describe_address_content(&data, analysis_type),
            data: data[..std::cmp::min(32, data.len())].to_vec(),
        })
    }

    fn detect_calling_convention(&self, data: &[u8]) -> CallingConvention {
        // Simplified detection based on common patterns
        if data.len() >= 4 {
            match &data[0..4] {
                [0x55, 0x48, 0x89, 0xE5] => CallingConvention::Cdecl, // push rbp; mov rbp, rsp
                [0x48, 0x89, 0x4C, 0x24] => CallingConvention::Fastcall, // mov [rsp+8], rcx
                _ => CallingConvention::Unknown,
            }
        } else {
            CallingConvention::Unknown
        }
    }

    fn estimate_function_size(&self, data: &[u8]) -> Option<usize> {
        // Look for function epilogue patterns
        for (i, window) in data.windows(3).enumerate() {
            if matches!(window, [0x48, 0x83, 0xC4] | [0x5D, 0xC3, _] | [0xC3, _, _]) {
                return Some(i + 1);
            }
        }
        None
    }

    fn analyze_function_parameters(&self, data: &[u8]) -> Vec<Parameter> {
        let mut parameters = Vec::new();
        
        // Simplified parameter detection
        // Look for register usage patterns in first few instructions
        if data.len() >= 8 {
            // Check for fastcall pattern (RCX, RDX, R8, R9)
            if matches!(&data[0..4], [0x48, 0x89, 0x4C, 0x24]) {
                parameters.push(Parameter {
                    index: 0,
                    data_type: DataType::Unknown,
                    location: ParameterLocation::Register("RCX".to_string()),
                });
            }
        }
        
        parameters
    }

    fn extract_strings(&self, data: &[u8], base_address: usize) -> Vec<StringReference> {
        let mut strings = Vec::new();
        let mut current_string = Vec::new();
        let mut start_offset = 0;

        for (i, &byte) in data.iter().enumerate() {
            if byte.is_ascii_graphic() || byte == b' ' {
                if current_string.is_empty() {
                    start_offset = i;
                }
                current_string.push(byte);
            } else {
                if current_string.len() >= 4 { // Minimum string length
                    if let Ok(string) = String::from_utf8(current_string.clone()) {
                        strings.push(StringReference {
                            address: base_address + start_offset,
                            value: string,
                            encoding: StringEncoding::Ascii,
                            references: Vec::new(),
                        });
                    }
                }
                current_string.clear();
            }
        }

        strings
    }

    fn is_data_pointer(&self, data: &[u8]) -> bool {
        if data.len() >= 8 {
            let ptr = usize::from_le_bytes(data[..8].try_into().unwrap_or([0; 8]));
            ptr > 0x1000 && ptr < 0x7FFFFFFFFFFF // Reasonable pointer range
        } else {
            false
        }
    }

    fn is_string_data(&self, data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        
        let ascii_count = data.iter().take(32).filter(|&&b| b.is_ascii_graphic() || b == b' ').count();
        ascii_count as f32 / data.len().min(32) as f32 > 0.7
    }

    fn describe_address_content(&self, data: &[u8], addr_type: AddressType) -> String {
        match addr_type {
            AddressType::Function => "Function entry point".to_string(),
            AddressType::DataPointer => "Data pointer".to_string(),
            AddressType::String => {
                if let Ok(s) = std::str::from_utf8(&data[..data.len().min(32)]) {
                    format!("String data: \"{}\"", s.chars().take(20).collect::<String>())
                } else {
                    "String data (binary)".to_string()
                }
            }
            AddressType::Unknown => "Unknown data".to_string(),
        }
    }
}

/// Analysis result for a specific address.
#[derive(Debug)]
pub struct AddressAnalysis {
    pub address: usize,
    pub analysis_type: AddressType,
    pub confidence: f32,
    pub description: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressType {
    Function,
    DataPointer,
    String,
    Unknown,
}

/// Database of common patterns used for analysis.
struct PatternDatabase {
    patterns: HashMap<PatternType, Vec<String>>,
}

impl PatternDatabase {
    fn new() -> Self {
        let mut patterns = HashMap::new();
        
        // Function prologue patterns
        patterns.insert(PatternType::FunctionPrologue, vec![
            "55 48 89 E5".to_string(),    // push rbp; mov rbp, rsp
            "48 83 EC ??".to_string(),    // sub rsp, imm8
            "48 89 5C 24 ??".to_string(), // mov [rsp+??], rbx
        ]);
        
        // Function epilogue patterns
        patterns.insert(PatternType::FunctionEpilogue, vec![
            "48 83 C4 ?? 5D C3".to_string(), // add rsp, imm8; pop rbp; ret
            "5D C3".to_string(),             // pop rbp; ret
            "C3".to_string(),                // ret
        ]);

        Self { patterns }
    }

    fn get_common_patterns(&self) -> Vec<&str> {
        self.patterns.values()
            .flatten()
            .map(|s| s.as_str())
            .collect()
    }

    fn is_function_prologue(&self, data: &[u8]) -> bool {
        if data.len() >= 4 {
            matches!(&data[0..4], 
                [0x55, 0x48, 0x89, 0xE5] | // push rbp; mov rbp, rsp
                [0x48, 0x83, 0xEC, _] |     // sub rsp, imm8
                [0x48, 0x89, 0x5C, 0x24]    // mov [rsp+??], rbx
            )
        } else {
            false
        }
    }
}

/// Errors that can occur during analysis.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Memory error: {0}")]
    MemoryError(String),
    #[error("Pattern error: {0}")]
    PatternError(#[from] PatternError),
    #[error("Scan error: {0}")]
    ScanError(String),
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calling_convention_detection() {
        let engine = AnalysisEngine::new(MemoryScanner::new().unwrap());
        
        let prologue1 = [0x55, 0x48, 0x89, 0xE5]; // push rbp; mov rbp, rsp
        assert_eq!(engine.detect_calling_convention(&prologue1), CallingConvention::Cdecl);
        
        let prologue2 = [0x48, 0x89, 0x4C, 0x24]; // mov [rsp+8], rcx
        assert_eq!(engine.detect_calling_convention(&prologue2), CallingConvention::Fastcall);
    }

    #[test]
    fn test_string_extraction() {
        let engine = AnalysisEngine::new(MemoryScanner::new().unwrap());
        let data = b"Hello World\0Some other text\0\x00\x01\x02";
        
        let strings = engine.extract_strings(data, 0x1000);
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].value, "Hello World");
        assert_eq!(strings[1].value, "Some other text");
    }

    #[test]
    fn test_pattern_database() {
        let db = PatternDatabase::new();
        let patterns = db.get_common_patterns();
        assert!(!patterns.is_empty());
        
        let prologue_data = [0x55, 0x48, 0x89, 0xE5];
        assert!(db.is_function_prologue(&prologue_data));
    }
}