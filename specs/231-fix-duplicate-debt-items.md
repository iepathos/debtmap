---
number: 231
title: Fix Duplicate Debt Items in Output
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 231: Fix Duplicate Debt Items in Output

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The debtmap JSON output (`debtmap.json`) contains duplicate debt items. Analysis found:

```json
// Line 8641-8643
{
  "file": "./src/tui/results/detail_pages/data_flow.rs",
  "line": 34,
  "function": "get_fix_suggestion"
}

// Line 8750-8752 (DUPLICATE)
{
  "file": "./src/tui/results/detail_pages/data_flow.rs",
  "line": 34,
  "function": "get_fix_suggestion"
}
```

Both entries have identical:
- File path
- Line number
- Function name
- All metrics (cyclomatic: 11, cognitive: 23, length: 23)
- All scores and recommendations

### Impact

- **Inflated metrics**: Total debt score is artificially high
- **Confusing output**: Users see same item multiple times
- **Wasted effort**: Recommendations are duplicated
- **Trust erosion**: Users question accuracy of other data

### Root Cause Hypothesis

Likely occurring in one of:
1. **Parallel analysis merging** - Race condition when combining parallel analysis results
2. **Debt item creation** - Same function analyzed multiple times
3. **Output conversion** - `convert_to_unified_format` creating duplicates

## Objective

Eliminate duplicate debt items from debtmap output by implementing deduplication logic and fixing the root cause of duplicate creation.

## Requirements

### Functional Requirements

1. **Detect Root Cause**: Identify where duplicates originate:
   - Add instrumentation to track item creation
   - Log when items with same (file, line, function) are created

2. **Implement Deduplication**: Add deduplication step in output pipeline:
   - Use (file, line, function) as unique key
   - Keep higher-scoring item if duplicates exist
   - Log when duplicates are removed

3. **Fix Root Cause**: Once identified, fix the source:
   - Prevent duplicate creation rather than filtering after

4. **Add Uniqueness Validation**: Integrate with spec 230's schema validation:
   - Validate no duplicate (file, line, function) tuples
   - Fail `--strict-output` if duplicates detected

### Non-Functional Requirements

- Zero duplicates in output
- Deduplication adds < 10ms overhead
- Clear logging when duplicates detected/removed

## Acceptance Criteria

- [ ] Root cause of duplicate creation identified and documented
- [ ] Deduplication logic added to output pipeline
- [ ] No duplicates in debtmap.json output
- [ ] Test verifying deduplication works
- [ ] Logging when duplicates are detected
- [ ] Integration with schema validation (spec 230)

## Technical Details

### Investigation Approach

1. **Add tracing**:
```rust
// In debt item creation
tracing::debug!(
    file = %location.file.display(),
    line = location.line,
    function = %location.function,
    "Creating debt item"
);
```

2. **Search for duplicate creation points**:
```bash
# Find where UnifiedDebtItem is created
rg "UnifiedDebtItem\s*\{" src/
rg "DebtItem::Function" src/
```

### Likely Root Cause: Parallel Analysis Merging

In `src/builders/parallel_unified_analysis.rs`, parallel tasks may create duplicate items:

```rust
// Hypothesis: Multiple tasks analyze same function
fn analyze_file_parallel(&self, path: &Path) -> Vec<DebtItem> {
    // If file is analyzed by multiple tasks, duplicates occur
}
```

### Deduplication Implementation

```rust
// src/output/unified.rs

use std::collections::HashSet;

/// Deduplicate debt items by (file, line, function) key
fn deduplicate_items(items: Vec<UnifiedDebtItemOutput>) -> Vec<UnifiedDebtItemOutput> {
    let mut seen: HashSet<(String, Option<usize>, Option<String>)> = HashSet::new();
    let mut result = Vec::with_capacity(items.len());
    let mut duplicate_count = 0;

    for item in items {
        let key = match &item {
            UnifiedDebtItemOutput::File(f) => {
                (f.location.file.clone(), None, None)
            }
            UnifiedDebtItemOutput::Function(f) => {
                (f.location.file.clone(), f.location.line, f.location.function.clone())
            }
        };

        if seen.insert(key.clone()) {
            result.push(item);
        } else {
            duplicate_count += 1;
            tracing::warn!(
                file = %key.0,
                line = ?key.1,
                function = ?key.2,
                "Removed duplicate debt item"
            );
        }
    }

    if duplicate_count > 0 {
        tracing::warn!(
            count = duplicate_count,
            "Removed {} duplicate debt items from output",
            duplicate_count
        );
    }

    result
}

/// Convert analysis results to unified format (with deduplication)
pub fn convert_to_unified_format(
    analysis: &UnifiedAnalysis,
    include_scoring_details: bool,
) -> UnifiedOutput {
    // ... existing code ...

    // Deduplicate before returning
    let unified_items = deduplicate_items(unified_items);

    UnifiedOutput {
        // ...
        items: unified_items,
    }
}
```

### Key Generation for Deduplication

```rust
/// Unique key for debt item deduplication
#[derive(Hash, Eq, PartialEq)]
struct DebtItemKey {
    file: String,
    line: Option<usize>,
    function: Option<String>,
}

impl From<&UnifiedDebtItemOutput> for DebtItemKey {
    fn from(item: &UnifiedDebtItemOutput) -> Self {
        match item {
            UnifiedDebtItemOutput::File(f) => DebtItemKey {
                file: f.location.file.clone(),
                line: None,
                function: None,
            },
            UnifiedDebtItemOutput::Function(f) => DebtItemKey {
                file: f.location.file.clone(),
                line: f.location.line,
                function: f.location.function.clone(),
            },
        }
    }
}
```

### Schema Validation Integration (spec 230)

```rust
// In output/schema.rs

/// Validate no duplicate items in output
fn validate_no_duplicates(items: &Value, path: &JsonPath) -> ValidationResult<()> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut errors = Vec::new();

    if let Some(arr) = items.as_array() {
        for (i, item) in arr.iter().enumerate() {
            let key = format!(
                "{}:{}:{}",
                item["location"]["file"].as_str().unwrap_or(""),
                item["location"]["line"].as_u64().unwrap_or(0),
                item["location"]["function"].as_str().unwrap_or("")
            );

            if !seen.insert(key.clone()) {
                errors.push(SchemaError::custom(
                    path.index(i),
                    format!("Duplicate item: {}", key)
                ));
            }
        }
    }

    if errors.is_empty() {
        Validation::success(())
    } else {
        Validation::failure(SchemaErrors::from_vec(errors))
    }
}
```

### Architecture Changes

- Modified: `src/output/unified.rs` (add deduplication)
- Modified: `src/output/schema.rs` (add uniqueness validation)
- May modify: `src/builders/parallel_unified_analysis.rs` (fix root cause)

## Dependencies

- **Prerequisites**: None (can implement independently)
- **Related**: Spec 230 (Output Schema Validation) for uniqueness validation
- **Affected Components**:
  - `src/output/unified.rs`
  - `src/builders/parallel_unified_analysis.rs` (root cause fix)

## Testing Strategy

- **Unit Tests**:
  - Test deduplication removes duplicates
  - Test highest-scoring item kept
  - Test non-duplicates preserved

- **Integration Tests**:
  - Run debtmap on codebase, verify no duplicates
  - Verify total item count is accurate

- **Regression Tests**:
  - Add test case with known duplicate scenario

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication_removes_duplicates() {
        let items = vec![
            create_function_item("a.rs", 10, "foo", 50.0),
            create_function_item("a.rs", 10, "foo", 45.0),  // Duplicate
            create_function_item("b.rs", 20, "bar", 30.0),
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 2);
        // Should keep first occurrence
        assert_eq!(result[0].score(), 50.0);
    }

    #[test]
    fn test_deduplication_preserves_unique_items() {
        let items = vec![
            create_function_item("a.rs", 10, "foo", 50.0),
            create_function_item("a.rs", 20, "bar", 45.0),  // Different line
            create_function_item("b.rs", 10, "foo", 30.0),  // Different file
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_file_items_deduplicated_by_path() {
        let items = vec![
            create_file_item("a.rs", 50.0),
            create_file_item("a.rs", 45.0),  // Duplicate
            create_file_item("b.rs", 30.0),
        ];

        let result = deduplicate_items(items);

        assert_eq!(result.len(), 2);
    }
}
```

## Documentation Requirements

- **Code Documentation**: Document deduplication logic and key generation
- **User Documentation**: Note that duplicates are automatically removed
- **Architecture Updates**: Document deduplication step in output pipeline

## Implementation Notes

1. **Order Preservation**: Use `IndexSet` or preserve insertion order to maintain sorted output.

2. **Merge vs Drop**: Currently dropping duplicates. Could alternatively merge metrics/scores.

3. **Root Cause Priority**: Fix the root cause of duplicate creation, not just filter.

4. **Performance**: HashSet lookup is O(1), minimal overhead for deduplication.

## Migration and Compatibility

- **No breaking changes**: Output format unchanged
- **Improved accuracy**: Total item counts will be accurate
- **Backward compatible**: Tools consuming JSON output unaffected
