---
number: 200
title: Fix Block-Based Nesting in languages/rust.rs
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-15
---

# Specification 200: Fix Block-Based Nesting in languages/rust.rs

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The `NestingCalculator` in `src/complexity/languages/rust.rs` incorrectly calculates nesting depth by incrementing for **every** `syn::Block` encountered, rather than only for control flow structures.

### Current Implementation (Buggy)

```rust
/// Visitor to calculate nesting depth
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
}

impl Visit<'_> for NestingCalculator {
    fn visit_block(&mut self, block: &Block) {
        self.current_depth += 1;  // BUG: Increments for ALL blocks!
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_block(self, block);
        self.current_depth -= 1;
    }
}
```

### Problem

This implementation counts **every** block as a nesting level, including:
- Function bodies (should be nesting 0)
- Impl block bodies
- Module bodies
- Closure bodies
- Match arm blocks
- Simple expression blocks `{ expr }`

### Example of Incorrect Behavior

```rust
fn simple() {     // Function body is a Block
    let x = 1;    // Current impl reports nesting = 1
}                 // Should report nesting = 0

fn with_if() {
    if true {     // Current impl: nesting = 2 (function block + if block)
        x         // Should report nesting = 1 (only the if)
    }
}
```

### Impact

This is used in `RustEntropyAnalyzer::analyze_structure()` which feeds into entropy-based complexity scoring. Incorrect nesting values affect:
- Entropy score calculations
- Complexity pattern detection
- Function classification

## Objective

Fix `NestingCalculator` to only count control flow structures as nesting increments, not arbitrary blocks.

## Requirements

### Functional Requirements

1. **Only count control flow nesting**: `if`, `while`, `for`, `loop`, `match`
2. **Function bodies should not count**: A function with no control flow should have nesting 0
3. **Handle closures appropriately**: Closure bodies may or may not count (align with other implementations)
4. **Align with pure.rs**: Results should match `complexity::pure::calculate_max_nesting_depth`

### Non-Functional Requirements

1. **Consistency**: Must align with other nesting calculations in the codebase
2. **Performance**: No regression in entropy calculation speed

## Acceptance Criteria

- [ ] Simple function with no control flow reports nesting 0
- [ ] Single `if` reports nesting 1
- [ ] Nested `if` inside `if` reports nesting 2
- [ ] `else if` chains report nesting 1 (per spec 198)
- [ ] Match with nested control flow in arms reports correct depth
- [ ] Results match `complexity::pure::calculate_max_nesting_depth` for same input
- [ ] Entropy scores remain reasonable after fix

## Technical Details

### Implementation Approach

Replace the block-based visitor with an expression-based visitor that only counts control flow:

```rust
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
}

impl<'ast> Visit<'ast> for NestingCalculator {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::If(if_expr) => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);

                // Visit condition (no nesting change)
                syn::visit::visit_expr(self, &if_expr.cond);
                // Visit then branch
                syn::visit::visit_block(self, &if_expr.then_branch);

                self.current_depth -= 1;

                // Visit else branch at original depth (handles else-if)
                if let Some((_, else_expr)) = &if_expr.else_branch {
                    syn::visit::visit_expr(self, else_expr);
                }
            }
            Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);
                syn::visit::visit_expr(self, expr);
                self.current_depth -= 1;
            }
            Expr::Match(match_expr) => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);

                // Visit the matched expression
                syn::visit::visit_expr(self, &match_expr.expr);

                // Visit each arm's body
                for arm in &match_expr.arms {
                    syn::visit::visit_expr(self, &arm.body);
                }

                self.current_depth -= 1;
            }
            _ => {
                syn::visit::visit_expr(self, expr);
            }
        }
    }

    // Don't increment for blocks - let expression handling manage depth
    fn visit_block(&mut self, block: &'ast Block) {
        syn::visit::visit_block(self, block);
    }
}
```

### Alternative: Use Pure Implementation

The cleanest fix is to delegate to `complexity::pure::calculate_max_nesting_depth`:

```rust
fn calculate_max_nesting(&self, item_fn: &ItemFn) -> u32 {
    crate::complexity::pure::calculate_max_nesting_depth(&item_fn.block)
}
```

This ensures consistency with the correct implementation. See Spec 201.

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/complexity/languages/rust.rs`
  - `src/complexity/entropy_core.rs` (uses this function)
- **Related Specs**: Spec 198 (else-if), Spec 201 (consolidation)

## Testing Strategy

- **Unit Tests**: Add tests for `calculate_max_nesting` function
- **Integration Tests**: Verify entropy calculations remain sensible
- **Comparison Tests**: Compare results with `pure.rs` implementation

## Documentation Requirements

- **Code Documentation**: Update docstrings for `NestingCalculator`

## Implementation Notes

- This may become obsolete if spec 201 consolidates all nesting to `pure.rs`
- The current implementation has been in use, so entropy scores will change after this fix

## Migration and Compatibility

- **Breaking Changes**: Entropy scores will change for all analyzed code
- **User Impact**: Complexity metrics may shift, affecting prioritization
- **Recommendation**: Implement as part of spec 201 consolidation to minimize churn
