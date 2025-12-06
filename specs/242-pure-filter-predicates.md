---
number: 242
title: Pure Filter Predicates with Telemetry
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-01-06
---

# Specification 242: Pure Filter Predicates with Telemetry

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current `add_item` function in `src/priority/unified_analysis_utils.rs` is a 53-line monolithic function that mixes multiple concerns:
- Score threshold filtering
- Risk threshold filtering
- Complexity threshold filtering with exemptions
- Duplicate detection
- Item storage

This creates several problems:
- **Invisible filtering**: No visibility into why items are filtered out
- **Untestable**: Can't test each filter in isolation
- **Single responsibility violation**: One function does too much
- **Hard to debug**: No telemetry on filtering behavior
- **Exemption complexity**: Type-based exemptions buried in conditionals

The god object bug was partly hidden because we had no visibility into filtering statistics. We didn't know that god objects were being filtered out.

## Objective

Refactor `add_item` into pure, composable predicate functions (<10 lines each) with comprehensive telemetry. Each filter concern should be a separate, testable function. Add `FilterStatistics` to track what gets filtered and why.

This follows functional programming principles:
- **Pure functions**: Each predicate is deterministic, no side effects
- **Single responsibility**: Each function does one thing
- **Composability**: Filters can be combined and reused
- **Testability**: Easy to test each filter independently

## Requirements

### Functional Requirements

**FR1**: Extract pure score threshold predicate
- Function: `meets_score_threshold(item: &UnifiedDebtItem, min_score: f64) -> bool`
- Returns `true` if `item.unified_score.final_score >= min_score`
- Pure function: no side effects, deterministic
- <10 lines of code

**FR2**: Extract pure risk threshold predicate
- Function: `meets_risk_threshold(item: &UnifiedDebtItem, min_risk: f64) -> bool`
- Returns `true` if non-risk item OR risk score >= threshold
- Handles `DebtType::Risk` variant specifically
- Pure function: no side effects, deterministic
- <10 lines of code

**FR3**: Extract pure complexity exemption predicate
- Function: `is_exempt_from_complexity_filter(item: &UnifiedDebtItem) -> bool`
- Returns `true` for test types and god object types
- Uses `matches!` macro for clean pattern matching
- Pure function: no side effects, deterministic
- <10 lines of code

**FR4**: Extract pure complexity threshold predicate
- Function: `meets_complexity_thresholds(item: &UnifiedDebtItem, min_cyc: u32, min_cog: u32) -> bool`
- Returns `true` if exempt OR meets both thresholds
- Composes with `is_exempt_from_complexity_filter`
- Pure function: no side effects, deterministic
- <10 lines of code

**FR5**: Extract pure duplicate detection predicate
- Function: `is_duplicate_of(item: &UnifiedDebtItem, existing: &UnifiedDebtItem) -> bool`
- Returns `true` if same location and same debt type discriminant
- Pure function: no side effects, deterministic
- <10 lines of code

**FR6**: Create `FilterStatistics` struct
- Tracks counts for each filter rejection reason
- Fields: `total_items_processed`, `filtered_by_score`, `filtered_by_risk`, `filtered_by_complexity`, `filtered_as_duplicate`, `items_added`
- Implements `Debug`, `Clone`, `Default`
- Serializable for JSON output

**FR7**: Update `add_item` to use predicates and track statistics
- Call each predicate in sequence
- Track which filter rejected the item
- Update statistics for debugging
- Maximum 20 lines (down from 53)
- I/O at boundary (config loading), pure logic in predicates

**FR8**: Expose filter statistics for debugging
- Method: `UnifiedAnalysis::filter_statistics() -> &FilterStatistics`
- Method: `UnifiedAnalysis::log_filter_summary()` - Print stats if `DEBTMAP_SHOW_FILTER_STATS` set
- Statistics available for troubleshooting

### Non-Functional Requirements

**NFR1**: Testability
- Each predicate testable in isolation (unit tests)
- Statistics testable independently
- No hidden state or side effects

**NFR2**: Performance
- No performance regression vs current implementation
- Inline predicates for zero overhead
- Statistics tracking has negligible cost

**NFR3**: Debuggability
- Filter statistics reveal why items filtered
- Can enable verbose logging via environment variable
- Statistics included in JSON output if requested

**NFR4**: Maintainability
- Each function <10 lines (current: 53 lines)
- Pure functions easy to understand
- Single responsibility per function

## Acceptance Criteria

- [ ] **AC1**: `meets_score_threshold` predicate implemented
  - Located in `src/priority/filter_predicates.rs`
  - Pure function, no side effects
  - <10 lines of code
  - Unit tests verify correct filtering

- [ ] **AC2**: `meets_risk_threshold` predicate implemented
  - Handles `DebtType::Risk` variant
  - Non-risk items always pass
  - <10 lines of code
  - Unit tests verify correct filtering

- [ ] **AC3**: `is_exempt_from_complexity_filter` predicate implemented
  - Uses `matches!` for clean pattern matching
  - Exempts test types: `TestComplexityHotspot`, `TestTodo`, `TestDuplication`
  - Exempts architectural types: `GodObject`, `GodModule`
  - <10 lines of code
  - Unit tests verify all exemptions

- [ ] **AC4**: `meets_complexity_thresholds` predicate implemented
  - Composes with exemption predicate
  - Checks both cyclomatic and cognitive complexity
  - <10 lines of code
  - Unit tests verify exemptions and thresholds

- [ ] **AC5**: `is_duplicate_of` predicate implemented
  - Checks location (file + line) equality
  - Checks debt type discriminant equality
  - <10 lines of code
  - Unit tests verify duplicate detection logic

- [ ] **AC6**: `FilterStatistics` struct defined
  - Located in `src/priority/filter_predicates.rs`
  - Implements `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`
  - Has field for each rejection reason
  - Can be pretty-printed for debugging

- [ ] **AC7**: `add_item` refactored to use predicates
  - Maximum 20 lines (down from 53)
  - Calls predicates in sequence
  - Tracks statistics for each rejection
  - No logic duplication with predicates

- [ ] **AC8**: `UnifiedAnalysis` exposes filter statistics
  - `filter_statistics()` method returns statistics
  - `log_filter_summary()` prints if `DEBTMAP_SHOW_FILTER_STATS=1`
  - Statistics included in serialized output

- [ ] **AC9**: Unit tests for all predicates
  - Test each predicate independently
  - Test edge cases (boundary values)
  - Test exemptions work correctly
  - 100% coverage of predicate logic

- [ ] **AC10**: Integration test verifies statistics accuracy
  - Test: Add items that should be filtered
  - Verify: Statistics count correctly
  - Verify: Each filter tracked independently
  - Verify: Can identify why specific item filtered

- [ ] **AC11**: No performance regression
  - Benchmark shows identical performance
  - Predicates inline to zero cost
  - Statistics tracking has <1% overhead

- [ ] **AC12**: Documentation complete
  - Module-level docs explain filtering architecture
  - Each predicate has rustdoc with examples
  - Statistics struct documented
  - Usage examples for debugging

## Technical Details

### Implementation Approach

**Phase 1: Create Filter Predicates Module**

Create `src/priority/filter_predicates.rs`:

```rust
//! Pure predicate functions for filtering debt items.
//!
//! This module provides composable, testable predicates for filtering
//! `UnifiedDebtItem` instances. Each predicate is a pure function that
//! takes an item and configuration, returning a boolean.
//!
//! # Design Principles
//!
//! - **Pure functions**: No side effects, deterministic output
//! - **Single responsibility**: Each predicate checks one thing
//! - **Composability**: Predicates can be combined
//! - **Testability**: Easy to unit test in isolation
//!
//! # Examples
//!
//! ```rust
//! use debtmap::priority::filter_predicates::*;
//! use debtmap::priority::UnifiedDebtItem;
//!
//! let item = create_test_item(score: 50.0);
//!
//! // Check individual predicates
//! assert!(meets_score_threshold(&item, 10.0));
//! assert!(!meets_score_threshold(&item, 75.0));
//!
//! // Combine predicates
//! let passes = meets_score_threshold(&item, 10.0)
//!     && meets_complexity_thresholds(&item, 2, 5);
//! ```

use crate::priority::{DebtType, UnifiedDebtItem};
use serde::{Deserialize, Serialize};

/// Tracks filtering statistics for debugging and telemetry.
///
/// These statistics help identify why items are being filtered out,
/// which is crucial for debugging filtering issues.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterStatistics {
    /// Total number of items processed (attempted to add)
    pub total_items_processed: usize,

    /// Items filtered due to score below threshold
    pub filtered_by_score: usize,

    /// Items filtered due to risk score below threshold
    pub filtered_by_risk: usize,

    /// Items filtered due to complexity below threshold
    pub filtered_by_complexity: usize,

    /// Items filtered as duplicates
    pub filtered_as_duplicate: usize,

    /// Items successfully added
    pub items_added: usize,
}

impl FilterStatistics {
    /// Create a new empty statistics tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total items filtered (all rejection reasons).
    pub fn total_filtered(&self) -> usize {
        self.filtered_by_score
            + self.filtered_by_risk
            + self.filtered_by_complexity
            + self.filtered_as_duplicate
    }

    /// Get acceptance rate (percentage added vs processed).
    pub fn acceptance_rate(&self) -> f64 {
        if self.total_items_processed == 0 {
            return 0.0;
        }
        (self.items_added as f64 / self.total_items_processed as f64) * 100.0
    }
}

/// Check if item meets minimum score threshold.
///
/// # Examples
///
/// ```rust
/// let high_score_item = create_item_with_score(50.0);
/// let low_score_item = create_item_with_score(2.0);
///
/// assert!(meets_score_threshold(&high_score_item, 10.0));
/// assert!(!meets_score_threshold(&low_score_item, 10.0));
/// ```
#[inline]
pub fn meets_score_threshold(item: &UnifiedDebtItem, min_score: f64) -> bool {
    item.unified_score.final_score >= min_score
}

/// Check if item meets minimum risk threshold.
///
/// Non-risk items always pass this check. For risk items,
/// the risk score must be >= min_risk.
///
/// # Examples
///
/// ```rust
/// let risk_item = create_risk_item(risk_score: 0.8);
/// let normal_item = create_complexity_item();
///
/// assert!(meets_risk_threshold(&risk_item, 0.5));
/// assert!(!meets_risk_threshold(&risk_item, 0.9));
/// assert!(meets_risk_threshold(&normal_item, 0.9)); // Non-risk always passes
/// ```
#[inline]
pub fn meets_risk_threshold(item: &UnifiedDebtItem, min_risk: f64) -> bool {
    match &item.debt_type {
        DebtType::Risk { risk_score, .. } => *risk_score >= min_risk,
        _ => true, // Non-risk items pass by default
    }
}

/// Check if item is exempt from complexity filtering.
///
/// Exempted types:
/// - Test-related: `TestComplexityHotspot`, `TestTodo`, `TestDuplication`
/// - Architectural: `GodObject`, `GodModule`
///
/// These types have different complexity characteristics and are
/// evaluated by other criteria.
///
/// # Examples
///
/// ```rust
/// let god_object = create_god_object_item();
/// let test_item = create_test_complexity_item();
/// let regular_item = create_complexity_hotspot();
///
/// assert!(is_exempt_from_complexity_filter(&god_object));
/// assert!(is_exempt_from_complexity_filter(&test_item));
/// assert!(!is_exempt_from_complexity_filter(&regular_item));
/// ```
#[inline]
pub fn is_exempt_from_complexity_filter(item: &UnifiedDebtItem) -> bool {
    matches!(
        item.debt_type,
        DebtType::TestComplexityHotspot { .. }
            | DebtType::TestTodo { .. }
            | DebtType::TestDuplication { .. }
            | DebtType::GodObject { .. }
            | DebtType::GodModule { .. }
    )
}

/// Check if item meets minimum complexity thresholds.
///
/// Exempt items (tests, god objects) always pass. Other items must
/// meet BOTH cyclomatic and cognitive complexity thresholds.
///
/// # Examples
///
/// ```rust
/// let complex_item = create_item(cyclomatic: 10, cognitive: 15);
/// let simple_item = create_item(cyclomatic: 1, cognitive: 2);
/// let exempt_item = create_god_object_item(cyclomatic: 0, cognitive: 0);
///
/// assert!(meets_complexity_thresholds(&complex_item, 5, 10));
/// assert!(!meets_complexity_thresholds(&simple_item, 5, 10));
/// assert!(meets_complexity_thresholds(&exempt_item, 5, 10)); // Exempt
/// ```
#[inline]
pub fn meets_complexity_thresholds(
    item: &UnifiedDebtItem,
    min_cyclomatic: u32,
    min_cognitive: u32,
) -> bool {
    if is_exempt_from_complexity_filter(item) {
        return true;
    }

    item.cyclomatic_complexity >= min_cyclomatic && item.cognitive_complexity >= min_cognitive
}

/// Check if two items are duplicates.
///
/// Items are duplicates if they have:
/// 1. Same file path
/// 2. Same line number
/// 3. Same debt type (by discriminant, not value)
///
/// # Examples
///
/// ```rust
/// let item1 = create_item("file.rs", 10, DebtType::ComplexityHotspot);
/// let item2 = create_item("file.rs", 10, DebtType::ComplexityHotspot);
/// let item3 = create_item("file.rs", 10, DebtType::GodObject);
/// let item4 = create_item("file.rs", 20, DebtType::ComplexityHotspot);
///
/// assert!(is_duplicate_of(&item2, &item1)); // Same location, same type
/// assert!(!is_duplicate_of(&item3, &item1)); // Same location, different type
/// assert!(!is_duplicate_of(&item4, &item1)); // Different location
/// ```
#[inline]
pub fn is_duplicate_of(item: &UnifiedDebtItem, existing: &UnifiedDebtItem) -> bool {
    existing.location.file == item.location.file
        && existing.location.line == item.location.line
        && std::mem::discriminant(&existing.debt_type) == std::mem::discriminant(&item.debt_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{ImpactMetrics, Location, UnifiedScore};
    use std::path::PathBuf;

    fn create_test_item(
        score: f64,
        cyclomatic: u32,
        cognitive: u32,
        debt_type: DebtType,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_fn".to_string(),
                line: 10,
            },
            debt_type,
            unified_score: UnifiedScore {
                final_score: score,
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            // ... other fields with defaults
        }
    }

    #[test]
    fn score_threshold_filters_correctly() {
        let high = create_test_item(50.0, 5, 5, DebtType::ComplexityHotspot { cyclomatic: 5, cognitive: 5, adjusted_cyclomatic: None });
        let low = create_test_item(2.0, 5, 5, DebtType::ComplexityHotspot { cyclomatic: 5, cognitive: 5, adjusted_cyclomatic: None });

        assert!(meets_score_threshold(&high, 10.0));
        assert!(!meets_score_threshold(&low, 10.0));
    }

    #[test]
    fn risk_threshold_filters_risk_items() {
        let risk = create_test_item(10.0, 5, 5, DebtType::Risk { risk_score: 0.8, reason: "test".to_string() });

        assert!(meets_risk_threshold(&risk, 0.5));
        assert!(!meets_risk_threshold(&risk, 0.9));
    }

    #[test]
    fn risk_threshold_passes_non_risk_items() {
        let normal = create_test_item(10.0, 5, 5, DebtType::ComplexityHotspot { cyclomatic: 5, cognitive: 5, adjusted_cyclomatic: None });

        assert!(meets_risk_threshold(&normal, 100.0)); // Always passes
    }

    #[test]
    fn god_objects_exempt_from_complexity() {
        let god_object = create_test_item(
            50.0,
            0, // Below threshold
            0, // Below threshold
            DebtType::GodObject {
                methods: 50,
                fields: 20,
                responsibilities: 10,
                god_object_score: 85.0,
            },
        );

        assert!(is_exempt_from_complexity_filter(&god_object));
        assert!(meets_complexity_thresholds(&god_object, 2, 5));
    }

    #[test]
    fn test_types_exempt_from_complexity() {
        let test_item = create_test_item(
            20.0,
            1,
            1,
            DebtType::TestComplexityHotspot {
                cyclomatic: 1,
                cognitive: 1,
                adjusted_cyclomatic: None,
            },
        );

        assert!(is_exempt_from_complexity_filter(&test_item));
        assert!(meets_complexity_thresholds(&test_item, 5, 10));
    }

    #[test]
    fn complexity_requires_both_thresholds() {
        let high_cyc = create_test_item(20.0, 10, 2, DebtType::ComplexityHotspot { cyclomatic: 10, cognitive: 2, adjusted_cyclomatic: None });
        let high_cog = create_test_item(20.0, 2, 10, DebtType::ComplexityHotspot { cyclomatic: 2, cognitive: 10, adjusted_cyclomatic: None });
        let both_high = create_test_item(20.0, 10, 10, DebtType::ComplexityHotspot { cyclomatic: 10, cognitive: 10, adjusted_cyclomatic: None });

        assert!(!meets_complexity_thresholds(&high_cyc, 5, 5)); // Cognitive too low
        assert!(!meets_complexity_thresholds(&high_cog, 5, 5)); // Cyclomatic too low
        assert!(meets_complexity_thresholds(&both_high, 5, 5)); // Both pass
    }

    #[test]
    fn duplicate_detection_checks_location_and_type() {
        let item1 = create_test_item(10.0, 5, 5, DebtType::ComplexityHotspot { cyclomatic: 5, cognitive: 5, adjusted_cyclomatic: None });
        let item2 = create_test_item(10.0, 5, 5, DebtType::ComplexityHotspot { cyclomatic: 10, cognitive: 10, adjusted_cyclomatic: None }); // Different values, same type
        let item3 = create_test_item(10.0, 5, 5, DebtType::GodObject { methods: 50, fields: 20, responsibilities: 10, god_object_score: 85.0 });

        assert!(is_duplicate_of(&item2, &item1)); // Same location + type discriminant
        assert!(!is_duplicate_of(&item3, &item1)); // Same location, different type
    }

    #[test]
    fn filter_statistics_calculates_totals() {
        let mut stats = FilterStatistics::new();
        stats.total_items_processed = 100;
        stats.filtered_by_score = 20;
        stats.filtered_by_complexity = 30;
        stats.filtered_as_duplicate = 10;
        stats.items_added = 40;

        assert_eq!(stats.total_filtered(), 60);
        assert_eq!(stats.acceptance_rate(), 40.0);
    }
}
```

**Phase 2: Update UnifiedAnalysis**

```rust
// src/priority/unified_analysis_utils.rs

use crate::priority::filter_predicates::*;

impl UnifiedAnalysisUtils for UnifiedAnalysis {
    fn add_item(&mut self, item: UnifiedDebtItem) {
        self.stats.total_items_processed += 1;

        // Get thresholds from configuration (I/O boundary)
        let min_score = crate::config::get_minimum_debt_score();
        let min_cyclomatic = crate::config::get_minimum_cyclomatic_complexity();
        let min_cognitive = crate::config::get_minimum_cognitive_complexity();
        let min_risk = crate::config::get_minimum_risk_score();

        // Apply filters using pure predicates
        if !meets_score_threshold(&item, min_score) {
            self.stats.filtered_by_score += 1;
            return;
        }

        if !meets_risk_threshold(&item, min_risk) {
            self.stats.filtered_by_risk += 1;
            return;
        }

        if !meets_complexity_thresholds(&item, min_cyclomatic, min_cognitive) {
            self.stats.filtered_by_complexity += 1;
            return;
        }

        if self.items.iter().any(|existing| is_duplicate_of(&item, existing)) {
            self.stats.filtered_as_duplicate += 1;
            return;
        }

        // Item passed all filters
        self.items.push_back(item);
        self.stats.items_added += 1;
    }
}

// Add statistics access methods
impl UnifiedAnalysis {
    /// Get filtering statistics for debugging.
    pub fn filter_statistics(&self) -> &FilterStatistics {
        &self.stats
    }

    /// Log filter summary if DEBTMAP_SHOW_FILTER_STATS is set.
    pub fn log_filter_summary(&self) {
        if std::env::var("DEBTMAP_SHOW_FILTER_STATS").is_ok() {
            let stats = self.filter_statistics();
            eprintln!("\n=== Filter Statistics ===");
            eprintln!("Total processed: {}", stats.total_items_processed);
            eprintln!("Items added: {}", stats.items_added);
            eprintln!("Acceptance rate: {:.1}%", stats.acceptance_rate());
            eprintln!("\nRejection reasons:");
            eprintln!("  Score threshold: {}", stats.filtered_by_score);
            eprintln!("  Risk threshold: {}", stats.filtered_by_risk);
            eprintln!("  Complexity threshold: {}", stats.filtered_by_complexity);
            eprintln!("  Duplicates: {}", stats.filtered_as_duplicate);
        }
    }
}

// Update struct to include statistics
pub struct UnifiedAnalysis {
    pub items: Vector<UnifiedDebtItem>,
    pub file_items: Vector<FileDebtItem>,
    // ... other fields
    #[serde(skip)]
    pub stats: FilterStatistics, // New field
}
```

**Phase 3: Add Integration Test**

```rust
// tests/filter_statistics_test.rs

#[test]
fn filter_statistics_track_rejections_accurately() {
    let mut analysis = UnifiedAnalysis::empty();

    // Create items that will be filtered for different reasons
    let low_score = create_test_item(score: 0.5, cyc: 10, cog: 10);
    let low_complexity = create_test_item(score: 50.0, cyc: 1, cog: 1);
    let duplicate = create_test_item(score: 50.0, cyc: 10, cog: 10);
    let duplicate_again = create_test_item(score: 50.0, cyc: 10, cog: 10);
    let good_item = create_test_item(score: 50.0, cyc: 10, cog: 10);

    // Add items
    analysis.add_item(low_score);
    analysis.add_item(low_complexity);
    analysis.add_item(duplicate);
    analysis.add_item(duplicate_again); // Should be filtered as duplicate
    analysis.add_item(good_item);

    // Verify statistics
    let stats = analysis.filter_statistics();
    assert_eq!(stats.total_items_processed, 5);
    assert_eq!(stats.filtered_by_score, 1);
    assert_eq!(stats.filtered_by_complexity, 1);
    assert_eq!(stats.filtered_as_duplicate, 1);
    assert_eq!(stats.items_added, 2); // duplicate and good_item
}
```

### Architecture Changes

**New Module**: `src/priority/filter_predicates.rs`
- Pure predicate functions for filtering
- `FilterStatistics` struct for telemetry
- Comprehensive tests

**Updated Module**: `src/priority/unified_analysis_utils.rs`
- Refactored `add_item` to use predicates
- Added statistics tracking
- Reduced from 53 lines to ~20 lines

**Updated Struct**: `UnifiedAnalysis`
- Added `stats: FilterStatistics` field
- Added `filter_statistics()` accessor
- Added `log_filter_summary()` method

### Data Structures

```rust
pub struct FilterStatistics {
    pub total_items_processed: usize,
    pub filtered_by_score: usize,
    pub filtered_by_risk: usize,
    pub filtered_by_complexity: usize,
    pub filtered_as_duplicate: usize,
    pub items_added: usize,
}
```

### APIs and Interfaces

**Public API**:
```rust
// Import predicates
use debtmap::priority::filter_predicates::*;

// Use predicates
if meets_score_threshold(&item, 10.0) && meets_complexity_thresholds(&item, 2, 5) {
    // Item passes filters
}

// Access statistics
let stats = analysis.filter_statistics();
println!("Acceptance rate: {:.1}%", stats.acceptance_rate());

// Debug logging
analysis.log_filter_summary(); // Prints if DEBTMAP_SHOW_FILTER_STATS=1
```

## Dependencies

### Prerequisites
- None

### Affected Components
- `src/priority/unified_analysis_utils.rs` - Refactored `add_item`
- `src/priority/mod.rs` - Updated `UnifiedAnalysis` struct
- `src/commands/analyze.rs` - Can call `log_filter_summary()`

### External Dependencies
None

## Testing Strategy

### Unit Tests
- Test each predicate independently
- Test edge cases and boundary values
- Test exemptions work correctly
- Test statistics calculations

### Integration Tests
- Test end-to-end filtering with statistics
- Test multiple items with different rejection reasons
- Test statistics accuracy
- Test god objects pass through correctly

### Performance Tests
- Benchmark vs old implementation
- Verify inline optimization works
- Measure statistics overhead (<1%)

## Documentation Requirements

### Code Documentation
- Module-level docs for `filter_predicates.rs`
- Rustdoc for each predicate with examples
- Document `FilterStatistics` usage
- Explain when to use statistics for debugging

### User Documentation
- Document `DEBTMAP_SHOW_FILTER_STATS` environment variable
- Explain how to debug filtering issues
- Show example output of filter statistics

### Architecture Updates
Update `ARCHITECTURE.md`:
- Add section on "Filtering Architecture"
- Document pure predicate pattern
- Explain telemetry approach

## Implementation Notes

### Pure Functions
All predicates are pure:
- No mutations
- No I/O
- Deterministic output
- Easy to test

### Inline Optimization
Mark predicates with `#[inline]` to ensure zero overhead:
- Compiler will inline these simple functions
- No function call overhead
- Identical performance to current code

### Composability
Predicates can be combined:
```rust
fn passes_all_filters(item: &UnifiedDebtItem, config: &FilterConfig) -> bool {
    meets_score_threshold(item, config.min_score)
        && meets_risk_threshold(item, config.min_risk)
        && meets_complexity_thresholds(item, config.min_cyc, config.min_cog)
}
```

### Testing Strategy
Each predicate is <10 lines, making unit tests trivial:
- Test true case
- Test false case
- Test boundary
- Test exemptions

## Migration and Compatibility

### Breaking Changes
- `UnifiedAnalysis` struct gains `stats` field (non-breaking, can default)

### Migration Steps
1. Create `filter_predicates.rs` module
2. Add `FilterStatistics` to `UnifiedAnalysis`
3. Refactor `add_item` to use predicates
4. Add unit tests for predicates
5. Add integration tests for statistics
6. Update analyze command to call `log_filter_summary()`

### Backward Compatibility
- Existing tests continue to work
- Statistics field can default to empty
- No API changes to `add_item` signature

### Rollback Plan
If issues:
1. Revert `add_item` to monolithic version
2. Keep predicates module (useful for future)
3. Remove statistics field

## Success Metrics

- [ ] All predicates <10 lines
- [ ] `add_item` <20 lines (down from 53)
- [ ] 100% test coverage on predicates
- [ ] No performance regression
- [ ] Filter statistics reveal why items filtered
- [ ] Can debug god object filtering easily
