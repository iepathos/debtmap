---
number: 206
title: Clean Match Dispatcher Pattern Recognition
category: optimization
priority: medium
status: draft
dependencies: [205]
created: 2025-12-11
---

# Specification 206: Clean Match Dispatcher Pattern Recognition

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 205 (Simple Arm Pattern Detection)

## Context

Debtmap's complexity pattern detection (`src/priority/complexity_patterns.rs`) classifies functions into patterns like `StateMachine`, `Coordinator`, `Dispatcher`, `HighBranching`, etc. to provide targeted refactoring recommendations.

### The Problem

The current `Dispatcher` pattern detection has overly strict criteria that miss common clean dispatcher patterns:

```rust
// Current detection (complexity_patterns.rs:266)
if metrics.cyclomatic >= 15 && ratio < 0.5 && metrics.coordinator_signals.is_none()
```

**Requires**: cognitive/cyclomatic ratio < 0.5

This is too strict for match dispatchers with `?` operators. Each arm's `?` adds 1 to both cyclomatic AND cognitive, resulting in ratio ~1.5, not < 0.5.

### Evidence: `write_section` Function

- **Cyclomatic**: 26 (12 match arms + 13 `?` operators + 1 base)
- **Cognitive**: 40 (match cost + arm body costs + `?` costs)
- **Ratio**: 40/26 = **1.54** (fails the < 0.5 test)
- **Nesting**: 1 (very flat structure)

**Result**: Classified as `HighBranching` instead of `Dispatcher`, leading to inappropriate "Split into 3 functions" recommendation.

### Current Pattern Detection Flow

1. RepetitiveValidation (entropy-based)
2. StateMachine (signals-based)
3. Coordinator (signals-based)
4. **Dispatcher** (ratio < 0.5) ← Too strict!
5. ChaoticStructure (entropy >= 0.45)
6. HighNesting (ratio > 3.0, nesting >= 4)
7. **HighBranching** (cyclomatic >= 15, ratio < 2.5) ← Catches dispatchers
8. MixedComplexity
9. ModerateComplexity (default)

## Objective

Enhance dispatcher pattern detection to recognize clean match dispatchers with:
1. High cyclomatic complexity (many match arms)
2. Low nesting (flat structure)
3. Cognitive/cyclomatic ratio between 0.5 and 2.0
4. Delegation-style arms (call helpers or use macros)

Also refine the recommendation generation to handle "clean dispatchers" that need no refactoring.

## Requirements

### Functional Requirements

1. **FR-1: Relax Dispatcher Ratio Threshold**
   - Change ratio threshold from < 0.5 to < 2.0
   - Add nesting check: nesting <= 2 (flat structure)
   - Prevents misclassification as HighBranching

2. **FR-2: Distinguish Clean vs Polluted Dispatchers**
   - Clean dispatcher: all arms delegate (calls, macros, returns)
   - Polluted dispatcher: some arms have inline logic (>2 statements)
   - Track `inline_logic_branches` count

3. **FR-3: Suppress Recommendations for Clean Dispatchers**
   - If dispatcher is clean (inline_logic_branches == 0), return None or minimal recommendation
   - Do not recommend splitting a well-structured dispatcher
   - Only recommend extraction for polluted dispatchers

4. **FR-4: Consider `?` Operator Context**
   - When ratio is 1.0-2.0 and nesting is low, consider `?` inflation
   - Heuristic: if cognitive ≈ cyclomatic, complexity is from `?` not logic

### Non-Functional Requirements

1. **NFR-1: Accuracy**
   - Clean dispatchers should NOT get refactoring recommendations
   - Polluted dispatchers should get specific extraction recommendations

2. **NFR-2: Backward Compatibility**
   - Existing Dispatcher pattern detections should not regress
   - Only expand coverage, don't change existing positive cases

## Acceptance Criteria

- [ ] `write_section` is classified as `Dispatcher`, not `HighBranching`
- [ ] Clean dispatchers (all delegation) return `None` from recommendation generation
- [ ] Polluted dispatchers (some inline logic) get extraction recommendations
- [ ] Dispatcher detection works for ratio 0.5-2.0 with low nesting
- [ ] Existing HighBranching patterns (non-dispatch) still detected correctly
- [ ] Test suite for dispatcher patterns passes

## Technical Details

### Implementation Approach

#### Phase 1: Relax Dispatcher Pattern Detection

**File**: `src/priority/complexity_patterns.rs` (around line 264)

```rust
// BEFORE:
if metrics.cyclomatic >= 15 && ratio < 0.5 && metrics.coordinator_signals.is_none() {

// AFTER:
// Clean delegation dispatcher: high cyclomatic, low nesting, moderate ratio
if metrics.cyclomatic >= 10
    && metrics.nesting <= 2
    && ratio < 2.0
    && metrics.coordinator_signals.is_none()
{
    // Estimate inline logic based on cognitive deviation from expected
    // Clean dispatcher: cognitive ≈ cyclomatic * 0.3-1.5 (delegation + ?)
    let expected_max_cognitive = (metrics.cyclomatic as f64 * 1.5) as u32;
    let inline_logic_branches = if metrics.cognitive > expected_max_cognitive {
        // Each inline logic section adds ~3-5 cognitive points beyond delegation
        ((metrics.cognitive - expected_max_cognitive) as f64 / 4.0).ceil() as u32
    } else {
        0
    };

    return ComplexityPattern::Dispatcher {
        branch_count: metrics.cyclomatic,
        cognitive_ratio: ratio,
        inline_logic_branches,
    };
}
```

#### Phase 2: Update HighBranching to Avoid Dispatcher Overlap

Ensure HighBranching doesn't catch dispatchers:

```rust
// BEFORE:
if metrics.cyclomatic >= 15 && ratio < 2.5 {

// AFTER:
// High branching: cyclomatic high, NOT a flat dispatcher
if metrics.cyclomatic >= 15 && ratio < 2.5 && metrics.nesting >= 2 {
    return ComplexityPattern::HighBranching { ... };
}
```

Or better, add explicit exclusion:

```rust
// High branching: many decision points, moderate nesting
// Excludes flat match dispatchers (handled by Dispatcher pattern above)
if metrics.cyclomatic >= 15 && ratio < 2.5 {
    // Only classify as HighBranching if not a flat dispatcher
    if metrics.nesting >= 2 || ratio >= 2.0 {
        return ComplexityPattern::HighBranching { ... };
    }
}
```

#### Phase 3: Handle Clean Dispatchers in Recommendations

**File**: `src/priority/scoring/concise_recommendation.rs` (around line 832)

```rust
fn generate_dispatcher_recommendation(
    branch_count: u32,
    cognitive_ratio: f64,
    inline_logic_branches: u32,
    cyclomatic: u32,
    cognitive: u32,
    metrics: &FunctionMetrics,
) -> Option<ActionableRecommendation> {
    // CHANGED: Clean dispatcher with no inline logic needs no refactoring
    if inline_logic_branches == 0 {
        // Check if nesting is low (flat structure)
        if metrics.nesting <= 2 {
            // This is a well-structured dispatcher - no recommendation needed
            return None;
        }
        // Fall back to generic recommendation for edge cases
        return Some(generate_moderate_recommendation(cyclomatic, cognitive, metrics));
    }

    // Existing polluted dispatcher logic...
    let extraction_impact = RefactoringImpact::extract_function(inline_logic_branches);
    // ...
}
```

### Architecture Changes

No architectural changes - modifications are localized to pattern detection and recommendation generation.

### Data Structures

No new data structures. The existing `Dispatcher` pattern variant is sufficient:

```rust
Dispatcher {
    branch_count: u32,
    cognitive_ratio: f64,
    inline_logic_branches: u32,  // 0 = clean, >0 = needs extraction
}
```

### APIs and Interfaces

No public API changes.

## Dependencies

- **Prerequisites**: Spec 205 (Simple Arm Pattern Detection) - enables proper detection of simple arms
- **Affected Components**:
  - `src/priority/complexity_patterns.rs` - Pattern detection
  - `src/priority/scoring/concise_recommendation.rs` - Recommendation generation
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_clean_dispatcher_detection() {
    // Flat match dispatcher with ? operators
    let metrics = ComplexityMetrics {
        cyclomatic: 26,   // Many arms + ?
        cognitive: 40,     // Arms + ?
        nesting: 1,        // Flat structure
        entropy_score: Some(0.30),
        state_signals: None,
        coordinator_signals: None,
        validation_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);

    assert!(
        matches!(pattern, ComplexityPattern::Dispatcher { inline_logic_branches: 0, .. }),
        "Should detect as clean Dispatcher, got: {:?}",
        pattern
    );
}

#[test]
fn test_polluted_dispatcher_detection() {
    // Dispatcher with some inline logic
    let metrics = ComplexityMetrics {
        cyclomatic: 20,
        cognitive: 45,     // Higher than expected for clean dispatcher
        nesting: 2,
        entropy_score: Some(0.35),
        state_signals: None,
        coordinator_signals: None,
        validation_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);

    match pattern {
        ComplexityPattern::Dispatcher { inline_logic_branches, .. } => {
            assert!(inline_logic_branches > 0, "Should detect inline logic");
        }
        _ => panic!("Should detect as Dispatcher, got: {:?}", pattern),
    }
}

#[test]
fn test_high_nesting_not_dispatcher() {
    // High nesting should NOT be classified as Dispatcher
    let metrics = ComplexityMetrics {
        cyclomatic: 20,
        cognitive: 60,     // High cognitive from nesting
        nesting: 4,        // Deep nesting
        entropy_score: Some(0.30),
        state_signals: None,
        coordinator_signals: None,
        validation_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);

    assert!(
        !matches!(pattern, ComplexityPattern::Dispatcher { .. }),
        "High nesting should not be Dispatcher, got: {:?}",
        pattern
    );
}

#[test]
fn test_clean_dispatcher_no_recommendation() {
    let metrics = create_test_metrics(26, 40);
    metrics.nesting = 1;  // Flat

    let result = generate_dispatcher_recommendation(
        26,   // branch_count
        1.54, // cognitive_ratio
        0,    // inline_logic_branches (clean)
        26,   // cyclomatic
        40,   // cognitive
        &metrics,
    );

    assert!(
        result.is_none(),
        "Clean dispatcher should return None (no refactoring needed)"
    );
}

#[test]
fn test_polluted_dispatcher_gets_recommendation() {
    let metrics = create_test_metrics(20, 45);
    metrics.nesting = 2;

    let result = generate_dispatcher_recommendation(
        20,   // branch_count
        2.25, // cognitive_ratio
        3,    // inline_logic_branches (polluted)
        20,   // cyclomatic
        45,   // cognitive
        &metrics,
    );

    assert!(result.is_some(), "Polluted dispatcher should get recommendation");
    let rec = result.unwrap();
    assert!(
        rec.primary_action.contains("Extract inline logic"),
        "Should recommend extraction, got: {}",
        rec.primary_action
    );
}
```

### Integration Tests

```rust
#[test]
fn test_write_section_not_flagged() {
    // Simulate write_section metrics
    let metrics = FunctionMetrics {
        name: "write_section".to_string(),
        cyclomatic: 26,
        cognitive: 40,
        nesting: 1,
        // ... other fields
    };

    let recommendation = generate_concise_recommendation(
        &DebtType::ComplexityHotspot { cyclomatic: 26, cognitive: 40 },
        &metrics,
        FunctionRole::Formatter,
        &None,
    );

    // Should return None (clean dispatcher needs no refactoring)
    // OR return a maintenance-only recommendation
    if let Some(rec) = recommendation {
        assert!(
            !rec.primary_action.contains("Split"),
            "Should NOT recommend splitting, got: {}",
            rec.primary_action
        );
    }
}
```

## Documentation Requirements

- **Code Documentation**: Update doc comments for `ComplexityPattern::Dispatcher` and `detect()` method
- **User Documentation**: None required
- **Architecture Updates**: None required

## Implementation Notes

1. **Order of Pattern Checks**: Dispatcher check must come before HighBranching to catch clean dispatchers first.

2. **Ratio Heuristics**:
   - ratio < 0.5: Very clean dispatcher (mostly literals/paths)
   - ratio 0.5-1.5: Dispatcher with delegation calls and `?`
   - ratio 1.5-2.0: Dispatcher with some complexity
   - ratio >= 2.0: Likely not a dispatcher (cognitive dominates)

3. **Nesting as Discriminator**: Flat structure (nesting <= 2) is the key indicator of a dispatcher. Even complex match arms don't create deep nesting if they delegate to helpers.

4. **Cognitive Deviation Heuristic**: Expected cognitive for clean dispatcher is roughly `cyclomatic * 1.0-1.5` (each arm adds ~1 from `?` or simple expression). Deviation beyond this suggests inline logic.

5. **False Positive Prevention**: Only suppress recommendations when we're confident it's a clean dispatcher:
   - nesting <= 2
   - inline_logic_branches == 0
   - ratio < 2.0

## Migration and Compatibility

- **Breaking Changes**: Some functions previously flagged as `HighBranching` will now be `Dispatcher`. This is an improvement, not a regression.
- **Migration**: Not required
- **Compatibility**: Output format unchanged. Only classification and recommendations change.

## Success Metrics

1. **Pattern Accuracy**: `write_section` classified as `Dispatcher`
2. **Recommendation Quality**: Clean dispatchers get no refactoring recommendations
3. **No False Positives**: Functions with genuine complexity issues still flagged
4. **Coverage**: All match dispatcher patterns in debtmap codebase correctly classified
