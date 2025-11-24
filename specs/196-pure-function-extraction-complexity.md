---
number: 196
title: Pure Function Extraction for Complexity Analysis
category: optimization
priority: high
status: draft
dependencies: [195]
created: 2025-11-24
---

# Specification 196: Pure Function Extraction for Complexity Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 195 (Stillwater Foundation Setup)

## Context

Debtmap's complexity analysis modules (`src/complexity/`) currently mix I/O operations with pure calculations. Functions that calculate cyclomatic complexity, cognitive complexity, and pattern detection read files, parse content, and perform calculations all in one operation.

This creates several problems:
- **Slow tests**: Every test requires file I/O (typically 50-100ms per test)
- **Hard to test edge cases**: Need to create temporary files for every scenario
- **Difficult to refactor**: Logic is tightly coupled to I/O
- **Poor testability**: Can't easily test pure calculations in isolation

Pure functions are functions that:
- Have no side effects (no I/O, no mutation)
- Return the same output for the same input (deterministic)
- Don't depend on external state
- Can't fail (no `Result` needed)

Research shows that pure functions are:
- **100x faster to test** (no I/O overhead)
- **Easier to reason about** (no hidden dependencies)
- **Simpler to maintain** (clear input → output relationship)
- **More reusable** (work anywhere, not tied to context)

This specification extracts debtmap's complexity calculations into pure functions, dramatically improving testability while maintaining backwards compatibility.

## Objective

Extract complexity calculations (cyclomatic, cognitive, pattern detection) from I/O-dependent functions into pure functions that operate directly on AST structures, enabling 100x faster tests and easier maintenance.

## Requirements

### Functional Requirements

#### Cyclomatic Complexity
- Extract `calculate_cyclomatic` to `calculate_cyclomatic_pure(&syn::File) -> u32`
- Extract branch counting logic to pure functions
- Remove `Result` type (pure functions can't fail)
- Keep original function as backwards-compatible wrapper

#### Cognitive Complexity
- Extract `calculate_cognitive` to `calculate_cognitive_pure(&syn::File) -> u32`
- Extract nesting depth calculation to pure function
- Extract complexity increment logic to pure function
- Remove `Result` type

#### Pattern Detection
- Extract `detect_patterns` to `detect_patterns_pure(&syn::File) -> Vec<Pattern>`
- Extract individual pattern matchers to pure functions
- Remove file reading from pattern detection
- Pure functions take AST, return patterns

#### Supporting Pure Functions
- `count_function_branches(&syn::ItemFn) -> u32`
- `count_expr_branches(&syn::Expr) -> u32`
- `calculate_nesting_depth(&syn::Block) -> u32`
- `is_pure_mapping_pattern(&syn::ExprMatch) -> bool`
- `detect_god_object(&syn::ItemStruct) -> Option<GodObjectPattern>`

### Non-Functional Requirements
- Pure functions are at least 50x faster than I/O versions in tests
- Test coverage for pure functions reaches 95%+
- All existing integration tests pass without modification
- No breaking changes to public API
- Backwards-compatible wrappers preserve existing behavior

## Acceptance Criteria

- [ ] `src/complexity/pure.rs` module created with pure functions
- [ ] `calculate_cyclomatic_pure(&syn::File) -> u32` implemented
- [ ] `calculate_cognitive_pure(&syn::File) -> u32` implemented
- [ ] `detect_patterns_pure(&syn::File) -> Vec<Pattern>` implemented
- [ ] All pure functions have zero `Result` types (deterministic)
- [ ] Pure functions have no I/O operations
- [ ] Backwards-compatible wrappers maintain existing API
- [ ] Unit tests for pure functions run in < 1ms each
- [ ] Test coverage for pure functions > 95%
- [ ] Property-based tests added for pure functions
- [ ] All existing integration tests pass
- [ ] Documentation updated with pure function examples
- [ ] Performance benchmarks show 50-100x speedup in tests

## Technical Details

### Implementation Approach

#### 1. Create Pure Module

```rust
// src/complexity/pure.rs

/// Calculate cyclomatic complexity from parsed AST
/// Pure function - deterministic, no I/O, no Result
pub fn calculate_cyclomatic_pure(file: &syn::File) -> u32 {
    file.items
        .iter()
        .map(count_item_branches)
        .sum()
}

/// Count branches in a single item
pub fn count_item_branches(item: &syn::Item) -> u32 {
    match item {
        syn::Item::Fn(func) => count_function_branches(func),
        syn::Item::Impl(impl_block) => {
            impl_block.items.iter()
                .filter_map(|item| {
                    if let syn::ImplItem::Fn(method) = item {
                        Some(count_function_branches_from_method(method))
                    } else {
                        None
                    }
                })
                .sum()
        }
        _ => 0,
    }
}

/// Count branches in a function
pub fn count_function_branches(func: &syn::ItemFn) -> u32 {
    1 + func.block.stmts.iter()
        .map(count_stmt_branches)
        .sum::<u32>()
}

fn count_stmt_branches(stmt: &syn::Stmt) -> u32 {
    match stmt {
        syn::Stmt::Expr(expr, _) | syn::Stmt::Semi(expr, _) => {
            count_expr_branches(expr)
        }
        _ => 0,
    }
}

fn count_expr_branches(expr: &syn::Expr) -> u32 {
    match expr {
        syn::Expr::If(_) => 1,
        syn::Expr::Match(m) => m.arms.len() as u32,
        syn::Expr::While(_) | syn::Expr::ForLoop(_) | syn::Expr::Loop(_) => 1,
        syn::Expr::Block(b) => {
            b.block.stmts.iter()
                .map(count_stmt_branches)
                .sum()
        }
        // Recursively traverse other expressions...
        _ => 0,
    }
}

/// Calculate cognitive complexity from parsed AST
pub fn calculate_cognitive_pure(file: &syn::File) -> u32 {
    file.items
        .iter()
        .map(|item| calculate_item_cognitive(item, 0))
        .sum()
}

fn calculate_item_cognitive(item: &syn::Item, nesting: u32) -> u32 {
    match item {
        syn::Item::Fn(func) => calculate_function_cognitive(&func.block, nesting),
        syn::Item::Impl(impl_block) => {
            impl_block.items.iter()
                .filter_map(|item| {
                    if let syn::ImplItem::Fn(method) = item {
                        Some(calculate_function_cognitive(&method.block, nesting))
                    } else {
                        None
                    }
                })
                .sum()
        }
        _ => 0,
    }
}

fn calculate_function_cognitive(block: &syn::Block, nesting: u32) -> u32 {
    block.stmts.iter()
        .map(|stmt| calculate_stmt_cognitive(stmt, nesting))
        .sum()
}

fn calculate_stmt_cognitive(stmt: &syn::Stmt, nesting: u32) -> u32 {
    match stmt {
        syn::Stmt::Expr(expr, _) | syn::Stmt::Semi(expr, _) => {
            calculate_expr_cognitive(expr, nesting)
        }
        _ => 0,
    }
}

fn calculate_expr_cognitive(expr: &syn::Expr, nesting: u32) -> u32 {
    match expr {
        syn::Expr::If(if_expr) => {
            // If adds 1 + nesting
            let cost = 1 + nesting;
            // Recursively calculate nested complexity
            let then_cost = calculate_block_cognitive(&if_expr.then_branch, nesting + 1);
            let else_cost = if_expr.else_branch.as_ref()
                .map(|(_, else_expr)| calculate_expr_cognitive(else_expr, nesting + 1))
                .unwrap_or(0);
            cost + then_cost + else_cost
        }
        syn::Expr::While(while_expr) => {
            1 + nesting + calculate_block_cognitive(&while_expr.body, nesting + 1)
        }
        syn::Expr::ForLoop(for_expr) => {
            1 + nesting + calculate_block_cognitive(&for_expr.body, nesting + 1)
        }
        syn::Expr::Match(match_expr) => {
            let match_cost = 1 + nesting;
            let arms_cost: u32 = match_expr.arms.iter()
                .map(|arm| calculate_expr_cognitive(&arm.body, nesting + 1))
                .sum();
            match_cost + arms_cost
        }
        syn::Expr::Block(block_expr) => {
            calculate_block_cognitive(&block_expr.block, nesting)
        }
        _ => 0,
    }
}

fn calculate_block_cognitive(block: &syn::Block, nesting: u32) -> u32 {
    block.stmts.iter()
        .map(|stmt| calculate_stmt_cognitive(stmt, nesting))
        .sum()
}

/// Detect patterns in parsed AST
pub fn detect_patterns_pure(file: &syn::File) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    for item in &file.items {
        patterns.extend(detect_item_patterns(item));
    }

    patterns
}

fn detect_item_patterns(item: &syn::Item) -> Vec<Pattern> {
    match item {
        syn::Item::Struct(s) => detect_struct_patterns(s),
        syn::Item::Fn(f) => detect_function_patterns(f),
        syn::Item::Impl(i) => detect_impl_patterns(i),
        _ => vec![],
    }
}

fn detect_struct_patterns(s: &syn::ItemStruct) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    // God object detection
    if s.fields.len() > 5 {
        patterns.push(Pattern::GodObject {
            name: s.ident.to_string(),
            field_count: s.fields.len(),
        });
    }

    patterns
}

fn detect_function_patterns(f: &syn::ItemFn) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    // Long function detection
    let line_count = count_function_lines(f);
    if line_count > 50 {
        patterns.push(Pattern::LongFunction {
            name: f.sig.ident.to_string(),
            lines: line_count,
        });
    }

    // Many parameters detection
    if f.sig.inputs.len() > 5 {
        patterns.push(Pattern::ManyParameters {
            name: f.sig.ident.to_string(),
            param_count: f.sig.inputs.len(),
        });
    }

    patterns
}

fn count_function_lines(f: &syn::ItemFn) -> usize {
    // Approximate line count by statement count
    count_stmts_recursive(&f.block)
}

fn count_stmts_recursive(block: &syn::Block) -> usize {
    let mut count = block.stmts.len();

    for stmt in &block.stmts {
        if let syn::Stmt::Expr(expr, _) = stmt {
            count += count_expr_stmts(expr);
        }
    }

    count
}

fn count_expr_stmts(expr: &syn::Expr) -> usize {
    match expr {
        syn::Expr::Block(b) => count_stmts_recursive(&b.block),
        syn::Expr::If(if_expr) => {
            let then_count = count_stmts_recursive(&if_expr.then_branch);
            let else_count = if_expr.else_branch.as_ref()
                .map(|(_, e)| count_expr_stmts(e))
                .unwrap_or(0);
            then_count + else_count
        }
        _ => 0,
    }
}
```

#### 2. Create Effect Wrappers

```rust
// src/complexity/effects.rs
use stillwater::{Effect, IO};
use super::pure::*;

/// Calculate cyclomatic complexity with I/O
pub fn calculate_cyclomatic_effect(
    path: PathBuf
) -> AnalysisEffect<u32> {
    IO::read_file(path.clone())
        .context(format!("Reading file: {}", path.display()))
        .and_then(|content| {
            syn::parse_file(&content)
                .map(Effect::pure)
                .map_err(|e| AnalysisError::ParseError(e.to_string()))
        })
        .context("Parsing Rust syntax")
        .map(|ast| calculate_cyclomatic_pure(&ast))
        .context("Calculating cyclomatic complexity")
}

/// Calculate cognitive complexity with I/O
pub fn calculate_cognitive_effect(
    path: PathBuf
) -> AnalysisEffect<u32> {
    IO::read_file(path.clone())
        .context(format!("Reading file: {}", path.display()))
        .and_then(|content| {
            syn::parse_file(&content)
                .map(Effect::pure)
                .map_err(|e| AnalysisError::ParseError(e.to_string()))
        })
        .context("Parsing Rust syntax")
        .map(|ast| calculate_cognitive_pure(&ast))
        .context("Calculating cognitive complexity")
}
```

#### 3. Maintain Backwards Compatibility

```rust
// src/complexity/mod.rs
pub mod pure;
pub mod effects;

// Backwards-compatible API
pub fn calculate_cyclomatic(path: &Path) -> anyhow::Result<u32> {
    use std::fs;
    let content = fs::read_to_string(path)?;
    let ast = syn::parse_file(&content)?;
    Ok(pure::calculate_cyclomatic_pure(&ast))
}

pub fn calculate_cognitive(path: &Path) -> anyhow::Result<u32> {
    use std::fs;
    let content = fs::read_to_string(path)?;
    let ast = syn::parse_file(&content)?;
    Ok(pure::calculate_cognitive_pure(&ast))
}
```

### Testing Improvements

#### Before: Slow Integration Tests

```rust
#[test]
fn test_cyclomatic_complexity() {
    // Create temporary file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    fs::write(&test_file, r#"
        fn example() {
            if true {
                while false {
                    println!("test");
                }
            }
        }
    "#).unwrap();

    // Run analysis (slow: file I/O)
    let result = calculate_cyclomatic(&test_file).unwrap();

    assert_eq!(result, 3);
}
// Runtime: ~50ms (file I/O overhead)
```

#### After: Fast Pure Function Tests

```rust
use quote::quote;

#[test]
fn test_cyclomatic_pure() {
    let code = quote! {
        fn example() {
            if true {
                while false {
                    println!("test");
                }
            }
        }
    };
    let ast: syn::File = syn::parse2(code).unwrap();

    let result = calculate_cyclomatic_pure(&ast);

    assert_eq!(result, 3);
}
// Runtime: ~0.5ms (100x faster, no I/O)

#[test]
fn test_simple_function() {
    let ast = syn::parse_str("fn foo() {}").unwrap();
    assert_eq!(calculate_cyclomatic_pure(&ast), 1);
}

#[test]
fn test_if_statement() {
    let ast = syn::parse_str(r#"
        fn foo(x: bool) {
            if x {
                println!("yes");
            }
        }
    "#).unwrap();
    assert_eq!(calculate_cyclomatic_pure(&ast), 2);
}

#[test]
fn test_match_expression() {
    let ast = syn::parse_str(r#"
        fn foo(x: Option<i32>) {
            match x {
                Some(v) => println!("{}", v),
                None => println!("none"),
            }
        }
    "#).unwrap();
    assert_eq!(calculate_cyclomatic_pure(&ast), 3);
}

// Property-based test
#[cfg(feature = "proptest")]
#[test]
fn test_complexity_monotonic() {
    use proptest::prelude::*;

    proptest!(|(branches in 0u32..50)| {
        let code = generate_if_chain(branches);
        let ast = syn::parse_str(&code).unwrap();

        let complexity = calculate_cyclomatic_pure(&ast);

        // Property: complexity >= number of branches
        prop_assert!(complexity >= branches);
    });
}
```

### Architecture Changes

**New Structure:**
```
src/complexity/
├── pure.rs          # NEW: Pure functions (no I/O)
├── effects.rs       # NEW: Effect wrappers
├── mod.rs           # Updated: Backwards-compatible exports
├── cyclomatic.rs    # Deprecated: Use pure.rs instead
└── cognitive.rs     # Deprecated: Use pure.rs instead
```

**Migration Path:**
1. Add `pure.rs` with pure functions
2. Add `effects.rs` with Effect wrappers
3. Update `mod.rs` to re-export for compatibility
4. Mark old modules as deprecated
5. Gradually migrate callers to pure functions

## Dependencies

- **Prerequisites**:
  - Spec 195 (Stillwater Foundation) - Provides Effect types
- **Blocked by**: None
- **Blocks**:
  - Spec 197 (Validation Accumulation) - Uses pure functions
  - Spec 200 (Testing Infrastructure) - Benefits from fast tests
- **Affected Components**:
  - `src/complexity/` - Major refactoring
  - `src/analyzers/` - Uses complexity functions
  - `tests/` - New pure function tests
- **External Dependencies**: None (syn already in Cargo.toml)

## Testing Strategy

- **Unit Tests**:
  - 50+ tests for pure functions covering all edge cases
  - Each test runs in < 1ms
  - Test coverage > 95% for pure functions
  - Use `quote!` macro for inline AST construction

- **Property-Based Tests**:
  - Complexity monotonicity (more branches = higher complexity)
  - Determinism (same input = same output)
  - Bounds checking (complexity >= number of branches)

- **Integration Tests**:
  - Keep existing integration tests
  - Verify backwards compatibility
  - Ensure Effect wrappers work correctly

- **Performance Benchmarks**:
  - Measure pure function execution time
  - Compare with file-based tests
  - Target: 50-100x speedup

## Documentation Requirements

- **Code Documentation**:
  - Add module docs to `pure.rs` explaining pure functions
  - Document why pure functions are beneficial
  - Add examples to each public function
  - Explain relationship between pure and Effect versions

- **User Documentation**:
  - Not needed (internal refactoring)

- **Architecture Updates**:
  - Update ARCHITECTURE.md with pure function pattern
  - Document separation of I/O and logic
  - Show example of pure vs Effect functions

## Implementation Notes

### Files to Create
- `src/complexity/pure.rs` - Pure complexity functions
- `src/complexity/effects.rs` - Effect wrappers

### Files to Modify
- `src/complexity/mod.rs` - Export new modules, deprecate old
- `src/complexity/cyclomatic.rs` - Add deprecation notice
- `src/complexity/cognitive.rs` - Add deprecation notice

### Estimated Effort
- Pure function extraction: 6-8 hours
- Effect wrappers: 2-3 hours
- Unit tests: 4-6 hours
- Documentation: 2-3 hours
- **Total: 14-20 hours**

### Performance Impact
- **Test speed**: 50-100x faster (no file I/O)
- **Runtime**: No change (same calculations)
- **Binary size**: +20KB (new functions)

## Migration and Compatibility

### Breaking Changes

None. All changes are additive:
- New pure functions added
- Old functions marked deprecated but still work
- Callers can migrate gradually

### Migration Examples

**Before:**
```rust
let complexity = calculate_cyclomatic(&file_path)?;
```

**After (pure):**
```rust
let ast = syn::parse_file(&content)?;
let complexity = calculate_cyclomatic_pure(&ast);
```

**After (effect):**
```rust
let complexity = calculate_cyclomatic_effect(file_path)
    .run(&env)?;
```

## Success Metrics

- **Test Speed**: Pure function tests run in < 1ms each
- **Coverage**: 95%+ coverage on pure functions
- **Performance**: 50-100x speedup in unit tests
- **Compatibility**: All existing tests pass
- **Quality**: No clippy warnings, all tests pass

## Future Considerations

After this spec:
- Can apply same pattern to scoring logic (Spec 197)
- Can apply to pattern detection (Spec 197)
- Can use pure functions in validation pipeline (Spec 197)
- Enables fast property-based testing
