---
number: 201
title: Use Dampened Complexity in Debt Classification
category: foundation
priority: critical
status: draft
dependencies: [182, 68]
created: 2025-12-12
---

# Specification 201: Use Dampened Complexity in Debt Classification

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 182 (Adjusted Complexity in Classification), Spec 68 (Entropy-Based Dampening)

## Context

Debtmap's entropy analysis correctly identifies dispatcher patterns and repetitive code structures, dampening cognitive complexity appropriately. However, the **debt type classification logic does not consistently use these dampened values**, causing false positives.

### The Bug

There are **two code paths** for complexity hotspot detection with **different threshold systems** and **different use of dampening**:

#### Path 1: Single Debt Classification (`check_complexity_hotspot`)
```rust
// classification.rs:70-78
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    let effective_cyclomatic = func
        .adjusted_complexity           // ✅ Uses entropy-adjusted cyclomatic
        .map(|adj| adj.round() as u32)
        .unwrap_or(func.cyclomatic);

    let is_complex = effective_cyclomatic > 10 || func.cognitive > 15;  // ❌ Uses RAW cognitive
}
```
- Thresholds: cyclomatic > 10 OR cognitive > 15
- Uses `adjusted_complexity` for cyclomatic ✅
- Uses **RAW** `func.cognitive` ❌

#### Path 2: Multi-Debt Accumulation (`check_complexity_hotspot_predicate`)
```rust
// classification.rs:250-251
fn check_complexity_hotspot_predicate(func: &FunctionMetrics) -> Option<DebtType> {
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {  // ❌ RAW values
```

```rust
// classification.rs:700-701
fn is_complexity_hotspot_by_metrics(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 5 || cognitive > 8    // ❌ Much lower thresholds, RAW values
}
```
- Thresholds: cyclomatic > 5 OR cognitive > 8 (much lower!)
- Uses **RAW** cyclomatic ❌
- Uses **RAW** cognitive ❌

### Real Example: False Positive

Function: `render` in `src/tui/results/detail_view.rs`
- Raw cyclomatic: 8
- Raw cognitive: 10
- **Dampened cognitive: 5** (entropy analysis recognizes dispatcher pattern)

**What happens:**
1. Entropy analysis correctly identifies this as a clean dispatcher (match with 6 similar arms)
2. Cognitive complexity is dampened from 10 → 5
3. Multi-debt path calls `is_complexity_hotspot_by_metrics(8, 10)` using **RAW values**
4. Check: `8 > 5 || 10 > 8` → TRUE (false positive!)
5. Function is flagged as "High Complexity" despite being a simple dispatcher

**What should happen:**
1. Classification should use dampened cognitive value
2. Check: `8 > 5 || 5 > 8` → TRUE || FALSE → still flagged on cyclomatic
3. **OR better:** Use entropy-adjusted cyclomatic too

### Scope of Problem

The current state creates **systematic false positives** for:
- Dispatcher/router functions (match statements routing to handlers)
- Visitor patterns (exhaustive enum matching)
- State machines (repetitive state transitions)
- Enum converters (From/Into implementations)

These patterns are correctly identified by entropy analysis but then incorrectly classified due to the mismatch between dampening and classification.

## Objective

Ensure debt type classification uses entropy-dampened complexity values consistently:

1. **Multi-debt accumulation path** should use dampened values, not raw
2. **Thresholds should be consistent** between classification paths
3. **Both cyclomatic AND cognitive** should use dampened values where available
4. **No false positives** for patterns correctly identified as low-entropy dispatchers

## Requirements

### Functional Requirements

**FR1: Use Dampened Cognitive in `is_complexity_hotspot_by_metrics`**

The helper function must receive and use dampened cognitive value:

```rust
// BEFORE:
fn is_complexity_hotspot_by_metrics(cyclomatic: u32, cognitive: u32) -> bool {
    cyclomatic > 5 || cognitive > 8
}

// AFTER:
fn is_complexity_hotspot_by_metrics(
    effective_cyclomatic: u32,  // May be dampened
    effective_cognitive: u32,   // May be dampened
) -> bool {
    effective_cyclomatic > 5 || effective_cognitive > 8
}
```

**FR2: Update `check_complexity_hotspot_predicate` to Pass Dampened Values**

```rust
fn check_complexity_hotspot_predicate(func: &FunctionMetrics) -> Option<DebtType> {
    // Get effective (dampened) values
    let effective_cyclomatic = get_effective_cyclomatic(func);
    let effective_cognitive = get_effective_cognitive(func);

    if is_complexity_hotspot_by_metrics(effective_cyclomatic, effective_cognitive) {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,  // Store raw for display
            cognitive: func.cognitive,     // Store raw for display
        })
    } else {
        None
    }
}
```

**FR3: Add Dampened Cognitive Access on FunctionMetrics**

Currently only `adjusted_complexity` exists (which is for cyclomatic). Need to access dampened cognitive:

Option A: Add `adjusted_cognitive` field to `FunctionMetrics`
Option B: Calculate dampened cognitive from `entropy_score` when needed
Option C: Use `UnifiedDebtItem.entropy_adjusted_cognitive` during classification

**FR4: Update `check_complexity_hotspot` to Use Dampened Cognitive**

```rust
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    let effective_cyclomatic = func
        .adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(func.cyclomatic);

    // CHANGE: Use dampened cognitive if available
    let effective_cognitive = get_effective_cognitive(func);

    let is_complex = effective_cyclomatic > 10 || effective_cognitive > 15;
    // ...
}
```

**FR5: Consistent Threshold Strategy**

Unify the threshold approach:
- Single threshold set used by both paths
- OR explicitly document why different thresholds exist
- Consider using the spec 180 tier system consistently

### Non-Functional Requirements

**NFR1: Backward Compatibility**
- `DebtType::ComplexityHotspot` struct unchanged (stores raw values for display)
- Output format unchanged (dampened values shown separately in display)
- Existing tests for genuinely complex functions should still pass

**NFR2: No Performance Impact**
- Dampening calculation already happens in pipeline
- Classification should reuse existing dampened values, not recalculate

**NFR3: Clear Audit Trail**
- When dampening causes a function to NOT be flagged, should be visible in verbose output
- Log: "Function X has raw complexity 8/10, dampened to 8/5 - below threshold"

## Acceptance Criteria

- [ ] `is_complexity_hotspot_by_metrics` receives dampened values
- [ ] `check_complexity_hotspot_predicate` passes dampened values
- [ ] `check_complexity_hotspot` uses dampened cognitive (not just cyclomatic)
- [ ] `render` function in `detail_view.rs` is NOT flagged as ComplexityHotspot (test case)
- [ ] Dispatcher patterns with low entropy are not false positives
- [ ] Genuinely complex functions (high entropy) are still correctly flagged
- [ ] Unit tests verify dampening is applied before classification
- [ ] Integration test with real dispatcher functions validates no false positives
- [ ] Verbose output shows when dampening affects classification decision

## Technical Details

### Implementation Approach

**Phase 1: Add Access to Dampened Cognitive**

Option A (Recommended): Use entropy score to calculate dampened cognitive

```rust
// Add helper function in classification.rs
fn get_effective_cognitive(func: &FunctionMetrics) -> u32 {
    // Check if entropy score indicates dampening should apply
    if let Some(entropy) = &func.entropy_score {
        let dampening_factor = calculate_dampening_factor(entropy.token_entropy);
        (func.cognitive as f64 * dampening_factor).round() as u32
    } else {
        func.cognitive
    }
}

fn get_effective_cyclomatic(func: &FunctionMetrics) -> u32 {
    func.adjusted_complexity
        .map(|adj| adj.round() as u32)
        .unwrap_or(func.cyclomatic)
}

fn calculate_dampening_factor(token_entropy: f64) -> f64 {
    // Matches logic in entropy.rs - spec 68
    if token_entropy < 0.2 {
        (0.5_f64).max(1.0 - (0.5 * (0.2 - token_entropy) / 0.2))
    } else {
        1.0
    }
}
```

**Phase 2: Update Multi-Debt Path**

```rust
fn check_complexity_hotspot_predicate(func: &FunctionMetrics) -> Option<DebtType> {
    let effective_cyclomatic = get_effective_cyclomatic(func);
    let effective_cognitive = get_effective_cognitive(func);

    if is_complexity_hotspot_by_metrics(effective_cyclomatic, effective_cognitive) {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        })
    } else {
        None
    }
}
```

**Phase 3: Update Single-Debt Path**

```rust
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    let effective_cyclomatic = get_effective_cyclomatic(func);
    let effective_cognitive = get_effective_cognitive(func);

    let is_complex = effective_cyclomatic > 10 || effective_cognitive > 15;

    if !is_complex {
        return None;
    }

    // Spec 180: Filter out Low tier
    let is_low_tier = effective_cyclomatic < 8 && effective_cognitive < 15;

    if is_low_tier {
        return None;
    }

    Some(DebtType::ComplexityHotspot {
        cyclomatic: func.cyclomatic,  // Raw for display
        cognitive: func.cognitive,     // Raw for display
    })
}
```

### Architecture Changes

**Modified Files**:

1. `src/priority/scoring/classification.rs`:
   - Add `get_effective_cyclomatic()` helper
   - Add `get_effective_cognitive()` helper
   - Add `calculate_dampening_factor()` (or import from entropy module)
   - Update `check_complexity_hotspot()`
   - Update `check_complexity_hotspot_predicate()`
   - Update `is_complexity_hotspot_by_metrics()` signature
   - Update `is_complexity_hotspot()` public function

2. `src/priority/scoring/validation.rs`:
   - Update `check_complexity_hotspot()` to use dampened values

### Data Structures

No changes to existing data structures. `DebtType::ComplexityHotspot` continues to store raw values:

```rust
pub enum DebtType {
    ComplexityHotspot {
        cyclomatic: u32,  // Raw, for display showing "10 → 5 (dampened)"
        cognitive: u32,   // Raw, for display
    },
    // ...
}
```

### APIs and Interfaces

**Internal Changes**:

```rust
// BEFORE:
fn is_complexity_hotspot_by_metrics(cyclomatic: u32, cognitive: u32) -> bool

// AFTER:
fn is_complexity_hotspot_by_metrics(
    effective_cyclomatic: u32,
    effective_cognitive: u32,
) -> bool
```

No public API changes.

## Dependencies

**Prerequisites**:
- Spec 182: Adjusted complexity in classification (partially implemented)
- Spec 68: Entropy-based dampening (implemented)

**Affected Components**:
- `src/priority/scoring/classification.rs`: Main changes
- `src/priority/scoring/validation.rs`: Parallel changes
- Tests using `is_complexity_hotspot_by_metrics`

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test 1: Dispatcher Pattern Not Flagged**

```rust
#[test]
fn test_dispatcher_with_low_entropy_not_flagged() {
    let mut func = create_test_function("render", None);
    func.cyclomatic = 8;
    func.cognitive = 10;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.12,  // Low entropy = dispatcher pattern
        ..Default::default()
    });

    let result = check_complexity_hotspot(&func);

    // Dampened cognitive: 10 * 0.7 = 7, below threshold
    // Should NOT be flagged
    assert!(
        result.is_none(),
        "Dispatcher pattern with low entropy should not be flagged as ComplexityHotspot"
    );
}
```

**Test 2: Genuinely Complex Function Still Flagged**

```rust
#[test]
fn test_high_entropy_complex_function_flagged() {
    let mut func = create_test_function("complex_logic", None);
    func.cyclomatic = 15;
    func.cognitive = 25;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.85,  // High entropy = genuine complexity
        ..Default::default()
    });

    let result = check_complexity_hotspot(&func);

    // No dampening applied (entropy > 0.2)
    // Should be flagged
    assert!(
        result.is_some(),
        "Genuinely complex function should still be flagged"
    );
}
```

**Test 3: Multi-Debt Path Uses Dampened Values**

```rust
#[test]
fn test_multi_debt_path_uses_dampened_cognitive() {
    let mut func = create_test_function("multi_debt_test", None);
    func.cyclomatic = 4;  // Below threshold even raw
    func.cognitive = 12;  // Above raw threshold (> 8)
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.1,  // Very low entropy
        ..Default::default()
    });

    // Dampened cognitive: 12 * 0.5 = 6, below threshold
    let result = check_complexity_hotspot_predicate(&func);

    assert!(
        result.is_none(),
        "Low-entropy function should not be flagged via multi-debt path"
    );
}
```

**Test 4: Consistent Behavior Between Paths**

```rust
#[test]
fn test_both_paths_give_consistent_results() {
    let mut func = create_test_function("consistency_test", None);
    func.cyclomatic = 8;
    func.cognitive = 10;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.12,
        ..Default::default()
    });

    let single_path = check_complexity_hotspot(&func);
    let multi_path = check_complexity_hotspot_predicate(&func);

    // Both should give same decision (either both Some or both None)
    assert_eq!(
        single_path.is_some(),
        multi_path.is_some(),
        "Single and multi-debt paths should give consistent classification"
    );
}
```

### Integration Tests

**Test 5: Real TUI Dispatcher Functions**

```rust
#[test]
fn test_tui_dispatchers_not_flagged() {
    // Test with actual patterns from src/tui/results/detail_view.rs
    let dispatcher_metrics = FunctionMetrics {
        name: "render".to_string(),
        file: "src/tui/results/detail_view.rs".to_string(),
        cyclomatic: 8,
        cognitive: 10,
        nesting: 2,
        length: 61,
        entropy_score: Some(EntropyScore {
            token_entropy: 0.12,  // As measured by entropy analyzer
            ..Default::default()
        }),
        ..Default::default()
    };

    let debt_types = classify_debt_type_with_exclusions(
        &dispatcher_metrics,
        &CallGraph::new(),
        &func_id,
        &HashSet::new(),
        None,
        None,
    );

    // Should NOT contain ComplexityHotspot
    assert!(
        !debt_types.iter().any(|dt| matches!(dt, DebtType::ComplexityHotspot { .. })),
        "TUI dispatcher should not be flagged as High Complexity"
    );
}
```

### Regression Tests

Ensure existing genuinely complex functions are still flagged:

```rust
#[test]
fn test_regression_complex_functions_still_flagged() {
    // Functions that SHOULD be flagged
    let test_cases = vec![
        ("high_cyclomatic", 25, 20, 0.8),  // High complexity, high entropy
        ("deeply_nested", 15, 45, 0.75),   // Nested logic, high entropy
        ("business_logic", 18, 30, 0.7),   // Genuine complexity
    ];

    for (name, cyclo, cog, entropy) in test_cases {
        let mut func = create_test_function(name, None);
        func.cyclomatic = cyclo;
        func.cognitive = cog;
        func.entropy_score = Some(EntropyScore {
            token_entropy: entropy,
            ..Default::default()
        });

        let result = check_complexity_hotspot(&func);
        assert!(
            result.is_some(),
            "Function {} should still be flagged as complex",
            name
        );
    }
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Get effective (dampened) cyclomatic complexity for classification.
///
/// Uses entropy-adjusted value if available (from spec 118 mapping patterns
/// or spec 68 entropy dampening), otherwise returns raw cyclomatic.
///
/// # Dampening Logic (Spec 68, 201)
/// - Token entropy < 0.2: Apply dampening factor (50-100% preserved)
/// - Token entropy >= 0.2: No dampening (100% preserved)
///
/// This prevents false positives for dispatcher patterns, visitors,
/// and other repetitive-but-simple code structures.
fn get_effective_cyclomatic(func: &FunctionMetrics) -> u32
```

### User Documentation

Update FAQ:

**Q: Why isn't my simple match statement flagged as high complexity?**

A: Debtmap uses entropy analysis to distinguish between genuinely complex code
and repetitive patterns like dispatchers or enum converters. A match statement
with similar arms has low token entropy, indicating the complexity is structural
(many paths) rather than cognitive (hard to understand). The dampened complexity
is used for classification decisions.

## Implementation Notes

### Gotchas

1. **Threshold Alignment**: The two paths use different thresholds (5/8 vs 10/15). This spec focuses on dampening, but a follow-up may want to unify thresholds.

2. **Entropy Score Availability**: Not all `FunctionMetrics` have `entropy_score` populated. The helper functions must handle `None` gracefully (fall back to raw values).

3. **Display vs Classification**: `DebtType::ComplexityHotspot` stores RAW values for display purposes (so users see "10 → 5 (dampened)"). Classification uses dampened values, but the stored values should remain raw.

4. **Spec 180 Tier System**: The Low tier filtering (`< 8 cyclomatic AND < 15 cognitive`) should use dampened values consistently.

### Edge Cases

1. **No Entropy Score**: Functions without entropy analysis should use raw values (no dampening assumed).

2. **Dampening to Zero**: Extreme dampening (token_entropy ≈ 0) could reduce complexity to near-zero. Ensure minimum values are respected.

3. **Cyclomatic vs Cognitive Mismatch**: A function might have dampened cognitive but raw cyclomatic. Both should be handled independently.

## Migration and Compatibility

### Breaking Changes

**None** - this is a bug fix that reduces false positives. No API changes.

### Output Changes

Functions that were incorrectly flagged as "High Complexity" will no longer appear in debt reports. This is the intended behavior.

### Rollback Plan

If issues discovered:
1. Revert helper functions to use raw values
2. Add feature flag to toggle dampened classification
3. Investigate which false negatives are occurring

## Success Metrics

1. **False Positive Reduction**: Dispatcher patterns no longer flagged as High Complexity
2. **True Positive Preservation**: Genuinely complex functions still flagged
3. **Consistency**: Both classification paths give same results for same inputs
4. **Test Coverage**: 100% of new code paths tested

## References

- **Spec 68**: Entropy-based complexity dampening
- **Spec 182**: Adjusted complexity in classification
- **Spec 180**: Complexity tier system (Low/Moderate/High)
- **Issue**: False positive on `src/tui/results/detail_view.rs:render`
