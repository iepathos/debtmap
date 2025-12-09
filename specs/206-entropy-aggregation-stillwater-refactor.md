---
number: 206
title: Refactor Entropy Aggregation to Stillwater Principles
category: optimization
priority: low
status: draft
dependencies: []
created: 2025-12-08
---

# Specification 206: Refactor Entropy Aggregation to Stillwater Principles

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: None

## Context

The `aggregate_entropy_from_raw()` function in `src/priority/god_object_aggregation.rs` (lines 300-363) was recently added to provide entropy analysis for god objects. While functional and tested, it doesn't fully align with the Stillwater philosophy from `../stillwater/PHILOSOPHY.md`.

**Current Issues:**

1. **Function too long** (~60 lines) - Should be decomposed into smaller functions (target: 5-20 lines)
2. **Mixed responsibilities** - Extraction, calculation, dampening, and result construction in one function
3. **Synthetic struct construction** - Creates fake `EntropyScore` with dummy values just to call `apply_dampening()`, violating type safety principles
4. **Repeated iteration** - Iterates over `entropy_data` multiple times for different calculations

**Current Implementation (simplified):**

```rust
pub fn aggregate_entropy_from_raw(functions: &[FunctionMetrics]) -> Option<EntropyDetails> {
    // 1. Extract entropy data (filter_map)
    let entropy_data: Vec<_> = functions.iter().filter_map(...).collect();

    if entropy_data.is_empty() { return None; }

    // 2. Calculate total length
    let total_length: usize = entropy_data.iter().map(...).sum();

    // 3. Calculate weighted entropy average
    let weighted_entropy = entropy_data.iter().map(...).sum::<f64>() / total_length as f64;

    // 4. Calculate weighted repetition average
    let weighted_repetition = entropy_data.iter().map(...).sum::<f64>() / total_length as f64;

    // 5. Create synthetic score (CODE SMELL!)
    let avg_score = EntropyScore {
        token_entropy: weighted_entropy,
        pattern_repetition: weighted_repetition,
        branch_similarity: 0.0,        // Dummy value
        effective_complexity: 0.0,     // Dummy value
        unique_variables: 0,           // Dummy value
        max_nesting: 0,                // Dummy value
        dampening_applied: 0.0,        // Dummy value
    };

    // 6. Calculate dampening
    let dampening_value = calculator.apply_dampening(&avg_score);

    // 7. Build result
    Some(EntropyDetails { ... })
}
```

According to Stillwater philosophy:
- **Composition over complexity** - Build from small, composable pieces
- **Types guide, don't restrict** - Don't create invalid type instances
- **Pure functions** - Extract calculation logic into focused functions

## Objective

Refactor `aggregate_entropy_from_raw()` to follow Stillwater principles:

1. **Decompose** into small, focused pure functions (5-10 lines each)
2. **Eliminate** synthetic struct construction by adding a focused dampening API
3. **Single iteration** where possible for efficiency
4. **Clear composition** of smaller functions in main aggregation

Result: Cleaner, more testable, Stillwater-aligned entropy aggregation code.

## Requirements

### Functional Requirements

1. **Function Decomposition**
   - Extract `extract_entropy_data()` - pulls entropy tuples from functions
   - Extract `weighted_average()` - generic weighted average calculation
   - Extract `calculate_dampening_from_values()` - dampening without full struct
   - Each function under 10 lines

2. **Type Safety Improvement**
   - Add `apply_dampening_from_values(entropy: f64, repetition: f64) -> f64` to `UniversalEntropyCalculator`
   - Eliminate need to construct synthetic `EntropyScore` with dummy values
   - OR use `Default` trait properly if struct supports it

3. **Preserve Functionality**
   - Same weighted average calculations
   - Same dampening behavior
   - Same output `EntropyDetails`
   - All existing tests pass

### Non-Functional Requirements

1. **Readability**
   - Self-documenting function names
   - Clear data flow through composition
   - Easy to understand each piece

2. **Testability**
   - Each extracted function independently testable
   - Pure functions with no side effects

3. **Performance**
   - No regression (same or better)
   - Consider single-pass iteration

## Acceptance Criteria

- [ ] `aggregate_entropy_from_raw()` under 15 lines (composition only)
- [ ] Each helper function under 10 lines
- [ ] No synthetic struct construction with dummy values
- [ ] All existing tests pass
- [ ] New unit tests for extracted functions
- [ ] No clippy warnings
- [ ] Code follows Stillwater "Composition over complexity" principle

## Technical Details

### Implementation Approach

**Phase 1: Extract Pure Functions**

```rust
/// Extracts entropy data tuples from function metrics.
/// Returns (entropy_score, length, cognitive_complexity) for each function with entropy.
fn extract_entropy_data(functions: &[FunctionMetrics]) -> Vec<(&EntropyScore, usize, u32)> {
    functions
        .iter()
        .filter_map(|f| f.entropy_score.as_ref().map(|e| (e, f.length, f.cognitive)))
        .collect()
}

/// Calculates weighted average of a metric across entropy data.
fn weighted_average<F>(data: &[(&EntropyScore, usize, u32)], total_length: usize, f: F) -> f64
where
    F: Fn(&EntropyScore) -> f64,
{
    data.iter()
        .map(|(e, len, _)| f(e) * (*len as f64))
        .sum::<f64>()
        / total_length as f64
}

/// Calculates total of a numeric field across entropy data.
fn sum_field<T, F>(data: &[(&EntropyScore, usize, u32)], f: F) -> T
where
    T: std::iter::Sum,
    F: Fn(&(&EntropyScore, usize, u32)) -> T,
{
    data.iter().map(f).sum()
}
```

**Phase 2: Add Focused Dampening API**

In `src/complexity/entropy_core.rs`:

```rust
impl UniversalEntropyCalculator {
    /// Calculates dampening factor from entropy and repetition values.
    ///
    /// This avoids constructing a full EntropyScore when only these values are known.
    pub fn calculate_dampening_factor(&self, token_entropy: f64, pattern_repetition: f64) -> f64 {
        // Reuse existing dampening logic
        let base_dampening = self.calculate_base_dampening(pattern_repetition);
        self.adjust_for_entropy(base_dampening, token_entropy)
    }
}
```

**Phase 3: Compose Main Function**

```rust
/// Aggregate entropy from raw FunctionMetrics.
///
/// Returns weighted average entropy based on function length from ALL functions.
pub fn aggregate_entropy_from_raw(functions: &[FunctionMetrics]) -> Option<EntropyDetails> {
    let data = extract_entropy_data(functions);
    let total_length: usize = sum_field(&data, |(_, len, _)| *len);

    if data.is_empty() || total_length == 0 {
        return None;
    }

    let entropy = weighted_average(&data, total_length, |e| e.token_entropy);
    let repetition = weighted_average(&data, total_length, |e| e.pattern_repetition);
    let total_cognitive: u32 = sum_field(&data, |(_, _, cog)| *cog);

    let dampening = calculate_dampening_factor(entropy, repetition);
    let adjusted = (total_cognitive as f64 * dampening) as u32;

    Some(EntropyDetails {
        entropy_score: entropy,
        pattern_repetition: repetition,
        original_complexity: total_cognitive,
        adjusted_complexity: adjusted,
        dampening_factor: dampening,
        adjusted_cognitive: adjusted,
    })
}
```

### Alternative: Single-Pass Iteration

For better performance, aggregate all values in one pass:

```rust
struct EntropyAggregates {
    weighted_entropy_sum: f64,
    weighted_repetition_sum: f64,
    total_length: usize,
    total_cognitive: u32,
}

fn aggregate_in_single_pass(functions: &[FunctionMetrics]) -> Option<EntropyAggregates> {
    let result = functions
        .iter()
        .filter_map(|f| f.entropy_score.as_ref().map(|e| (e, f.length, f.cognitive)))
        .fold(EntropyAggregates::default(), |mut acc, (e, len, cog)| {
            acc.weighted_entropy_sum += e.token_entropy * (len as f64);
            acc.weighted_repetition_sum += e.pattern_repetition * (len as f64);
            acc.total_length += len;
            acc.total_cognitive += cog;
            acc
        });

    (result.total_length > 0).then_some(result)
}
```

### Architecture Changes

- Add `calculate_dampening_factor()` method to `UniversalEntropyCalculator`
- Extract helper functions in `god_object_aggregation.rs`
- No public API changes

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/god_object_aggregation.rs` - Main refactoring target
  - `src/complexity/entropy_core.rs` - Add focused dampening API
- **External Dependencies**: None

## Testing Strategy

### Unit Tests (New)

```rust
#[test]
fn test_extract_entropy_data_filters_correctly() {
    let functions = vec![
        create_function_with_entropy(0.4, 100),
        create_function_without_entropy(),
        create_function_with_entropy(0.6, 50),
    ];

    let data = extract_entropy_data(&functions);
    assert_eq!(data.len(), 2); // Only functions with entropy
}

#[test]
fn test_weighted_average_calculation() {
    let data = vec![
        (&entropy_score(0.4), 100usize, 20u32),
        (&entropy_score(0.6), 50usize, 10u32),
    ];

    // (100*0.4 + 50*0.6) / 150 = 70/150 ≈ 0.467
    let avg = weighted_average(&data, 150, |e| e.token_entropy);
    assert!((avg - 0.467).abs() < 0.01);
}

#[test]
fn test_dampening_factor_direct() {
    let calculator = UniversalEntropyCalculator::new(EntropyConfig::default());
    let dampening = calculator.calculate_dampening_factor(0.4, 0.6);
    assert!(dampening >= 0.5 && dampening <= 1.0);
}
```

### Existing Tests (Must Pass)

```rust
// These existing tests must continue to pass
test_aggregate_entropy_from_raw
test_aggregate_from_raw_metrics_includes_entropy
```

## Documentation Requirements

### Code Documentation

```rust
/// Extracts entropy data tuples from function metrics.
///
/// Pure function - filters functions that have entropy scores and returns
/// tuples of (entropy_score, length, cognitive_complexity).
///
/// # Arguments
/// * `functions` - Slice of function metrics to extract from
///
/// # Returns
/// Vector of tuples for functions with entropy data
fn extract_entropy_data(functions: &[FunctionMetrics]) -> Vec<(&EntropyScore, usize, u32)>
```

## Implementation Notes

### Refactoring Workflow

1. Add `calculate_dampening_factor()` to entropy calculator
2. Extract `extract_entropy_data()` helper
3. Extract `weighted_average()` helper
4. Refactor main function to use helpers
5. Run existing tests
6. Add new unit tests for helpers
7. Remove synthetic struct construction

### Stillwater Alignment Checklist

- [ ] Each function does one thing (Single Responsibility)
- [ ] Functions are composable (Composition over Complexity)
- [ ] No fake struct instances (Types Guide)
- [ ] Pure functions with no side effects (Pure Core)
- [ ] Clear data transformation pipeline (Railway Pattern)

## Migration and Compatibility

### Breaking Changes

**None** - Internal refactoring only. Public API unchanged.

### Migration Steps

No migration needed. Internal improvement only.

## Success Metrics

- Function decomposition complete
- Main function under 15 lines
- All tests pass
- Code review confirms Stillwater alignment
- No clippy warnings

## References

- **Stillwater PHILOSOPHY.md** - Core principles (Pure Core, Composition)
- **Spec 187** - Similar refactoring pattern for analyzers
- **CLAUDE.md** - Function design guidelines (max 20 lines)
