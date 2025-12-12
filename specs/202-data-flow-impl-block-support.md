---
number: 202
title: Data Flow Analysis Support for Impl Block Methods
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-11
---

# Specification 202: Data Flow Analysis Support for Impl Block Methods

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The data flow analysis feature in debtmap is effectively broken for the vast majority of functions. The TUI detail view (page 5) rarely shows data flow information because the data population functions only handle top-level `fn` items and completely ignore methods defined inside `impl` blocks.

### Root Cause Analysis

Four functions responsible for populating data flow information share the same bug - they only iterate over `syn::Item::Fn` and ignore `syn::Item::Impl`:

| Location | Function | Line |
|----------|----------|------|
| `src/builders/parallel_unified_analysis.rs` | `extract_purity_analysis` | 69 |
| `src/data_flow/population.rs` | `populate_io_operations` | 127 |
| `src/data_flow/population.rs` | `extract_variable_deps` | 194 |
| `src/data_flow/population.rs` | `populate_data_transformations` | 247 |

### Current (Broken) Pattern

```rust
for item in &file_ast.items {
    if let syn::Item::Fn(item_fn) = item {
        // Only top-level functions are processed
        // Methods in impl blocks are completely ignored
    }
}
```

### Impact

1. **Majority of functions missing**: In typical Rust codebases, 80-90%+ of functions are methods in `impl` blocks, not top-level functions
2. **TUI page hidden**: The Data Flow page (page 5) only appears when at least one of `get_mutation_info()`, `get_io_operations()`, or `get_cfg_analysis()` returns `Some` - which requires the function to have been analyzed
3. **Inconsistency**: Function metrics ARE correctly collected for impl methods (via `visit_impl_item_fn` in `rust.rs:1049`), but data flow analysis is not populated for them
4. **User confusion**: Users see detailed complexity and dependency information but no data flow analysis for most functions

### Evidence

The main rust analyzer correctly handles impl blocks:
```rust
// src/analyzers/rust.rs:1049
fn visit_impl_item_fn(&mut self, impl_fn: &'ast syn::ImplItemFn) {
    // Correctly processes methods in impl blocks
}
```

But data flow population does not:
```rust
// src/builders/parallel_unified_analysis.rs:68-83
for item in &file_ast.items {
    if let syn::Item::Fn(item_fn) = item {  // Only Item::Fn, not Item::Impl
        // ...
    }
}
```

## Objective

Extend all four data flow population functions to analyze methods inside `impl` blocks, ensuring data flow information is available for all analyzed functions regardless of whether they are top-level functions or methods.

## Requirements

### Functional Requirements

1. **FR-1**: `extract_purity_analysis` must find and analyze functions in both:
   - Top-level `syn::Item::Fn` items
   - Methods in `syn::Item::Impl` blocks (`syn::ImplItem::Fn`)

2. **FR-2**: `populate_io_operations` must detect I/O operations in both:
   - Top-level `syn::Item::Fn` items
   - Methods in `syn::Item::Impl` blocks

3. **FR-3**: `extract_variable_deps` must extract variable dependencies from both:
   - Top-level `syn::Item::Fn` items
   - Methods in `syn::Item::Impl` blocks

4. **FR-4**: `populate_data_transformations` must detect transformation patterns in both:
   - Top-level `syn::Item::Fn` items
   - Methods in `syn::Item::Impl` blocks

5. **FR-5**: Function matching must use the same criteria for both:
   - Match by function name AND line number
   - Handle the FunctionMetrics naming convention for methods (`Type::method_name`)

6. **FR-6**: The TUI Data Flow page must appear for all functions that have data flow information, not just top-level functions

### Non-Functional Requirements

1. **NFR-1**: Performance impact must be minimal - parsing each file once and iterating through impl blocks is O(n) and should not significantly impact analysis time
2. **NFR-2**: Code should be DRY - consider extracting a shared helper function for finding functions by name/line in AST
3. **NFR-3**: Maintain backward compatibility - existing functionality for top-level functions must continue to work

## Acceptance Criteria

- [ ] Running debtmap on itself shows Data Flow page (page 5) available for methods in impl blocks
- [ ] `extract_purity_analysis` returns results for impl block methods
- [ ] `populate_io_operations` detects I/O in impl block methods
- [ ] `extract_variable_deps` extracts dependencies from impl block methods
- [ ] `populate_data_transformations` detects transformations in impl block methods
- [ ] All existing tests continue to pass
- [ ] New tests verify impl block method handling for each function
- [ ] Data flow coverage increases from ~10% to ~80%+ of analyzed functions

## Technical Details

### Implementation Approach

#### Option 1: Inline Pattern Matching (Recommended)

Extend each function to also match `syn::Item::Impl`:

```rust
for item in &file_ast.items {
    match item {
        syn::Item::Fn(item_fn) => {
            if let Some(ident_span) = item_fn.sig.ident.span().start().line.checked_sub(1) {
                if ident_span == m.line && item_fn.sig.ident == m.name {
                    // Process top-level function
                }
            }
        }
        syn::Item::Impl(item_impl) => {
            for impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(method) = impl_item {
                    if let Some(ident_span) = method.sig.ident.span().start().line.checked_sub(1) {
                        // Handle both simple name and Type::name format
                        let method_name = method.sig.ident.to_string();
                        let matches_name = m.name == method_name
                            || m.name.ends_with(&format!("::{}", method_name));

                        if ident_span == m.line && matches_name {
                            // Process impl method - note: use method.block, not method.block.clone()
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
```

#### Option 2: Extract Helper Function (DRY approach)

Create a helper that finds a function by name/line:

```rust
/// Find a function in the AST by name and line number
/// Returns the function block if found, handling both top-level fns and impl methods
fn find_function_block<'a>(
    ast: &'a syn::File,
    name: &str,
    line: usize,
) -> Option<&'a syn::Block> {
    for item in &ast.items {
        match item {
            syn::Item::Fn(item_fn) => {
                if let Some(span_line) = item_fn.sig.ident.span().start().line.checked_sub(1) {
                    if span_line == line && item_fn.sig.ident == name {
                        return Some(&item_fn.block);
                    }
                }
            }
            syn::Item::Impl(item_impl) => {
                for impl_item in &item_impl.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        if let Some(span_line) = method.sig.ident.span().start().line.checked_sub(1) {
                            let method_name = method.sig.ident.to_string();
                            let matches = name == method_name
                                || name.ends_with(&format!("::{}", method_name));

                            if span_line == line && matches {
                                return Some(&method.block);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}
```

### Name Matching Considerations

The `FunctionMetrics` stores method names in the format `TypeName::method_name` (see `rust.rs:1052-1056`):

```rust
let name = if let Some(ref impl_type) = self.current_impl_type {
    format!("{impl_type}::{method_name}")
} else {
    method_name.clone()
};
```

The function finding logic must handle this by checking if the metric name ends with `::method_name`.

### Files to Modify

1. `src/builders/parallel_unified_analysis.rs`:
   - Function: `extract_purity_analysis` (lines 49-87)
   - Add `syn::Item::Impl` handling

2. `src/data_flow/population.rs`:
   - Function: `populate_io_operations` (lines 99-144)
   - Function: `extract_variable_deps` (lines 171-213)
   - Function: `populate_data_transformations` (lines 218-260)
   - Add `syn::Item::Impl` handling to all three

### API Signature Consideration

For `extract_purity_analysis`, the function signature needs to handle that `syn::ImplItemFn` has a `block` field (not `Box<Block>` like `ItemFn`), but the `PurityDetector::is_pure_function` expects `&ItemFn`. Consider:

1. Creating a synthetic `ItemFn` from the method (current approach in `rust.rs:1073-1076`)
2. Extending `PurityDetector` to accept `&syn::ImplItemFn` directly
3. Creating a trait that both implement

The first approach (creating synthetic `ItemFn`) is already used in the codebase and should be followed for consistency.

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/builders/parallel_unified_analysis.rs`
  - `src/data_flow/population.rs`
  - `src/tui/results/app.rs` (verification only, no changes expected)
- **External Dependencies**: None (uses existing `syn` crate)

## Testing Strategy

### Unit Tests

1. **Test impl method purity analysis**:
   - Create test file with method in impl block
   - Verify `extract_purity_analysis` returns result for the method

2. **Test impl method I/O detection**:
   - Create test file with I/O operations in impl method
   - Verify `populate_io_operations` detects them

3. **Test impl method variable deps**:
   - Create test file with impl method having parameters
   - Verify `extract_variable_deps` extracts them

4. **Test impl method transformation detection**:
   - Create test file with iterator chains in impl method
   - Verify `populate_data_transformations` detects them

5. **Test name matching**:
   - Verify methods match when name is `TypeName::method_name`
   - Verify methods match when name is just `method_name`

### Integration Tests

1. **Run debtmap on itself**:
   - Verify Data Flow page available for impl methods
   - Compare before/after coverage percentages

2. **TUI verification**:
   - Navigate to a known impl method
   - Verify Data Flow page (5) is available and shows data

### Regression Tests

1. **Verify existing top-level function handling unchanged**
2. **Verify all existing tests continue to pass**

## Documentation Requirements

- **Code Documentation**: Add doc comments explaining the dual Item::Fn/Item::Impl handling
- **User Documentation**: None needed (this is a bug fix, behavior should "just work")
- **Architecture Updates**: None needed

## Implementation Notes

1. **Consistency**: Use the same pattern across all four functions for maintainability
2. **Existing Pattern**: Follow the synthetic `ItemFn` creation pattern from `rust.rs:1073-1076`
3. **Performance**: Parse each file once (already done), just iterate impl blocks too
4. **Consider Traits**: `syn::Item::Trait` also contains methods (`syn::TraitItem::Fn`) but these typically don't have implementations to analyze - consider adding support but it's lower priority

## Migration and Compatibility

- **Breaking Changes**: None
- **Migration Requirements**: None
- **Backward Compatibility**: Fully backward compatible - top-level functions continue to work as before
