---
number: 266
title: Option/Collection Unwrap Elimination
category: safety
priority: medium
status: draft
dependencies: [263]
created: 2025-12-13
---

# Specification 266: Option/Collection Unwrap Elimination

**Category**: safety
**Priority**: medium
**Status**: draft
**Dependencies**: 263 (Critical Unwrap Elimination - Lock Safety)

## Context

After eliminating critical lock unwraps (spec 263), debtmap still contains ~90+ unwraps on Options and collections that can panic under edge conditions:

1. **Option field access** (~30 instances) - Accessing `.unwrap()` on optional fields
2. **Collection operations** (~60 instances) - Iterator `.next().unwrap()`, slice indexing
3. **HashMap/BTreeMap access** - `.get().unwrap()` on maps

**Current Problems:**

```rust
// Option unwrap - panics if field is None
let callees = metric.downstream_callees.as_ref().unwrap();

// Iterator unwrap - panics on empty input
let first_char = identifier.chars().next().unwrap();

// HashMap unwrap - panics if key missing
let cluster = method_to_cluster.get(method).unwrap();

// Slice indexing - panics if out of bounds
let first = items[0];
```

**Stillwater Philosophy:**

> "Errors Should Tell Stories" - When something fails, explain what was expected and what was actually found.

## Objective

Replace all Option/collection unwraps with proper error handling that:

1. Provides contextual error messages
2. Fails gracefully with meaningful diagnostics
3. Uses appropriate patterns (`.ok_or_else()`, `.get()`, etc.)

Result: No silent panics on edge cases; all failures are reported with context.

## Requirements

### Functional Requirements

1. **Option Field Access Elimination**
   - All `.unwrap()` on `Option<T>` replaced with `.ok_or_else()`
   - Error messages include field name and context
   - Use `?` for propagation in Result-returning functions

2. **Iterator Unwrap Elimination**
   - `.next().unwrap()` → `.next().ok_or_else()`
   - `.last().unwrap()` → `.last().ok_or_else()`
   - Peek operations handled safely

3. **Collection Access Elimination**
   - `map.get(key).unwrap()` → `map.get(key).ok_or_else()`
   - `slice[index]` → `slice.get(index).ok_or_else()` where fallible
   - Error includes which key/index was expected

4. **Contextual Error Messages**
   - Include what operation was attempted
   - Include what data was expected
   - Include relevant identifiers (file path, function name, etc.)

### Non-Functional Requirements

1. **No New Panics**
   - All error paths produce `Result::Err`
   - Edge cases handled gracefully

2. **Performance**
   - Negligible overhead from error handling
   - Hot paths remain efficient

3. **Debuggability**
   - Error messages are actionable
   - Context enables reproduction

## Acceptance Criteria

- [ ] Zero `.unwrap()` on `Option<T>` in production code
- [ ] Zero `.unwrap()` on iterator operations in production code
- [ ] Zero unchecked collection indexing in fallible contexts
- [ ] All error messages include context (field name, key, etc.)
- [ ] Test coverage for error paths
- [ ] All existing tests pass
- [ ] No clippy warnings

## Technical Details

### Implementation Patterns

**Pattern 1: Option Field Access**

```rust
// Before
fn get_callees(metric: &FileMetric) -> Vec<String> {
    metric.downstream_callees.as_ref().unwrap().clone()
}

// After
fn get_callees(metric: &FileMetric) -> Result<Vec<String>, AnalysisError> {
    metric.downstream_callees
        .as_ref()
        .ok_or_else(|| AnalysisError::validation(format!(
            "Missing downstream_callees for file: {}",
            metric.file_path.display()
        )))
        .map(|c| c.clone())
}

// Alternative: With default for non-critical cases
fn get_callees_or_empty(metric: &FileMetric) -> Vec<String> {
    metric.downstream_callees
        .as_ref()
        .cloned()
        .unwrap_or_default()
}
```

**Pattern 2: Iterator First/Last Element**

```rust
// Before
fn first_char(s: &str) -> char {
    s.chars().next().unwrap()
}

// After
fn first_char(s: &str) -> Result<char, AnalysisError> {
    s.chars()
        .next()
        .ok_or_else(|| AnalysisError::validation(format!(
            "Expected non-empty string, got empty"
        )))
}

// Alternative: Return Option when appropriate
fn first_char_opt(s: &str) -> Option<char> {
    s.chars().next()
}
```

**Pattern 3: HashMap Access**

```rust
// Before
fn get_cluster(method: &str, map: &HashMap<String, ClusterId>) -> ClusterId {
    *map.get(method).unwrap()
}

// After
fn get_cluster(
    method: &str,
    map: &HashMap<String, ClusterId>,
) -> Result<ClusterId, AnalysisError> {
    map.get(method)
        .copied()
        .ok_or_else(|| AnalysisError::other(format!(
            "Method '{}' not found in cluster map (available: {:?})",
            method,
            map.keys().take(5).collect::<Vec<_>>()
        )))
}
```

**Pattern 4: Slice Indexing**

```rust
// Before
fn first_item(items: &[Item]) -> &Item {
    &items[0]
}

// After
fn first_item(items: &[Item]) -> Result<&Item, AnalysisError> {
    items.first()
        .ok_or_else(|| AnalysisError::validation(
            "Expected at least one item, got empty slice"
        ))
}

// Alternative: Use get with index for specific positions
fn item_at(items: &[Item], index: usize) -> Result<&Item, AnalysisError> {
    items.get(index)
        .ok_or_else(|| AnalysisError::validation(format!(
            "Index {} out of bounds for slice of length {}",
            index, items.len()
        )))
}
```

**Pattern 5: Chained Operations**

```rust
// Before
fn extract_module_name(path: &Path) -> String {
    path.file_stem().unwrap().to_str().unwrap().to_string()
}

// After
fn extract_module_name(path: &Path) -> Result<String, AnalysisError> {
    let stem = path.file_stem()
        .ok_or_else(|| AnalysisError::validation(format!(
            "Path has no file stem: {}",
            path.display()
        )))?;

    stem.to_str()
        .ok_or_else(|| AnalysisError::validation(format!(
            "File stem is not valid UTF-8: {:?}",
            stem
        )))
        .map(|s| s.to_string())
}
```

### Files to Modify (Priority Order)

**High Priority - Core Analysis Paths:**

1. **`src/analyzers/call_graph_integration.rs`**
   - Line 327: `split.next().unwrap()`
   - Line 357: `chars.next().unwrap()`
   - Line 474: `scope_parts.first().unwrap()`

2. **`src/organization/behavioral_decomposition/clustering.rs`**
   - Multiple `method_to_cluster.get().unwrap()` calls
   - Cluster iteration assumptions

3. **`src/organization/data_flow_analyzer.rs`**
   - Line 89: `func.body.as_ref().unwrap()`
   - AST traversal assumptions

4. **`src/analyzers/type_tracker.rs`**
   - Line 118: `chars.next().unwrap()`
   - Type name parsing

**Medium Priority - Supporting Modules:**

5. **`src/analyzers/cyclomatic.rs`**
   - Iterator operations on AST nodes

6. **`src/analyzers/cognitive.rs`**
   - Nesting level calculations

7. **`src/priority/scoring/mod.rs`**
   - Score calculations with optional data

8. **`src/debt/patterns/*.rs`**
   - Pattern matching on optional AST nodes

**Lower Priority - I/O and Formatting:**

9. **`src/io/formatters/*.rs`**
   - String formatting operations

10. **`src/tui/results/*.rs`**
    - Display formatting

### Error Type Guidelines

Use the appropriate error variant:

```rust
// For data validation failures (expected data missing)
AnalysisError::validation("message")

// For internal logic errors (shouldn't happen)
AnalysisError::other("message")

// For parsing failures
AnalysisError::parse_error(source, "message")
```

### Migration Strategy

1. **Audit phase** - Run `grep -n "\.unwrap()" src/` to identify all instances
2. **Categorize** - Sort by pattern (Option, Iterator, HashMap, Slice)
3. **Fix by file** - Address one file at a time, run tests after each
4. **Add tests** - Add edge case tests for newly added error paths
5. **Final audit** - Ensure zero production unwraps remain

### Helper Functions

Consider adding to `src/utils/`:

```rust
// src/utils/option_ext.rs

pub trait OptionExt<T> {
    fn ok_or_validation(self, msg: impl Into<String>) -> Result<T, AnalysisError>;
    fn ok_or_context(self, context: impl FnOnce() -> String) -> Result<T, AnalysisError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_validation(self, msg: impl Into<String>) -> Result<T, AnalysisError> {
        self.ok_or_else(|| AnalysisError::validation(msg.into()))
    }

    fn ok_or_context(self, context: impl FnOnce() -> String) -> Result<T, AnalysisError> {
        self.ok_or_else(|| AnalysisError::other(context()))
    }
}

// Usage
let first = items.first().ok_or_validation("Expected non-empty list")?;
```

## Dependencies

- **Prerequisites**: 263 (Lock unwraps should be fixed first)
- **Affected Components**: Nearly all analysis modules
- **External Dependencies**: None

## Testing Strategy

### Unit Tests for Error Paths

```rust
#[test]
fn test_get_callees_missing() {
    let metric = FileMetric {
        downstream_callees: None,
        ..Default::default()
    };
    let result = get_callees(&metric);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing downstream_callees"));
}

#[test]
fn test_first_char_empty_string() {
    let result = first_char("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[test]
fn test_get_cluster_missing_method() {
    let map = HashMap::new();
    let result = get_cluster("unknown", &map);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown"));
}
```

### Property Tests

```rust
proptest! {
    #[test]
    fn first_char_works_on_nonempty(s in ".+") {
        let result = first_char(&s);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn get_item_within_bounds(
        items in prop::collection::vec(any::<i32>(), 1..100),
        index in 0usize..100
    ) {
        let result = item_at(&items, index);
        if index < items.len() {
            prop_assert!(result.is_ok());
        } else {
            prop_assert!(result.is_err());
        }
    }
}
```

### Integration Tests

Ensure analysis handles edge cases gracefully:

```rust
#[test]
fn test_analyze_file_with_no_functions() {
    let result = analyze_file("// empty file");
    assert!(result.is_ok()); // Should not panic
}

#[test]
fn test_analyze_malformed_ast() {
    let result = analyze_with_partial_ast(incomplete_ast);
    // Should produce error, not panic
    assert!(result.is_err());
}
```

## Documentation Requirements

### Code Documentation

Each replacement should include a brief comment explaining:
- What error condition it handles
- What context is provided in the error

### Error Catalog

Create `docs/errors.md` documenting common errors and their meanings.

## Implementation Notes

### When to Use Default vs Error

- **Use `.unwrap_or_default()`** when missing data is acceptable:
  - Optional metrics that can be zero
  - Display strings that can be empty

- **Use `.ok_or_else()?`** when missing data indicates a bug:
  - Required fields that should always be populated
  - Invariants that should be maintained

### Preserving Performance

For hot paths, avoid expensive error formatting until needed:

```rust
// Good - lazy formatting
.ok_or_else(|| AnalysisError::validation(format!("expensive: {:?}", data)))

// Bad - eager formatting (even on success)
let msg = format!("expensive: {:?}", data);
.ok_or(AnalysisError::validation(msg))
```

## Migration and Compatibility

### Breaking Changes

- Functions that previously returned `T` may now return `Result<T, AnalysisError>`
- Callers must handle the `Result`

### Backward Compatibility

For public API functions, consider providing both:

```rust
// New fallible version
pub fn analyze_file(path: &Path) -> Result<FileMetrics, AnalysisError>;

// Convenience wrapper that panics (with deprecation warning)
#[deprecated(note = "Use analyze_file() which returns Result")]
pub fn analyze_file_or_panic(path: &Path) -> FileMetrics {
    analyze_file(path).expect("analysis failed")
}
```

## Success Metrics

- Zero `.unwrap()` calls on `Option<T>` in production code
- Zero `.unwrap()` calls on iterators in production code
- Zero unchecked slice indexing in fallible code paths
- All error messages include contextual information
- Test coverage includes edge cases (empty, missing, out-of-bounds)
