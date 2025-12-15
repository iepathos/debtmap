---
number: 204
title: Complete File Parsing Migration to UnifiedFileExtractor
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-15
---

# Specification 204: Complete File Parsing Migration to UnifiedFileExtractor

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None (continues work from spec 202)

## Context

Spec 202 partially migrated file parsing to `UnifiedFileExtractor`. The high-priority production paths were migrated, and secondary paths now call `reset_span_locations()` after parsing. However, ~15 production `syn::parse_file` calls remain unmigrated.

### Current State

**Already migrated (spec 202)**:
- `src/builders/validated_analysis.rs` ✅
- `src/analyzers/batch.rs` ✅

**Remaining production calls** (excluding test code and extractor itself):

| File | Line | Context |
|------|------|---------|
| `codebase_type_analyzer.rs` | 195 | Production analysis |
| `enhanced_analyzer.rs` | 92 | `parse_rust_file` function |
| `macro_definition_collector.rs` | 139, 148, 157 | Macro detection |
| `file_analyzer.rs` | 154 | File analysis fallback |
| `module_structure/rust_analyzer.rs` | 27 | Module structure parsing |
| `builders/call_graph.rs` | 147 | Legacy call graph |
| `builders/parallel_call_graph.rs` | 242, 292 | Call graph building |
| `complexity/effects_wrappers.rs` | 145, 204, 236, 248, 260 | Effect-based analysis |

### Why Complete This

1. **Consistency**: All parsing should go through one path
2. **SourceMap safety**: Centralized reset prevents overflow on large codebases
3. **Future optimization**: Single parse point enables caching
4. **Maintenance**: One parsing path to maintain and improve

## Objective

Migrate all remaining production `syn::parse_file` calls to use `UnifiedFileExtractor` or receive pre-extracted data, achieving 100% migration of production code.

## Requirements

### Functional Requirements

1. **Migrate remaining files**: Each file listed above must use extractor or receive extracted data
2. **Preserve functionality**: All analysis must produce identical results
3. **Handle AST-dependent code**: Some code needs raw AST - provide adapter pattern

### Non-Functional Requirements

1. **No performance regression**: Single-pass parsing should improve or maintain speed
2. **Backward compatibility**: Public APIs unchanged
3. **Test code excluded**: Test code may continue using direct parsing

## Acceptance Criteria

- [ ] `codebase_type_analyzer.rs` uses UnifiedFileExtractor or receives ExtractedFileData
- [ ] `enhanced_analyzer.rs::parse_rust_file` delegates to extractor
- [ ] `macro_definition_collector.rs` uses extractor with SourceMap reset
- [ ] `file_analyzer.rs` uses extractor for Rust files
- [ ] `module_structure/rust_analyzer.rs` uses extractor
- [ ] `builders/call_graph.rs` uses extractor or receives extracted data
- [ ] `builders/parallel_call_graph.rs` uses extractor with batch processing
- [ ] `complexity/effects_wrappers.rs` uses extractor for all effect functions
- [ ] Zero production `syn::parse_file` calls outside extractor itself
- [ ] All tests pass
- [ ] Analysis on large codebase (10k+ files) completes without SourceMap overflow

## Technical Details

### Implementation Approach

#### Pattern 1: Direct Replacement (Simple Cases)

For code that just needs to check parseability:

```rust
// Before:
let ast = syn::parse_file(&content)?;

// After:
let _data = UnifiedFileExtractor::extract(path, &content)?;
```

#### Pattern 2: Using Extracted Data

For code that extracts function/struct information:

```rust
// Before:
let ast = syn::parse_file(&content)?;
for item in &ast.items {
    if let syn::Item::Fn(func) = item { /* ... */ }
}

// After:
let data = UnifiedFileExtractor::extract(path, &content)?;
for func in &data.functions {
    // Use func.name, func.line, etc.
}
```

#### Pattern 3: Raw AST Access (When Needed)

For code that genuinely needs AST traversal not covered by extractor:

```rust
// Option A: Extend ExtractedFileData to include needed data
// (Preferred - add to extractor once, use everywhere)

// Option B: Parse with immediate SourceMap reset
let ast = syn::parse_file(&content)?;
let result = analyze_ast(&ast);
crate::core::parsing::reset_span_locations();
result
```

### File-by-File Migration

#### 1. `codebase_type_analyzer.rs:195`

```rust
// Current:
let ast = syn::parse_file(&content)
    .map_err(|e| format!("Parse error: {}", e))?;

// Migrated:
let data = UnifiedFileExtractor::extract(path, &content)
    .map_err(|e| format!("Parse error: {}", e))?;
// Use data.structs, data.functions, data.impls
```

#### 2. `enhanced_analyzer.rs:92`

```rust
// Current:
pub fn parse_rust_file(content: &str) -> Result<syn::File, syn::Error> {
    syn::parse_file(content)
}

// Migrated - deprecate this function, callers should use:
pub fn parse_rust_file(path: &Path, content: &str) -> Result<ExtractedFileData, anyhow::Error> {
    UnifiedFileExtractor::extract(path, content)
}
```

#### 3. `macro_definition_collector.rs:139,148,157`

Macro detection may need raw AST. Options:
- Extend extractor to collect macro definitions
- Use Pattern 3 (parse with reset)

```rust
// Add to ExtractedFileData:
pub macro_definitions: Vec<MacroInfo>,

// Or use parse-with-reset pattern
```

#### 4. `file_analyzer.rs:154`

```rust
// Current:
if let Ok(ast) = syn::parse_file(content) {
    // analyze
}

// Migrated:
if let Ok(data) = UnifiedFileExtractor::extract(path, content) {
    // Use data.functions, data.structs, etc.
}
```

#### 5. `module_structure/rust_analyzer.rs:27`

```rust
// Current:
let result = match syn::parse_file(content) {
    Ok(ast) => analyze_module_structure(&ast),
    Err(e) => /* ... */
};

// Migrated - may need to extend extractor for module structure
// or use parse-with-reset for this specialized analysis
```

#### 6. `builders/call_graph.rs:147`

```rust
// Current:
if let Ok(parsed) = syn::parse_file(&content) {
    extract_calls(&parsed)
}

// Migrated:
if let Ok(data) = UnifiedFileExtractor::extract(path, &content) {
    // Use data.functions[].calls
}
```

#### 7. `builders/parallel_call_graph.rs:242,292`

```rust
// Current:
let parsed = syn::parse_file(content).ok()?;

// Migrated - use batch extraction for parallelism:
// In the parallel processing loop, collect files first, then:
let results = UnifiedFileExtractor::extract_batch(&files, 200);
```

#### 8. `complexity/effects_wrappers.rs`

Multiple calls - all effect functions should:
1. Use extractor for standard analysis
2. For pattern detection (needs AST), extend extractor or use parse-with-reset

### Extractor Extensions Needed

May need to add to `ExtractedFileData`:

```rust
pub struct ExtractedFileData {
    // Existing fields...

    // New fields for complete migration:
    pub macro_definitions: Vec<MacroDefinition>,
    pub module_declarations: Vec<ModuleDeclaration>,
    pub patterns: Vec<DetectedPattern>,  // Move pattern detection into extractor
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: All files listed in acceptance criteria
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Each migrated file maintains its existing tests
- **Integration Tests**: Run full analysis pipeline on test codebases
- **Stress Test**: Analyze large codebase (Zed, rustc) to verify no SourceMap overflow
- **Regression Tests**: Compare analysis output before/after migration

## Documentation Requirements

- **Code Documentation**: Update module docs to reference single parsing path
- **Deprecation**: Mark any deprecated parsing functions

## Implementation Notes

- Start with simplest files (direct replacement pattern)
- Extend extractor as needed for complex cases
- Pattern detection is the most complex - may need separate handling
- Consider making `parse_rust_file` in enhanced_analyzer.rs deprecated

## Migration and Compatibility

- **Breaking Changes**: None for public APIs
- **Internal Changes**: Functions may take `ExtractedFileData` instead of raw content
- **Deprecation Path**: Old parsing functions marked deprecated, removed in future version
