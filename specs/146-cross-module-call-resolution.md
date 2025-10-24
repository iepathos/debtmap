---
number: 146
title: Cross-Module Call Resolution Enhancement
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-10-24
---

# Specification 146: Cross-Module Call Resolution Enhancement

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The call graph currently fails to resolve many cross-module function calls, resulting in functions incorrectly showing "0 callers" when they have legitimate callers in other modules. This significantly undermines the accuracy of the dependency scoring and risk analysis.

**Current Issue Example:**
```rust
// src/io/writers/enhanced_markdown/health_writer.rs:160
pub fn write_quick_wins_section<W: Write>(writer: &mut W, quick_wins: &QuickWins) -> Result<()>

// Called from: src/io/writers/enhanced_markdown/mod.rs:126
write_quick_wins_section(&mut self.writer, &summary.quick_wins)?;

// But debtmap reports: "Dependency Score: 5.0 × 20% = 1.00 (0 callers)"
```

The issue affects:
- Standalone functions called from methods in other files
- Generic functions with type parameters
- Functions accessed through `use` statements
- Re-exported functions

## Objective

Enhance the call graph's cross-module call resolution to accurately detect and record function calls between different files and modules, ensuring that dependency counts reflect actual usage patterns in the codebase.

## Requirements

### Functional Requirements

1. **Import-Aware Resolution**
   - Parse and track all `use` statements in each file
   - Resolve simple function names to their full qualified paths using imports
   - Handle glob imports (`use module::*`)
   - Support re-exports and pub use declarations

2. **Module Hierarchy Resolution**
   - Infer module structure from file system layout
   - Build complete module path for each function
   - Support nested module hierarchies (mod.rs, lib.rs patterns)
   - Handle both inline modules (`mod foo { }`) and file-based modules

3. **Enhanced PathResolver**
   - Extend PathResolver to handle more import patterns
   - Add fallback matching using module hierarchies
   - Support relative imports (self::, super::, crate::)
   - Handle conditional compilation imports (#[cfg])

4. **Generic Function Matching**
   - Strip type parameters for matching (match `foo<T>` to `foo`)
   - Support turbofish syntax (`::<>`) in call sites
   - Handle trait bounds in generic functions
   - Match generic impls to concrete types when possible

### Non-Functional Requirements

1. **Performance**: Resolution should add < 10% overhead to call graph building
2. **Accuracy**: Resolve >95% of valid cross-module calls
3. **Maintainability**: Clear separation of concerns between resolution strategies
4. **Testability**: Comprehensive test coverage for each resolution pattern

## Acceptance Criteria

- [ ] Functions called from other modules show correct caller counts
- [ ] `write_quick_wins_section` example shows 1 caller (not 0)
- [ ] Generic functions like `extract_call_graph<T>` are correctly matched
- [ ] Glob imports (`use module::*`) correctly resolve function calls
- [ ] Re-exported functions (`pub use`) are correctly attributed
- [ ] Module hierarchy is correctly inferred for all supported patterns
- [ ] PathResolver handles relative imports (self::, super::, crate::)
- [ ] Resolution success rate >95% for debtmap's own codebase
- [ ] Integration tests cover cross-module call scenarios
- [ ] Performance overhead < 10% compared to baseline

## Technical Details

### Implementation Approach

1. **Phase 1: Enhanced Import Tracking**
   - Extend `ImportMap` to capture all import types
   - Add support for glob imports and re-exports
   - Track import aliases and renaming
   - Build reverse lookup: simple name → qualified paths

2. **Phase 2: Improved PathResolver**
   - Add fallback resolution strategies in priority order:
     1. Exact qualified path match
     2. Import-based resolution (using ImportMap)
     3. Module hierarchy search (same-module preference)
     4. Fuzzy matching with type parameter stripping
   - Cache resolution results for performance
   - Log failed resolutions for debugging

3. **Phase 3: Generic Function Support**
   - Normalize function names by stripping type parameters
   - Create secondary index without generics
   - Match calls with and without turbofish syntax
   - Handle trait method calls through generics

### Architecture Changes

**File**: `src/analyzers/call_graph/import_map.rs`
- Add `GlobImport` tracking
- Add `ReExport` tracking
- Implement `resolve_simple_name` → `Vec<QualifiedPath>`

**File**: `src/analyzers/call_graph/path_resolver.rs`
- Add `ResolutionStrategy` enum (Exact, Import, Hierarchy, Fuzzy)
- Implement fallback chain through strategies
- Add `ResolutionCache` for performance

**File**: `src/analyzers/call_graph/call_resolution.rs`
- Refactor `resolve_call` to use strategy chain
- Add `normalize_generic_name` helper
- Improve `is_function_match` to handle generics

### Data Structures

```rust
/// Enhanced import tracking
pub struct ImportMap {
    // Existing fields...

    /// Glob imports: file → imported modules
    glob_imports: HashMap<PathBuf, Vec<String>>,

    /// Re-exports: original path → re-exported paths
    reexports: HashMap<String, Vec<String>>,

    /// Simple name → all possible qualified paths
    name_index: HashMap<String, Vec<String>>,
}

/// Resolution strategy with priority
pub enum ResolutionStrategy {
    Exact,           // Priority 1: Exact qualified match
    Import,          // Priority 2: Via use statements
    ModuleHierarchy, // Priority 3: Same module search
    Fuzzy,           // Priority 4: Normalized matching
}

/// Cache for resolved calls
pub struct ResolutionCache {
    cache: HashMap<(FunctionId, String), Option<FunctionId>>,
}
```

### APIs and Interfaces

```rust
impl PathResolver {
    /// Resolve a simple function name to qualified paths using imports
    pub fn resolve_simple_name(
        &self,
        caller_file: &Path,
        simple_name: &str,
    ) -> Vec<String>;

    /// Try multiple resolution strategies in order
    pub fn resolve_with_strategies(
        &self,
        caller: &FunctionId,
        callee_name: &str,
        call_graph: &CallGraph,
    ) -> Option<FunctionId>;
}

impl ImportMap {
    /// Add a glob import
    pub fn add_glob_import(&mut self, file: PathBuf, module: String);

    /// Add a re-export
    pub fn add_reexport(&mut self, original: String, exported_as: String);

    /// Get all possible qualified names for a simple name
    pub fn get_qualified_names(&self, file: &Path, name: &str) -> Vec<String>;
}
```

## Dependencies

- **Prerequisites**: None (enhancement of existing system)
- **Affected Components**:
  - `src/analyzers/rust_call_graph.rs` (multi-file extraction)
  - `src/analyzers/call_graph/call_resolution.rs` (resolution logic)
  - `src/analyzers/call_graph/path_resolver.rs` (import resolution)
  - `src/analyzers/call_graph/import_map.rs` (import tracking)
- **External Dependencies**: None (uses existing syn crate)

## Testing Strategy

### Unit Tests

1. **Import Resolution Tests** (`tests/call_graph_import_resolution_test.rs`)
   - Simple imports: `use module::function`
   - Glob imports: `use module::*`
   - Re-exports: `pub use other::function`
   - Nested imports: `use crate::a::b::c::function`
   - Aliases: `use module::func as alias`

2. **Generic Function Tests** (`tests/call_graph_generic_functions_test.rs`)
   - Generic standalone functions: `fn foo<T>()`
   - Generic methods: `impl<T> Struct { fn foo() }`
   - Turbofish calls: `foo::<Type>()`
   - Trait bounds: `fn foo<T: Trait>()`

3. **Module Hierarchy Tests** (`tests/call_graph_module_hierarchy_test.rs`)
   - File-based modules (mod.rs pattern)
   - Inline modules
   - Nested module paths
   - Relative imports (self::, super::, crate::)

### Integration Tests

1. **Cross-Module Call Test**
   - Create test files with cross-module calls
   - Verify all calls are correctly resolved
   - Check caller/callee counts are accurate
   - Test with various import patterns

2. **Real-World Validation**
   - Run on debtmap's own codebase
   - Manually verify sample of functions with known callers
   - Check resolution success rate > 95%
   - Identify and fix remaining edge cases

### Performance Tests

1. **Benchmark Resolution Time**
   - Measure call graph building time before/after
   - Verify overhead < 10%
   - Profile resolution hot paths
   - Optimize caching strategy

## Documentation Requirements

### Code Documentation

- Document each resolution strategy and when it applies
- Add examples for each import pattern supported
- Explain fallback chain and priority order
- Document performance characteristics

### User Documentation

- Update ARCHITECTURE.md with call graph resolution flow
- Document known limitations and edge cases
- Provide troubleshooting guide for unresolved calls

### Architecture Updates

**ARCHITECTURE.md sections to update:**
- Call Graph Resolution → Add multi-strategy resolution section
- Import Tracking → Document enhanced ImportMap capabilities
- Performance → Document resolution caching strategy

## Implementation Notes

### Resolution Strategy Priority

The fallback chain should try strategies in order of specificity:

1. **Exact**: Full qualified path match (fastest, most specific)
2. **Import**: Match via use statements (accurate, handles most cases)
3. **Hierarchy**: Search within module tree (catches same-module calls)
4. **Fuzzy**: Normalized matching (last resort, may have false positives)

### Performance Considerations

- Cache successful resolutions to avoid repeated work
- Build ImportMap once during Phase 1
- Use hash-based lookups for all matching
- Short-circuit on first successful strategy

### Edge Cases

- Conditional compilation (`#[cfg]`) may cause import inconsistencies
- Macro-generated code may have unusual import patterns
- Trait methods with default implementations
- Associated functions vs methods disambiguation

### Known Limitations

- Dynamic dispatch cannot be fully resolved statically
- Macro-expanded code may not have source locations
- FFI calls to external code won't be tracked
- Async trait transformations may alter call patterns

## Migration and Compatibility

### Breaking Changes

None - this is a pure enhancement that improves existing functionality.

### Backward Compatibility

- Existing call graph structure unchanged
- Resolution improvements are transparent to consumers
- No changes to public APIs

### Migration Path

1. Deploy enhancement
2. Rebuild call graph cache (automatic on next run)
3. Verify improved resolution metrics
4. Monitor for any regression in existing functionality

## Success Metrics

- **Primary**: Functions with known callers no longer show "0 callers"
- **Secondary**: Resolution success rate > 95% on debtmap codebase
- **Tertiary**: No more than 10% performance overhead
- **User-Facing**: Dependency scores more accurately reflect actual usage

## Related Work

- Spec 147: Caller/Callee Output Section (will display improved data)
- Spec 148: Enhanced FunctionId Matching (complementary approach)
- Spec 149: Call Graph Debug Tools (helps diagnose remaining issues)
