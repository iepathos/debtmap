---
number: 202
title: Migrate Duplicate File Parsing to UnifiedFileExtractor
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-15
---

# Specification 202: Migrate Duplicate File Parsing to UnifiedFileExtractor

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `UnifiedFileExtractor` was created to parse files exactly once and extract all data needed by downstream analysis phases. However, **50+ locations** in the codebase still call `syn::parse_file` directly, defeating the purpose of single-pass extraction.

### Current State

**Files using UnifiedFileExtractor (4 files)**:
- `src/builders/parallel_unified_analysis.rs`
- `src/commands/analyze/project_analysis.rs`
- `src/extraction/adapters/mod.rs`
- `src/extraction/extractor.rs`

**Files still calling syn::parse_file directly (partial list)**:

| File | Usage |
|------|-------|
| `src/builders/parallel_call_graph.rs` | Parses files for call graph building |
| `src/builders/validated_analysis.rs` | Parses files for validation |
| `src/builders/call_graph.rs` | Legacy call graph builder |
| `src/complexity/effects_wrappers.rs` | Multiple parse calls for complexity analysis |
| `src/analyzers/file_analyzer.rs` | Parses files for analysis |
| `src/analyzers/batch.rs` | Batch analysis parsing |
| `src/analyzers/enhanced_analyzer.rs` | Enhanced analysis parsing |
| `src/analysis/module_structure/rust_analyzer.rs` | Module structure analysis |
| `src/analysis/module_structure/facade.rs` | Module facade analysis |
| `src/organization/struct_ownership.rs` | Struct ownership analysis |
| `src/organization/dependency_analyzer.rs` | Dependency analysis |
| `src/organization/cohesion_calculator.rs` | Cohesion calculation |
| `src/organization/call_graph_cohesion.rs` | Call graph cohesion |
| `src/organization/god_object/detector.rs` | God object detection |
| `src/organization/codebase_type_analyzer.rs` | Codebase type analysis |
| `src/context/detector.rs` | Context detection |
| `src/context/async_detector.rs` | Async detection |

### Problems with Current State

1. **Duplicate parsing**: Same file may be parsed multiple times in a single analysis run
2. **SourceMap overflow**: proc-macro2's SourceMap can overflow on large codebases when parsing repeatedly
3. **Inconsistent data**: Different parsers may have slightly different error handling
4. **Performance waste**: Parsing is expensive; doing it multiple times is wasteful
5. **Maintenance burden**: Changes to extraction must be replicated across parsers

### UnifiedFileExtractor Capabilities

The extractor already extracts:
- All functions with complexity metrics (cyclomatic, cognitive, nesting)
- Purity analysis
- I/O operations
- Parameter names
- Transformation patterns
- Call sites
- Struct definitions
- Impl blocks
- Import statements
- Test function detection

## Objective

Migrate all direct `syn::parse_file` calls to use `UnifiedFileExtractor` or receive pre-extracted data, ensuring files are parsed exactly once per analysis run.

## Requirements

### Functional Requirements

1. **No direct syn::parse_file in analysis code**: All analysis should use extracted data
2. **Extractor provides all needed data**: Extend extractor if any caller needs data not currently extracted
3. **Batch processing supported**: `UnifiedFileExtractor::extract_batch` for efficient multi-file processing
4. **Legacy compatibility**: Provide adapter functions for code that needs `syn::File` AST

### Non-Functional Requirements

1. **Performance improvement**: Overall analysis should be faster due to single-pass parsing
2. **Memory efficiency**: SourceMap resets prevent overflow
3. **Gradual migration**: Can be done incrementally, module by module

## Acceptance Criteria

- [ ] No production code calls `syn::parse_file` directly (tests excluded)
- [ ] All analysis uses `ExtractedFileData` or receives data through adapters
- [ ] Large codebase analysis (10k+ files) doesn't overflow SourceMap
- [ ] Overall analysis time improves or stays constant
- [ ] All existing tests pass
- [ ] Extractor is extended to provide any missing data

## Technical Details

### Migration Strategy

#### Phase 1: Identify Missing Data

Review each caller of `syn::parse_file` to identify what data they extract:

1. **Call graph builders** - Need function calls, may need raw AST for advanced resolution
2. **God object detector** - Need struct/impl data (already extracted)
3. **Module structure** - Need import/module hierarchy
4. **Purity analysis** - Already extracted
5. **Complexity** - Already extracted

#### Phase 2: Extend Extractor if Needed

If callers need data not in `ExtractedFileData`, extend the extractor:

```rust
pub struct ExtractedFileData {
    // Existing fields...

    // Potential additions:
    pub module_declarations: Vec<ModuleDeclaration>,  // For module structure
    pub type_aliases: Vec<TypeAlias>,                  // For type analysis
    pub const_definitions: Vec<ConstDef>,              // For constant analysis
}
```

#### Phase 3: Create Adapter Layer

For code that genuinely needs raw AST access, provide an adapter:

```rust
/// Adapter for code that needs raw AST access.
/// DEPRECATED: Prefer using ExtractedFileData directly.
pub struct AstAccessAdapter {
    extracted: ExtractedFileData,
    ast: syn::File,  // Cached AST, only created if needed
}

impl AstAccessAdapter {
    pub fn from_extracted(extracted: ExtractedFileData, content: &str) -> Result<Self> {
        // Parse only if raw AST access is requested
    }
}
```

#### Phase 4: Migrate Callers

For each file with direct `syn::parse_file`:

1. **If using parallel_unified_analysis**: Receive `ExtractedFileData` from pipeline
2. **If standalone analysis**: Use `UnifiedFileExtractor::extract`
3. **If batch processing**: Use `UnifiedFileExtractor::extract_batch`

### Migration Examples

#### Before (direct parsing):

```rust
// src/organization/god_object/detector.rs
pub fn analyze_file(content: &str) -> Vec<GodObjectAnalysis> {
    let ast = syn::parse_file(content).ok()?;
    // ... extract structs, impls, methods manually
}
```

#### After (using extractor):

```rust
// src/organization/god_object/detector.rs
pub fn analyze_file(data: &ExtractedFileData) -> Vec<GodObjectAnalysis> {
    // Use data.structs, data.impls, data.functions directly
}

// Or with path for standalone use:
pub fn analyze_file_standalone(path: &Path, content: &str) -> Result<Vec<GodObjectAnalysis>> {
    let data = UnifiedFileExtractor::extract(path, content)?;
    Ok(analyze_file(&data))
}
```

### Files to Migrate (Priority Order)

**High Priority** (frequently called, high impact):
1. `src/builders/parallel_call_graph.rs`
2. `src/builders/validated_analysis.rs`
3. `src/analyzers/batch.rs`
4. `src/organization/god_object/detector.rs`

**Medium Priority** (less frequently called):
5. `src/complexity/effects_wrappers.rs`
6. `src/analyzers/file_analyzer.rs`
7. `src/analyzers/enhanced_analyzer.rs`
8. `src/analysis/module_structure/*.rs`

**Low Priority** (specialized, rarely called):
9. `src/organization/struct_ownership.rs`
10. `src/organization/dependency_analyzer.rs`
11. `src/context/*.rs`

### Data Flow Architecture

```
┌─────────────────────┐
│  File Content       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ UnifiedFileExtractor│  ← Single parse point
│   ::extract()       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  ExtractedFileData  │  ← Immutable extracted data
└─────────┬───────────┘
          │
    ┌─────┴─────┬──────────┬──────────┐
    ▼           ▼          ▼          ▼
┌───────┐  ┌────────┐  ┌────────┐  ┌────────┐
│ Call  │  │Comple- │  │ God    │  │ Module │
│ Graph │  │xity    │  │ Object │  │ Struct │
└───────┘  └────────┘  └────────┘  └────────┘
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 15+ files with direct parsing
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test extractor provides all required data
- **Integration Tests**: Run full analysis pipeline, verify no SourceMap overflow
- **Performance Tests**: Benchmark before/after migration
- **Regression Tests**: Ensure analysis results unchanged

### Performance Benchmark

```rust
#[bench]
fn bench_analysis_before_migration(b: &mut Bencher) {
    // Measure with direct parsing
}

#[bench]
fn bench_analysis_after_migration(b: &mut Bencher) {
    // Measure with extractor
}
```

## Documentation Requirements

- **Code Documentation**: Document `UnifiedFileExtractor` as the single parsing entry point
- **Architecture Updates**: Update ARCHITECTURE.md with data flow diagram
- **Migration Guide**: Document how to migrate existing code

## Implementation Notes

- Start with high-priority files to see immediate benefits
- Some test files may still use `syn::parse_file` directly - this is acceptable
- Consider deprecating direct `syn::parse_file` with clippy lint

## Migration and Compatibility

- **Breaking Changes**: Functions that took raw `&str` content may now take `&ExtractedFileData`
- **Deprecation Path**: Mark old APIs as deprecated, provide migration path
- **Version Strategy**: Can be done incrementally across multiple releases
