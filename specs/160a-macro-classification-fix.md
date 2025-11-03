---
number: 160a
title: Fix Macro Classification in Purity Detection
category: bug-fix
priority: high
status: draft
dependencies: [156, 157]
created: 2025-11-01
updated: 2025-11-03
---

# Specification 160a: Fix Macro Classification in Purity Detection

**Category**: bug-fix
**Priority**: high
**Status**: draft
**Dependencies**: Specs 156, 157

## Context

**Current Bug** (`purity_detector.rs:635-650`): Macros use substring matching causing false positives.

### Critical Problems

1. **Substring matching is too broad**
   ```rust
   if macro_path.contains("assert")  // ← Matches "debug_assert_eq", "assert_eq", etc.
   ```
   - `debug_assert!` marked impure in ALL builds (should be pure in release)
   - `macro_path.contains("debug")` flags ANY macro with "debug" in the name
   - Custom macros like `my_debug_helper!` incorrectly flagged

2. **Only checks statement macros** (`Stmt::Macro`)
   - Misses expression macros like `let x = dbg!(value);`
   - Inconsistent analysis across macro contexts

3. **No build configuration awareness**
   - `debug_assert!` is compiled out in release builds (pure)
   - Currently marked as I/O operation regardless of build mode

## Objective

Fix immediate macro classification bugs using exact name matching and conditional compilation awareness.

## Requirements

### 1. **Exact Macro Name Matching**
   - Replace `.contains()` with exact `match` on macro name
   - No false positives from substring matches
   - Clear categorization of known macros

### 2. **Conditional Compilation Support**
   - `debug_assert!` and variants: pure in release, impure in debug
   - `assert!` and variants: always impure (panics on failure)
   - Respect `cfg!(debug_assertions)` in analysis

### 3. **Handle Both Statement and Expression Macros**
   - Process `Stmt::Macro` (statement context)
   - Process `Expr::Macro` (expression context)
   - Consistent classification across contexts

## Implementation

### Step 1: Extract macro name helper

```rust
/// Extract the last segment of a macro path
/// e.g., "std::println" -> "println", "assert_eq" -> "assert_eq"
fn extract_macro_name(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|seg| seg.ident.to_string())
        .unwrap_or_default()
}
```

### Step 2: Add macro handler to PurityDetector

```rust
impl PurityDetector {
    /// Classify a macro and update purity state
    fn handle_macro(&mut self, mac: &syn::Macro) {
        let name = extract_macro_name(&mac.path);

        match name.as_str() {
            // Pure macros - no side effects
            "vec" | "format" | "concat" | "stringify" | "matches"
            | "include_str" | "include_bytes" | "env" | "option_env" => {
                // No effect on purity
            }

            // I/O macros - always impure
            "println" | "eprintln" | "print" | "eprint"
            | "dbg" | "write" | "writeln" => {
                self.has_io_operations = true;
                self.has_side_effects = true;
            }

            // Panic macros - always impure
            "panic" | "unimplemented" | "unreachable" | "todo" => {
                self.has_side_effects = true;
            }

            // Debug-only assertions - conditional purity
            "debug_assert" | "debug_assert_eq" | "debug_assert_ne" => {
                #[cfg(debug_assertions)]
                {
                    self.has_side_effects = true;
                }
                // In release builds, these are compiled out (pure)
            }

            // Regular assertions - always impure (panic on failure)
            "assert" | "assert_eq" | "assert_ne" => {
                self.has_side_effects = true;
            }

            // Unknown macro - reduce confidence slightly
            _ => {
                self.confidence *= 0.95;
            }
        }
    }
}
```

### Step 3: Update visitor methods

```rust
impl<'ast> Visit<'ast> for PurityDetector {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // Handle expression macros: let x = dbg!(value);
            Expr::Macro(expr_macro) => {
                self.handle_macro(&expr_macro.mac);
            }
            // ... other expression handling
            _ => {}
        }

        syn::visit::visit_expr(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Track local variable declarations
        if let Stmt::Local(local) = stmt {
            self.visit_local(local);
        }

        // Handle statement macros: println!("test");
        if let Stmt::Macro(stmt_macro) = stmt {
            self.handle_macro(&stmt_macro.mac);
        }

        syn::visit::visit_stmt(self, stmt);
    }
}
```

## Testing

### Test 1: Debug assertions are pure in release

```rust
#[test]
#[cfg(not(debug_assertions))]
fn test_debug_assert_pure_in_release() {
    let code = r#"
        fn check_bounds(x: usize) -> bool {
            debug_assert!(x < 100);
            debug_assert_eq!(x, x);
            x < 100
        }
    "#;

    let analysis = analyze_purity(code);
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
#[cfg(debug_assertions)]
fn test_debug_assert_impure_in_debug() {
    let code = r#"
        fn check_bounds(x: usize) -> bool {
            debug_assert!(x < 100);
            x < 100
        }
    "#;

    let analysis = analyze_purity(code);
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}
```

### Test 2: I/O macros always impure

```rust
#[test]
fn test_io_macros_always_impure() {
    let test_cases = vec![
        r#"fn f() { println!("test"); }"#,
        r#"fn f() { eprintln!("error"); }"#,
        r#"fn f() { dbg!(42); }"#,
        r#"fn f() { print!("no newline"); }"#,
    ];

    for code in test_cases {
        let analysis = analyze_purity(code);
        assert_eq!(analysis.purity_level, PurityLevel::Impure, "Failed for: {}", code);
    }
}
```

### Test 3: Expression macros detected

```rust
#[test]
fn test_expression_macros() {
    let code = r#"
        fn example() -> i32 {
            let x = dbg!(42);  // Expression macro
            x
        }
    "#;

    let analysis = analyze_purity(code);
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}
```

### Test 4: Pure macros don't affect purity

```rust
#[test]
fn test_pure_macros() {
    let code = r#"
        fn create_list() -> Vec<i32> {
            vec![1, 2, 3]  // Pure macro
        }
    "#;

    let analysis = analyze_purity(code);
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}
```

### Test 5: No false positives from substring matching

```rust
#[test]
fn test_no_substring_false_positives() {
    let code = r#"
        macro_rules! my_debug_helper {
            () => { 42 };  // Pure macro with "debug" in name
        }

        fn example() -> i32 {
            my_debug_helper!()
        }
    "#;

    let analysis = analyze_purity(code);
    // Should not be marked impure just because name contains "debug"
    // Confidence reduced slightly due to unknown macro, but not impure
    assert_ne!(analysis.purity_level, PurityLevel::Impure);
}
```

### Test 6: Regular assertions always impure

```rust
#[test]
fn test_assert_always_impure() {
    let code = r#"
        fn validate(x: i32) -> i32 {
            assert!(x > 0);  // Panics on failure - impure
            x
        }
    "#;

    let analysis = analyze_purity(code);
    assert_eq!(analysis.purity_level, PurityLevel::Impure);
}
```

## Acceptance Criteria

- [x] Replace substring matching with exact macro name matching
- [x] Handle both `Stmt::Macro` and `Expr::Macro` contexts
- [x] `debug_assert!` variants pure in release, impure in debug
- [x] Regular `assert!` variants always impure
- [x] I/O macros (`println!`, etc.) always impure
- [x] Pure macros (`vec!`, etc.) don't affect purity score
- [x] Unknown macros reduce confidence, not marked as impure
- [x] All tests pass in both debug and release modes
- [x] No false positives from substring matching

## Performance Impact

- **Change**: Replace string `.contains()` with `match` on extracted name
- **Expected**: No measurable performance difference (both O(1) operations)
- **Memory**: No additional allocations

## Migration Notes

This is a **bug fix** with no breaking API changes. Existing code will see:
- **Fewer false positives** (functions incorrectly marked impure)
- **More accurate confidence scores** (unknown macros vs impure macros)
- **Build-aware analysis** (`debug_assert!` handling)

## Related Specifications

- **Spec 160b**: Macro Definition Collection (builds on this)
- **Spec 160c**: Custom Macro Heuristic Analysis (builds on this)
