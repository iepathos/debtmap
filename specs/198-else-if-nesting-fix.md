---
number: 198
title: Fix Else-If Chain Nesting Calculation
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-15
---

# Specification 198: Fix Else-If Chain Nesting Calculation

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently overcounts nesting depth for `else if` chains due to how Rust's AST represents them. In syn's AST, `else if` is represented as:

```rust
// Source code:
if a {
} else if b {
} else if c {
}

// AST representation:
ExprIf {
    cond: a,
    then_branch: { ... },
    else_branch: Some(ExprIf {  // <-- nested ExprIf
        cond: b,
        then_branch: { ... },
        else_branch: Some(ExprIf {  // <-- doubly nested ExprIf
            cond: c,
            then_branch: { ... },
            else_branch: None
        })
    })
}
```

The current `NestingVisitor` implementations treat each `ExprIf` as a nesting increment, causing `else if` chains to report artificially high nesting depths.

### Real-World Example

The `parse` function in Zed's `crates/acp_thread/src/mention.rs` has:
- **Visual nesting**: ~3 levels (match → if → nested if)
- **Debtmap reported nesting**: 10 levels

This is because the function has a `match` containing an `if let ... else if let ...` chain with 8+ branches. Each `else if` is incorrectly counted as +1 nesting.

### Industry Standard

According to SonarQube's Cognitive Complexity specification (the industry standard):

1. **`else if` does NOT increase nesting level** - it's a flat continuation of the same decision structure
2. **Only truly nested structures** (an `if` inside another `if`'s body) get nesting penalties
3. The nesting increment is about tracking mental context depth, not AST structure depth

### Affected Code Paths

There are **three** nesting calculation implementations that need fixing:

1. **`src/analyzers/rust_complexity_calculation.rs`** - `calculate_nesting()` function (lines 85-159)
2. **`src/extraction/extractor.rs`** - `NestingVisitor` (lines 814-857)
3. **`src/complexity/pure.rs`** - `calculate_expr_nesting_depth()` (lines 593-622) - **Already correct!**

The `pure.rs` implementation already handles this correctly:

```rust
fn calculate_expr_nesting_depth(expr: &Expr, current_depth: u32) -> u32 {
    match expr {
        Expr::If(if_expr) => {
            let new_depth = current_depth + 1;
            let then_depth = calculate_block_nesting_depth(&if_expr.then_branch, new_depth);
            let else_depth = if_expr
                .else_branch
                .as_ref()
                .map(|(_, e)| calculate_expr_nesting_depth(e, current_depth))  // <-- correct: passes current_depth
                .unwrap_or(current_depth);
            then_depth.max(else_depth)
        }
        // ...
    }
}
```

Note: The else branch is evaluated at `current_depth`, not `new_depth`. This is the correct behavior.

## Objective

Fix all nesting depth calculations to treat `else if` chains as flat (same nesting level) rather than nested, aligning with industry standards for cognitive complexity measurement.

## Requirements

### Functional Requirements

1. **`else if` chains must not increase nesting depth**
   - An `if ... else if ... else if ...` chain should have nesting depth 1, not N
   - Only an `if` inside a `then_branch` (or inside a block in an else branch) should increase nesting

2. **Preserve correct nesting for truly nested structures**
   - `if a { if b { } }` should have nesting depth 2
   - `match x { _ => if a { } }` should have nesting depth 2
   - `for i in x { if a { } }` should have nesting depth 2

3. **Handle `else { if ... }` blocks correctly**
   - `if a { } else { if b { } }` is visually flat - nesting depth 1 (the inner if is the direct child)
   - `if a { } else { let x = 1; if b { } }` has a statement before the if - the `if b` should be nesting depth 2

### Non-Functional Requirements

1. **Backward compatibility**: Test suite must continue to pass (update test expectations where they were wrong)
2. **Performance**: No regression in analysis speed
3. **Consistency**: All three nesting calculation paths must produce identical results

## Acceptance Criteria

- [ ] `else if` chains report correct nesting depth (1, not N)
- [ ] All three nesting implementations (`rust_complexity_calculation.rs`, `extractor.rs`, `pure.rs`) produce consistent results
- [ ] Existing tests pass (with updated expectations where needed)
- [ ] New test cases cover `else if` chain scenarios:
  - [ ] Simple `if-else if-else if` chain → nesting 1
  - [ ] `match` with `if-else if` in arm → nesting 2
  - [ ] `if` inside `then_branch` of another `if` → nesting 2
  - [ ] `else { stmt; if }` (statement before nested if) → nesting 2
- [ ] Running debtmap on Zed's `mention.rs::parse` reports nesting ≤ 4 (not 10)

## Technical Details

### Implementation Approach

#### Fix 1: `src/analyzers/rust_complexity_calculation.rs`

Replace the current `visit_expr_if` implementation:

```rust
// Current (wrong):
fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
    self.visit_nested(|v| syn::visit::visit_expr_if(v, i));
}

// Fixed:
fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
    // Increment nesting for the if itself
    self.current_depth += 1;
    self.max_depth = self.max_depth.max(self.current_depth);

    // Visit condition (no nesting change)
    self.visit_expr(&i.cond);

    // Visit then branch (already at incremented depth)
    self.visit_block(&i.then_branch);

    // Visit else branch WITHOUT incrementing nesting
    // This handles else-if chains correctly
    if let Some((_, else_expr)) = &i.else_branch {
        // Decrement before visiting else, so else-if stays flat
        self.current_depth -= 1;
        self.visit_expr(else_expr);
        // Don't re-increment - we're done with this if
    } else {
        self.current_depth -= 1;
    }
}
```

#### Fix 2: `src/extraction/extractor.rs`

The `NestingVisitor` uses a simpler pattern. Replace:

```rust
// Current (wrong):
fn visit_expr(&mut self, expr: &'ast syn::Expr) {
    match expr {
        syn::Expr::If(_)
        | syn::Expr::While(_)
        | syn::Expr::ForLoop(_)
        | syn::Expr::Loop(_)
        | syn::Expr::Match(_) => {
            self.enter_nested();
            syn::visit::visit_expr(self, expr);
            self.leave_nested();
        }
        _ => {
            syn::visit::visit_expr(self, expr);
        }
    }
}

// Fixed - handle If specially:
fn visit_expr(&mut self, expr: &'ast syn::Expr) {
    match expr {
        syn::Expr::If(if_expr) => {
            self.enter_nested();
            // Visit condition
            syn::visit::visit_expr(self, &if_expr.cond);
            // Visit then branch
            syn::visit::visit_block(self, &if_expr.then_branch);
            // Leave nesting BEFORE visiting else (handles else-if)
            self.leave_nested();
            // Visit else branch at original nesting level
            if let Some((_, else_expr)) = &if_expr.else_branch {
                self.visit_expr(else_expr);
            }
        }
        syn::Expr::While(_)
        | syn::Expr::ForLoop(_)
        | syn::Expr::Loop(_)
        | syn::Expr::Match(_) => {
            self.enter_nested();
            syn::visit::visit_expr(self, expr);
            self.leave_nested();
        }
        _ => {
            syn::visit::visit_expr(self, expr);
        }
    }
}
```

### Edge Cases

1. **`else { if }` vs `else if`**: Both should be nesting-flat (the if is the direct else expression)
2. **`else { let x = 1; if }` should increment nesting**: The block has statements before the if, so it's a truly nested if
3. **Match arms with if**: `match x { _ => if a {} }` - the if is inside the arm body, so +1 nesting from match

### Test Cases to Add

```rust
#[test]
fn test_else_if_chain_flat_nesting() {
    let code = r#"
    {
        if a {
            x
        } else if b {
            y
        } else if c {
            z
        } else {
            w
        }
    }
    "#;
    let block: syn::Block = syn::parse_str(code).unwrap();
    assert_eq!(calculate_nesting(&block), 1, "else-if chain should have nesting 1");
}

#[test]
fn test_nested_if_inside_then() {
    let code = r#"
    {
        if a {
            if b {
                x
            }
        }
    }
    "#;
    let block: syn::Block = syn::parse_str(code).unwrap();
    assert_eq!(calculate_nesting(&block), 2, "if inside then should have nesting 2");
}

#[test]
fn test_match_with_else_if_chain() {
    let code = r#"
    {
        match x {
            A => {
                if a {
                } else if b {
                } else if c {
                }
            }
            _ => {}
        }
    }
    "#;
    let block: syn::Block = syn::parse_str(code).unwrap();
    assert_eq!(calculate_nesting(&block), 2, "match + else-if chain should have nesting 2");
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/rust_complexity_calculation.rs`
  - `src/extraction/extractor.rs`
  - `tests/nesting_calculation_test.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Add tests in `rust_complexity_calculation.rs` for else-if chain scenarios
- **Integration Tests**: Update `tests/nesting_calculation_test.rs` with else-if chain test cases
- **Regression Tests**: Run full test suite to ensure no regressions
- **Real-World Validation**: Verify Zed's `mention.rs::parse` reports reasonable nesting (≤4)

## Documentation Requirements

- **Code Documentation**: Update docstrings for `calculate_nesting` functions
- **User Documentation**: None needed (internal fix)
- **Architecture Updates**: None needed

## Implementation Notes

- The `pure.rs` implementation is already correct - use it as reference
- Be careful with the visitor pattern - the syn crate's default visitor visits all children recursively
- Consider consolidating to a single nesting calculation function to avoid future drift

## Migration and Compatibility

- **Breaking Changes**: Nesting depth values will decrease for codebases with `else if` chains
- **User Impact**: Complexity scores and recommendations may change for affected functions
- **Backward Compatibility**: This is a bug fix - the new behavior is more accurate
