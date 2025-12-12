---
number: 201
title: Use Dampened Complexity in Debt Classification
category: foundation
priority: critical
status: draft
dependencies: [68]
created: 2025-12-12
---

# Specification 201: Use Dampened Complexity in Debt Classification

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 68 (Entropy-Based Dampening)

## Context

Debtmap's entropy analysis correctly identifies dispatcher patterns and dampens cognitive complexity. However, **the classification logic uses RAW values instead of dampened values**, causing false positives.

### The Bug

Production path (`classify_debt_type_with_exclusions` → `classify_all_debt_types`):

```rust
// classification.rs:250-251
fn check_complexity_hotspot_predicate(func: &FunctionMetrics) -> Option<DebtType> {
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {  // ❌ RAW values
```

```rust
// classification.rs:700-701
fn is_complexity_hotspot_by_metrics(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 5 || cognitive > 8    // Uses RAW values
}
```

### Real Example: False Positive

Function: `render` in `src/tui/results/detail_view.rs`
- Raw cyclomatic: 8, Raw cognitive: 10
- **Dampened cognitive: 5** (entropy correctly identifies dispatcher pattern)

**What happens:**
1. Entropy analysis dampens cognitive from 10 → 5
2. Classification calls `is_complexity_hotspot_by_metrics(8, 10)` with **RAW** cognitive
3. Check: `8 > 5 || 10 > 8` → TRUE (false positive!)

**What should happen:**
1. Classification uses dampened cognitive: `8 > 5 || 5 > 8` → TRUE || FALSE
2. Still flagged on cyclomatic alone, but dampened cyclomatic would also help

### Legacy Code to Remove

There's a **duplicate single-debt classification path** that's no longer used in production:

- `determine_debt_type()` in `classification.rs` - only called by legacy APIs
- `check_complexity_hotspot()` in `classification.rs` (lines 70-99) - different thresholds (>10/>15)
- `check_complexity_hotspot()` in `validation.rs` - yet another duplicate
- `create_unified_debt_item()` / `create_unified_debt_item_with_data_flow()` - legacy APIs

Production uses `create_unified_debt_item_with_aggregator_and_data_flow` → `classify_debt_type_with_exclusions` exclusively.

## Objective

1. **Fix the production path** to use dampened complexity values
2. **Remove legacy duplicate code** - single source of truth for complexity classification
3. **Delete unused APIs** and update tests that rely on them

## Requirements

### Functional Requirements

**FR1: Single Complexity Hotspot Check Function**

Consolidate to one function that uses dampened values:

```rust
/// Check for complexity hotspot using entropy-dampened values (spec 201)
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    let effective_cyclomatic = get_effective_cyclomatic(func);
    let effective_cognitive = get_effective_cognitive(func);

    // Use consistent thresholds
    if effective_cyclomatic > 10 || effective_cognitive > 15 {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,  // Raw for display
            cognitive: func.cognitive,     // Raw for display
        })
    } else {
        None
    }
}
```

**FR2: Helper Functions for Dampened Values**

```rust
fn get_effective_cyclomatic(func: &FunctionMetrics) -> u32 {
    func.adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(func.cyclomatic)
}

fn get_effective_cognitive(func: &FunctionMetrics) -> u32 {
    if let Some(entropy) = &func.entropy_score {
        let factor = calculate_dampening_factor(entropy.token_entropy);
        (func.cognitive as f64 * factor).round() as u32
    } else {
        func.cognitive
    }
}

fn calculate_dampening_factor(token_entropy: f64) -> f64 {
    // From spec 68
    if token_entropy < 0.2 {
        (0.5_f64).max(1.0 - (0.5 * (0.2 - token_entropy) / 0.2))
    } else {
        1.0
    }
}
```

**FR3: Remove Legacy Code**

Delete from `classification.rs`:
- `determine_debt_type()` function
- Old `check_complexity_hotspot()` with different thresholds
- `is_complexity_hotspot_by_metrics()` (inline into single function)

Delete from `validation.rs`:
- `check_complexity_hotspot()` duplicate

Delete from `construction.rs`:
- `create_unified_debt_item()`
- `create_unified_debt_item_with_data_flow()`

Delete from `debt_item.rs`:
- Re-exports of removed functions

**FR4: Update Tests**

- Remove tests for deleted functions
- Add tests for dampened classification
- Ensure production path tests cover dampening behavior

## Acceptance Criteria

- [ ] Single `check_complexity_hotspot()` function exists (no duplicates)
- [ ] Classification uses `get_effective_cyclomatic()` and `get_effective_cognitive()`
- [ ] `render` function in `detail_view.rs` is NOT flagged as ComplexityHotspot
- [ ] Legacy `determine_debt_type()` removed
- [ ] Legacy `create_unified_debt_item()` APIs removed
- [ ] All tests pass after cleanup
- [ ] Dispatcher patterns with low entropy avoid false positives

## Technical Details

### Files to Modify

1. **`src/priority/scoring/classification.rs`**:
   - Add `get_effective_cyclomatic()`, `get_effective_cognitive()`, `calculate_dampening_factor()`
   - Update `check_complexity_hotspot_predicate()` to use helpers
   - Delete `determine_debt_type()`, old `check_complexity_hotspot()`, `is_complexity_hotspot_by_metrics()`
   - Remove tests for deleted functions

2. **`src/priority/scoring/validation.rs`**:
   - Delete `check_complexity_hotspot()` duplicate and its tests

3. **`src/priority/scoring/construction.rs`**:
   - Delete `create_unified_debt_item()`, `create_unified_debt_item_with_data_flow()`

4. **`src/priority/scoring/debt_item.rs`**:
   - Remove re-exports: `determine_debt_type`, `create_unified_debt_item`, `create_unified_debt_item_with_data_flow`

5. **`src/priority/scoring/mod.rs`**:
   - Remove `determine_debt_type` from public exports

6. **Tests**:
   - `src/priority/unified_scorer/tests.rs`: Update test using `create_unified_debt_item`
   - Remove/update any tests referencing deleted functions

### Data Structures

No changes. `DebtType::ComplexityHotspot` stores raw values for display:

```rust
ComplexityHotspot {
    cyclomatic: u32,  // Raw, display shows "10 → 5 (dampened)"
    cognitive: u32,   // Raw
}
```

## Testing Strategy

### New Tests

```rust
#[test]
fn test_dispatcher_with_low_entropy_not_flagged() {
    let mut func = create_test_function("render", None);
    func.cyclomatic = 8;
    func.cognitive = 10;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.12,  // Low entropy
        ..Default::default()
    });

    let result = check_complexity_hotspot_predicate(&func);

    // Dampened cognitive: 10 * 0.7 ≈ 7, below threshold 15
    // Effective cyclomatic: 8, below threshold 10
    assert!(result.is_none(), "Dispatcher should not be flagged");
}

#[test]
fn test_genuinely_complex_still_flagged() {
    let mut func = create_test_function("complex", None);
    func.cyclomatic = 15;
    func.cognitive = 25;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.85,  // High entropy = no dampening
        ..Default::default()
    });

    let result = check_complexity_hotspot_predicate(&func);
    assert!(result.is_some(), "Complex function should be flagged");
}
```

## Migration

### Breaking Changes

- Removed `determine_debt_type()` - not used in production
- Removed `create_unified_debt_item()` family - replaced by `create_unified_debt_item_with_aggregator_and_data_flow()`

### Output Changes

Functions incorrectly flagged as "High Complexity" will no longer appear. This is the intended fix.

## References

- **Spec 68**: Entropy-based complexity dampening
- **Issue**: False positive on `src/tui/results/detail_view.rs:render`
