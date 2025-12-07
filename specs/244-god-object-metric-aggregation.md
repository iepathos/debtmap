---
number: 244
title: God Object Metric Aggregation from Member Functions
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-01-06
---

# Specification 244: God Object Metric Aggregation from Member Functions

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

God objects currently show placeholder values for metrics that exist on their member functions:
- **Complexity**: `cyclomatic=0, cognitive=0, nesting=0` (should aggregate from functions)
- **Coverage**: `None` (should show weighted average of member functions)
- **Dependencies**: `upstream=0, downstream=0` (should aggregate unique dependencies)
- **Git context**: `None` (should aggregate churn, authors, modification recency)
- **Callers/Callees**: Empty vectors (should aggregate unique function references)

This makes god objects appear less problematic than they are:
- A god object with 50 functions averaging complexity=10 shows as complexity=0
- A god object with 30% average coverage shows as "No coverage data"
- A god object called by 20 unique functions shows as "0 dependencies"

The data exists—we have function-level metrics for all member functions. We just need to aggregate it during god object item creation.

## Objective

Aggregate metrics from member functions when creating god object debt items. Use appropriate aggregation strategies for each metric type:
- **Sum** for complexity (total refactoring burden)
- **Weighted average** for coverage (by function length)
- **Unique set** for dependencies (deduplicate)
- **Aggregate** for git context (sum churn, unique authors, max recency)

This will provide meaningful data for prioritization and impact analysis.

## Requirements

### Functional Requirements

**FR1**: Extract member functions during god object creation
- Function: `extract_member_functions(items, file_path) -> Vec<&UnifiedDebtItem>`
- Filters `unified.items` to only functions from this file
- Pure function: no side effects
- <10 lines of code

**FR2**: Aggregate complexity metrics
- Function: `aggregate_complexity_metrics(members) -> (u32, u32, u32)`
- Returns: `(total_cyclomatic, total_cognitive, max_nesting)`
- Strategy: SUM cyclomatic and cognitive, MAX nesting
- Rationale: Sum shows total burden, max shows worst hotspot
- Pure function: <15 lines

**FR3**: Aggregate coverage metrics
- Function: `aggregate_coverage_metrics(members) -> Option<TransitiveCoverage>`
- Strategy: Weighted average by function length
- Formula: `sum(cov * len) / sum(len)` for each coverage field
- Returns aggregated `TransitiveCoverage` with weighted direct/transitive coverage
- Pure function: <20 lines

**FR4**: Aggregate dependency metrics
- Function: `aggregate_dependency_metrics(members) -> (Vec<String>, Vec<String>, usize, usize)`
- Returns: `(unique_callers, unique_callees, upstream_count, downstream_count)`
- Strategy: Unique set with deduplication
- Rationale: Avoid counting same dependency multiple times
- Pure function: <20 lines

**FR5**: Aggregate git context metrics
- Function: `aggregate_git_context(members) -> Option<ContextualRisk>`
- Strategy: Sum churn, unique authors, max recency, avg frequency
- Returns aggregated `ContextualRisk` or None if no members have git data
- Pure function: <20 lines

**FR6**: Compose all aggregations
- Function: `aggregate_god_object_metrics(members) -> GodObjectAggregatedMetrics`
- Calls all individual aggregation functions
- Returns struct with all aggregated metrics
- Pure function: <15 lines (mostly just composition)

**FR7**: Update `create_god_object_debt_item` to accept aggregated metrics
- Add parameter: `aggregated_metrics: GodObjectAggregatedMetrics`
- Use aggregated values instead of 0/None placeholders
- Update `UnifiedDebtItem` fields with aggregated data
- Breaking change to function signature

**FR8**: Extract and aggregate at call site
- In `apply_file_analysis_results`, extract member functions from `unified.items`
- Aggregate metrics before calling `create_god_object_debt_item`
- Pass aggregated metrics to creation function

### Non-Functional Requirements

**NFR1**: Performance
- Aggregation adds <10ms overhead per god object
- Member function extraction is O(n) where n = total items
- No repeated filtering (extract once, aggregate many times)

**NFR2**: Testability
- All aggregation functions pure (no side effects)
- Each aggregation testable in isolation
- Property tests verify aggregation invariants

**NFR3**: Correctness
- Weighted average coverage always in [0.0, 1.0]
- Complexity sums never overflow (use appropriate types)
- Deduplication actually eliminates duplicates

**NFR4**: Maintainability
- Each aggregation function <20 lines
- Single responsibility per function
- Clear, documented aggregation strategies

## Acceptance Criteria

- [ ] **AC1**: `extract_member_functions` implemented
  - Located in `src/priority/god_object_aggregation.rs`
  - Filters items by file path
  - Pure function, <10 lines
  - Unit test verifies correct filtering

- [ ] **AC2**: `aggregate_complexity_metrics` implemented
  - Sums cyclomatic and cognitive complexity
  - Returns max nesting depth
  - Pure function, <15 lines
  - Unit test verifies sum and max logic

- [ ] **AC3**: `aggregate_coverage_metrics` implemented
  - Weighted average by function length
  - Returns `Option<TransitiveCoverage>`
  - Pure function, <20 lines
  - Unit test verifies weighted average calculation

- [ ] **AC4**: `aggregate_dependency_metrics` implemented
  - Deduplicates callers and callees
  - Returns unique sets and counts
  - Pure function, <20 lines
  - Unit test verifies deduplication

- [ ] **AC5**: `aggregate_git_context` implemented
  - Sums churn, unique authors, max recency
  - Returns `Option<ContextualRisk>`
  - Pure function, <20 lines
  - Unit test verifies aggregation logic

- [ ] **AC6**: `aggregate_god_object_metrics` composes all aggregations
  - Calls all individual aggregation functions
  - Returns `GodObjectAggregatedMetrics` struct
  - Pure function, <15 lines
  - Integration test verifies end-to-end

- [ ] **AC7**: `create_god_object_debt_item` updated
  - Accepts `aggregated_metrics` parameter
  - Uses aggregated values in UnifiedDebtItem
  - No longer sets complexity=0, coverage=None
  - Signature change documented

- [ ] **AC8**: Call site updated in `apply_file_analysis_results`
  - Extracts member functions from unified.items
  - Aggregates metrics before god object creation
  - Passes aggregated metrics to create function
  - Integration test verifies metrics populated

- [ ] **AC9**: TUI displays aggregated metrics
  - List view shows complexity, coverage, dependencies
  - Detail view shows all aggregated metrics
  - Metrics are non-zero for god objects with functions
  - Visual test verifies display

- [ ] **AC10**: Unit tests for all aggregation functions
  - Test each aggregation independently
  - Test edge cases (empty members, all zeros)
  - Test weighted average correctness
  - 100% coverage on aggregation logic

- [ ] **AC11**: Property tests verify invariants
  - Complexity sums are >= individual values
  - Weighted coverage always in [0.0, 1.0]
  - Unique sets have no duplicates
  - Aggregation is deterministic

- [ ] **AC12**: Integration test verifies end-to-end
  - Test: Analyze project with god object
  - Verify: God object has cyclomatic > 0
  - Verify: God object has coverage data
  - Verify: God object has dependencies > 0

## Technical Details

### Implementation Approach

**Phase 1: Create Aggregation Module**

Create `src/priority/god_object_aggregation.rs`:

```rust
//! Pure aggregation functions for god object metrics.
//!
//! This module provides composable functions to aggregate metrics from
//! member functions into god object-level metrics.
//!
//! # Aggregation Strategies
//!
//! - **Complexity**: SUM (total burden)
//! - **Coverage**: Weighted average by function length
//! - **Dependencies**: Unique set (deduplicate)
//! - **Git context**: Sum churn, unique authors, max recency
//!
//! # Examples
//!
//! ```rust
//! let members = extract_member_functions(items.iter(), &file_path);
//! let metrics = aggregate_god_object_metrics(&members);
//!
//! assert!(metrics.total_cyclomatic > 0);
//! assert!(metrics.weighted_coverage.is_some());
//! ```

use crate::priority::{TransitiveCoverage, UnifiedDebtItem};
use crate::risk::context::ContextualRisk;
use std::path::Path;

/// Aggregated metrics from member functions.
#[derive(Debug, Clone)]
pub struct GodObjectAggregatedMetrics {
    pub total_cyclomatic: u32,
    pub total_cognitive: u32,
    pub max_nesting_depth: u32,
    pub weighted_coverage: Option<TransitiveCoverage>,
    pub unique_upstream_callers: Vec<String>,
    pub unique_downstream_callees: Vec<String>,
    pub upstream_dependencies: usize,
    pub downstream_dependencies: usize,
    pub aggregated_git_context: Option<ContextualRisk>,
}

/// Extract member functions for a file.
///
/// Pure function that filters items by file path.
#[inline]
pub fn extract_member_functions<'a>(
    items: impl Iterator<Item = &'a UnifiedDebtItem>,
    file_path: &Path,
) -> Vec<&'a UnifiedDebtItem> {
    items.filter(|item| item.location.file == file_path).collect()
}

/// Aggregate complexity: sum cyclomatic/cognitive, max nesting.
pub fn aggregate_complexity_metrics(members: &[&UnifiedDebtItem]) -> (u32, u32, u32) {
    let total_cyclomatic = members.iter().map(|m| m.cyclomatic_complexity).sum();
    let total_cognitive = members.iter().map(|m| m.cognitive_complexity).sum();
    let max_nesting = members.iter().map(|m| m.nesting_depth).max().unwrap_or(0);

    (total_cyclomatic, total_cognitive, max_nesting)
}

/// Aggregate coverage: weighted average by function length.
pub fn aggregate_coverage_metrics(
    members: &[&UnifiedDebtItem],
) -> Option<TransitiveCoverage> {
    let coverages: Vec<_> = members
        .iter()
        .filter_map(|m| m.transitive_coverage.as_ref().map(|c| (c, m.function_length)))
        .collect();

    if coverages.is_empty() {
        return None;
    }

    let total_length: usize = coverages.iter().map(|(_, len)| len).sum();
    if total_length == 0 {
        return None;
    }

    let weighted_direct = coverages
        .iter()
        .map(|(cov, len)| cov.direct * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    let weighted_transitive = coverages
        .iter()
        .map(|(cov, len)| cov.transitive * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    let total_uncovered = coverages.iter().map(|(cov, _)| cov.uncovered_dependencies).sum();

    Some(TransitiveCoverage {
        direct: weighted_direct,
        transitive: weighted_transitive,
        uncovered_dependencies: total_uncovered,
    })
}

/// Aggregate dependencies: unique set deduplication.
pub fn aggregate_dependency_metrics(
    members: &[&UnifiedDebtItem],
) -> (Vec<String>, Vec<String>, usize, usize) {
    use std::collections::HashSet;

    let mut unique_callers: HashSet<String> = HashSet::new();
    let mut unique_callees: HashSet<String> = HashSet::new();

    for item in members {
        unique_callers.extend(item.upstream_callers.iter().cloned());
        unique_callees.extend(item.downstream_callees.iter().cloned());
    }

    let upstream_count = unique_callers.len();
    let downstream_count = unique_callees.len();

    (
        unique_callers.into_iter().collect(),
        unique_callees.into_iter().collect(),
        upstream_count,
        downstream_count,
    )
}

/// Aggregate git context: sum churn, unique authors, max recency.
pub fn aggregate_git_context(members: &[&UnifiedDebtItem]) -> Option<ContextualRisk> {
    let contexts: Vec<_> = members.iter().filter_map(|m| m.contextual_risk.as_ref()).collect();

    if contexts.is_empty() {
        return None;
    }

    let total_churn = contexts.iter().map(|ctx| ctx.churn_score).sum();

    let unique_authors: std::collections::HashSet<_> =
        contexts.iter().flat_map(|ctx| &ctx.authors).cloned().collect();

    let most_recent = contexts.iter().filter_map(|ctx| ctx.last_modified).max();

    let avg_frequency = if !contexts.is_empty() {
        contexts.iter().map(|ctx| ctx.modification_frequency).sum::<f64>()
            / contexts.len() as f64
    } else {
        0.0
    };

    Some(ContextualRisk {
        churn_score: total_churn,
        authors: unique_authors.into_iter().collect(),
        last_modified: most_recent,
        modification_frequency: avg_frequency,
        commit_count: contexts.iter().map(|ctx| ctx.commit_count).sum(),
        recent_changes: contexts.iter().map(|ctx| ctx.recent_changes).max().unwrap_or(0),
        risk_level: contexts.iter().map(|ctx| ctx.risk_level).max().unwrap_or(0.0),
    })
}

/// Aggregate all metrics (composition of above functions).
pub fn aggregate_god_object_metrics(members: &[&UnifiedDebtItem]) -> GodObjectAggregatedMetrics {
    let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(members);
    let weighted_cov = aggregate_coverage_metrics(members);
    let (callers, callees, up_count, down_count) = aggregate_dependency_metrics(members);
    let git_context = aggregate_git_context(members);

    GodObjectAggregatedMetrics {
        total_cyclomatic: total_cyc,
        total_cognitive: total_cog,
        max_nesting_depth: max_nest,
        weighted_coverage: weighted_cov,
        unique_upstream_callers: callers,
        unique_downstream_callees: callees,
        upstream_dependencies: up_count,
        downstream_dependencies: down_count,
        aggregated_git_context: git_context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create test items
    fn create_test_item(
        file: &str,
        cyc: u32,
        cog: u32,
        nest: u32,
        length: usize,
        coverage: Option<f64>,
    ) -> UnifiedDebtItem {
        // ... create minimal UnifiedDebtItem for testing
    }

    #[test]
    fn test_complexity_aggregation_sums_and_maxes() {
        let members = vec![
            create_test_item("file.rs", 5, 10, 2, 50, None),
            create_test_item("file.rs", 10, 15, 5, 100, None),
            create_test_item("file.rs", 15, 20, 3, 75, None),
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(&member_refs);

        assert_eq!(total_cyc, 30); // 5 + 10 + 15
        assert_eq!(total_cog, 45); // 10 + 15 + 20
        assert_eq!(max_nest, 5); // max(2, 5, 3)
    }

    #[test]
    fn test_coverage_weighted_average() {
        let members = vec![
            create_test_item("file.rs", 0, 0, 0, 10, Some(0.8)),
            create_test_item("file.rs", 0, 0, 0, 50, Some(0.2)),
            create_test_item("file.rs", 0, 0, 0, 40, Some(0.5)),
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let cov = aggregate_coverage_metrics(&member_refs).unwrap();

        // (10*0.8 + 50*0.2 + 40*0.5) / 100 = 38/100 = 0.38
        assert!((cov.direct - 0.38).abs() < 0.01);
    }

    #[test]
    fn test_dependencies_deduplicate() {
        // Test that duplicate callers/callees are removed
        // ...
    }
}
```

**Phase 2: Update create_god_object_debt_item**

```rust
// src/builders/unified_analysis.rs

pub fn create_god_object_debt_item(
    file_path: &Path,
    file_metrics: &FileDebtMetrics,
    god_analysis: &crate::organization::GodObjectAnalysis,
    aggregated_metrics: GodObjectAggregatedMetrics, // NEW parameter
) -> UnifiedDebtItem {
    let base_score = god_analysis.god_object_score;
    let tier = if base_score >= 50.0 { 1 } else { 2 };

    // Use aggregated coverage in score calculation
    let coverage_factor = aggregated_metrics
        .weighted_coverage
        .as_ref()
        .map(|cov| (1.0 - cov.direct) * 10.0)
        .unwrap_or(0.0);

    let unified_score = UnifiedScore {
        final_score: base_score,
        complexity_factor: file_metrics.total_complexity as f64 / 10.0,
        coverage_factor, // Use aggregated coverage!
        dependency_factor: calculate_god_object_risk(god_analysis) / 10.0,
        role_multiplier: 1.0,
        base_score: Some(base_score),
        exponential_factor: None,
        risk_boost: None,
        pre_adjustment_score: None,
        adjustment_applied: None,
        purity_factor: None,
        refactorability_factor: None,
        pattern_factor: None,
    };

    // ... determine debt_type, file_name, recommendation, tier_enum ...

    UnifiedDebtItem {
        location: Location {
            file: file_path.to_path_buf(),
            function: file_name.to_string(),
            line: 1,
        },
        debt_type,
        unified_score,
        function_role: FunctionRole::Unknown,
        recommendation,
        expected_impact,
        transitive_coverage: aggregated_metrics.weighted_coverage, // Use aggregated!
        upstream_dependencies: aggregated_metrics.upstream_dependencies, // Use aggregated!
        downstream_dependencies: aggregated_metrics.downstream_dependencies, // Use aggregated!
        upstream_callers: aggregated_metrics.unique_upstream_callers, // Use aggregated!
        downstream_callees: aggregated_metrics.unique_downstream_callees, // Use aggregated!
        nesting_depth: aggregated_metrics.max_nesting_depth, // Use aggregated!
        function_length: god_analysis.lines_of_code,
        cyclomatic_complexity: aggregated_metrics.total_cyclomatic, // Use aggregated!
        cognitive_complexity: aggregated_metrics.total_cognitive, // Use aggregated!
        entropy_details: None,
        entropy_adjusted_cyclomatic: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        god_object_indicators: Some(god_analysis.clone()),
        tier: Some(tier_enum),
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        file_context: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: None,
        contextual_risk: aggregated_metrics.aggregated_git_context, // Use aggregated!
        file_line_count: Some(god_analysis.lines_of_code),
    }
}
```

**Phase 3: Update Call Site**

```rust
// src/builders/unified_analysis.rs

fn apply_file_analysis_results(
    unified: &mut UnifiedAnalysis,
    processed_files: Vec<ProcessedFileData>,
) {
    use crate::priority::god_object_aggregation::*;

    for file_data in processed_files {
        if let Some(god_analysis) = &file_data.god_analysis {
            update_function_god_indicators(unified, &file_data.file_path, god_analysis);

            // Extract member functions from unified.items
            let member_functions = extract_member_functions(
                unified.items.iter(),
                &file_data.file_path,
            );

            // Aggregate metrics from member functions
            let aggregated_metrics = aggregate_god_object_metrics(&member_functions);

            // Create god object item with aggregated metrics
            let god_item = create_god_object_debt_item(
                &file_data.file_path,
                &file_data.file_metrics,
                god_analysis,
                aggregated_metrics, // Pass aggregated metrics!
            );

            unified.add_item(god_item);
        }

        let file_item = create_file_debt_item(file_data);
        unified.add_file_item(file_item);
    }
}
```

### Architecture Changes

**New Module**: `src/priority/god_object_aggregation.rs`
- Pure aggregation functions
- `GodObjectAggregatedMetrics` struct
- Comprehensive unit tests

**Updated Function**: `src/builders/unified_analysis.rs::create_god_object_debt_item`
- Add `aggregated_metrics` parameter (breaking change)
- Use aggregated values instead of 0/None

**Updated Function**: `src/builders/unified_analysis.rs::apply_file_analysis_results`
- Extract member functions
- Aggregate metrics
- Pass to create function

### Data Structures

```rust
pub struct GodObjectAggregatedMetrics {
    pub total_cyclomatic: u32,              // Sum of member complexities
    pub total_cognitive: u32,               // Sum of member complexities
    pub max_nesting_depth: u32,             // Max across members
    pub weighted_coverage: Option<TransitiveCoverage>, // Weighted avg
    pub unique_upstream_callers: Vec<String>,   // Deduplicated
    pub unique_downstream_callees: Vec<String>, // Deduplicated
    pub upstream_dependencies: usize,       // Count of unique callers
    pub downstream_dependencies: usize,     // Count of unique callees
    pub aggregated_git_context: Option<ContextualRisk>, // Aggregated
}
```

### APIs and Interfaces

**Public API**:
```rust
// Extract member functions
let members = extract_member_functions(items.iter(), &file_path);

// Aggregate all metrics
let metrics = aggregate_god_object_metrics(&members);

// Use in god object creation
let god_item = create_god_object_debt_item(
    &file_path,
    &file_metrics,
    &god_analysis,
    metrics,
);
```

## Dependencies

### Prerequisites
- None (independent feature)

### Affected Components
- `src/priority/god_object_aggregation.rs` - New module
- `src/builders/unified_analysis.rs` - Updated creation function
- `src/tui/results/list_view.rs` - Display aggregated metrics
- `src/tui/results/detail_pages/overview.rs` - Display details

### External Dependencies
None

## Testing Strategy

### Unit Tests
- Test each aggregation function independently
- Test edge cases (empty members, all zeros, missing data)
- Test weighted average correctness
- Test deduplication logic
- Test composition in `aggregate_god_object_metrics`

### Property Tests
```rust
proptest! {
    #[test]
    fn complexity_sum_equals_individual_sums(
        cyc_values in prop::collection::vec(0u32..100, 1..50),
    ) {
        let members: Vec<_> = cyc_values.iter().map(|&c| create_item(c, 0, 0)).collect();
        let member_refs: Vec<_> = members.iter().collect();
        let (total, _, _) = aggregate_complexity_metrics(&member_refs);

        prop_assert_eq!(total, cyc_values.iter().sum());
    }

    #[test]
    fn weighted_coverage_in_valid_range(
        data in prop::collection::vec((1usize..1000, 0.0f64..1.0), 1..50),
    ) {
        let members: Vec<_> = data.iter().map(|&(len, cov)| create_item_with_cov(len, cov)).collect();
        let member_refs: Vec<_> = members.iter().collect();

        if let Some(result) = aggregate_coverage_metrics(&member_refs) {
            prop_assert!(result.direct >= 0.0 && result.direct <= 1.0);
        }
    }
}
```

### Integration Tests
```rust
#[test]
fn test_god_object_has_aggregated_metrics() {
    let analysis = analyze_test_project_with_god_object();

    let god_object = analysis
        .items
        .iter()
        .find(|item| matches!(item.debt_type, DebtType::GodObject { .. }))
        .expect("God object should exist");

    // Complexity aggregated
    assert!(god_object.cyclomatic_complexity > 0, "Should have aggregated cyclomatic");
    assert!(god_object.cognitive_complexity > 0, "Should have aggregated cognitive");

    // Coverage aggregated
    assert!(god_object.transitive_coverage.is_some(), "Should have coverage data");

    // Dependencies aggregated
    assert!(god_object.upstream_dependencies > 0, "Should have upstream deps");
    assert!(god_object.downstream_dependencies > 0, "Should have downstream deps");
}
```

## Documentation Requirements

### Code Documentation
- Module-level docs for `god_object_aggregation.rs`
- Rustdoc for each aggregation function
- Examples showing aggregation strategies
- Document why each strategy chosen

### User Documentation
- Update TUI documentation
- Explain aggregated metrics in god object cards
- Show examples of metric interpretation

### Architecture Updates
Update `ARCHITECTURE.md`:
- Add section on "God Object Metric Aggregation"
- Document aggregation strategies
- Explain pure functional approach

## Implementation Notes

### Pure Functions
All aggregation functions are pure:
- `extract_member_functions`: Pure filter
- `aggregate_complexity_metrics`: Pure sum/max
- `aggregate_coverage_metrics`: Pure weighted average
- `aggregate_dependency_metrics`: Pure set operations
- `aggregate_git_context`: Pure aggregation
- `aggregate_god_object_metrics`: Pure composition

### Performance Considerations
- Member extraction is O(n) where n = total items
- Each aggregation is O(m) where m = member count
- Total overhead: O(n + m) per god object
- Expected: <10ms per god object

### Edge Cases
- **No member functions**: Return zeros/None (god object with no functions)
- **Missing coverage**: Return None (some functions have coverage, some don't)
- **Empty callers/callees**: Return empty vectors
- **Division by zero**: Check before weighted average

## Migration and Compatibility

### Breaking Changes
- `create_god_object_debt_item` signature changes (add parameter)
- Call sites must pass `aggregated_metrics`

### Migration Steps
1. Create `god_object_aggregation.rs` module
2. Add aggregation functions with tests
3. Update `create_god_object_debt_item` signature
4. Update call site in `apply_file_analysis_results`
5. Update TUI display logic
6. Run integration tests

### Backward Compatibility
- Existing god objects will show new metrics
- TUI might show previously hidden data
- No config changes needed

### Rollback Plan
If issues:
1. Revert `create_god_object_debt_item` signature
2. Revert call site to pass None/zeros
3. Keep aggregation module (useful for future)

## Success Metrics

- [ ] God objects show cyclomatic > 0 (unless truly no functions)
- [ ] God objects show cognitive > 0 (unless truly no functions)
- [ ] God objects show coverage % when functions have coverage
- [ ] God objects show unique dependency counts
- [ ] God objects show git context when available
- [ ] TUI displays metrics clearly
- [ ] All aggregation functions <20 lines
- [ ] All aggregation functions pure (no side effects)
- [ ] 100% test coverage on aggregations
- [ ] Integration test verifies end-to-end
- [ ] No performance regression (<10ms overhead)
- [ ] Property tests verify invariants
