# God Object Metric Aggregation Strategy

## Problem Statement

Currently, god objects show placeholder values for metrics that exist on their member functions:
- **Complexity**: cyclomatic=0, cognitive=0, nesting=0 (should aggregate from functions)
- **Coverage**: `None` (should show average of member functions)
- **Dependencies**: upstream=0, downstream=0 (should aggregate unique dependencies)
- **Git context**: `None` (should aggregate churn, authors, recency)
- **Callers/Callees**: empty vectors (should aggregate unique function references)

## Data Flow Analysis

### Current Flow
```
1. Functions analyzed → UnifiedDebtItems created → Added to unified.items
2. File analyzed → GodObjectAnalysis created
3. create_god_object_debt_item(file_path, file_metrics, god_analysis)
   └─> Has access to file_path but NOT to member function metrics
   └─> Sets complexity=0, coverage=None, deps=0
4. God object item added to unified.items
```

### Access Point
At line `unified_analysis.rs:1691`, we call:
```rust
let god_item = create_god_object_debt_item(
    &file_data.file_path,
    &file_data.file_metrics,
    god_analysis,
);
```

**Key insight**: We have `unified` in scope, which contains `unified.items` with all function-level items!

### Proposed Flow
```
1. Functions analyzed → UnifiedDebtItems created → Added to unified.items
2. File analyzed → GodObjectAnalysis created
3. Extract member functions from unified.items for this file_path
4. Aggregate metrics from member functions (pure functions)
5. create_god_object_debt_item(..., aggregated_metrics)
   └─> Uses aggregated complexity, coverage, dependencies
6. God object item with meaningful metrics added to unified.items
```

## Aggregation Strategies

### 1. Complexity Metrics

**Cyclomatic Complexity**: SUM
- **Rationale**: Total decision points across all functions
- **Formula**: `sum(function.cyclomatic_complexity for function in members)`
- **Example**: 3 functions with cyc=5,10,15 → god_object.cyc = 30

**Cognitive Complexity**: SUM
- **Rationale**: Total mental burden across all functions
- **Formula**: `sum(function.cognitive_complexity for function in members)`
- **Example**: 3 functions with cog=8,12,20 → god_object.cog = 40

**Nesting Depth**: MAX
- **Rationale**: Worst-case nesting in any function
- **Formula**: `max(function.nesting_depth for function in members)`
- **Example**: 3 functions with nest=2,5,3 → god_object.nest = 5

**Why not average?**
- SUM represents **total refactoring burden**
- MAX identifies **worst hotspot** within the god object
- Average would dilute the signal (100 functions, avg=5 looks fine, but total=500 is huge)

### 2. Coverage Metrics

**Strategy**: WEIGHTED AVERAGE by function length
- **Rationale**: Longer functions contribute more to overall coverage
- **Formula**:
  ```
  weighted_cov = sum(func.coverage * func.length) / sum(func.length)
  ```
- **Example**:
  ```
  func1: length=10, cov=0.8 → weight=8.0
  func2: length=50, cov=0.2 → weight=10.0
  func3: length=40, cov=0.5 → weight=20.0
  total_weight = 38.0, total_length = 100
  weighted_avg = 38.0 / 100 = 0.38 (38% coverage)
  ```

**Fields to aggregate**:
- `transitive_coverage.direct`: Weighted average
- `transitive_coverage.transitive`: Weighted average
- `transitive_coverage.uncovered_dependencies`: SUM (total uncovered callees)

**Why weighted?**
- Simple average treats 5-line getter same as 100-line algorithm
- Coverage of long functions matters more for risk
- Aligns with how coverage affects refactoring safety

### 3. Dependency Metrics

**Strategy**: UNIQUE SET aggregation
- **Rationale**: Multiple functions might call same upstream/downstream
- **Formula**:
  ```
  upstream_deps = count(unique(union(func.upstream_callers for func in members)))
  downstream_deps = count(unique(union(func.downstream_callees for func in members)))
  ```
- **Example**:
  ```
  func1.upstream_callers = ["main", "init"]
  func2.upstream_callers = ["main", "process"]
  func3.upstream_callers = ["init"]

  god_object.upstream_callers = unique(["main", "init", "process"]) = 3
  ```

**Blast Radius**: upstream + downstream
- Total unique functions that interact with this god object
- Indicates refactoring impact scope

**Why unique set?**
- Avoid double-counting same caller across multiple functions
- Represents true dependency surface area
- Useful for impact analysis

### 4. Git Context Metrics

**Strategy**: Aggregate `ContextualRisk` from all member functions

**Churn Score**: SUM
- **Rationale**: Total file instability
- **Formula**: `sum(func.contextual_risk.churn_score for func in members)`

**Author Count**: UNIQUE SET
- **Rationale**: Total contributors to this file
- **Formula**: `count(unique(union(func.contextual_risk.authors)))`

**Recent Changes**: MAX (most recent)
- **Rationale**: When was file last touched
- **Formula**: `max(func.contextual_risk.last_modified_date)`

**Modification Frequency**: AVERAGE
- **Rationale**: Average change frequency across functions
- **Formula**: `avg(func.contextual_risk.modification_frequency)`

**Example**:
```
func1.contextual_risk:
  churn_score: 10.0
  authors: ["alice", "bob"]
  last_modified: 2025-01-01

func2.contextual_risk:
  churn_score: 15.0
  authors: ["bob", "charlie"]
  last_modified: 2025-01-05

god_object.contextual_risk:
  churn_score: 25.0 (sum)
  authors: ["alice", "bob", "charlie"] (unique)
  last_modified: 2025-01-05 (max)
  modification_frequency: avg of both
```

### 5. Caller/Callee Lists

**Strategy**: UNIQUE SET with deduplication

**Upstream Callers**: Unique set
- All functions that call ANY function in this god object
- Deduplicate by function name
- Limit to top N by call frequency (e.g., top 20)

**Downstream Callees**: Unique set
- All functions called by ANY function in this god object
- Deduplicate by function name
- Limit to top N by call frequency

**Why limit?**
- God objects can have 100+ dependencies
- Top N shows most critical dependencies
- Keeps data manageable for display

**Example**:
```
func1.upstream_callers = ["main", "init"]
func2.upstream_callers = ["main", "process"]
func3.upstream_callers = ["init", "main"] // duplicate "main"

god_object.upstream_callers = ["main", "init", "process"] // deduplicated
```

## Implementation Design

### Pure Aggregation Functions

```rust
// src/priority/god_object_aggregation.rs

/// Aggregated metrics from member functions of a god object.
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

/// Extract member functions for a specific file from unified analysis.
///
/// Pure function that filters items by file path.
pub fn extract_member_functions<'a>(
    items: impl Iterator<Item = &'a UnifiedDebtItem>,
    file_path: &Path,
) -> Vec<&'a UnifiedDebtItem> {
    items
        .filter(|item| item.location.file == file_path)
        .collect()
}

/// Aggregate complexity metrics from member functions.
///
/// Pure function: sum cyclomatic/cognitive, max nesting.
pub fn aggregate_complexity_metrics(
    members: &[&UnifiedDebtItem],
) -> (u32, u32, u32) {
    let total_cyclomatic = members
        .iter()
        .map(|item| item.cyclomatic_complexity)
        .sum();

    let total_cognitive = members
        .iter()
        .map(|item| item.cognitive_complexity)
        .sum();

    let max_nesting = members
        .iter()
        .map(|item| item.nesting_depth)
        .max()
        .unwrap_or(0);

    (total_cyclomatic, total_cognitive, max_nesting)
}

/// Aggregate coverage metrics with weighted average by function length.
///
/// Pure function: computes weighted average of coverage.
pub fn aggregate_coverage_metrics(
    members: &[&UnifiedDebtItem],
) -> Option<TransitiveCoverage> {
    let coverages: Vec<_> = members
        .iter()
        .filter_map(|item| {
            item.transitive_coverage.as_ref().map(|cov| {
                (cov, item.function_length)
            })
        })
        .collect();

    if coverages.is_empty() {
        return None;
    }

    let total_length: usize = coverages
        .iter()
        .map(|(_, len)| len)
        .sum();

    let weighted_direct: f64 = coverages
        .iter()
        .map(|(cov, len)| cov.direct * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    let weighted_transitive: f64 = coverages
        .iter()
        .map(|(cov, len)| cov.transitive * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    let total_uncovered: usize = coverages
        .iter()
        .map(|(cov, _)| cov.uncovered_dependencies)
        .sum();

    Some(TransitiveCoverage {
        direct: weighted_direct,
        transitive: weighted_transitive,
        uncovered_dependencies: total_uncovered,
    })
}

/// Aggregate dependency metrics with unique set deduplication.
///
/// Pure function: collects unique callers/callees.
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

    let upstream_deps = unique_callers.len();
    let downstream_deps = unique_callees.len();

    (
        unique_callers.into_iter().collect(),
        unique_callees.into_iter().collect(),
        upstream_deps,
        downstream_deps,
    )
}

/// Aggregate git context metrics from member functions.
///
/// Pure function: sums churn, unique authors, max recency.
pub fn aggregate_git_context(
    members: &[&UnifiedDebtItem],
) -> Option<ContextualRisk> {
    let contexts: Vec<_> = members
        .iter()
        .filter_map(|item| item.contextual_risk.as_ref())
        .collect();

    if contexts.is_empty() {
        return None;
    }

    let total_churn: f64 = contexts
        .iter()
        .map(|ctx| ctx.churn_score)
        .sum();

    let unique_authors: std::collections::HashSet<_> = contexts
        .iter()
        .flat_map(|ctx| &ctx.authors)
        .cloned()
        .collect();

    let most_recent = contexts
        .iter()
        .filter_map(|ctx| ctx.last_modified)
        .max();

    let avg_frequency = contexts
        .iter()
        .map(|ctx| ctx.modification_frequency)
        .sum::<f64>()
        / contexts.len() as f64;

    Some(ContextualRisk {
        churn_score: total_churn,
        authors: unique_authors.into_iter().collect(),
        last_modified: most_recent,
        modification_frequency: avg_frequency,
        // ... other fields with appropriate aggregation
    })
}

/// Aggregate all metrics from member functions.
///
/// Pure function that composes all individual aggregations.
pub fn aggregate_god_object_metrics(
    members: &[&UnifiedDebtItem],
) -> GodObjectAggregatedMetrics {
    let (total_cyclomatic, total_cognitive, max_nesting) =
        aggregate_complexity_metrics(members);

    let weighted_coverage = aggregate_coverage_metrics(members);

    let (unique_callers, unique_callees, upstream_deps, downstream_deps) =
        aggregate_dependency_metrics(members);

    let aggregated_git_context = aggregate_git_context(members);

    GodObjectAggregatedMetrics {
        total_cyclomatic,
        total_cognitive,
        max_nesting_depth: max_nesting,
        weighted_coverage,
        unique_upstream_callers: unique_callers,
        unique_downstream_callees: unique_callees,
        upstream_dependencies: upstream_deps,
        downstream_dependencies: downstream_deps,
        aggregated_git_context,
    }
}
```

### Updated create_god_object_debt_item

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
        // ... rest of fields
    };

    // ... debt_type, file_name, recommendation, tier_enum ...

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
        // ... entropy fields stay None for god objects
        contextual_risk: aggregated_metrics.aggregated_git_context, // Use aggregated!
        god_object_indicators: Some(god_analysis.clone()),
        tier: Some(tier_enum),
        // ... rest of fields
        file_line_count: Some(god_analysis.lines_of_code),
    }
}
```

### Updated apply_file_analysis_results

```rust
// src/builders/unified_analysis.rs

fn apply_file_analysis_results(
    unified: &mut UnifiedAnalysis,
    processed_files: Vec<ProcessedFileData>,
) {
    for file_data in processed_files {
        // Update god object indicators for functions in this file
        if let Some(god_analysis) = &file_data.god_analysis {
            update_function_god_indicators(unified, &file_data.file_path, god_analysis);

            // NEW: Extract and aggregate member function metrics
            let member_functions = extract_member_functions(
                unified.items.iter(),
                &file_data.file_path,
            );

            let aggregated_metrics = aggregate_god_object_metrics(&member_functions);

            // Create god object debt item with aggregated metrics
            let god_item = create_god_object_debt_item(
                &file_data.file_path,
                &file_data.file_metrics,
                god_analysis,
                aggregated_metrics, // Pass aggregated metrics!
            );
            unified.add_item(god_item);
        }

        // Create and add file debt item
        let file_item = create_file_debt_item(file_data);
        unified.add_file_item(file_item);
    }
}
```

## Display Improvements

### TUI List View
```
Current:
  #1  critical  95.0  Config.rs (God Object)  (LOC:2000 Resp:8 Fns:50)

Proposed:
  #1  critical  95.0  Config.rs (God Object)  (Cyc:120 Cog:180 Cov:45% Deps:25↑/18↓)
```

### TUI Detail View

**Overview Tab** (new aggregated metrics):
```
┌─ Overview ─────────────────────────────────────┐
│ God Object: Config.rs                          │
│                                                │
│ Complexity (aggregated from 50 functions):    │
│   Cyclomatic:  120 (avg: 2.4 per function)   │
│   Cognitive:   180 (avg: 3.6 per function)   │
│   Max Nesting: 5   (worst function)          │
│                                                │
│ Coverage (weighted by function length):       │
│   Direct:      45.2%                          │
│   Transitive:  62.8%                          │
│   Uncovered:   18 dependencies                │
│                                                │
│ Dependencies (unique across all functions):   │
│   Upstream:    25 callers                     │
│   Downstream:  18 callees                     │
│   Blast Radius: 43 functions                  │
│                                                │
│ Git Context:                                   │
│   Churn Score: 125.5 (high instability)      │
│   Contributors: 7 developers                  │
│   Last Modified: 2025-01-05                   │
│   Change Freq: 2.3 changes/week              │
└────────────────────────────────────────────────┘
```

**Dependencies Tab** (show aggregated unique dependencies):
```
┌─ Dependencies ─────────────────────────────────┐
│ Upstream Callers (25):                        │
│   • main::run                                 │
│   • cli::parse_args                           │
│   • server::initialize                        │
│   ... (show top 20, link to full list)       │
│                                                │
│ Downstream Callees (18):                      │
│   • logger::init                              │
│   • db::connect                               │
│   ... (show top 20)                           │
└────────────────────────────────────────────────┘
```

## Benefits

### For Users
1. **Meaningful metrics**: God objects show actual aggregated data, not placeholders
2. **Better prioritization**: Coverage, complexity, dependencies inform refactoring decisions
3. **Impact analysis**: Blast radius shows true refactoring scope
4. **Risk assessment**: Git context shows instability and team dynamics

### For Developers
1. **Pure functions**: Easy to test aggregations in isolation
2. **Composable**: Can add new aggregations without changing existing code
3. **Type-safe**: Aggregated metrics passed explicitly, not hidden
4. **Incremental**: Can implement aggregations one at a time

### For Functional Programming
1. **Separation of concerns**: Extract → Aggregate → Create (3 pure stages)
2. **No side effects**: All aggregations are pure transformations
3. **Testable**: Each aggregation function tests independently
4. **Reusable**: Aggregation logic can be used elsewhere

## Complexity Analysis

### Current create_god_object_debt_item
- **Lines**: ~110 lines
- **Complexity**: Low (just struct construction)
- **Dependencies**: file_path, file_metrics, god_analysis

### Proposed create_god_object_debt_item
- **Lines**: ~120 lines (10 more to use aggregated metrics)
- **Complexity**: Low (still just struct construction)
- **Dependencies**: + aggregated_metrics parameter

### New Aggregation Module
- **Each aggregation function**: 10-20 lines
- **Total**: ~100 lines for all aggregations
- **Complexity**: Low (pure data transformations)
- **Testability**: 100% (all pure functions)

**Total new code**: ~110 lines
**Total refactored code**: ~10 lines
**Complexity added**: Minimal (all pure functions <20 lines)

## Testing Strategy

### Unit Tests for Aggregations
```rust
#[test]
fn test_aggregate_complexity_sums_cyclomatic() {
    let members = vec![
        create_item(cyc: 5, cog: 10, nest: 2),
        create_item(cyc: 10, cog: 15, nest: 3),
        create_item(cyc: 15, cog: 20, nest: 1),
    ];

    let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(&members);

    assert_eq!(total_cyc, 30); // 5 + 10 + 15
    assert_eq!(total_cog, 45); // 10 + 15 + 20
    assert_eq!(max_nest, 3);   // max(2, 3, 1)
}

#[test]
fn test_aggregate_coverage_weighted_by_length() {
    let members = vec![
        create_item_with_coverage(length: 10, cov: 0.8),
        create_item_with_coverage(length: 50, cov: 0.2),
        create_item_with_coverage(length: 40, cov: 0.5),
    ];

    let coverage = aggregate_coverage_metrics(&members).unwrap();

    // (10*0.8 + 50*0.2 + 40*0.5) / 100 = 38/100 = 0.38
    assert!((coverage.direct - 0.38).abs() < 0.01);
}

#[test]
fn test_aggregate_dependencies_deduplicates() {
    let members = vec![
        create_item_with_deps(
            upstream: vec!["main", "init"],
            downstream: vec!["log"],
        ),
        create_item_with_deps(
            upstream: vec!["main", "process"], // "main" is duplicate
            downstream: vec!["log", "db"], // "log" is duplicate
        ),
    ];

    let (callers, callees, up_count, down_count) =
        aggregate_dependency_metrics(&members);

    assert_eq!(up_count, 3); // main, init, process (deduplicated)
    assert_eq!(down_count, 2); // log, db (deduplicated)
}
```

### Integration Tests
```rust
#[test]
fn test_god_object_shows_aggregated_metrics() {
    let analysis = analyze_project_with_god_object("test_project");

    let god_object = analysis
        .items
        .iter()
        .find(|item| matches!(item.debt_type, DebtType::GodObject { .. }))
        .expect("God object should exist");

    // Complexity should be aggregated (not 0)
    assert!(god_object.cyclomatic_complexity > 0);
    assert!(god_object.cognitive_complexity > 0);

    // Coverage should exist (not None)
    assert!(god_object.transitive_coverage.is_some());
    let cov = god_object.transitive_coverage.unwrap();
    assert!(cov.direct >= 0.0 && cov.direct <= 1.0);

    // Dependencies should be aggregated (not 0)
    assert!(god_object.upstream_dependencies > 0);
    assert!(god_object.downstream_dependencies > 0);
}
```

### Property Tests
```rust
proptest! {
    #[test]
    fn complexity_sum_never_negative(
        cyc_values in prop::collection::vec(0u32..100, 1..20),
    ) {
        let members: Vec<_> = cyc_values
            .iter()
            .map(|&cyc| create_item(cyc, 0, 0))
            .collect();
        let member_refs: Vec<_> = members.iter().collect();

        let (total, _, _) = aggregate_complexity_metrics(&member_refs);

        prop_assert!(total >= 0);
        prop_assert_eq!(total, cyc_values.iter().sum());
    }

    #[test]
    fn weighted_coverage_in_bounds(
        coverages in prop::collection::vec((1usize..1000, 0.0f64..1.0), 1..20),
    ) {
        let members: Vec<_> = coverages
            .iter()
            .map(|&(len, cov)| create_item_with_coverage(len, cov))
            .collect();
        let member_refs: Vec<_> = members.iter().collect();

        if let Some(result) = aggregate_coverage_metrics(&member_refs) {
            prop_assert!(result.direct >= 0.0 && result.direct <= 1.0);
        }
    }
}
```

## Migration Path

### Stage 1: Add Aggregation Module (2 hours)
1. Create `src/priority/god_object_aggregation.rs`
2. Implement pure aggregation functions
3. Add unit tests for each aggregation
4. Add property tests

### Stage 2: Update God Object Creation (1 hour)
1. Update `create_god_object_debt_item` signature
2. Use aggregated metrics in UnifiedDebtItem
3. Update call site in `apply_file_analysis_results`
4. Add integration test

### Stage 3: Update TUI Display (1 hour)
1. Update list view to show aggregated metrics
2. Update detail view Overview tab
3. Update Dependencies tab with unique sets
4. Test TUI displays correctly

**Total effort**: 4 hours

## Success Criteria

- [ ] God objects show cyclomatic > 0 (sum of member functions)
- [ ] God objects show cognitive > 0 (sum of member functions)
- [ ] God objects show coverage percentage (weighted average)
- [ ] God objects show dependencies count (unique set)
- [ ] God objects show git context (aggregated churn/authors)
- [ ] TUI displays aggregated metrics clearly
- [ ] All aggregation functions <20 lines
- [ ] All aggregation functions are pure (no side effects)
- [ ] 100% test coverage on aggregations
- [ ] Integration test verifies end-to-end
- [ ] No performance regression
