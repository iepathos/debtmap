---
number: 142
title: Populate FunctionId Module Paths for Qualified Call Resolution
category: foundation
priority: high
status: draft
dependencies: [141]
created: 2025-10-23
---

# Specification 142: Populate FunctionId Module Paths for Qualified Call Resolution

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 141 (Qualified Module Call Detection Infrastructure)

## Context

Spec 141 implemented the infrastructure for detecting qualified module calls (e.g., `call_graph::process_rust_files_for_call_graph()`), including:
- PathResolver integration into multi-file call graph extraction
- Import tracking via ImportMap
- Super/self/crate path resolution via ModuleTree

However, the implementation is incomplete because the `module_path` field on `FunctionId` objects is not being populated during function registration. The PathResolver can correctly resolve qualified paths like `"builders::call_graph::process_rust_files_for_call_graph"`, but it cannot match these to actual functions because all functions have an empty `module_path` field.

**Current State**:
- Diagnostic test shows `process_rust_files_for_call_graph` has 0 callers
- Test output shows "Module path:" is empty for all functions
- PathResolver's `find_function_by_path` checks `func.module_path == path`, but this always fails

**Root Cause**:
Functions are registered without their module path, making it impossible for PathResolver to match resolved qualified calls to function definitions.

## Objective

Populate the `module_path` field on all `FunctionId` objects during function registration, enabling PathResolver to match resolved qualified calls to their target functions and fixing the false positive "0 callers" issue for functions called via qualified paths.

## Requirements

### Functional Requirements

1. **Module Path Inference**
   - Infer module path from file path using `ModuleTree::infer_module_from_file`
   - Handle nested module structures (e.g., `src/builders/unified_analysis.rs` → `"builders::unified_analysis"`)
   - Support both regular files and mod.rs files

2. **Function Registration Enhancement**
   - Update all function registration points to include module_path
   - Ensure module_path is set for:
     - Top-level functions
     - Impl block methods
     - Trait implementations
     - Test functions

3. **Module Tree Population**
   - Ensure ModuleTree is properly built before function registration
   - Add all discovered modules to ModuleTree with correct parent-child relationships
   - Handle mod.rs files correctly (e.g., `builders/mod.rs` → `"builders"`)

4. **FunctionId Construction**
   - Use existing `FunctionId::with_module_path` constructor where available
   - Update `FunctionId::new` calls to include module_path parameter if needed
   - Ensure consistency across all construction sites

### Non-Functional Requirements

1. **Performance**: Module path inference should not significantly impact build time (<5% overhead)
2. **Correctness**: All functions must have accurate module paths
3. **Consistency**: Module path format must match PathResolver expectations
4. **Maintainability**: Changes should be localized and not require widespread refactoring

## Acceptance Criteria

- [ ] All `FunctionId` objects have non-empty `module_path` field after registration
- [ ] Module paths correctly reflect Rust module hierarchy
- [ ] `process_rust_files_for_call_graph` shows >= 3 callers (from validate.rs and unified_analysis.rs)
- [ ] Diagnostic test `diagnose_missing_calls` passes
- [ ] Cross-file resolution test `test_self_referential_call_detection` passes
- [ ] No regression in existing call graph tests
- [ ] Module paths for nested modules are correct (e.g., `builders::call_graph`)
- [ ] Module paths for mod.rs files are correct (e.g., `builders` not `builders::mod`)

## Technical Details

### Implementation Approach

**Phase 1: Module Path Inference**

1. In `CallGraphExtractor::new`, compute module path from file path:
   ```rust
   let module_path = ModuleTree::infer_module_from_file(&file);
   ```

2. Store module_path in CallGraphExtractor state:
   ```rust
   pub struct CallGraphExtractor {
       // ... existing fields
       module_path: String,  // Add this
   }
   ```

**Phase 2: Function Registration Updates**

1. Update `GraphBuilder::add_function` to accept module_path:
   ```rust
   pub fn add_function(
       &mut self,
       name: String,
       line: usize,
       is_test: bool,
       is_async: bool,
       module_path: String,  // Add this
   ) -> FunctionId
   ```

2. Use `FunctionId::with_module_path`:
   ```rust
   let function_id = FunctionId::with_module_path(
       self.current_file.clone(),
       name.clone(),
       line,
       module_path.clone(),
   );
   ```

3. Update all call sites:
   - `add_function_from_item`
   - `add_impl_method`
   - Direct function registration in visitors

**Phase 3: ModuleTree Integration**

1. Build ModuleTree during multi-file extraction:
   ```rust
   // Already done in PathResolverBuilder.analyze_file:
   let module_path = ModuleTree::infer_module_from_file(&file_path);
   module_tree.add_module(module_path.clone(), file_path.clone());
   ```

2. Ensure parent-child relationships are correct:
   ```rust
   // ModuleTree automatically handles this in add_module
   // via extract_parent_module
   ```

**Phase 4: Verification**

1. Add debug logging to verify module paths:
   ```rust
   log::trace!("Registered function {} with module_path: {}",
               func_id.name, func_id.module_path);
   ```

2. Update diagnostic test to check module_path is non-empty:
   ```rust
   assert!(!func.module_path.is_empty(),
           "Function {} has empty module_path", func.name);
   ```

### Architecture Changes

**Modified Components**:
- `GraphBuilder`: Add module_path parameter to function registration methods
- `CallGraphExtractor`: Store and propagate module_path
- Function registration visitors: Pass module_path when creating functions

**No Changes Needed**:
- `FunctionId`: Already has `module_path` field and `with_module_path` constructor
- `ModuleTree`: Already has `infer_module_from_file` method
- `PathResolver`: Already uses `module_path` for matching

### Data Structures

No new data structures needed. Using existing:

```rust
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub line: usize,
    pub module_path: String,  // Already exists, just needs to be populated
}
```

## Dependencies

- **Prerequisites**: Spec 141 (qualified module call detection infrastructure)
- **Affected Components**:
  - `src/analyzers/call_graph/graph_builder.rs`
  - `src/analyzers/call_graph/mod.rs` (CallGraphExtractor)
  - `src/analyzers/rust_call_graph.rs`
- **External Dependencies**: None (uses existing ModuleTree functionality)

## Testing Strategy

### Unit Tests

1. **Module Path Inference Test**:
   ```rust
   #[test]
   fn test_module_path_inference() {
       assert_eq!(
           ModuleTree::infer_module_from_file(Path::new("src/builders/call_graph.rs")),
           "builders::call_graph"
       );
       assert_eq!(
           ModuleTree::infer_module_from_file(Path::new("src/builders/mod.rs")),
           "builders"
       );
   }
   ```

2. **Function Registration Test**:
   ```rust
   #[test]
   fn test_function_has_module_path() {
       let file = Path::new("src/builders/call_graph.rs");
       let extractor = CallGraphExtractor::new(file.to_path_buf());
       // ... register a function
       let func = extractor.graph_builder.call_graph.get_all_functions().next().unwrap();
       assert_eq!(func.module_path, "builders::call_graph");
   }
   ```

### Integration Tests

1. **Cross-File Call Resolution** (already exists):
   - `test_self_referential_call_detection` should pass
   - Should find >= 3 callers for `process_rust_files_for_call_graph`

2. **Diagnostic Test** (already exists):
   - `diagnose_missing_calls` should show non-empty module paths
   - Should detect calls from `build_and_cache_graph` to `process_rust_files_for_call_graph`

### Regression Tests

- Run all existing call graph tests
- Ensure no functions lost their callers
- Verify call graph accuracy metrics don't degrade

## Documentation Requirements

### Code Documentation

- Document module_path parameter in GraphBuilder methods
- Add examples showing how module paths are inferred
- Update CallGraphExtractor documentation

### Implementation Notes

**Key Locations**:
- Function registration: `src/analyzers/call_graph/graph_builder.rs:54-73`
- Visitor implementation: `src/analyzers/call_graph/mod.rs:431-450` (visit_item_fn)
- Module path inference: `src/analyzers/call_graph/module_tree.rs:170-189`

**Common Pitfalls**:
- Don't forget impl methods - they also need module_path
- Handle trait implementations correctly
- Ensure test functions get module_path too
- Mod.rs files should map to parent module, not `module::mod`

## Migration and Compatibility

**Breaking Changes**: None - this is an enhancement to existing functionality

**Compatibility**: Fully backward compatible - FunctionId already has module_path field

**Migration**: No migration needed - this fixes existing bug

## Implementation Notes

### Quick Wins

1. Start with `CallGraphExtractor` - add module_path field
2. Update `GraphBuilder::add_function` to accept and use module_path
3. Update all function registration call sites
4. Run diagnostic test to verify

### Validation

After implementation, the diagnostic test output should show:
```
=== TARGET FUNCTION ===
Found 1 instances of process_rust_files_for_call_graph
  File: "./src/builders/call_graph.rs"
  Line: 53
  Module path: builders::call_graph  // <-- Should be populated!
  Callers: 3  // <-- Should be >= 3!
```

### Performance Considerations

- Module path inference is O(1) string operations
- No additional file I/O required
- Minimal memory overhead (one String per function)
- Should have negligible impact on build time
