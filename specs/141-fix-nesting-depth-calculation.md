---
number: 141
title: Fix Nesting Depth Calculation for Boolean Operators
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-23
---

# Specification 141: Fix Nesting Depth Calculation for Boolean Operators

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

**Current Problem**: The nesting depth calculation incorrectly counts boolean operators (`||` and `&&`) as nesting levels, inflating complexity metrics for functions with compound conditionals.

**Example of Incorrect Behavior**:
```rust
fn check_conditions(x: i32, y: i32, z: i32) {
    // Currently reports nesting depth > 1 due to || and &&
    // Should report nesting depth = 1 (only the if block counts)
    if (x > 0 && y > 0) || z > 10 {
        println!("Condition met");
    }
}
```

**Root Cause**: The `NestingCalculator` visitor in `src/complexity/languages/rust.rs:282-294` counts ALL `Block` nodes indiscriminately:

```rust
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
}

impl Visit<'_> for NestingCalculator {
    fn visit_block(&mut self, block: &Block) {
        self.current_depth += 1;  // ❌ Counts every block including those from binary ops
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_block(self, block);
        self.current_depth -= 1;
    }
}
```

The issue is that the Rust `syn` AST creates implicit `Block` nodes for:
- Binary operators with short-circuit evaluation (`&&`, `||`)
- Match arm bodies
- Closure bodies
- Other expression contexts

**Impact**:
- **Inflated complexity scores**: Functions with simple boolean logic appear more complex than they are
- **False positives**: Well-written functions with compound conditionals flagged as "deeply nested"
- **Misleading metrics**: Nesting depth doesn't reflect actual cognitive load
- **Inconsistent results**: Different languages handle boolean operators differently in nesting calculation

**Why This Matters**:
- Nesting depth is a critical complexity metric for identifying hard-to-understand code
- Boolean operators (`&&`, `||`) are fundamental building blocks, not structural complexity
- Current implementation penalizes idiomatic code patterns
- Affects prioritization and scoring of technical debt items

## Objective

Fix nesting depth calculation to count only **actual control flow block nesting** (if, loop, match, while, for) and exclude boolean operators, match arms, and other expression-level constructs.

**Success Criteria**:
- `if x > 0 && y > 0` reports nesting depth of 1 (not 2+)
- Nested if statements correctly report their depth: `if { if { } }` = depth 2
- Match expressions count blocks correctly (match itself + arm bodies)
- All three complexity calculators (cyclomatic, cognitive, nesting) are aligned

## Requirements

### Functional Requirements

**FR1: Control Flow Block Detection**
- Only increment nesting depth for actual control flow structures:
  - `if` and `if-else` blocks
  - `match` expression and match arm blocks
  - Loop blocks: `for`, `while`, `loop`
  - Function bodies (baseline depth of 0 or 1 depending on convention)
- Do NOT increment for:
  - Binary operators (`&&`, `||`, `+`, `-`, etc.)
  - Unary operators (`!`, `-`, `*`, etc.)
  - Method call chains
  - Closure bodies (unless they contain control flow)

**FR2: Accurate Depth Tracking**
- Track entry and exit of control flow blocks correctly
- Handle nested structures: `if { for { while { } } }`
- Handle match arms with nested control flow
- Reset depth appropriately when exiting blocks

**FR3: Language-Specific Implementation**
- Fix Rust nesting calculator (primary)
- Fix Python nesting calculator (same issue likely exists)
- Fix JavaScript/TypeScript nesting calculator
- Ensure consistency across all language analyzers

**FR4: Backward Compatibility**
- Nesting depth values will decrease for many functions (this is expected and correct)
- Update test assertions to match corrected behavior
- Document the change in behavior for users
- Provide migration guide for interpreting new vs old nesting values

### Non-Functional Requirements

**NFR1: Performance**
- No measurable performance degradation (<1% difference)
- Maintain same visitor pattern architecture
- Avoid expensive AST traversals or pattern matching

**NFR2: Correctness**
- 100% alignment between calculated depth and actual structural nesting
- Deterministic results (same code always produces same depth)
- No edge cases where depth is miscounted

**NFR3: Maintainability**
- Clear, documented logic for what counts as nesting
- Consistent implementation across all language analyzers
- Easy to extend for new control flow constructs

## Acceptance Criteria

### AC1: Boolean Operators Don't Increase Nesting

Test cases that MUST pass:

```rust
#[test]
fn test_boolean_operators_no_nesting() {
    let code = r#"
        fn test(x: i32, y: i32) {
            if x > 0 && y > 0 {  // Nesting = 1
                println!("both positive");
            }
        }
    "#;
    assert_nesting_depth(code, "test", 1);
}

#[test]
fn test_complex_boolean_expression() {
    let code = r#"
        fn test(x: i32, y: i32, z: i32) {
            if (x > 0 && y > 0) || (z < 0 && x != y) {  // Nesting = 1
                println!("complex condition");
            }
        }
    "#;
    assert_nesting_depth(code, "test", 1);
}
```

### AC2: Actual Nesting Is Counted Correctly

Test cases for real nesting:

```rust
#[test]
fn test_nested_if_statements() {
    let code = r#"
        fn test(x: i32) {
            if x > 0 {           // Nesting = 1
                if x < 10 {      // Nesting = 2
                    if x == 5 {  // Nesting = 3
                        println!("x is 5");
                    }
                }
            }
        }
    "#;
    assert_nesting_depth(code, "test", 3);
}

#[test]
fn test_nested_loops() {
    let code = r#"
        fn test() {
            for i in 0..10 {     // Nesting = 1
                for j in 0..10 { // Nesting = 2
                    println!("{} {}", i, j);
                }
            }
        }
    "#;
    assert_nesting_depth(code, "test", 2);
}
```

### AC3: Match Expressions Handled Correctly

```rust
#[test]
fn test_match_nesting() {
    let code = r#"
        fn test(x: Option<i32>) {
            match x {            // Nesting = 1 (match block)
                Some(val) => {
                    if val > 0 { // Nesting = 2 (if inside match arm)
                        println!("positive");
                    }
                },
                None => {}
            }
        }
    "#;
    assert_nesting_depth(code, "test", 2);
}
```

### AC4: Mixed Control Flow

```rust
#[test]
fn test_mixed_control_flow() {
    let code = r#"
        fn test(x: i32) {
            if x > 0 && x < 100 {  // Nesting = 1 (if block, && doesn't count)
                for i in 0..x {     // Nesting = 2
                    while i > 0 {   // Nesting = 3
                        match i {   // Nesting = 4
                            0 => break,
                            _ => {}
                        }
                    }
                }
            }
        }
    "#;
    assert_nesting_depth(code, "test", 4);
}
```

### AC5: Python Boolean Operators

```python
def test_python_boolean():
    # Should be nesting depth 1, not 2+
    if x > 0 and y > 0 or z < 0:
        print("condition met")
```

### AC6: JavaScript Boolean Operators

```javascript
function test(x, y, z) {
    // Should be nesting depth 1, not 2+
    if ((x > 0 && y > 0) || z < 0) {
        console.log("condition met");
    }
}
```

### AC7: Existing Tests Updated

- [ ] Update `tests/nesting_calculation_test.rs` to reflect corrected behavior
- [ ] Update `tests/complexity_tests.rs` assertions
- [ ] Update `tests/cognitive_complexity_tests.rs` if affected
- [ ] All existing tests pass with updated expected values

### AC8: Documentation

- [ ] Document what counts as nesting in code comments
- [ ] Update CLAUDE.md with nesting calculation rules
- [ ] Add examples to function documentation
- [ ] Include migration notes for users with existing baselines

## Technical Details

### Implementation Approach

**Phase 1: Fix Rust NestingCalculator**

Current (incorrect) implementation:
```rust
// src/complexity/languages/rust.rs:282-294
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
}

impl Visit<'_> for NestingCalculator {
    fn visit_block(&mut self, block: &Block) {
        self.current_depth += 1;  // ❌ Too broad
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_block(self, block);
        self.current_depth -= 1;
    }
}
```

**Proposed Solution 1: Visit Control Flow Expressions Instead of Blocks**

```rust
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
}

impl Visit<'_> for NestingCalculator {
    // Increment for if expressions
    fn visit_expr_if(&mut self, node: &syn::ExprIf) {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_expr_if(self, node);
        self.current_depth -= 1;
    }

    // Increment for match expressions
    fn visit_expr_match(&mut self, node: &syn::ExprMatch) {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_expr_match(self, node);
        self.current_depth -= 1;
    }

    // Increment for while loops
    fn visit_expr_while(&mut self, node: &syn::ExprWhile) {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_expr_while(self, node);
        self.current_depth -= 1;
    }

    // Increment for for loops
    fn visit_expr_for_loop(&mut self, node: &syn::ExprForLoop) {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_expr_for_loop(self, node);
        self.current_depth -= 1;
    }

    // Increment for loop loops
    fn visit_expr_loop(&mut self, node: &syn::ExprLoop) {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
        syn::visit::visit_expr_loop(self, node);
        self.current_depth -= 1;
    }

    // Do NOT implement visit_expr_binary - let it pass through without incrementing
    // Do NOT implement visit_block - this was the root of the problem
}
```

**Proposed Solution 2: Contextual Block Tracking (Alternative)**

```rust
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
    in_control_flow: bool,  // Track if we're inside a control flow structure
}

impl Visit<'_> for NestingCalculator {
    fn visit_expr(&mut self, expr: &Expr) {
        let is_control_flow = matches!(
            expr,
            Expr::If(_) | Expr::Match(_) | Expr::While(_) |
            Expr::ForLoop(_) | Expr::Loop(_)
        );

        if is_control_flow {
            self.current_depth += 1;
            self.max_depth = self.max_depth.max(self.current_depth);
        }

        syn::visit::visit_expr(self, expr);

        if is_control_flow {
            self.current_depth -= 1;
        }
    }
}
```

**Recommendation**: Use Solution 1 (explicit visitor methods) because:
- More explicit and easier to understand
- Follows syn visitor pattern idioms
- No risk of miscounting edge cases
- Better performance (no dynamic dispatch)

**Phase 2: Fix Python NestingCalculator**

```python
# Similar issue likely exists in src/complexity/languages/python/core.rs
# or wherever Python nesting is calculated
#
# Python AST nodes to count for nesting:
# - If, While, For, With, Try
# NOT: BoolOp (And, Or), Compare, UnaryOp
```

**Phase 3: Fix JavaScript/TypeScript Calculator**

```javascript
// Similar issue in src/complexity/languages/javascript.rs
// or src/analyzers/javascript/complexity.rs
//
// JavaScript nodes to count:
// - IfStatement, SwitchStatement, WhileStatement, ForStatement, ForInStatement, ForOfStatement
// NOT: LogicalExpression (&&, ||), BinaryExpression
```

**Phase 4: Update Tests**

1. Add new test cases for boolean operators
2. Update existing test assertions to match corrected values
3. Add regression tests for edge cases
4. Verify all three languages produce consistent results

### Architecture Changes

**Modified Files**:
- `src/complexity/languages/rust.rs` - Fix `NestingCalculator` visitor
- `src/complexity/languages/python/core.rs` - Fix Python nesting calculation
- `src/complexity/languages/javascript.rs` - Fix JavaScript nesting calculation

**New Test Files**:
- `tests/nesting_boolean_operators_test.rs` - Comprehensive test suite for fix

**Updated Test Files**:
- `tests/nesting_calculation_test.rs` - Update expected values
- `tests/complexity_tests.rs` - Update assertions
- `tests/cognitive_complexity_tests.rs` - Verify alignment

### Data Structures

No new data structures needed. The existing `NestingCalculator` struct remains the same:

```rust
struct NestingCalculator {
    current_depth: u32,
    max_depth: u32,
}
```

### APIs and Interfaces

No API changes. The public interface remains:

```rust
// Internal function (not public)
fn calculate_max_nesting(&self, item_fn: &ItemFn) -> u32
```

This is called internally by:
- `RustEntropyAnalyzer::analyze_structure()` (line 126)
- Used in entropy analysis and complexity metrics

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/complexity/languages/rust.rs` - Rust nesting calculator
- `src/complexity/languages/python/core.rs` - Python nesting calculator
- `src/complexity/languages/javascript.rs` - JavaScript nesting calculator
- All complexity-related tests

**External Dependencies**: None (uses existing `syn` visitor API)

## Testing Strategy

### Unit Tests

**New Test File**: `tests/nesting_boolean_operators_test.rs`

```rust
use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::Analyzer;
use std::path::PathBuf;

/// Helper function to assert nesting depth
fn assert_nesting_depth(code: &str, fn_name: &str, expected_depth: u32) {
    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("test.rs");
    let ast = analyzer.parse(code, path).unwrap();
    let metrics = analyzer.analyze(&ast);

    let func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name == fn_name)
        .unwrap_or_else(|| panic!("Function '{}' not found", fn_name));

    assert_eq!(
        func.nesting, expected_depth,
        "Function '{}' should have nesting depth {}, got {}",
        fn_name, expected_depth, func.nesting
    );
}

#[test]
fn test_single_and_operator() {
    let code = r#"
        fn test(x: i32, y: i32) {
            if x > 0 && y > 0 {
                println!("both positive");
            }
        }
    "#;
    assert_nesting_depth(code, "test", 1);
}

#[test]
fn test_single_or_operator() {
    let code = r#"
        fn test(x: i32, y: i32) {
            if x > 0 || y > 0 {
                println!("at least one positive");
            }
        }
    "#;
    assert_nesting_depth(code, "test", 1);
}

#[test]
fn test_complex_boolean_expression() {
    let code = r#"
        fn test(x: i32, y: i32, z: i32) {
            if (x > 0 && y > 0) || (z < 0 && x != y) {
                println!("complex condition");
            }
        }
    "#;
    assert_nesting_depth(code, "test", 1);
}

#[test]
fn test_nested_if_with_boolean_operators() {
    let code = r#"
        fn test(x: i32, y: i32) {
            if x > 0 && y > 0 {      // Nesting = 1
                if x < 10 || y < 10 { // Nesting = 2
                    println!("nested with booleans");
                }
            }
        }
    "#;
    assert_nesting_depth(code, "test", 2);
}

#[test]
fn test_match_with_boolean_guard() {
    let code = r#"
        fn test(x: Option<i32>, flag: bool) {
            match x {
                Some(val) if val > 0 && flag => {  // Guard doesn't add nesting
                    println!("matched");
                },
                _ => {}
            }
        }
    "#;
    assert_nesting_depth(code, "test", 1);
}

#[test]
fn test_deeply_nested_without_booleans() {
    let code = r#"
        fn test() {
            if true {           // 1
                for i in 0..10 { // 2
                    while i > 0 { // 3
                        match i { // 4
                            0 => loop { // 5
                                break;
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    "#;
    assert_nesting_depth(code, "test", 5);
}

#[test]
fn test_no_nesting_with_only_booleans() {
    let code = r#"
        fn test(x: i32, y: i32, z: i32) {
            let result = x > 0 && y > 0 || z < 0;
            println!("{}", result);
        }
    "#;
    assert_nesting_depth(code, "test", 0);
}
```

### Integration Tests

**Update Existing Test**: `tests/nesting_calculation_test.rs`

```rust
// Update the assertions in the existing test file to match corrected behavior
// This may mean LOWERING some expected values because we're fixing over-counting

#[test]
fn test_rust_nesting_calculation() {
    // ... existing test setup ...

    // test_function: Should be 5 (unchanged - no boolean operators)
    assert_eq!(test_fn.nesting, 5);

    // simple_function: Should be 0 (unchanged)
    assert_eq!(simple_fn.nesting, 0);

    // single_if_function: Should be 1 (unchanged)
    assert_eq!(single_if_fn.nesting, 1);

    // nested_loops: Should be 3 (unchanged)
    assert_eq!(nested_loops_fn.nesting, 3);
}
```

### Python and JavaScript Tests

```rust
// tests/python_nesting_test.rs
#[test]
fn test_python_boolean_operators() {
    let code = r#"
def test(x, y):
    if x > 0 and y > 0:  # Should be nesting depth 1
        print("both positive")
    "#;

    // Test Python analyzer produces correct depth
}

// tests/javascript_nesting_test.rs
#[test]
fn test_javascript_boolean_operators() {
    let code = r#"
function test(x, y) {
    if (x > 0 && y > 0) {  // Should be nesting depth 1
        console.log("both positive");
    }
}
    "#;

    // Test JavaScript analyzer produces correct depth
}
```

### Regression Tests

Ensure the fix doesn't break existing behavior:

```rust
#[test]
fn test_regression_match_nesting() {
    // Verify match expressions still counted correctly
}

#[test]
fn test_regression_loop_nesting() {
    // Verify loop nesting still works
}

#[test]
fn test_regression_closure_nesting() {
    // Verify closures don't add unexpected nesting
}
```

## Documentation Requirements

### Code Documentation

**Update `src/complexity/languages/rust.rs`**:

```rust
/// Calculate maximum nesting depth
///
/// Counts only actual control flow block nesting:
/// - if/else expressions
/// - match expressions
/// - for loops
/// - while loops
/// - loop loops
///
/// Does NOT count as nesting:
/// - Boolean operators (&&, ||)
/// - Match arm guard expressions
/// - Closure bodies (unless containing control flow)
/// - Binary/unary operators
///
/// # Examples
///
/// ```
/// // Nesting depth = 1 (only the if block)
/// if x > 0 && y > 0 {
///     println!("both positive");
/// }
///
/// // Nesting depth = 2 (if + nested if)
/// if x > 0 {
///     if y > 0 {
///         println!("both positive");
///     }
/// }
/// ```
fn calculate_max_nesting(&self, item_fn: &ItemFn) -> u32 {
    // ... implementation ...
}
```

### User Documentation

**Update CLAUDE.md**:

```markdown
## Nesting Depth Calculation

Debtmap calculates nesting depth by counting **actual control flow block nesting**, not expression complexity.

### What Counts as Nesting:
- `if` and `if-else` blocks
- `match` expressions
- Loop structures: `for`, `while`, `loop`
- Each level of nesting adds 1 to the depth

### What Does NOT Count:
- Boolean operators (`&&`, `||`) - these are expression operators, not structural nesting
- Match guards - part of pattern matching, not nested blocks
- Method chains - sequential operations, not nested control flow

### Examples:

**Nesting Depth = 1:**
```rust
if x > 0 && y > 0 && z > 0 {  // Complex boolean, but still depth 1
    process();
}
```

**Nesting Depth = 3:**
```rust
if x > 0 {           // Depth 1
    for i in 0..10 { // Depth 2
        while i > 0 { // Depth 3
            work();
        }
    }
}
```

This approach aligns with cognitive complexity principles: structural nesting increases mental load, but boolean operators are fundamental building blocks.
```

### Migration Guide

**For Users with Existing Baselines**:

```markdown
## Migration Guide: Nesting Depth Calculation Fix

**Version**: 0.3.0+
**Breaking Change**: Nesting depth values will be lower for many functions

### What Changed

We fixed a bug where boolean operators (`&&`, `||`) were incorrectly counted as nesting levels. This caused functions with compound conditionals to report inflated nesting depths.

### Impact on Your Baselines

Functions with boolean operators will now report **lower** nesting depth values:

**Before (incorrect)**:
```rust
if x > 0 && y > 0 || z < 0 {  // Reported depth = 3
    work();
}
```

**After (correct)**:
```rust
if x > 0 && y > 0 || z < 0 {  // Reports depth = 1
    work();
}
```

### What To Do

1. **Regenerate baselines**: Run debtmap analysis again to get corrected values
2. **Review changes**: Functions that decreased significantly may have been false positives
3. **Adjust thresholds**: If you have custom nesting depth thresholds, they may need adjustment
4. **Celebrate**: Your code is less complex than you thought!

### When To Worry

If a function's nesting depth **increased**, this indicates a real issue was being masked. Investigate these cases.
```

## Implementation Notes

### Visitor Pattern Best Practices

1. **Use explicit visitor methods** for each control flow type:
   - More maintainable than generic `visit_expr` matching
   - Clearer intent in code
   - Easier to debug and test

2. **Don't override `visit_block`**:
   - Blocks are created for many non-nesting contexts
   - Too broad to use for structural counting
   - Root cause of the current bug

3. **Maintain symmetry**:
   - Every `current_depth += 1` must have corresponding `-= 1`
   - Use RAII-like pattern: increment before visiting, decrement after

### Gotchas

1. **Closure bodies**: Should closures count as nesting?
   - **Decision**: No, unless they contain control flow
   - Rationale: Closures are more like function definitions

2. **Match arms**: Does each arm add nesting?
   - **Decision**: No, only the match expression itself
   - Rationale: Arms are alternatives, not nested depth

3. **Async blocks**: Do `async` blocks count?
   - **Decision**: No, they're like closures
   - Exception: If they contain control flow, that counts

4. **Guard clauses**: Do match guards add nesting?
   - **Decision**: No, guards are part of pattern matching
   - Example: `Some(x) if x > 0 && flag` - the `&&` doesn't count

### Testing Edge Cases

- [ ] Empty functions (depth = 0)
- [ ] Function with only boolean expressions (depth = 0)
- [ ] Deeply nested (depth > 10) to test counter overflow
- [ ] Match with many arms but no nesting in arms (depth = 1)
- [ ] Closure containing nested if (does closure count?)
- [ ] Try-catch blocks (if/when implemented)

## Migration and Compatibility

### Breaking Changes

**Nesting Depth Values Will Change**:
- Many functions will report **lower** nesting depth
- This is a **correction**, not a regression
- Users with baselines will need to regenerate

### Backward Compatibility

- No API changes
- No config file changes
- Output format remains the same
- Only the numeric values change

### Version Strategy

1. **v0.2.x**: Current (buggy) behavior
2. **v0.3.0**: This fix implemented
3. Release notes must clearly explain the change

### Configuration

No new configuration needed. Nesting calculation is deterministic and not configurable.

## Success Metrics

### Quantitative Metrics

1. **Correctness**:
   - ✅ 100% of boolean operator cases report depth 1 (or actual nesting level)
   - ✅ 0% false positives from boolean operators
   - ✅ Alignment with cognitive complexity metrics

2. **Coverage**:
   - ✅ All test cases pass (new and updated)
   - ✅ All three languages fixed (Rust, Python, JavaScript)
   - ✅ No regression in existing functionality

3. **Performance**:
   - ✅ <1% performance impact
   - ✅ Same time complexity (O(n) AST traversal)

### Qualitative Metrics

1. **Accuracy**:
   - Nesting depth reflects actual cognitive load
   - Boolean operators don't inflate complexity scores
   - Metrics align with developer intuition

2. **Consistency**:
   - All languages use same nesting rules
   - Documentation clearly explains what counts
   - Edge cases handled uniformly

## Future Enhancements

### Post-v0.3.0 Improvements

1. **Weighted Nesting** (v0.4.0):
   - Different weights for different nesting types
   - Match might be "cheaper" than nested if
   - Loop might be "more expensive" than single if

2. **Context-Aware Nesting** (v0.5.0):
   - Closures in iterators might not count
   - Early returns reduce perceived nesting
   - Guard clauses should reduce score

3. **Cross-Language Normalization** (v0.4.0):
   - Ensure Python, Rust, JS produce comparable metrics
   - Account for language-specific idioms
   - Normalize for ecosystem conventions

## Related Issues

- **Cognitive Complexity**: Already handles boolean operators correctly - should align
- **Cyclomatic Complexity**: Counts decision points - different from nesting but related
- **Entropy Analysis**: Uses nesting depth - will benefit from fix

## References

- **Cognitive Complexity Paper**: https://www.sonarsource.com/docs/CognitiveComplexity.pdf
- **syn Documentation**: https://docs.rs/syn/latest/syn/visit/
- **Nesting Depth Best Practices**: Various academic papers on code complexity
