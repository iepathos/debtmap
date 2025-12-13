---
number: 259
title: Fix Constants False Positive in Purity Analysis
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-12
---

# Specification 259: Fix Constants False Positive in Purity Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

**Current Problem**: Using compile-time constants like `std::i32::MAX` triggers "accesses external state" and demotes functions from `StrictlyPure` to `ReadOnly`:

```rust
fn is_valid(x: i32) -> bool {
    x < std::i32::MAX  // Triggers ReadOnly classification
}
```

This is overly conservative. `std::i32::MAX` is a compile-time constant, not external mutable state. The function should be `StrictlyPure`.

**Root Cause** (in `src/analyzers/purity_detector.rs:1043-1048`):
```rust
Expr::Path(path) => {
    let path_str = quote::quote!(#path).to_string();
    // Check if it's accessing a module path (like std::i32::MAX)
    if path_str.contains("::") && !self.scope.is_local(&path_str) {
        self.accesses_external_state = true;  // Too aggressive!
    }
}
```

## Objective

Distinguish compile-time constants from actual external state access, preserving `StrictlyPure` classification for functions that only use constants.

## Requirements

### Functional Requirements

1. **Constant Recognition**
   - Recognize standard library constants (`std::i32::MAX`, `std::f64::INFINITY`, etc.)
   - Recognize `const` items from external crates
   - Recognize enum variants without data
   - Maintain conservative approach for truly unknown paths

2. **Classification Accuracy**
   - Functions using only constants should remain `StrictlyPure`
   - Functions accessing `static` items should be `ReadOnly` or `Impure`
   - Functions accessing `static mut` should be `Impure`

3. **Backward Compatibility**
   - Existing impure classifications should not change
   - Only affects false positives (over-conservative classifications)

### Non-Functional Requirements

- Performance: <1ms overhead per function analyzed
- No external dependencies required

## Implementation

### Approach 1: Whitelist Common Constant Paths (Quick Win)

```rust
/// Known constant paths that don't affect purity
const KNOWN_CONSTANT_PREFIXES: &[&str] = &[
    // Numeric constants
    "std::i8::", "std::i16::", "std::i32::", "std::i64::", "std::i128::", "std::isize::",
    "std::u8::", "std::u16::", "std::u32::", "std::u64::", "std::u128::", "std::usize::",
    "std::f32::", "std::f64::",
    // Core versions
    "core::i8::", "core::i16::", "core::i32::", "core::i64::", "core::i128::", "core::isize::",
    "core::u8::", "core::u16::", "core::u32::", "core::u64::", "core::u128::", "core::usize::",
    "core::f32::", "core::f64::",
    // Common constants
    "std::mem::size_of",
    "std::mem::align_of",
    "core::mem::size_of",
    "core::mem::align_of",
];

const KNOWN_CONSTANT_SUFFIXES: &[&str] = &[
    "::MAX", "::MIN", "::BITS", "::EPSILON", "::INFINITY", "::NEG_INFINITY", "::NAN",
    "::RADIX", "::MANTISSA_DIGITS", "::DIGITS", "::MIN_EXP", "::MAX_EXP",
    "::MIN_10_EXP", "::MAX_10_EXP", "::MIN_POSITIVE",
];

fn is_known_constant(path_str: &str) -> bool {
    // Check prefixes (std::i32::, core::u64::, etc.)
    for prefix in KNOWN_CONSTANT_PREFIXES {
        if path_str.starts_with(prefix) {
            return true;
        }
    }

    // Check suffixes (::MAX, ::MIN, etc.)
    for suffix in KNOWN_CONSTANT_SUFFIXES {
        if path_str.ends_with(suffix) {
            return true;
        }
    }

    false
}
```

### Approach 2: Heuristic Detection (More Comprehensive)

```rust
fn classify_path_purity(&self, path_str: &str) -> PathPurity {
    // 1. Check known constants
    if is_known_constant(path_str) {
        return PathPurity::Constant;
    }

    // 2. Check for SCREAMING_CASE (likely constant)
    let last_segment = path_str.rsplit("::").next().unwrap_or(path_str);
    if is_screaming_case(last_segment) && !last_segment.contains("mut") {
        return PathPurity::ProbablyConstant; // Reduce confidence, not mark impure
    }

    // 3. Check for enum variants (PascalCase after ::)
    if is_pascal_case(last_segment) && !path_str.contains("fn") {
        return PathPurity::ProbablyConstant;
    }

    // 4. Default: unknown, conservative
    PathPurity::Unknown
}

fn is_screaming_case(s: &str) -> bool {
    s.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
}

fn is_pascal_case(s: &str) -> bool {
    s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && !s.chars().all(|c| c.is_uppercase() || c == '_')
}

#[derive(Debug, Clone, Copy)]
enum PathPurity {
    Constant,         // Definitely a constant, no purity impact
    ProbablyConstant, // Likely a constant, reduce confidence slightly
    Unknown,          // Conservative: assume external state access
}
```

### Integration in PurityDetector

Update `visit_expr` in `src/analyzers/purity_detector.rs`:

```rust
// Path expressions may access external state (constants, statics, etc.)
Expr::Path(path) => {
    let path_str = quote::quote!(#path).to_string();

    // Check if it's accessing a module path (like std::i32::MAX)
    if path_str.contains("::") && !self.scope.is_local(&path_str) {
        match self.classify_path_purity(&path_str) {
            PathPurity::Constant => {
                // No impact on purity - it's a compile-time constant
            }
            PathPurity::ProbablyConstant => {
                // Slight confidence reduction but not impure
                self.unknown_macros_count += 1; // Reuse existing confidence reduction
            }
            PathPurity::Unknown => {
                // Conservative: assume external state access
                self.accesses_external_state = true;
            }
        }
    }
}
```

## Acceptance Criteria

- [ ] `fn is_valid(x: i32) -> bool { x < std::i32::MAX }` is classified as `StrictlyPure`
- [ ] `fn max_val() -> i32 { std::i32::MAX }` is classified as `StrictlyPure`
- [ ] `fn get_pi() -> f64 { std::f64::consts::PI }` is classified as `StrictlyPure`
- [ ] All existing tests pass
- [ ] New tests cover constant recognition:
  - `std::i32::MAX`, `core::u64::MIN`, `std::f64::INFINITY`
  - SCREAMING_CASE constants: `MY_CONST`, `CONFIG::MAX_SIZE`
  - Enum variants: `Option::None`, `Result::Ok`
- [ ] Functions accessing `static` items remain classified as `ReadOnly`
- [ ] Confidence score appropriately adjusted for unknown paths

## Technical Details

### Files to Modify

| File | Changes |
|------|---------|
| `src/analyzers/purity_detector.rs` | Add constant detection logic in `visit_expr` |
| `src/analyzers/purity_detector.rs` | Add `is_known_constant()` and `classify_path_purity()` functions |

### Test Cases

```rust
#[test]
fn test_std_max_constant_is_pure() {
    let analysis = analyze_function_str(r#"
        fn is_valid(x: i32) -> bool {
            x < std::i32::MAX
        }
    "#);
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_core_constant_is_pure() {
    let analysis = analyze_function_str(r#"
        fn min_val() -> u64 {
            core::u64::MIN
        }
    "#);
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_float_constants_are_pure() {
    let analysis = analyze_function_str(r#"
        fn is_infinite(x: f64) -> bool {
            x == std::f64::INFINITY || x == std::f64::NEG_INFINITY
        }
    "#);
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_enum_variant_is_pure() {
    let analysis = analyze_function_str(r#"
        fn default_option() -> Option<i32> {
            Option::None
        }
    "#);
    assert_eq!(analysis.purity_level, PurityLevel::StrictlyPure);
}

#[test]
fn test_unknown_path_is_conservative() {
    let analysis = analyze_function_str(r#"
        fn get_config() -> Config {
            external_crate::get_global_config()
        }
    "#);
    // Should remain conservative for truly unknown paths
    assert!(analysis.purity_level != PurityLevel::StrictlyPure);
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: `src/analyzers/purity_detector.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test each constant pattern recognition
- **Integration Tests**: Analyze real codebases with heavy constant usage
- **Regression Tests**: Ensure no existing pure/impure classifications change incorrectly

## Documentation Requirements

- Update inline documentation in `purity_detector.rs`
- No user-facing documentation changes needed

## Implementation Notes

- Start with Approach 1 (whitelist) for quick win
- Consider Approach 2 (heuristics) for follow-up if needed
- SCREAMING_CASE heuristic may have false positives (mutable statics)
- Enum variant detection is safe (variants are always constants)

## Migration and Compatibility

- No breaking changes
- Functions previously marked as `ReadOnly` may become `StrictlyPure`
- This is a correction, not a change in semantics
