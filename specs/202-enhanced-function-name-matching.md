---
number: 202
title: Enhanced Function Name Matching for Coverage
category: foundation
priority: critical
status: draft
dependencies: [201]
created: 2025-12-06
---

# Specification 202: Enhanced Function Name Matching for Coverage

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 201 (Robust Path Normalization)

## Context

After path matching succeeds, debtmap must match function names between AST analysis and LCOV coverage data. Current implementation handles basic cases (exact names, simple variants, generics) but fails on:

- **Async functions**: May appear as `foo::{{closure}}` in LCOV
- **Trait methods**: LCOV stores `visit_expr`, debtmap has `RecursiveDetector::visit_expr`
- **Macro-generated code**: Complex mangled names
- **Lifetime parameters**: `parse<'a, 'b>` vs `parse`
- **Closures**: Parent function attribution

These failures cause `Cov:N/A` for functions that actually have coverage data.

**Current Implementation** (src/risk/coverage_index.rs:13-49):
- Generates method name variant (strips type prefix)
- Limited to single `::` split
- No async/closure handling
- No macro demangling

## Objective

Implement comprehensive function name matching that handles all Rust function name variations, ensuring coverage data is matched correctly for:
- Async functions and their generated closures
- Trait implementations with varying name formats
- Generic functions with type parameters
- Functions with lifetime annotations
- Macro-generated and compiler-generated names

## Requirements

### Functional Requirements

**FR1**: Generate comprehensive name variants for matching
- Strip type qualifiers (`Type::method` → `method`)
- Strip generic parameters (`func<T, U>` → `func`)
- Strip lifetime parameters (`parse<'a>` → `parse`)
- Handle nested paths (`crate::module::Type::method` → `method`)

**FR2**: Match async function closures to parent functions
- Detect `{{closure}}` suffix in LCOV names
- Extract parent function name from closure
- Attribute closure coverage to parent function
- Handle numbered closures (`{{closure}}#0`, `{{closure}}#1`)

**FR3**: Support fuzzy matching for similar names
- Substring matching (one name contains the other)
- Suffix matching for qualified names
- Prefix matching for macro-generated names
- Configurable matching strictness

**FR4**: Handle compiler-generated name mangling
- Demangle Rust symbols when possible
- Strip `_impl_Trait_for_Type` prefixes
- Handle angle bracket normalization
- Extract semantic name from mangled form

### Non-Functional Requirements

**NFR1**: **Performance** - O(n) complexity per variant generation
- Single-pass string processing
- Minimal allocations (reuse buffers)
- No regex for basic operations

**NFR2**: **Accuracy** - Minimize false positives
- Exact match preferred over fuzzy match
- Document matching confidence levels
- Provide diagnostic information

**NFR3**: **Maintainability** - Clear, testable code
- Pure functions for all matching logic
- Comprehensive test coverage
- Well-documented edge cases

## Acceptance Criteria

- [ ] `generate_function_name_variants()` produces all relevant variants
  - `Type::method` → `["Type::method", "method"]`
  - `func<T>` → `["func<T>", "func"]`
  - `parse<'a, 'b>` → `["parse<'a, 'b>", "parse"]`
  - Nested: `crate::mod::Type::method` → includes `"method"`

- [ ] Async function closure matching works
  - `async_fn::{{closure}}` matches `async_fn`
  - `process::{{closure}}#0` matches `process`
  - Coverage attributed to parent function

- [ ] Trait method matching works across formats
  - LCOV: `visit_expr` matches query: `RecursiveDetector::visit_expr`
  - LCOV: `method` matches query: `impl Trait for Type::method`
  - Bidirectional matching (either can be shorter form)

- [ ] Generic function matching works
  - `process<WorkflowExecutor>` matches `process`
  - `execute<T, U, V>` matches `execute`
  - Multiple monomorphizations aggregated correctly

- [ ] Edge cases handled gracefully
  - Empty function names return empty variant list
  - Unicode function names preserved correctly
  - Very long names (>256 chars) handled
  - Special characters don't cause panics

- [ ] Matching confidence reported
  - Exact match = High confidence
  - Variant match = Medium confidence
  - Fuzzy match = Low confidence
  - No match = None with diagnostic

- [ ] Integration with existing coverage index
  - All matching strategies use new variant generation
  - No regression in existing tests
  - New tests for edge cases added

## Technical Details

### Implementation Approach

**Pure Function Architecture**:

```rust
// ============================================================================
// PURE CORE: Function name matching logic (100% testable, no I/O)
// ============================================================================

/// Pure function: Generate all name variants for matching
///
/// Produces variants by stripping qualifiers, generics, and lifetimes.
/// Returns variants in order of specificity (exact → most general).
///
/// # Examples
/// ```
/// let variants = generate_function_name_variants("Type::method<T>");
/// assert_eq!(variants, vec![
///     "Type::method<T>",  // Original
///     "Type::method",     // Without generics
///     "method",           // Method name only
/// ]);
/// ```
pub fn generate_function_name_variants(name: &str) -> Vec<String> {
    let mut variants = Vec::with_capacity(4);

    // Always include original
    variants.push(name.to_string());

    // Strip generics: func<T> → func
    if let Some(without_generics) = name.split('<').next() {
        if without_generics != name && !without_generics.is_empty() {
            variants.push(without_generics.to_string());
        }
    }

    // Extract method name: Type::method → method
    if let Some(method_name) = name.rsplit("::").next() {
        if method_name != name && !method_name.is_empty() {
            variants.push(method_name.to_string());

            // Also strip generics from method name
            if let Some(method_no_generics) = method_name.split('<').next() {
                if method_no_generics != method_name && !method_no_generics.is_empty() {
                    variants.push(method_no_generics.to_string());
                }
            }
        }
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    variants.retain(|v| seen.insert(v.clone()));

    variants
}

/// Pure function: Extract parent function from closure name
///
/// Detects {{closure}} pattern and extracts parent function name.
///
/// # Examples
/// ```
/// assert_eq!(
///     extract_closure_parent("async_fn::{{closure}}"),
///     Some("async_fn".to_string())
/// );
/// assert_eq!(
///     extract_closure_parent("process::{{closure}}#0"),
///     Some("process".to_string())
/// );
/// ```
pub fn extract_closure_parent(name: &str) -> Option<String> {
    if !name.contains("{{closure}}") {
        return None;
    }

    name.split("::{{closure}}")
        .next()
        .map(|s| s.to_string())
}

/// Pure function: Calculate match confidence
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchConfidence {
    None = 0,
    Low = 1,      // Fuzzy/substring match
    Medium = 2,   // Variant match
    High = 3,     // Exact match
}

/// Pure function: Check if function names match with confidence level
pub fn function_names_match(query: &str, lcov: &str) -> (bool, MatchConfidence) {
    // Exact match - highest confidence
    if query == lcov {
        return (true, MatchConfidence::High);
    }

    // Check closure parent attribution
    if let Some(parent) = extract_closure_parent(lcov) {
        if query == parent {
            return (true, MatchConfidence::High);
        }
    }

    // Generate variants for both query and LCOV
    let query_variants = generate_function_name_variants(query);
    let lcov_variants = generate_function_name_variants(lcov);

    // Variant match - medium confidence
    for qv in &query_variants {
        for lv in &lcov_variants {
            if qv == lv {
                return (true, MatchConfidence::Medium);
            }
        }
    }

    // Fuzzy match - low confidence
    // Check if one name contains the other
    if query.contains(lcov) || lcov.contains(query) {
        return (true, MatchConfidence::Low);
    }

    (false, MatchConfidence::None)
}

/// Pure function: Find best matching function from candidates
pub fn find_matching_function<'a>(
    query_name: &str,
    candidates: &'a [FunctionCoverage],
) -> Option<(&'a FunctionCoverage, MatchConfidence)> {
    let mut best_match = None;
    let mut best_confidence = MatchConfidence::None;

    for candidate in candidates {
        let (matches, confidence) = function_names_match(query_name, &candidate.name);

        if matches && confidence > best_confidence {
            best_match = Some(candidate);
            best_confidence = confidence;

            // Early exit on exact match
            if confidence == MatchConfidence::High {
                break;
            }
        }
    }

    best_match.map(|func| (func, best_confidence))
}
```

### Architecture Changes

**New Module**: `src/risk/function_name_matching.rs`
- Pure functions for name variant generation
- Closure parent extraction
- Match confidence calculation
- Fuzzy matching logic

**Updated Module**: `src/risk/coverage_index.rs`
- Use new matching functions in lookup methods
- Report match confidence in results
- Add diagnostic information

### Data Structures

```rust
/// Function name with all variants for matching
#[derive(Debug, Clone)]
pub struct FunctionName {
    /// Original name as provided
    pub original: String,
    /// All generated variants for matching
    pub variants: Vec<String>,
    /// Closure parent if this is a closure
    pub closure_parent: Option<String>,
}

impl FunctionName {
    pub fn new(name: &str) -> Self {
        Self {
            original: name.to_string(),
            variants: generate_function_name_variants(name),
            closure_parent: extract_closure_parent(name),
        }
    }
}

/// Match result with diagnostic information
#[derive(Debug, Clone)]
pub struct FunctionMatchResult {
    /// Matched function coverage data
    pub coverage: Option<f64>,
    /// Match confidence level
    pub confidence: MatchConfidence,
    /// Diagnostic message explaining match/failure
    pub diagnostic: String,
    /// Name variant that matched (if any)
    pub matched_variant: Option<(String, String)>, // (query_variant, lcov_variant)
}
```

## Dependencies

**Prerequisites**:
- Spec 201: Robust Path Normalization (path matching must succeed first)

**Affected Components**:
- `src/risk/coverage_index.rs` - Uses new function matching
- `src/priority/coverage_propagation.rs` - May benefit from match confidence
- `src/priority/unified_scorer.rs` - Can report matching quality

**External Dependencies**: None - uses only `std`

## Testing Strategy

### Unit Tests

**Test Variant Generation**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_variants_simple() {
        let variants = generate_function_name_variants("simple_func");
        assert_eq!(variants, vec!["simple_func"]);
    }

    #[test]
    fn test_generate_variants_type_method() {
        let variants = generate_function_name_variants("Type::method");
        assert!(variants.contains(&"Type::method".to_string()));
        assert!(variants.contains(&"method".to_string()));
    }

    #[test]
    fn test_generate_variants_with_generics() {
        let variants = generate_function_name_variants("process<T, U>");
        assert!(variants.contains(&"process<T, U>".to_string()));
        assert!(variants.contains(&"process".to_string()));
    }

    #[test]
    fn test_generate_variants_nested_path() {
        let variants = generate_function_name_variants("crate::module::Type::method<T>");
        assert!(variants.contains(&"method".to_string()));
        assert!(variants.len() >= 3);
    }

    #[test]
    fn test_extract_closure_parent() {
        assert_eq!(
            extract_closure_parent("async_fn::{{closure}}"),
            Some("async_fn".to_string())
        );
        assert_eq!(
            extract_closure_parent("process::{{closure}}#0"),
            Some("process".to_string())
        );
        assert_eq!(extract_closure_parent("regular_function"), None);
    }

    #[test]
    fn test_function_names_match_exact() {
        let (matches, confidence) = function_names_match("foo", "foo");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::High);
    }

    #[test]
    fn test_function_names_match_variant() {
        let (matches, confidence) = function_names_match("Type::method", "method");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::Medium);
    }

    #[test]
    fn test_function_names_match_closure() {
        let (matches, confidence) = function_names_match("async_fn", "async_fn::{{closure}}");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::High);
    }
}
```

**Property-Based Tests**:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn variant_generation_never_panics(name in ".*") {
        let _ = generate_function_name_variants(&name);
    }

    #[test]
    fn original_always_in_variants(name in "[a-zA-Z_][a-zA-Z0-9_]*") {
        let variants = generate_function_name_variants(&name);
        prop_assert!(variants.contains(&name.to_string()));
    }

    #[test]
    fn matching_is_reflexive(name in "[a-zA-Z_][a-zA-Z0-9_::<>]*") {
        let (matches, _) = function_names_match(&name, &name);
        prop_assert!(matches);
    }
}
```

### Integration Tests

**Real-World Scenarios** (`tests/function_name_matching_integration_test.rs`):

```rust
#[test]
fn test_async_function_coverage_attribution() {
    let mut functions = vec![
        create_function_coverage("process_data::{{closure}}", 100, 85.0),
    ];

    let result = find_matching_function("process_data", &functions);
    assert!(result.is_some());
    let (matched, confidence) = result.unwrap();
    assert_eq!(matched.coverage_percentage, 85.0);
    assert_eq!(confidence, MatchConfidence::High);
}

#[test]
fn test_trait_method_variant_matching() {
    let functions = vec![
        create_function_coverage("visit_expr", 3507, 90.2),
    ];

    let result = find_matching_function("RecursiveDetector::visit_expr", &functions);
    assert!(result.is_some());
    let (matched, confidence) = result.unwrap();
    assert_eq!(confidence, MatchConfidence::Medium);
}

#[test]
fn test_generic_monomorphization_matching() {
    let functions = vec![
        create_function_coverage("execute<WorkflowExecutor>", 100, 75.0),
        create_function_coverage("execute<TestExecutor>", 50, 80.0),
    ];

    // Should match both monomorphizations
    let result1 = find_matching_function("execute", &functions);
    assert!(result1.is_some());
}
```

### Edge Case Tests

```rust
#[test]
fn test_empty_function_name() {
    let variants = generate_function_name_variants("");
    assert_eq!(variants, vec![""]);
}

#[test]
fn test_unicode_function_name() {
    let variants = generate_function_name_variants("测试函数");
    assert!(variants.contains(&"测试函数".to_string()));
}

#[test]
fn test_very_long_function_name() {
    let long_name = "a".repeat(1000);
    let variants = generate_function_name_variants(&long_name);
    assert!(variants.len() > 0);
}

#[test]
fn test_special_characters() {
    let variants = generate_function_name_variants("func_with_$special");
    assert!(variants.len() > 0);
}
```

## Documentation Requirements

### Code Documentation

- Rustdoc for all public functions with examples
- Document matching strategy and confidence levels
- Explain closure attribution logic
- Provide migration guide from old matching

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## Coverage Function Name Matching

### Name Variant Generation

Pure functions generate all possible name variants:
- Strip type qualifiers
- Strip generic parameters
- Extract closure parents
- Generate fuzzy match candidates

### Match Confidence

- **High**: Exact match or closure parent
- **Medium**: Variant match (method name, etc.)
- **Low**: Fuzzy/substring match

This ensures best available match is always selected.
```

## Implementation Notes

### Best Practices

1. **Pure Functions**: All matching logic is pure and testable
2. **Composability**: Build complex matching from simple predicates
3. **Performance**: Minimize allocations, reuse buffers
4. **Diagnostics**: Report why matches succeeded or failed

### Gotchas

- Closure numbering (`{{closure}}#0`) varies by compilation
- Macro-generated names can be arbitrarily complex
- Unicode in identifiers must be preserved
- Generic parameter order may differ

### Future Enhancements

- Caching of variant generation results
- Configurable matching strictness levels
- Symbol demangling for better macro support
- Machine learning for fuzzy matching confidence

## Migration and Compatibility

### Breaking Changes

None - this enhances existing matching without breaking changes.

### Backward Compatibility

Existing matching continues to work:
- Exact matches still preferred
- Variant matching is additive
- New edge cases now handled

### Migration Path

1. Implement new pure functions
2. Add comprehensive tests
3. Update `coverage_index.rs` incrementally
4. Monitor match success rates
5. Tune matching strategies based on real data
