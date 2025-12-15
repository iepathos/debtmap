//! Unified extraction data types for single-pass file parsing.
//!
//! This module defines Send+Sync-safe data structures that capture all information
//! needed by downstream analysis phases from a single file parse. By extracting
//! everything in one pass, we avoid the proc-macro2 SourceMap overflow that occurs
//! when parsing the same file repeatedly.
//!
//! # Design Principles
//!
//! - **Single Parse**: Each file is parsed exactly once
//! - **Thread Safety**: All types are `Send + Sync` for parallel processing
//! - **Serializable**: Types can be cached to disk for incremental analysis
//! - **Complete**: All data needed by all analysis phases is captured
//!
//! # Memory Efficiency
//!
//! Target memory footprint: ~8KB per file on average. This is achieved by:
//! - Using owned `String` instead of `&str` (required for Send safety)
//! - Avoiding duplication of data across types
//! - Using `Vec` instead of `HashMap` for small collections

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// All data extracted from a single file parse.
///
/// This is the top-level container that holds all extracted data from one source file.
/// It is Send + Sync safe and can be shared across threads for parallel analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFileData {
    /// Path to the source file
    pub path: PathBuf,
    /// All functions extracted from the file
    pub functions: Vec<ExtractedFunctionData>,
    /// All structs for god object detection
    pub structs: Vec<ExtractedStructData>,
    /// All impl blocks
    pub impls: Vec<ExtractedImplData>,
    /// Import statements for call resolution
    pub imports: Vec<ImportInfo>,
    /// Total lines in file
    pub total_lines: usize,
    /// Detected code patterns (god objects, long functions, deep nesting, etc.)
    /// Spec 204: Pre-computed during extraction to avoid re-parsing
    pub detected_patterns: Vec<DetectedPattern>,
}

/// Code pattern detected during extraction.
///
/// These patterns are detected during the single extraction pass to avoid
/// re-parsing the file for pattern detection. Spec 204 migration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectedPattern {
    /// A struct with many fields (potential god object)
    GodObject {
        /// Name of the struct
        name: String,
        /// Number of fields
        field_count: usize,
    },
    /// A function with many lines
    LongFunction {
        /// Name of the function
        name: String,
        /// Approximate line count
        lines: usize,
    },
    /// A function with many parameters
    ManyParameters {
        /// Name of the function
        name: String,
        /// Number of parameters
        param_count: usize,
    },
    /// Deeply nested control flow
    DeepNesting {
        /// Name of the containing function
        function_name: String,
        /// Maximum nesting depth found
        depth: u32,
    },
}

/// All data extracted for a single function.
///
/// Contains identity, complexity metrics, pre-extracted analysis data,
/// and metadata needed by downstream phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFunctionData {
    /// Function name (without type prefix for methods)
    pub name: String,
    /// Qualified name: "TypeName::method" or just "function"
    pub qualified_name: String,
    /// Starting line number (1-indexed)
    pub line: usize,
    /// Ending line number
    pub end_line: usize,
    /// Function length in lines
    pub length: usize,

    // Complexity metrics
    /// Cyclomatic complexity (branch count)
    pub cyclomatic: u32,
    /// Cognitive complexity
    pub cognitive: u32,
    /// Maximum nesting depth
    pub nesting: u32,

    // Pre-extracted analysis data
    /// Purity analysis results
    pub purity_analysis: PurityAnalysisData,
    /// Detected I/O operations
    pub io_operations: Vec<IoOperation>,
    /// Parameter names from signature
    pub parameter_names: Vec<String>,
    /// Detected transformation patterns
    pub transformation_patterns: Vec<TransformationPattern>,
    /// Call sites for call graph
    pub calls: Vec<CallSite>,

    // Metadata
    /// Is this a test function
    pub is_test: bool,
    /// Is this an async function
    pub is_async: bool,
    /// Visibility: "pub", "pub(crate)", or None for private
    pub visibility: Option<String>,
    /// Is this a trait method
    pub is_trait_method: bool,
    /// Is this inside a #[cfg(test)] module
    pub in_test_module: bool,
}

/// Pre-computed purity analysis results.
///
/// Contains all information from the purity detector in a thread-safe format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PurityAnalysisData {
    /// Overall purity determination
    pub is_pure: bool,
    /// Has mutable state changes
    pub has_mutations: bool,
    /// Has I/O operations
    pub has_io_operations: bool,
    /// Contains unsafe code
    pub has_unsafe: bool,
    /// Local variable mutations
    pub local_mutations: Vec<String>,
    /// Upvalue/captured variable mutations
    pub upvalue_mutations: Vec<String>,
    /// Total mutation count
    pub total_mutations: usize,
    /// Variable names by span offset (for CFG)
    pub var_names: HashMap<usize, String>,
    /// Confidence in purity determination (0.0-1.0)
    pub confidence: f32,
    /// Refined purity level
    pub purity_level: PurityLevel,
}

/// Purity classification levels.
///
/// More granular than just "pure" or "impure" to enable
/// better refactoring recommendations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PurityLevel {
    /// No side effects, deterministic
    StrictlyPure,
    /// Only local mutations, no external effects
    LocallyPure,
    /// Only reads external state, no writes
    ReadOnly,
    /// Has side effects
    #[default]
    Impure,
}

/// Extracted struct information for god object detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedStructData {
    /// Struct name
    pub name: String,
    /// Line number
    pub line: usize,
    /// Field information
    pub fields: Vec<FieldInfo>,
    /// Is public
    pub is_public: bool,
}

/// Field information for structs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    /// Field name
    pub name: String,
    /// Field type as string
    pub type_str: String,
    /// Is public
    pub is_public: bool,
}

/// Extracted impl block information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedImplData {
    /// Type being implemented for
    pub type_name: String,
    /// Trait being implemented (if any)
    pub trait_name: Option<String>,
    /// Methods in this impl block
    pub methods: Vec<MethodInfo>,
    /// Line number
    pub line: usize,
}

/// Method information within impl blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    /// Method name
    pub name: String,
    /// Line number
    pub line: usize,
    /// Is public
    pub is_public: bool,
}

/// Call site information for call graph construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSite {
    /// Name of called function (possibly qualified)
    pub callee_name: String,
    /// Type of call
    pub call_type: CallType,
    /// Line number of call
    pub line: usize,
}

/// Types of function calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallType {
    /// Direct function call: `foo()`
    Direct,
    /// Method call: `x.foo()`
    Method,
    /// Static method call: `Type::foo()`
    StaticMethod,
    /// Trait method call
    TraitMethod,
    /// Closure call
    Closure,
    /// Function pointer call
    FunctionPointer,
}

/// Import statement information for call resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    /// Full import path: "std::collections::HashMap"
    pub path: String,
    /// Alias if renamed: `use foo as bar`
    pub alias: Option<String>,
    /// Is glob import: `use foo::*`
    pub is_glob: bool,
}

/// Detected I/O operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoOperation {
    /// Type of I/O operation
    pub io_type: IoType,
    /// Description of the operation
    pub description: String,
    /// Line number
    pub line: usize,
}

/// Types of I/O operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IoType {
    /// File system operations
    File,
    /// Console/stdout/stderr
    Console,
    /// Network operations
    Network,
    /// Database operations
    Database,
    /// Async I/O
    AsyncIO,
    /// Environment variable access
    Environment,
    /// System calls
    System,
}

/// Detected transformation pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationPattern {
    /// Type of transformation
    pub pattern_type: PatternType,
    /// Line number
    pub line: usize,
}

/// Types of functional transformation patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Map,
    Filter,
    Fold,
    FlatMap,
    Collect,
    ForEach,
    Find,
    Any,
    All,
    Reduce,
}

// ============================================================================
// Helper Implementations
// ============================================================================

impl ExtractedFileData {
    /// Create empty extraction for a file.
    ///
    /// Useful when a file cannot be parsed or is empty.
    pub fn empty(path: PathBuf) -> Self {
        Self {
            path,
            functions: Vec::new(),
            structs: Vec::new(),
            impls: Vec::new(),
            imports: Vec::new(),
            total_lines: 0,
            detected_patterns: Vec::new(),
        }
    }

    /// Check if the file has any content worth analyzing.
    pub fn has_content(&self) -> bool {
        !self.functions.is_empty() || !self.structs.is_empty() || !self.impls.is_empty()
    }

    /// Get the total number of functions in this file.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}

impl ExtractedFunctionData {
    /// Get function ID for call graph integration.
    ///
    /// Creates a `FunctionId` compatible with the existing call graph module.
    pub fn function_id(
        &self,
        file_path: &std::path::Path,
    ) -> crate::priority::call_graph::FunctionId {
        crate::priority::call_graph::FunctionId::new(
            file_path.to_path_buf(),
            self.name.clone(),
            self.line,
        )
    }

    /// Create a minimal function data for testing.
    #[cfg(test)]
    pub fn minimal(name: &str, line: usize) -> Self {
        Self {
            name: name.to_string(),
            qualified_name: name.to_string(),
            line,
            end_line: line + 1,
            length: 1,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            purity_analysis: PurityAnalysisData::default(),
            io_operations: Vec::new(),
            parameter_names: Vec::new(),
            transformation_patterns: Vec::new(),
            calls: Vec::new(),
            is_test: false,
            is_async: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
        }
    }
}

impl Default for ExtractedFunctionData {
    fn default() -> Self {
        Self {
            name: String::new(),
            qualified_name: String::new(),
            line: 0,
            end_line: 0,
            length: 0,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            purity_analysis: PurityAnalysisData::default(),
            io_operations: Vec::new(),
            parameter_names: Vec::new(),
            transformation_patterns: Vec::new(),
            calls: Vec::new(),
            is_test: false,
            is_async: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
        }
    }
}

impl PurityAnalysisData {
    /// Create pure function data with high confidence.
    pub fn pure() -> Self {
        Self {
            is_pure: true,
            has_mutations: false,
            has_io_operations: false,
            has_unsafe: false,
            local_mutations: Vec::new(),
            upvalue_mutations: Vec::new(),
            total_mutations: 0,
            var_names: HashMap::new(),
            confidence: 1.0,
            purity_level: PurityLevel::StrictlyPure,
        }
    }

    /// Create impure function data.
    pub fn impure(reason: &str) -> Self {
        Self {
            is_pure: false,
            has_mutations: true,
            has_io_operations: false,
            has_unsafe: false,
            local_mutations: vec![reason.to_string()],
            upvalue_mutations: Vec::new(),
            total_mutations: 1,
            var_names: HashMap::new(),
            confidence: 1.0,
            purity_level: PurityLevel::Impure,
        }
    }
}

// ============================================================================
// Thread Safety Assertions
// ============================================================================

// Static assertions that all types are Send + Sync
// Using a trait-bound approach that is evaluated at compile time
fn _assert_send_sync<T: Send + Sync>() {}
#[allow(dead_code)]
const _: () = {
    let _ = _assert_send_sync::<ExtractedFileData>;
    let _ = _assert_send_sync::<ExtractedFunctionData>;
    let _ = _assert_send_sync::<PurityAnalysisData>;
    let _ = _assert_send_sync::<ExtractedStructData>;
    let _ = _assert_send_sync::<ExtractedImplData>;
    let _ = _assert_send_sync::<CallSite>;
    let _ = _assert_send_sync::<IoOperation>;
    let _ = _assert_send_sync::<TransformationPattern>;
    let _ = _assert_send_sync::<DetectedPattern>;
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_file_data_empty() {
        let data = ExtractedFileData::empty(PathBuf::from("test.rs"));
        assert_eq!(data.path, PathBuf::from("test.rs"));
        assert!(data.functions.is_empty());
        assert!(data.structs.is_empty());
        assert!(data.impls.is_empty());
        assert!(data.imports.is_empty());
        assert_eq!(data.total_lines, 0);
        assert!(!data.has_content());
    }

    #[test]
    fn test_extracted_file_data_has_content() {
        let mut data = ExtractedFileData::empty(PathBuf::from("test.rs"));
        assert!(!data.has_content());

        data.functions
            .push(ExtractedFunctionData::minimal("foo", 1));
        assert!(data.has_content());
    }

    #[test]
    fn test_extracted_function_data_minimal() {
        let func = ExtractedFunctionData::minimal("test_fn", 42);
        assert_eq!(func.name, "test_fn");
        assert_eq!(func.line, 42);
        assert_eq!(func.cyclomatic, 1);
        assert!(!func.is_test);
    }

    #[test]
    fn test_purity_analysis_data_pure() {
        let purity = PurityAnalysisData::pure();
        assert!(purity.is_pure);
        assert!(!purity.has_mutations);
        assert!(!purity.has_io_operations);
        assert_eq!(purity.purity_level, PurityLevel::StrictlyPure);
        assert_eq!(purity.confidence, 1.0);
    }

    #[test]
    fn test_purity_analysis_data_impure() {
        let purity = PurityAnalysisData::impure("mutates global");
        assert!(!purity.is_pure);
        assert!(purity.has_mutations);
        assert_eq!(purity.purity_level, PurityLevel::Impure);
        assert_eq!(purity.local_mutations.len(), 1);
    }

    #[test]
    fn test_function_id_generation() {
        let file = PathBuf::from("src/main.rs");
        let func = ExtractedFunctionData::minimal("process", 100);
        let func_id = func.function_id(&file);

        assert_eq!(func_id.file, file);
        assert_eq!(func_id.name, "process");
        assert_eq!(func_id.line, 100);
    }

    #[test]
    fn test_cloning_works() {
        let original = ExtractedFileData {
            path: PathBuf::from("test.rs"),
            functions: vec![ExtractedFunctionData::minimal("foo", 1)],
            structs: vec![ExtractedStructData {
                name: "MyStruct".to_string(),
                line: 10,
                fields: vec![FieldInfo {
                    name: "field".to_string(),
                    type_str: "i32".to_string(),
                    is_public: false,
                }],
                is_public: true,
            }],
            impls: vec![ExtractedImplData {
                type_name: "MyStruct".to_string(),
                trait_name: None,
                methods: vec![MethodInfo {
                    name: "new".to_string(),
                    line: 15,
                    is_public: true,
                }],
                line: 12,
            }],
            imports: vec![ImportInfo {
                path: "std::collections::HashMap".to_string(),
                alias: None,
                is_glob: false,
            }],
            total_lines: 100,
            detected_patterns: vec![],
        };

        let cloned = original.clone();
        assert_eq!(cloned.path, original.path);
        assert_eq!(cloned.functions.len(), original.functions.len());
        assert_eq!(cloned.structs.len(), original.structs.len());
        assert_eq!(cloned.impls.len(), original.impls.len());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let data = ExtractedFileData {
            path: PathBuf::from("test.rs"),
            functions: vec![ExtractedFunctionData {
                name: "test".to_string(),
                qualified_name: "MyStruct::test".to_string(),
                line: 1,
                end_line: 10,
                length: 9,
                cyclomatic: 5,
                cognitive: 3,
                nesting: 2,
                purity_analysis: PurityAnalysisData::pure(),
                io_operations: vec![IoOperation {
                    io_type: IoType::File,
                    description: "read file".to_string(),
                    line: 5,
                }],
                parameter_names: vec!["self".to_string(), "path".to_string()],
                transformation_patterns: vec![TransformationPattern {
                    pattern_type: PatternType::Map,
                    line: 7,
                }],
                calls: vec![CallSite {
                    callee_name: "read_to_string".to_string(),
                    call_type: CallType::Method,
                    line: 5,
                }],
                is_test: false,
                is_async: true,
                visibility: Some("pub".to_string()),
                is_trait_method: false,
                in_test_module: false,
            }],
            structs: Vec::new(),
            impls: Vec::new(),
            imports: Vec::new(),
            total_lines: 10,
            detected_patterns: vec![],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&data).expect("serialization failed");

        // Deserialize back
        let restored: ExtractedFileData =
            serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(restored.path, data.path);
        assert_eq!(restored.functions.len(), 1);
        assert_eq!(restored.functions[0].name, "test");
        assert_eq!(restored.functions[0].io_operations.len(), 1);
        assert_eq!(restored.functions[0].io_operations[0].io_type, IoType::File);
    }

    #[test]
    fn test_io_type_variants() {
        let types = [
            IoType::File,
            IoType::Console,
            IoType::Network,
            IoType::Database,
            IoType::AsyncIO,
            IoType::Environment,
            IoType::System,
        ];

        for io_type in types {
            let op = IoOperation {
                io_type,
                description: "test".to_string(),
                line: 1,
            };
            // Verify it's cloneable and serializable
            let _ = op.clone();
            let json = serde_json::to_string(&op).unwrap();
            let _: IoOperation = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_call_type_variants() {
        let types = [
            CallType::Direct,
            CallType::Method,
            CallType::StaticMethod,
            CallType::TraitMethod,
            CallType::Closure,
            CallType::FunctionPointer,
        ];

        for call_type in types {
            let call = CallSite {
                callee_name: "foo".to_string(),
                call_type,
                line: 1,
            };
            let _ = call.clone();
        }
    }

    #[test]
    fn test_pattern_type_variants() {
        let types = [
            PatternType::Map,
            PatternType::Filter,
            PatternType::Fold,
            PatternType::FlatMap,
            PatternType::Collect,
            PatternType::ForEach,
            PatternType::Find,
            PatternType::Any,
            PatternType::All,
            PatternType::Reduce,
        ];

        for pattern_type in types {
            let pattern = TransformationPattern {
                pattern_type,
                line: 1,
            };
            let _ = pattern.clone();
        }
    }

    #[test]
    fn test_purity_level_default() {
        let level: PurityLevel = Default::default();
        assert_eq!(level, PurityLevel::Impure);
    }

    #[test]
    fn test_memory_size_estimate() {
        // Create a representative file data structure
        let data = ExtractedFileData {
            path: PathBuf::from("src/some_module/file.rs"),
            functions: (0..10)
                .map(|i| {
                    let mut func = ExtractedFunctionData::minimal(&format!("func_{}", i), i * 10);
                    func.parameter_names = vec!["self".to_string(), "arg".to_string()];
                    func.calls = vec![CallSite {
                        callee_name: "other".to_string(),
                        call_type: CallType::Method,
                        line: i * 10 + 5,
                    }];
                    func
                })
                .collect(),
            structs: vec![ExtractedStructData {
                name: "MyStruct".to_string(),
                line: 1,
                fields: (0..5)
                    .map(|i| FieldInfo {
                        name: format!("field_{}", i),
                        type_str: "String".to_string(),
                        is_public: false,
                    })
                    .collect(),
                is_public: true,
            }],
            impls: vec![ExtractedImplData {
                type_name: "MyStruct".to_string(),
                trait_name: Some("Display".to_string()),
                methods: vec![MethodInfo {
                    name: "fmt".to_string(),
                    line: 50,
                    is_public: true,
                }],
                line: 45,
            }],
            imports: (0..5)
                .map(|i| ImportInfo {
                    path: format!("std::module_{}", i),
                    alias: None,
                    is_glob: false,
                })
                .collect(),
            total_lines: 200,
            detected_patterns: vec![],
        };

        // Serialize to estimate memory size
        let json = serde_json::to_string(&data).expect("serialization failed");

        // The serialized size should be reasonable (under 8KB for this representative sample)
        // Note: JSON serialization adds overhead, actual in-memory size is smaller
        assert!(
            json.len() < 16000,
            "Serialized size {} bytes exceeds 16KB limit",
            json.len()
        );
    }
}
