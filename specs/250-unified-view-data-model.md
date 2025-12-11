---
number: 250
title: Unified View Data Model (PreparedDebtView)
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-10
---

# Specification 250: Unified View Data Model (PreparedDebtView)

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Currently, debtmap has **5 different data paths** that produce inconsistent results:

| Mode | Data Source | Includes File Items? | Groups? | Filters T4? | Score Threshold? |
|------|-------------|---------------------|---------|-------------|-----------------|
| TUI | `analysis.items` directly | No | Yes (summed) | No | No |
| Terminal (--no-tui) | `get_top_mixed_priorities()` | Yes | No | Yes | Yes (3.0) |
| JSON | `get_top_mixed_priorities()` | Yes | No | Yes | Yes (3.0) |
| Markdown | `apply_filters()` + `get_top_mixed_priorities()` | Yes | No | Yes | Yes (3.0) |
| Tiered | `get_top_mixed_priorities_with_metrics()` | Yes | No | Yes | Yes (3.0) |

This causes visible differences between output modes:
- TUI shows different scores than `--no-tui` mode
- Markdown applies double filtering
- TUI never shows file-level items (god objects)
- Grouping only available in TUI

Following the Stillwater philosophy of "Pure Core, Imperative Shell", we need a **single canonical data model** that all output formats consume.

## Objective

Create `PreparedDebtView` - a unified data model that represents the **single source of truth** for how debt items should be displayed across all output formats.

This model will:
1. Combine function and file items into a unified representation
2. Pre-compute grouping for location-based display
3. Include summary statistics
4. Be the **only** input for all output formatters

## Requirements

### Functional Requirements

1. **ViewItem Enum**
   - Unified wrapper for both `UnifiedDebtItem` and `FileDebtItem`
   - Provides common interface for score, location, severity
   - Preserves full item details for detailed views
   - Enables heterogeneous collections

2. **LocationGroup Struct**
   - Pre-computed groups by (file, function, line)
   - Combined score (sum of all items at location)
   - Maximum severity across items
   - List of all items in group
   - Supports both grouped and ungrouped display

3. **ViewSummary Struct**
   - Total item count (before/after filtering)
   - Total debt score
   - Score distribution by severity
   - Category breakdown
   - Filter statistics (items filtered by tier, score threshold)

4. **PreparedDebtView Struct**
   - `items: Vec<ViewItem>` - All items sorted by score
   - `groups: Vec<LocationGroup>` - Pre-computed groups
   - `summary: ViewSummary` - Statistics
   - Immutable once created

5. **Pure Data Model**
   - No I/O operations in types
   - No environment variable access
   - Fully serializable (JSON, etc.)
   - Deterministic construction

### Non-Functional Requirements

1. **Performance**
   - Construction should be O(n log n) where n = items
   - Grouping pre-computed once, not on every render
   - Memory efficient (avoid cloning large items)

2. **Testability**
   - All types easily constructable in tests
   - No external dependencies needed for testing
   - Deterministic behavior

3. **Composability**
   - Works with existing `UnifiedAnalysis`
   - Can be extended for new output formats
   - Clear separation from I/O

## Acceptance Criteria

- [ ] `ViewItem` enum created with `Function` and `File` variants
- [ ] `ViewItem` provides `.score()`, `.location()`, `.severity()` methods
- [ ] `LocationGroup` struct created with combined score calculation
- [ ] `ViewSummary` struct captures all statistics
- [ ] `PreparedDebtView` struct combines items, groups, summary
- [ ] All types implement `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] Unit tests for all type methods
- [ ] No I/O or environment access in any type
- [ ] Types documented with examples

## Technical Details

### Implementation Approach

**File Location**: `src/priority/view.rs`

**Type Definitions**:

```rust
//! Unified view data model for debt display.
//!
//! This module provides the canonical data model that all output formats
//! consume. Following Stillwater's "still pond" principle, this is pure
//! data with no I/O operations.

use crate::priority::{
    classification::Severity,
    file_metrics::FileDebtItem,
    tiers::RecommendationTier,
    unified_scorer::{Location, UnifiedDebtItem},
    DebtCategory,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unified item type wrapping both function and file debt items.
///
/// This enables heterogeneous collections and consistent interfaces
/// across all output formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewItem {
    /// Function-level debt item
    Function(Box<UnifiedDebtItem>),
    /// File-level debt item (god objects, large files)
    File(Box<FileDebtItem>),
}

impl ViewItem {
    /// Returns the debt score for this item.
    pub fn score(&self) -> f64 {
        match self {
            ViewItem::Function(item) => item.unified_score.final_score.value(),
            ViewItem::File(item) => item.score,
        }
    }

    /// Returns the location of this item.
    pub fn location(&self) -> ItemLocation {
        match self {
            ViewItem::Function(item) => ItemLocation {
                file: item.location.file.clone(),
                function: Some(item.location.function.clone()),
                line: Some(item.location.line),
            },
            ViewItem::File(item) => ItemLocation {
                file: item.metrics.path.clone(),
                function: None,
                line: None,
            },
        }
    }

    /// Returns the severity classification for this item.
    pub fn severity(&self) -> Severity {
        Severity::from_score_100(self.score())
    }

    /// Returns the recommendation tier for this item.
    pub fn tier(&self) -> Option<RecommendationTier> {
        match self {
            ViewItem::Function(item) => item.tier,
            ViewItem::File(_) => Some(RecommendationTier::T1Critical), // File items are critical
        }
    }

    /// Returns the debt category for this item.
    pub fn category(&self) -> DebtCategory {
        match self {
            ViewItem::Function(item) => DebtCategory::from_debt_type(&item.debt_type),
            ViewItem::File(_) => DebtCategory::Architecture, // File items are architectural
        }
    }

    /// Returns display type label.
    pub fn display_type(&self) -> &'static str {
        match self {
            ViewItem::Function(_) => "FUNCTION",
            ViewItem::File(_) => "FILE",
        }
    }
}

/// Location information for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ItemLocation {
    pub file: PathBuf,
    pub function: Option<String>,
    pub line: Option<usize>,
}

impl ItemLocation {
    /// Returns grouping key for location-based grouping.
    pub fn group_key(&self) -> (PathBuf, String, usize) {
        (
            self.file.clone(),
            self.function.clone().unwrap_or_default(),
            self.line.unwrap_or(0),
        )
    }
}

/// Pre-computed group of items at the same location.
///
/// Used for TUI grouped view and potential grouped display in other formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationGroup {
    /// Representative location for this group
    pub location: ItemLocation,
    /// All items at this location
    pub items: Vec<ViewItem>,
    /// Combined score (sum of all item scores)
    pub combined_score: f64,
    /// Highest severity among items
    pub max_severity: Severity,
    /// Number of items in group
    pub item_count: usize,
}

impl LocationGroup {
    /// Creates a new group from items at the same location.
    pub fn new(location: ItemLocation, items: Vec<ViewItem>) -> Self {
        let combined_score = items.iter().map(|i| i.score()).sum();
        let max_severity = items
            .iter()
            .map(|i| i.severity())
            .max_by(|a, b| a.rank().cmp(&b.rank()))
            .unwrap_or(Severity::Low);
        let item_count = items.len();

        Self {
            location,
            items,
            combined_score,
            max_severity,
            item_count,
        }
    }
}

/// Summary statistics for the view.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViewSummary {
    /// Total items before filtering
    pub total_items_before_filter: usize,
    /// Total items after filtering
    pub total_items_after_filter: usize,
    /// Items filtered by T4 tier
    pub filtered_by_tier: usize,
    /// Items filtered by score threshold
    pub filtered_by_score: usize,
    /// Total debt score (sum of all item scores)
    pub total_debt_score: f64,
    /// Score distribution by severity
    pub score_distribution: ScoreDistribution,
    /// Items by category
    pub category_counts: CategoryCounts,
    /// Total lines of code analyzed
    pub total_lines_of_code: usize,
    /// Debt density per 1K LOC
    pub debt_density: f64,
    /// Overall coverage if available
    pub overall_coverage: Option<f64>,
}

/// Distribution of items by severity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreDistribution {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

/// Count of items by category.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryCounts {
    pub architecture: usize,
    pub testing: usize,
    pub performance: usize,
    pub code_quality: usize,
}

/// The canonical view model for all output formats.
///
/// This is the **single source of truth** for displaying debt items.
/// All output formats (TUI, terminal, JSON, markdown) consume this
/// same data structure, ensuring consistent results.
///
/// # Stillwater Pattern
///
/// This follows the "still pond" model - a pure data structure with
/// no I/O operations. The "flowing water" (I/O) happens in the output
/// formatters that consume this model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedDebtView {
    /// All items sorted by score (highest first)
    pub items: Vec<ViewItem>,
    /// Pre-computed groups by location
    pub groups: Vec<LocationGroup>,
    /// Summary statistics
    pub summary: ViewSummary,
    /// Configuration used to create this view
    pub config: ViewConfig,
}

/// Configuration for view preparation.
///
/// This captures all the parameters that affect how items are
/// filtered, sorted, and grouped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    /// Minimum score threshold (items below are filtered)
    pub min_score_threshold: f64,
    /// Whether to exclude T4 maintenance tier items
    pub exclude_t4_maintenance: bool,
    /// Maximum number of items (None = unlimited)
    pub limit: Option<usize>,
    /// Sort criteria
    pub sort_by: SortCriteria,
    /// Whether to compute groups
    pub compute_groups: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 3.0,
            exclude_t4_maintenance: true,
            limit: None,
            sort_by: SortCriteria::Score,
            compute_groups: true,
        }
    }
}

/// Sort criteria for items.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SortCriteria {
    #[default]
    Score,
    Coverage,
    Complexity,
    FilePath,
    FunctionName,
}

impl PreparedDebtView {
    /// Returns items suitable for ungrouped display.
    pub fn ungrouped_items(&self) -> &[ViewItem] {
        &self.items
    }

    /// Returns groups suitable for grouped display.
    pub fn grouped_items(&self) -> &[LocationGroup] {
        &self.groups
    }

    /// Returns whether this view has any items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns the number of groups.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
}
```

### Architecture Changes

**Before**:
```
UnifiedAnalysis
    ├── items: Vector<UnifiedDebtItem>      ← TUI uses directly
    ├── file_items: Vector<FileDebtItem>    ← TUI ignores!
    └── get_top_mixed_priorities()          ← Terminal/JSON/Markdown use
            └── Different filtering logic
```

**After**:
```
UnifiedAnalysis
    └── prepare_view(config) → PreparedDebtView
                                    ├── items: Vec<ViewItem>     ← All formats use
                                    ├── groups: Vec<LocationGroup>
                                    └── summary: ViewSummary
```

### Data Structures

| Type | Purpose | Size |
|------|---------|------|
| `ViewItem` | Unified item wrapper | Small (enum, boxed inner) |
| `ItemLocation` | Location for grouping | Small (path + optional strings) |
| `LocationGroup` | Pre-computed group | Medium (contains items) |
| `ViewSummary` | Statistics | Small |
| `ViewConfig` | Filtering/sorting params | Small |
| `PreparedDebtView` | Main view model | Large (contains all data) |

### APIs and Interfaces

**Public Types** (re-exported from `src/priority/mod.rs`):

```rust
pub use view::{
    PreparedDebtView, ViewItem, ViewConfig, ViewSummary,
    LocationGroup, ItemLocation, SortCriteria,
    ScoreDistribution, CategoryCounts,
};
```

## Dependencies

- **Prerequisites**: None (foundation spec)
- **Affected Components**:
  - `src/priority/mod.rs` - Re-exports
  - `src/priority/classification.rs` - Severity ranking method needed
- **External Dependencies**: None (uses existing serde)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_item_score() {
        let func_item = create_test_function_item(50.0);
        let file_item = create_test_file_item(75.0);

        assert_eq!(ViewItem::Function(Box::new(func_item)).score(), 50.0);
        assert_eq!(ViewItem::File(Box::new(file_item)).score(), 75.0);
    }

    #[test]
    fn test_view_item_severity() {
        let critical = create_test_function_item(95.0);
        let high = create_test_function_item(75.0);
        let medium = create_test_function_item(50.0);
        let low = create_test_function_item(25.0);

        assert_eq!(ViewItem::Function(Box::new(critical)).severity(), Severity::Critical);
        assert_eq!(ViewItem::Function(Box::new(high)).severity(), Severity::High);
        assert_eq!(ViewItem::Function(Box::new(medium)).severity(), Severity::Medium);
        assert_eq!(ViewItem::Function(Box::new(low)).severity(), Severity::Low);
    }

    #[test]
    fn test_location_group_combined_score() {
        let items = vec![
            ViewItem::Function(Box::new(create_test_function_item(30.0))),
            ViewItem::Function(Box::new(create_test_function_item(20.0))),
            ViewItem::Function(Box::new(create_test_function_item(10.0))),
        ];
        let location = ItemLocation {
            file: PathBuf::from("test.rs"),
            function: Some("test_fn".to_string()),
            line: Some(10),
        };

        let group = LocationGroup::new(location, items);

        assert_eq!(group.combined_score, 60.0);
        assert_eq!(group.item_count, 3);
    }

    #[test]
    fn test_location_group_max_severity() {
        let items = vec![
            ViewItem::Function(Box::new(create_test_function_item(25.0))),  // Low
            ViewItem::Function(Box::new(create_test_function_item(95.0))),  // Critical
            ViewItem::Function(Box::new(create_test_function_item(50.0))),  // Medium
        ];
        let location = ItemLocation {
            file: PathBuf::from("test.rs"),
            function: Some("test_fn".to_string()),
            line: Some(10),
        };

        let group = LocationGroup::new(location, items);

        assert_eq!(group.max_severity, Severity::Critical);
    }

    #[test]
    fn test_view_config_default() {
        let config = ViewConfig::default();

        assert_eq!(config.min_score_threshold, 3.0);
        assert!(config.exclude_t4_maintenance);
        assert!(config.limit.is_none());
        assert_eq!(config.sort_by, SortCriteria::Score);
        assert!(config.compute_groups);
    }

    #[test]
    fn test_prepared_debt_view_accessors() {
        let view = PreparedDebtView {
            items: vec![
                ViewItem::Function(Box::new(create_test_function_item(50.0))),
            ],
            groups: vec![],
            summary: ViewSummary::default(),
            config: ViewConfig::default(),
        };

        assert!(!view.is_empty());
        assert_eq!(view.len(), 1);
        assert_eq!(view.group_count(), 0);
    }
}
```

### Serialization Tests

```rust
#[test]
fn test_prepared_debt_view_json_roundtrip() {
    let view = create_test_view();

    let json = serde_json::to_string(&view).unwrap();
    let deserialized: PreparedDebtView = serde_json::from_str(&json).unwrap();

    assert_eq!(view.items.len(), deserialized.items.len());
    assert_eq!(view.summary.total_debt_score, deserialized.summary.total_debt_score);
}
```

## Documentation Requirements

### Code Documentation

All types have comprehensive rustdoc with:
- Purpose explanation
- Stillwater pattern context
- Usage examples
- Field descriptions

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Unified View Model

All output formats consume `PreparedDebtView` - a single canonical
data model that ensures consistent results across TUI, terminal,
JSON, and markdown outputs.

### Data Flow

```
UnifiedAnalysis → prepare_view() → PreparedDebtView → OutputFormatter
                       ↑                                    ↓
                  ViewConfig                          Formatted Output
```

### Key Types

- `ViewItem` - Unified wrapper for function/file items
- `LocationGroup` - Pre-computed groups for display
- `ViewSummary` - Statistics and filter metrics
- `PreparedDebtView` - The canonical view model
```

## Implementation Notes

### Refactoring Steps

1. Create `src/priority/view.rs` with all types
2. Add `Severity::rank()` method to `classification.rs` if missing
3. Add re-exports to `src/priority/mod.rs`
4. Write unit tests
5. Verify compilation

### Design Decisions

1. **Box inner items** - Prevents large enum variants, improves cache locality
2. **Pre-compute groups** - Avoids repeated grouping in TUI render loop
3. **Include config in view** - Enables output formats to know how data was filtered
4. **Clone-friendly** - All types derive Clone for flexibility

### Common Pitfalls

1. **Serialization** - Ensure all nested types are serializable
2. **Severity ranking** - Need consistent ordering for max calculation
3. **Empty groups** - Handle edge case of no items

## Migration and Compatibility

### Breaking Changes

**None** - This is a new type that will be added alongside existing code.

### Internal Changes

Existing code unchanged in this spec. Integration happens in spec 251.

## Success Metrics

- All types created and documented
- Unit tests pass
- Types are serializable
- No I/O in any type method
- Clear, consistent API

## Follow-up Work

- Spec 251: View Pipeline (creates PreparedDebtView from UnifiedAnalysis)
- Spec 252: Output Format Unification (update all formatters to use PreparedDebtView)

## References

- **Stillwater PHILOSOPHY.md** - "The Pond Model" (pure data)
- **Spec 183** - Analyzer I/O separation pattern
- **src/priority/mod.rs** - Current UnifiedAnalysis structure
- **src/tui/results/grouping.rs** - Current grouping logic to unify
