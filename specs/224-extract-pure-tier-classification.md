---
number: 224
title: Extract Pure Tier Classification Functions
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-05
---

# Specification 224: Extract Pure Tier Classification Functions

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The tiering system (`src/priority/tiers.rs`) currently contains complex classification logic mixed with imperative conditionals. According to the Stillwater philosophy evaluation, this violates the "Pure Core, Imperative Shell" principle and makes the code difficult to test and reason about.

**Current Issues:**

1. **Complex Conditionals** - `classify_tier()` has nested if/else logic (lines 133-158)
2. **Mixed Concerns** - Pure classification logic intertwined with control flow
3. **Poor Testability** - Hard to test individual classification criteria in isolation
4. **High Cyclomatic Complexity** - Helper functions like `is_architectural_issue()` exceed complexity threshold (162-219)
5. **Violation of Functional Principles** - Not following "Composition Over Complexity" from Stillwater

**Current Code Pattern:**

```rust
// src/priority/tiers.rs (lines 133-158)
pub fn classify_tier(item: &UnifiedDebtItem, config: &TierConfig) -> RecommendationTier {
    if is_architectural_issue(item, config) {
        RecommendationTier::T1CriticalArchitecture
    } else if is_complex_untested(item, config) || is_moderate_complexity_hotspot(item) {
        RecommendationTier::T2ComplexUntested
    } else if is_moderate_untested(item, config) {
        RecommendationTier::T3TestingGaps
    } else {
        RecommendationTier::T4Maintenance
    }
}

// is_architectural_issue() is 57 lines (162-219) with many nested conditions
```

**Stillwater Philosophy Violation:**

The current implementation mixes:
- Pure logic (should this be T1? T2?)
- Imperative control flow (if/else chains)
- No clear composition of smaller predicates

## Objective

Refactor tier classification to follow Stillwater's "Pure Core, Imperative Shell" principle:

1. **Extract predicates** - Break complex functions into small, composable predicate functions
2. **Pure classification** - All classification logic in pure, testable functions (< 10 lines each)
3. **Clear composition** - Build complex checks from simple, named predicates
4. **100% testable** - Each predicate independently unit testable
5. **Reduce complexity** - All functions cyclomatic complexity < 5

Result: Clean, functional tier classification that's easy to test, understand, and extend.

## Requirements

### Functional Requirements

1. **Predicate Extraction**
   - Extract all classification checks into pure predicate functions
   - Each predicate tests ONE thing (< 10 lines)
   - Clear, descriptive function names (is_*, has_*, should_*)
   - No side effects, fully deterministic
   - Examples:
     - `is_god_object(debt_type: &DebtType) -> bool`
     - `has_extreme_complexity(metrics: &ItemMetrics) -> bool`
     - `has_deep_nesting(metrics: &ItemMetrics) -> bool`

2. **Pure Classification Core**
   - Create `src/priority/tiers/pure.rs` module for pure functions
   - Create `src/priority/tiers/predicates.rs` for predicate functions
   - Move all classification logic to pure functions
   - No mutation, no I/O, no external dependencies
   - Composable via boolean operators (&&, ||)

3. **Preserve Functionality**
   - All existing tier classification behavior preserved
   - Same tier assignments for same inputs
   - Backward compatible API
   - No changes to public interfaces

4. **Clear Structure**
   - Organize predicates by tier level:
     - T1 predicates (architectural issues)
     - T2 predicates (complex untested)
     - T3 predicates (moderate untested)
   - Group related predicates together
   - Document predicate composition

### Non-Functional Requirements

1. **Testability**
   - Each predicate unit tested independently
   - No mocks required (pure functions)
   - Fast tests (no I/O)
   - Property-based tests for invariants
   - Test coverage 95%+ for pure functions

2. **Maintainability**
   - Each function < 10 lines
   - Cyclomatic complexity < 5
   - Self-documenting function names
   - Clear predicate composition

3. **Performance**
   - No performance regression
   - Inline small predicates
   - Zero-cost abstractions

## Acceptance Criteria

- [ ] New `src/priority/tiers/pure.rs` module created
- [ ] New `src/priority/tiers/predicates.rs` module created
- [ ] All predicate functions extracted (< 10 lines each)
- [ ] `is_architectural_issue()` broken into composable predicates
- [ ] `is_complex_untested()` broken into predicates
- [ ] `is_moderate_complexity_hotspot()` broken into predicates
- [ ] `is_moderate_untested()` broken into predicates
- [ ] All functions cyclomatic complexity < 5
- [ ] Unit tests for all predicates (95%+ coverage)
- [ ] Property-based tests for classification invariants
- [ ] All existing integration tests pass
- [ ] No clippy warnings
- [ ] Performance benchmarks show no regression
- [ ] Documentation added for predicate composition

## Technical Details

### Implementation Approach

**Phase 1: Create Module Structure**

```rust
// src/priority/tiers/mod.rs
pub mod predicates;  // Pure predicate functions
pub mod pure;        // Pure classification logic

// Re-export for backward compatibility
pub use self::pure::classify_tier;
```

**Phase 2: Extract Predicates**

```rust
// src/priority/tiers/predicates.rs

/// Pure predicates for tier classification.
/// Each predicate tests ONE specific condition.

// ============================================================================
// T1 ARCHITECTURAL ISSUE PREDICATES
// ============================================================================

/// Checks if debt type is a god object or god module.
pub fn is_god_object(debt_type: &DebtType) -> bool {
    matches!(debt_type, DebtType::GodObject | DebtType::GodModule)
}

/// Checks if debt type is an error handling issue.
pub fn is_error_handling_issue(debt_type: &DebtType) -> bool {
    matches!(debt_type, DebtType::ErrorSwallowing | DebtType::AsyncMisuse)
}

/// Checks if item has extreme cyclomatic complexity (> 50).
pub fn has_extreme_cyclomatic(metrics: &ItemMetrics) -> bool {
    metrics.effective_cyclomatic > 50
}

/// Checks if item has extreme cognitive complexity (>= 20).
pub fn has_extreme_cognitive(metrics: &ItemMetrics) -> bool {
    metrics.cognitive >= 20
}

/// Checks if item has very deep nesting (>= 5 levels).
pub fn has_deep_nesting(metrics: &ItemMetrics) -> bool {
    metrics.nesting_depth >= 5
}

/// Checks if item has high complexity factor (> 5.0).
/// Complexity factor is weighted: 30% cyclomatic + 70% cognitive.
pub fn has_high_complexity_factor(metrics: &ItemMetrics) -> bool {
    complexity_factor(metrics) > 5.0
}

/// Checks if item has extremely high score (> 10.0, exponential scaling).
pub fn has_extreme_score(score: f64) -> bool {
    score > 10.0
}

// ============================================================================
// T2 COMPLEX UNTESTED PREDICATES
// ============================================================================

/// Checks if debt type is a testing gap.
pub fn is_testing_gap(debt_type: &DebtType) -> bool {
    matches!(debt_type, DebtType::TestingGap)
}

/// Checks if debt type is a complexity hotspot.
pub fn is_complexity_hotspot(debt_type: &DebtType) -> bool {
    matches!(debt_type, DebtType::ComplexityHotspot)
}

/// Checks if item has high cyclomatic complexity for its tier.
pub fn has_high_cyclomatic(metrics: &ItemMetrics, threshold: u32) -> bool {
    metrics.cyclomatic >= threshold
}

/// Checks if item has moderate cognitive complexity (>= 12).
pub fn has_moderate_cognitive(metrics: &ItemMetrics) -> bool {
    metrics.cognitive_complexity >= 12
}

/// Checks if item has moderate nesting (>= 3 levels).
pub fn has_moderate_nesting(metrics: &ItemMetrics) -> bool {
    metrics.nesting_depth >= 3
}

/// Checks if item has moderate complexity factor (>= 2.0).
pub fn has_moderate_complexity_factor(metrics: &ItemMetrics) -> bool {
    complexity_factor(metrics) >= 2.0
}

/// Checks if item has adjusted cyclomatic in moderate range (8-50).
pub fn has_moderate_adjusted_cyclomatic(metrics: &ItemMetrics) -> bool {
    let adjusted = adjusted_cyclomatic(metrics);
    adjusted >= 8 && adjusted < 50
}

/// Checks if item has many dependencies (>= threshold).
pub fn has_many_dependencies(deps: usize, threshold: usize) -> bool {
    deps >= threshold
}

/// Checks if item is an entry point function.
pub fn is_entry_point(function_type: Option<&FunctionType>) -> bool {
    matches!(function_type, Some(FunctionType::EntryPoint))
}

// ============================================================================
// T3 TESTING GAP PREDICATES
// ============================================================================

/// Checks if item has moderate cyclomatic complexity for T3.
pub fn has_t3_cyclomatic(metrics: &ItemMetrics, threshold: u32) -> bool {
    metrics.cyclomatic >= threshold
}

// ============================================================================
// HELPER FUNCTIONS (Pure)
// ============================================================================

/// Calculates complexity factor: 30% cyclomatic + 70% cognitive.
fn complexity_factor(metrics: &ItemMetrics) -> f64 {
    let cyclomatic_weight = 0.3;
    let cognitive_weight = 0.7;
    (metrics.cyclomatic as f64 * cyclomatic_weight)
        + (metrics.cognitive as f64 * cognitive_weight)
}

/// Calculates entropy-dampened cyclomatic complexity.
fn adjusted_cyclomatic(metrics: &ItemMetrics) -> f64 {
    // Implementation from existing code
    let entropy = calculate_entropy(metrics);
    metrics.cyclomatic as f64 * (1.0 - entropy)
}

fn calculate_entropy(metrics: &ItemMetrics) -> f64 {
    // Existing entropy calculation (pure)
    // ...
}
```

**Phase 3: Compose Predicates**

```rust
// src/priority/tiers/pure.rs

use super::predicates::*;

/// Pure tier classification using composed predicates.
///
/// This is a pure function that composes smaller predicates to determine tier.
/// No side effects, fully deterministic, easily testable.
pub fn classify_tier(item: &UnifiedDebtItem, config: &TierConfig) -> RecommendationTier {
    if is_t1_architectural(item, config) {
        RecommendationTier::T1CriticalArchitecture
    } else if is_t2_complex_untested(item, config) {
        RecommendationTier::T2ComplexUntested
    } else if is_t3_testing_gap(item, config) {
        RecommendationTier::T3TestingGaps
    } else {
        RecommendationTier::T4Maintenance
    }
}

/// Checks if item is T1 architectural issue (composed from predicates).
fn is_t1_architectural(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    is_god_or_error_issue(&item.debt_type)
        || has_t1_complexity(&item.metrics, &item.unified_score)
}

/// Checks if debt type is god object/module or error handling issue.
fn is_god_or_error_issue(debt_type: &DebtType) -> bool {
    is_god_object(debt_type) || is_error_handling_issue(debt_type)
}

/// Checks if metrics indicate T1 complexity level.
fn has_t1_complexity(metrics: &ItemMetrics, score: &UnifiedScore) -> bool {
    has_extreme_score(score.final_score)
        || has_extreme_cyclomatic(metrics)
        || has_extreme_cognitive(metrics)
        || has_deep_nesting(metrics)
        || has_high_complexity_factor(metrics)
}

/// Checks if item is T2 complex untested (composed from predicates).
fn is_t2_complex_untested(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    is_t2_testing_gap(item, config) || is_t2_complexity_hotspot(item)
}

/// Checks if item is T2-level testing gap.
fn is_t2_testing_gap(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    is_testing_gap(&item.debt_type) && (
        has_high_cyclomatic(&item.metrics, config.t2_complexity_threshold)
        || has_many_dependencies(item.total_dependencies, config.t2_dependency_threshold)
        || is_entry_point(item.function_type.as_ref())
    )
}

/// Checks if item is T2-level complexity hotspot.
fn is_t2_complexity_hotspot(item: &UnifiedDebtItem) -> bool {
    is_complexity_hotspot(&item.debt_type) && (
        has_moderate_complexity_factor(&item.metrics)
        || has_moderate_cognitive(&item.metrics)
        || has_moderate_nesting(&item.metrics)
        || has_moderate_adjusted_cyclomatic(&item.metrics)
    )
}

/// Checks if item is T3 testing gap.
fn is_t3_testing_gap(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    is_testing_gap(&item.debt_type)
        && has_t3_cyclomatic(&item.metrics, config.t3_complexity_threshold)
}
```

### Function Size Comparison

**Before:**
- `is_architectural_issue()`: 57 lines (162-219)
- `is_complex_untested()`: 22 lines (221-243)
- `is_moderate_complexity_hotspot()`: 45 lines (257-302)
- `is_moderate_untested()`: 10 lines (245-255)

**After:**
- Each predicate: 2-5 lines
- Each composition function: 5-8 lines
- Maximum function length: 10 lines
- Average cyclomatic complexity: 2

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/priority/tiers.rs` (refactor internals)
  - `src/priority/tiers/pure.rs` (new)
  - `src/priority/tiers/predicates.rs` (new)
- **External Dependencies**: None
- **Enables**: Spec 225 (Filter Transparency), Spec 226 (Composable Filter Pipeline)

## Testing Strategy

### Unit Tests (Predicates)

```rust
#[cfg(test)]
mod predicate_tests {
    use super::*;

    #[test]
    fn test_is_god_object() {
        assert!(is_god_object(&DebtType::GodObject));
        assert!(is_god_object(&DebtType::GodModule));
        assert!(!is_god_object(&DebtType::TestingGap));
    }

    #[test]
    fn test_has_extreme_cyclomatic() {
        let high = ItemMetrics { effective_cyclomatic: 51, ..default() };
        assert!(has_extreme_cyclomatic(&high));

        let low = ItemMetrics { effective_cyclomatic: 50, ..default() };
        assert!(!has_extreme_cyclomatic(&low));
    }

    #[test]
    fn test_has_deep_nesting() {
        let deep = ItemMetrics { nesting_depth: 5, ..default() };
        assert!(has_deep_nesting(&deep));

        let shallow = ItemMetrics { nesting_depth: 4, ..default() };
        assert!(!has_deep_nesting(&shallow));
    }

    #[test]
    fn test_complexity_factor() {
        let metrics = ItemMetrics {
            cyclomatic: 10,
            cognitive: 20,
            ..default()
        };
        // 30% * 10 + 70% * 20 = 3 + 14 = 17
        assert_eq!(complexity_factor(&metrics), 17.0);
    }
}
```

### Integration Tests (Classification)

```rust
#[cfg(test)]
mod classification_tests {
    use super::*;

    #[test]
    fn test_classify_god_object_as_t1() {
        let item = create_test_item(DebtType::GodObject, low_metrics());
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_extreme_complexity_as_t1() {
        let metrics = ItemMetrics {
            effective_cyclomatic: 60,
            ..default()
        };
        let item = create_test_item(DebtType::ComplexityHotspot, metrics);
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_high_complexity_testing_gap_as_t2() {
        let metrics = ItemMetrics {
            cyclomatic: 15,
            ..default()
        };
        let item = create_test_item(DebtType::TestingGap, metrics);
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T2ComplexUntested);
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_classification_deterministic(
        debt_type in any::<DebtType>(),
        metrics in any::<ItemMetrics>(),
    ) {
        let item = create_test_item(debt_type, metrics);
        let tier1 = classify_tier(&item, &TierConfig::default());
        let tier2 = classify_tier(&item, &TierConfig::default());
        prop_assert_eq!(tier1, tier2);
    }

    #[test]
    fn test_god_objects_always_t1(
        metrics in any::<ItemMetrics>(),
    ) {
        let item = create_test_item(DebtType::GodObject, metrics);
        let tier = classify_tier(&item, &TierConfig::default());
        prop_assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_extreme_complexity_always_t1_or_t2(
        debt_type in any::<DebtType>(),
    ) {
        let metrics = ItemMetrics {
            effective_cyclomatic: 100,
            cognitive: 50,
            ..default()
        };
        let item = create_test_item(debt_type, metrics);
        let tier = classify_tier(&item, &TierConfig::default());
        prop_assert!(
            tier == RecommendationTier::T1CriticalArchitecture
            || tier == RecommendationTier::T2ComplexUntested
        );
    }
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Checks if item has extreme cyclomatic complexity (> 50).
///
/// Pure predicate function - deterministic, no side effects.
/// Can be unit tested independently without mocks.
///
/// # Arguments
///
/// * `metrics` - Item metrics containing cyclomatic complexity
///
/// # Returns
///
/// `true` if effective cyclomatic complexity exceeds 50, `false` otherwise
///
/// # Examples
///
/// ```
/// let high = ItemMetrics { effective_cyclomatic: 51, ..default() };
/// assert!(has_extreme_cyclomatic(&high));
/// ```
pub fn has_extreme_cyclomatic(metrics: &ItemMetrics) -> bool {
    metrics.effective_cyclomatic > 50
}
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Tier Classification Architecture

### Pure Functional Design

Tier classification follows Stillwater's "Pure Core, Imperative Shell" principle:

**Predicate Layer** (`predicates.rs`):
- Small, pure boolean functions (2-5 lines each)
- Each tests ONE condition
- No side effects, fully deterministic
- 100% independently testable

**Composition Layer** (`pure.rs`):
- Composes predicates into tier checks
- Pure functions (5-10 lines each)
- Clear boolean logic
- Easy to reason about

**Public API** (`mod.rs`):
- Exposes `classify_tier()` function
- Delegates to pure composition layer

### Example

```rust
// Predicate (pure, 2 lines)
fn has_extreme_cyclomatic(m: &ItemMetrics) -> bool {
    m.effective_cyclomatic > 50
}

// Composition (pure, 5 lines)
fn is_t1_architectural(item: &Item) -> bool {
    is_god_object(&item.debt_type)
        || has_extreme_cyclomatic(&item.metrics)
        || has_extreme_cognitive(&item.metrics)
}

// Public API (pure, delegates)
pub fn classify_tier(item: &Item) -> Tier {
    if is_t1_architectural(item) { Tier::T1 }
    else if is_t2_complex(item) { Tier::T2 }
    else { Tier::T4 }
}
```

### Benefits

- **Testability**: Each predicate tested independently
- **Clarity**: Named predicates self-document logic
- **Maintainability**: Easy to add/modify predicates
- **Composability**: Build complex checks from simple pieces
```

## Implementation Notes

### Refactoring Workflow

1. Create new module files (`pure.rs`, `predicates.rs`)
2. Extract simplest predicates first (god object, testing gap checks)
3. Add unit tests for each predicate as you extract
4. Extract composition functions
5. Update `mod.rs` to re-export pure functions
6. Run tests continuously to ensure behavior preserved
7. Delete old imperative implementations
8. Add documentation

### Common Pitfalls

1. **Over-extraction** - Don't create predicates for trivial checks (e.g., `is_true(x: bool)`)
2. **Poor naming** - Predicate names must clearly describe what they check
3. **Breaking invariants** - Ensure composed predicates maintain same logic as original
4. **Forgetting tests** - Add test for each predicate BEFORE deleting old code

## Migration and Compatibility

### Breaking Changes

**None** - Internal refactoring only. Public API (`classify_tier()`) unchanged.

### Migration Steps

No user or developer migration needed. Internal improvement only.

## Success Metrics

- ✅ All predicate functions < 10 lines
- ✅ All functions cyclomatic complexity < 5
- ✅ 95%+ test coverage for predicates
- ✅ All existing integration tests pass
- ✅ No clippy warnings
- ✅ Performance benchmarks show no regression (< 5% variance)
- ✅ Code review confirms improved readability

## References

- **Stillwater PHILOSOPHY.md** - Pure Core, Imperative Shell principle
- **CLAUDE.md** - Function design guidelines (max 20 lines, complexity < 5)
- **Stillwater Evaluation** - Composition Over Complexity section
- **Spec 187** - Extract Pure Analyzer Functions (similar pattern)
