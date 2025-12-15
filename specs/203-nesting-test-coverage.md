---
number: 203
title: Add Comprehensive Nesting Calculation Test Coverage
category: testing
priority: high
status: draft
dependencies: [198, 201]
created: 2025-12-15
---

# Specification 203: Add Comprehensive Nesting Calculation Test Coverage

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: Spec 198 (else-if fix), Spec 201 (consolidation)

## Context

The nesting calculation bugs (specs 198, 199, 200) went undetected because test coverage for nesting depth is insufficient. Current tests:

1. **Don't test else-if chains specifically for nesting** - The cognitive complexity test has `else_if_chain` but doesn't assert nesting depth
2. **Don't test `else { if }` vs `else if`** - These should behave the same but aren't tested
3. **Don't verify consistency** - No test ensures all nesting implementations return identical values
4. **Missing edge cases** - Complex real-world patterns aren't covered

### Current Test Gaps

| Scenario | Tested? | File |
|----------|---------|------|
| Simple if | Yes | `nesting_calculation_test.rs` |
| Nested if (if inside if) | Yes | `nesting_calculation_test.rs` |
| else-if chain nesting | **NO** | - |
| `else { if }` block | **NO** | - |
| match with control flow in arms | Partial | - |
| for/while/loop nesting | Yes | `nesting_calculation_test.rs` |
| Consistency across implementations | **NO** | - |
| Real-world complex functions | **NO** | - |

### Why This Matters

The nesting value directly affects:
- Cognitive complexity calculation (nesting adds penalty)
- Debt scoring and prioritization
- Refactoring recommendations
- Function classification

Incorrect nesting values cascade into incorrect analysis results across the entire system.

## Objective

Add comprehensive test coverage for nesting calculations that:
1. Covers all edge cases and control flow patterns
2. Verifies consistency across all nesting implementations
3. Tests real-world code patterns
4. Prevents regression of fixed bugs

## Requirements

### Functional Requirements

1. **Test else-if chains**: Verify nesting is 1, not N
2. **Test `else { if }` equivalence**: Should behave same as `else if`
3. **Test consistency**: All implementations return same value
4. **Test edge cases**: Complex nested patterns
5. **Test real-world patterns**: Patterns from actual codebases

### Non-Functional Requirements

1. **Fast execution**: Tests should complete in <1 second
2. **Clear failure messages**: Easy to diagnose what went wrong
3. **Maintainable**: Tests should be easy to update when behavior changes

## Acceptance Criteria

- [ ] Test for else-if chain returns nesting 1
- [ ] Test for `else { if }` returns nesting 1
- [ ] Test for nested if inside then returns nesting 2
- [ ] Test for match with if in arm returns nesting 2
- [ ] Test for complex multi-level nesting
- [ ] Consistency test comparing all implementations
- [ ] Tests for each control flow type (if, while, for, loop, match)
- [ ] All tests pass after spec 201 consolidation

## Technical Details

### Test File Location

Create new test file: `tests/nesting_depth_comprehensive_test.rs`

### Test Cases

#### 1. Else-If Chain Tests

```rust
#[test]
fn test_else_if_chain_nesting_depth_1() {
    let code = r#"
    fn test() {
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

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(nesting, 1, "else-if chain should have nesting 1, not {}", nesting);
}

#[test]
fn test_long_else_if_chain() {
    let code = r#"
    fn test() {
        if a { 1 }
        else if b { 2 }
        else if c { 3 }
        else if d { 4 }
        else if e { 5 }
        else if f { 6 }
        else if g { 7 }
        else if h { 8 }
        else { 9 }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(nesting, 1, "8-branch else-if chain should still have nesting 1");
}
```

#### 2. Else Block Equivalence Tests

```rust
#[test]
fn test_else_block_with_if_equals_else_if() {
    let else_if_code = r#"
    fn test() {
        if a {
            x
        } else if b {
            y
        }
    }
    "#;

    let else_block_code = r#"
    fn test() {
        if a {
            x
        } else {
            if b {
                y
            }
        }
    }
    "#;

    let nesting1 = calculate_nesting_for_code(else_if_code);
    let nesting2 = calculate_nesting_for_code(else_block_code);

    assert_eq!(nesting1, nesting2,
        "else if and else {{ if }} should have same nesting");
    assert_eq!(nesting1, 1, "Both should have nesting 1");
}

#[test]
fn test_else_block_with_statement_before_if() {
    let code = r#"
    fn test() {
        if a {
            x
        } else {
            let y = 1;  // Statement before if
            if b {
                y
            }
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    // This is debatable - the inner if IS inside a block
    // Document expected behavior
    assert_eq!(nesting, 2, "if after statement in else block has nesting 2");
}
```

#### 3. Nested Control Flow Tests

```rust
#[test]
fn test_if_inside_if() {
    let code = r#"
    fn test() {
        if a {
            if b {
                x
            }
        }
    }
    "#;

    assert_eq!(calculate_nesting_for_code(code), 2);
}

#[test]
fn test_if_inside_for() {
    let code = r#"
    fn test() {
        for i in items {
            if i > 0 {
                x
            }
        }
    }
    "#;

    assert_eq!(calculate_nesting_for_code(code), 2);
}

#[test]
fn test_match_inside_while() {
    let code = r#"
    fn test() {
        while condition {
            match x {
                A => {}
                B => {}
            }
        }
    }
    "#;

    assert_eq!(calculate_nesting_for_code(code), 2);
}
```

#### 4. Match Arm Tests

```rust
#[test]
fn test_match_with_if_in_arm() {
    let code = r#"
    fn test() {
        match x {
            A => {
                if y {
                    z
                }
            }
            B => {}
        }
    }
    "#;

    assert_eq!(calculate_nesting_for_code(code), 2);
}

#[test]
fn test_match_with_else_if_in_arm() {
    let code = r#"
    fn test() {
        match x {
            A => {
                if a { 1 }
                else if b { 2 }
                else { 3 }
            }
            B => {}
        }
    }
    "#;

    // match is nesting 1, else-if inside is still flat at nesting 2
    assert_eq!(calculate_nesting_for_code(code), 2);
}
```

#### 5. Complex Real-World Pattern Tests

```rust
#[test]
fn test_url_parser_pattern() {
    // Pattern from Zed's mention.rs that triggered the bug discovery
    let code = r#"
    fn parse(input: &str) -> Result<Self> {
        let url = parse_url(input)?;
        match url.scheme() {
            "file" => {
                if let Some(fragment) = url.fragment() {
                    if let Some(name) = get_param(&url, "symbol")? {
                        Ok(Symbol { name })
                    } else {
                        Ok(Selection { })
                    }
                } else {
                    Ok(File { })
                }
            }
            "zed" => {
                if let Some(id) = path.strip_prefix("/thread/") {
                    Ok(Thread { id })
                } else if let Some(path) = path.strip_prefix("/text-thread/") {
                    Ok(TextThread { path })
                } else if let Some(id) = path.strip_prefix("/rule/") {
                    Ok(Rule { id })
                } else if path.starts_with("/pasted-image") {
                    Ok(PastedImage)
                } else if path.starts_with("/untitled-buffer") {
                    Ok(Selection { })
                } else if let Some(name) = path.strip_prefix("/symbol/") {
                    Ok(Symbol { name })
                } else if path.starts_with("/file") {
                    Ok(File { })
                } else if path.starts_with("/directory") {
                    Ok(Directory { })
                } else {
                    Err("invalid")
                }
            }
            _ => Err("unknown")
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);

    // match (1) -> if let in file arm (2) -> nested if let (3)
    // The zed arm has else-if chain at level 2, doesn't go deeper
    assert!(nesting <= 4, "Complex parser should have nesting <= 4, got {}", nesting);
}
```

#### 6. Consistency Tests

```rust
#[test]
fn test_nesting_consistency_all_implementations() {
    let test_cases = vec![
        ("simple", "fn f() { let x = 1; }"),
        ("if", "fn f() { if a { x } }"),
        ("if_else", "fn f() { if a { x } else { y } }"),
        ("else_if", "fn f() { if a { x } else if b { y } else { z } }"),
        ("nested_if", "fn f() { if a { if b { x } } }"),
        ("for_if", "fn f() { for i in x { if a { y } } }"),
        ("match_if", "fn f() { match x { A => { if a { y } }, B => {} } }"),
    ];

    for (name, code) in test_cases {
        let pure = calculate_via_pure(code);
        let extractor = calculate_via_extractor(code);
        let rust_calc = calculate_via_rust_complexity_calculation(code);

        assert_eq!(pure, extractor,
            "Case '{}': pure ({}) != extractor ({})", name, pure, extractor);
        assert_eq!(pure, rust_calc,
            "Case '{}': pure ({}) != rust_calc ({})", name, pure, rust_calc);
    }
}
```

### Helper Functions

```rust
fn calculate_nesting_for_code(code: &str) -> u32 {
    let ast = syn::parse_file(code).unwrap();
    // Find first function and calculate its nesting
    for item in &ast.items {
        if let syn::Item::Fn(func) = item {
            return crate::complexity::pure::calculate_max_nesting_depth(&func.block);
        }
    }
    panic!("No function found in test code");
}

fn calculate_via_pure(code: &str) -> u32 { /* ... */ }
fn calculate_via_extractor(code: &str) -> u32 { /* ... */ }
fn calculate_via_rust_complexity_calculation(code: &str) -> u32 { /* ... */ }
```

## Dependencies

- **Prerequisites**: Specs 198 and 201 should be implemented first
- **Affected Components**: Test infrastructure only
- **External Dependencies**: None

## Testing Strategy

- These ARE the tests, so meta-testing isn't needed
- Verify tests fail before fixes and pass after
- Run as part of CI

## Documentation Requirements

- **Code Documentation**: Document expected behavior in test comments
- **Test Documentation**: Explain why each test case matters

## Implementation Notes

- Add tests BEFORE implementing fixes to verify they fail
- Tests should remain after implementation to prevent regression
- Consider property-based testing with proptest for generating random code

## Migration and Compatibility

- **Breaking Changes**: None (adding tests only)
- **CI Impact**: Test suite will grow, may need to update CI timeouts
