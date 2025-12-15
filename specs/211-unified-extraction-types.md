---
number: 211
title: Unified Extraction Data Types
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-01-14
---

# Specification 211: Unified Extraction Data Types

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently parses the same source files multiple times across different analysis phases:
- `populate_io_operations` parses per-function (~20,000 parses for large codebases)
- `extract_variable_deps` parses per-function
- `populate_data_transformations` parses per-function
- Call graph building parses all files
- Metrics extraction parses all files
- God object detection parses all files

For a codebase with 20,000 functions across 2,000 files, this results in ~86,000 parses instead of 2,000, causing:
1. proc-macro2 SourceMap overflow (crashes on large codebases like zed)
2. 43x slower analysis than necessary

The solution is to parse each file exactly once and extract ALL needed data into Send+Sync-safe structures that can be passed to all analysis phases.

## Objective

Define the core data types for the unified extraction architecture that capture all data needed by downstream analysis phases in a thread-safe, reusable format.

## Requirements

### Functional Requirements

1. **ExtractedFileData**: Top-level container for all data extracted from a single file
   - Path and metadata
   - All functions with their extracted data
   - All structs and impl blocks (for god object detection)
   - All imports (for call resolution)
   - Total line count

2. **ExtractedFunctionData**: All data for a single function
   - Identity: name, qualified name, line number, end line
   - Complexity: cyclomatic, cognitive, nesting depth, length
   - Purity analysis data (pre-computed)
   - I/O operations detected
   - Parameter names for dependency tracking
   - Transformation patterns (map, filter, fold)
   - Call sites for call graph
   - Metadata: is_test, visibility, is_async, is_trait_method

3. **PurityAnalysisData**: Pre-computed purity analysis results
   - is_pure flag
   - Mutation information (local, upvalue)
   - I/O operation presence
   - Unsafe code presence
   - Variable name mapping
   - Confidence score
   - Purity level enum

4. **ExtractedStructData**: Struct information for god object detection
   - Name and line number
   - Fields with types
   - Visibility

5. **ExtractedImplData**: Impl block information
   - Type name and optional trait name
   - Method summaries
   - Line number

6. **CallSite**: Function call information for call graph
   - Callee name (possibly qualified)
   - Call type (direct, method, trait)
   - Line number

7. **IoOperation**: I/O operation for data flow analysis
   - Operation type (File, Console, Network, Database, AsyncIO)
   - Description

8. **TransformationPattern**: Functional transformation pattern
   - Pattern type (Map, Filter, Fold, FlatMap, etc.)
   - Source expression info

### Non-Functional Requirements

- All types must implement `Send + Sync` for parallel processing
- All types must implement `Clone` for sharing across phases
- All types must implement `Debug` for diagnostics
- Memory footprint should be ~8KB per file average
- Types should be serializable (derive Serialize/Deserialize) for caching

## Acceptance Criteria

- [ ] `ExtractedFileData` struct defined with all required fields
- [ ] `ExtractedFunctionData` struct defined with all required fields
- [ ] `PurityAnalysisData` struct captures all purity detector output
- [ ] `ExtractedStructData` and `ExtractedImplData` capture god object needs
- [ ] `CallSite` captures all call graph edge information
- [ ] `IoOperation` enum matches existing I/O detector output
- [ ] `TransformationPattern` enum matches existing pattern detector output
- [ ] All types implement Send + Sync + Clone + Debug
- [ ] All types derive Serialize/Deserialize
- [ ] Unit tests verify type construction and cloning
- [ ] Memory size test validates ~8KB average per file

## Technical Details

### Module Location

```
src/extraction/
├── mod.rs           # Public exports
└── types.rs         # All type definitions (this spec)
```

### Type Definitions

```rust
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// All data extracted from a single file parse.
/// This is Send + Sync safe and can be shared across threads.
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
}

/// All data extracted for a single function.
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
```

### Conversion Traits

```rust
impl ExtractedFileData {
    /// Create empty extraction for a file
    pub fn empty(path: PathBuf) -> Self {
        Self {
            path,
            functions: Vec::new(),
            structs: Vec::new(),
            impls: Vec::new(),
            imports: Vec::new(),
            total_lines: 0,
        }
    }
}

impl ExtractedFunctionData {
    /// Get function ID for call graph
    pub fn function_id(&self, file_path: &PathBuf) -> crate::priority::call_graph::FunctionId {
        crate::priority::call_graph::FunctionId::new(
            file_path.clone(),
            self.name.clone(),
            self.line,
        )
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: None (new module)
- **External Dependencies**: serde for serialization

## Testing Strategy

- **Unit Tests**: Test type construction, cloning, serialization roundtrip
- **Memory Tests**: Verify memory footprint with representative data
- **Integration Tests**: Types can hold real extracted data from test files

## Documentation Requirements

- **Code Documentation**: Comprehensive rustdoc on all public types
- **Architecture Updates**: Document new extraction module in ARCHITECTURE.md

## Implementation Notes

- Use `#[derive(Default)]` where sensible for builder patterns
- Consider using `SmallVec` for small collections (parameter_names, calls) to reduce allocations
- Ensure all string fields use owned `String` not `&str` for Send safety

## Migration and Compatibility

No migration needed - these are new types. Adapters (spec 214) will convert to existing types.
