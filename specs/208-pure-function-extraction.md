---
number: 208
title: Pure Function Extraction from Analysis Pipeline
category: foundation
priority: critical
status: draft
dependencies: [207]
created: 2025-12-05
---

# Specification 208: Pure Function Extraction from Analysis Pipeline

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 207 (Stillwater Effects Integration)

## Context

The current analysis pipeline in `src/builders/unified_analysis.rs` contains a 453-line function (`perform_unified_analysis_computation`) that violates functional programming principles:

**Current Issues**:
- **Mixed Concerns**: I/O operations interleaved with business logic
- **Mutation Heavy**: Results built through `mut` variables and iterative modification
- **Untestable**: Requires mocking file system, progress bars, spinners
- **Non-Reusable**: Logic tightly coupled to specific execution context
- **Large Functions**: Multiple functions exceed 100 lines with cyclomatic complexity > 10

**Example Problem Code**:
```rust
fn perform_unified_analysis_computation(...) -> Result<UnifiedAnalysis> {
    let mut call_graph = build_initial_call_graph(&metrics);  // Pure

    if multi_pass {
        let spinner = create_spinner("Analyzing...");         // I/O
        perform_multi_pass_analysis(...)?;                    // I/O + Logic
        spinner.finish_and_clear();                           // I/O
    }

    let coverage_data = load_coverage_data(...)?;             // I/O

    let mut unified = UnifiedAnalysis::new(call_graph.clone());  // Mutation
    unified.populate_purity_analysis(metrics);                   // Mutation

    for metric in metrics {                                      // Imperative loop
        if let Some(item) = create_debt_item(...) {
            unified.add_item(item);                              // Mutation
        }
    }

    analyze_files_for_debt(&mut unified, ...);                  // More mutation
    unified.sort_by_priority();                                  // More mutation
    unified.calculate_total_impact();                            // More mutation

    Ok(unified)
}
```

**Stillwater Philosophy Violation**: Business logic (call graph building, debt detection, scoring) is entangled with I/O (file loading, progress reporting) and mutation (building results iteratively).

## Objective

Extract pure business logic from the analysis pipeline into small, focused, testable functions that:
1. **Never perform I/O** - All side effects removed
2. **Are deterministic** - Same input always produces same output
3. **Are small** - Each function < 20 lines with single responsibility
4. **Are composable** - Can be combined to build complex transformations
5. **Are 100% testable** - No mocks needed, easy to unit test

**Target**: Reduce `perform_unified_analysis_computation` from 453 lines to < 50 lines of pure effect composition, with all logic extracted to pure functions averaging 5-15 lines each.

## Requirements

### Functional Requirements

1. **Call Graph Construction** (Pure)
   - Extract `build_call_graph(metrics: &[FunctionMetrics]) -> CallGraph`
   - Pure function that constructs graph from metrics
   - No file I/O, no progress reporting
   - 100% deterministic and testable

2. **Trait Resolution** (Pure Core + I/O Shell)
   - **Pure**: `resolve_trait_calls(graph: &CallGraph, trait_info: &TraitInfo) -> CallGraph`
   - **I/O**: `load_trait_info(path: &Path) -> AnalysisEffect<TraitInfo>`
   - Separate loading trait information from applying resolution

3. **Purity Analysis** (Pure)
   - Extract `analyze_purity(metrics: &[FunctionMetrics], graph: &CallGraph) -> PurityAnalysis`
   - Pure function computing purity based on graph structure
   - No mutation, returns new data structure
   - Deterministic analysis algorithm

4. **Debt Detection** (Pure)
   - Extract `detect_complexity_debt(metric: &FunctionMetrics, thresholds: &Thresholds) -> Option<DebtItem>`
   - Extract `detect_nesting_debt(metric: &FunctionMetrics) -> Option<DebtItem>`
   - Extract `detect_parameter_debt(metric: &FunctionMetrics) -> Option<DebtItem>`
   - Extract `detect_length_debt(metric: &FunctionMetrics) -> Option<DebtItem>`
   - Small focused functions, each detecting one type of debt

5. **Debt Scoring** (Pure)
   - Extract `score_debt_item(item: &DebtItem, context: &ScoringContext) -> f64`
   - Extract `apply_coverage_penalty(score: f64, coverage: Option<&Coverage>) -> f64`
   - Extract `apply_risk_multiplier(score: f64, risk: &RiskAnalysis) -> f64`
   - Pure scoring functions that compose

6. **Debt Prioritization** (Pure)
   - Extract `prioritize_debt(items: Vec<DebtItem>, context: &PriorityContext) -> Vec<PrioritizedDebt>`
   - Extract `calculate_total_impact(items: &[PrioritizedDebt]) -> ImpactMetrics`
   - Pure sorting and aggregation logic

7. **Data Transformation Pipeline** (Pure)
   - Extract `enrich_metrics(metrics: Vec<FunctionMetrics>, graph: &CallGraph) -> Vec<EnrichedMetrics>`
   - Extract `aggregate_by_file(items: Vec<DebtItem>) -> HashMap<PathBuf, Vec<DebtItem>>`
   - Extract `filter_test_functions(metrics: &[FunctionMetrics], graph: &CallGraph) -> Vec<FunctionMetrics>`
   - Pure data transformations using functional patterns

### Non-Functional Requirements

1. **Function Size**
   - Maximum 20 lines per function (prefer 5-10)
   - Single responsibility principle strictly enforced
   - Cyclomatic complexity < 5

2. **Testability**
   - Every pure function has comprehensive unit tests
   - No mocking required (pure functions)
   - Property-based tests where appropriate
   - 100% test coverage for pure functions

3. **Performance**
   - Zero-cost abstractions (compile away)
   - Prefer iterator chains over intermediate collections
   - Use `rayon` for data-parallel operations where beneficial
   - No unnecessary cloning

4. **Readability**
   - Function names clearly describe transformation
   - Type signatures document data flow
   - Minimal type annotations (rely on inference)
   - Clear separation of concerns

## Acceptance Criteria

- [ ] All business logic extracted from `perform_unified_analysis_computation` into pure functions
- [ ] Each pure function < 20 lines (average 5-10 lines)
- [ ] All pure functions in `src/pipeline/stages/` module
- [ ] Zero I/O operations in pure functions (verified by code review)
- [ ] Comprehensive unit tests for every pure function (no mocks)
- [ ] Integration tests validate composed pipeline produces same results
- [ ] Performance benchmarks show no regression (< 5% variance)
- [ ] Documentation for each pure function with examples
- [ ] All existing tests pass (backward compatibility)
- [ ] Cyclomatic complexity < 5 for all extracted functions

## Technical Details

### Implementation Approach

#### Phase 1: Module Structure

```
src/pipeline/
├── mod.rs
├── effects/              # From Spec 207
│   ├── mod.rs
│   ├── types.rs
│   ├── io.rs
│   └── combinators.rs
└── stages/               # Pure functions (this spec)
    ├── mod.rs
    ├── call_graph.rs     # Pure call graph operations
    ├── purity.rs         # Pure purity analysis
    ├── debt.rs           # Pure debt detection
    ├── scoring.rs        # Pure scoring functions
    ├── filtering.rs      # Pure data filtering
    └── aggregation.rs    # Pure data aggregation
```

#### Phase 2: Call Graph Extraction

```rust
// src/pipeline/stages/call_graph.rs

/// Build call graph from function metrics (pure)
pub fn build_call_graph(metrics: &[FunctionMetrics]) -> CallGraph {
    let mut graph = CallGraph::new();

    for metric in metrics {
        graph.add_node(function_id(metric), metric.clone());
    }

    for metric in metrics {
        for call in &metric.calls {
            graph.add_edge(function_id(metric), call.clone());
        }
    }

    graph
}

/// Resolve trait method calls in graph (pure)
pub fn resolve_trait_calls(
    graph: &CallGraph,
    trait_info: &TraitInfo,
) -> CallGraph {
    let mut resolved = graph.clone();

    for (caller_id, trait_call) in graph.trait_calls() {
        if let Some(impl_id) = trait_info.resolve(&trait_call) {
            resolved.add_edge(caller_id, impl_id);
        }
    }

    resolved
}

/// Find test-only functions using graph reachability (pure)
pub fn find_test_only_functions(
    graph: &CallGraph,
    test_roots: &[FunctionId],
) -> HashSet<FunctionId> {
    test_roots
        .iter()
        .flat_map(|root| graph.reachable_from(root))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_call_graph_empty() {
        let graph = build_call_graph(&[]);
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_build_call_graph_single_function() {
        let metrics = vec![test_metric("foo", vec![])];
        let graph = build_call_graph(&metrics);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_resolve_trait_calls() {
        let graph = test_graph_with_trait_calls();
        let trait_info = test_trait_info();

        let resolved = resolve_trait_calls(&graph, &trait_info);

        assert_eq!(resolved.edge_count(), graph.edge_count() + 2);
    }
}
```

#### Phase 3: Purity Analysis Extraction

```rust
// src/pipeline/stages/purity.rs

/// Analyze function purity based on local inspection (pure)
pub fn analyze_local_purity(metric: &FunctionMetrics) -> PurityCategory {
    if has_io_operations(metric) {
        return PurityCategory::Impure;
    }

    if has_mutation(metric) {
        return PurityCategory::ImpureLocal;
    }

    PurityCategory::Pure
}

/// Propagate purity through call graph (pure)
pub fn propagate_purity(
    initial: HashMap<FunctionId, PurityCategory>,
    graph: &CallGraph,
    max_iterations: usize,
) -> HashMap<FunctionId, PurityCategory> {
    let mut purity = initial;

    for _ in 0..max_iterations {
        let updated = propagate_one_step(&purity, graph);
        if updated == purity {
            break; // Converged
        }
        purity = updated;
    }

    purity
}

/// Single propagation step (pure)
fn propagate_one_step(
    current: &HashMap<FunctionId, PurityCategory>,
    graph: &CallGraph,
) -> HashMap<FunctionId, PurityCategory> {
    current
        .iter()
        .map(|(id, category)| {
            let callees = graph.callees(id);
            let updated = refine_purity(*category, callees, current);
            (*id, updated)
        })
        .collect()
}

/// Refine purity based on callees (pure)
fn refine_purity(
    current: PurityCategory,
    callees: &[FunctionId],
    purity_map: &HashMap<FunctionId, PurityCategory>,
) -> PurityCategory {
    if current == PurityCategory::Impure {
        return PurityCategory::Impure;
    }

    let callee_purity = callees
        .iter()
        .filter_map(|id| purity_map.get(id))
        .copied();

    if callee_purity.any(|p| p == PurityCategory::Impure) {
        PurityCategory::Impure
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_local_purity_pure() {
        let metric = test_metric_pure();
        assert_eq!(analyze_local_purity(&metric), PurityCategory::Pure);
    }

    #[test]
    fn test_propagate_purity_simple() {
        let initial = hashmap! {
            fn_id("foo") => PurityCategory::Pure,
            fn_id("bar") => PurityCategory::Pure,
        };
        let graph = test_graph(); // foo calls bar

        let result = propagate_purity(initial, &graph, 10);

        assert_eq!(result[&fn_id("foo")], PurityCategory::Pure);
    }

    #[test]
    fn test_propagate_purity_impure_spreads() {
        let initial = hashmap! {
            fn_id("foo") => PurityCategory::Pure,
            fn_id("bar") => PurityCategory::Impure,
        };
        let graph = test_graph(); // foo calls bar

        let result = propagate_purity(initial, &graph, 10);

        assert_eq!(result[&fn_id("foo")], PurityCategory::Impure);
    }
}
```

#### Phase 4: Debt Detection Extraction

```rust
// src/pipeline/stages/debt.rs

/// Detect all debt in a function metric (pure)
pub fn detect_all_debt(
    metric: &FunctionMetrics,
    thresholds: &Thresholds,
) -> Vec<DebtItem> {
    [
        detect_complexity_debt(metric, thresholds),
        detect_nesting_debt(metric, thresholds),
        detect_length_debt(metric, thresholds),
        detect_parameter_debt(metric, thresholds),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Detect high complexity (pure)
pub fn detect_complexity_debt(
    metric: &FunctionMetrics,
    thresholds: &Thresholds,
) -> Option<DebtItem> {
    if metric.cyclomatic > thresholds.complexity {
        Some(DebtItem {
            debt_type: DebtType::HighComplexity,
            location: metric.location(),
            complexity: metric.cyclomatic,
            message: format!(
                "Cyclomatic complexity {} exceeds threshold {}",
                metric.cyclomatic, thresholds.complexity
            ),
        })
    } else {
        None
    }
}

/// Detect deep nesting (pure)
pub fn detect_nesting_debt(
    metric: &FunctionMetrics,
    thresholds: &Thresholds,
) -> Option<DebtItem> {
    if metric.nesting_depth > thresholds.nesting {
        Some(DebtItem {
            debt_type: DebtType::DeepNesting,
            location: metric.location(),
            nesting_depth: metric.nesting_depth,
            message: format!(
                "Nesting depth {} exceeds threshold {}",
                metric.nesting_depth, thresholds.nesting
            ),
        })
    } else {
        None
    }
}

/// Detect long functions (pure)
pub fn detect_length_debt(
    metric: &FunctionMetrics,
    thresholds: &Thresholds,
) -> Option<DebtItem> {
    if metric.length > thresholds.length {
        Some(DebtItem {
            debt_type: DebtType::LongFunction,
            location: metric.location(),
            length: metric.length,
            message: format!(
                "Function length {} exceeds threshold {}",
                metric.length, thresholds.length
            ),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_complexity_debt_none() {
        let metric = test_metric_with_complexity(5);
        let thresholds = Thresholds { complexity: 10, ..default() };

        let debt = detect_complexity_debt(&metric, &thresholds);

        assert!(debt.is_none());
    }

    #[test]
    fn test_detect_complexity_debt_found() {
        let metric = test_metric_with_complexity(15);
        let thresholds = Thresholds { complexity: 10, ..default() };

        let debt = detect_complexity_debt(&metric, &thresholds);

        assert!(debt.is_some());
        assert_eq!(debt.unwrap().debt_type, DebtType::HighComplexity);
    }

    #[test]
    fn test_detect_all_debt_multiple() {
        let metric = test_metric_complex_and_long();
        let thresholds = Thresholds::default();

        let debt = detect_all_debt(&metric, &thresholds);

        assert_eq!(debt.len(), 2); // Both complexity and length
    }
}
```

#### Phase 5: Scoring Extraction

```rust
// src/pipeline/stages/scoring.rs

/// Score a debt item (pure)
pub fn score_debt_item(
    item: &DebtItem,
    context: &ScoringContext,
) -> f64 {
    let base_score = base_score_for_type(&item.debt_type);
    let severity_multiplier = severity_multiplier(item);
    let coverage_penalty = coverage_penalty(item, context.coverage);
    let risk_multiplier = risk_multiplier(item, context.risk);

    base_score * severity_multiplier * coverage_penalty * risk_multiplier
}

/// Base score for debt type (pure)
fn base_score_for_type(debt_type: &DebtType) -> f64 {
    match debt_type {
        DebtType::HighComplexity => 50.0,
        DebtType::DeepNesting => 40.0,
        DebtType::LongFunction => 30.0,
        DebtType::HighParameterCount => 20.0,
        DebtType::CodeDuplication => 35.0,
    }
}

/// Severity multiplier based on magnitude (pure)
fn severity_multiplier(item: &DebtItem) -> f64 {
    match &item.debt_type {
        DebtType::HighComplexity => {
            1.0 + (item.complexity as f64 - 10.0) * 0.1
        }
        DebtType::DeepNesting => {
            1.0 + (item.nesting_depth as f64 - 4.0) * 0.15
        }
        _ => 1.0,
    }
}

/// Coverage penalty multiplier (pure)
fn coverage_penalty(item: &DebtItem, coverage: Option<&Coverage>) -> f64 {
    match coverage {
        Some(cov) if cov.get_function_coverage(&item.location) == 0.0 => 1.5,
        Some(cov) if cov.get_function_coverage(&item.location) < 0.5 => 1.2,
        _ => 1.0,
    }
}

/// Risk multiplier based on change frequency (pure)
fn risk_multiplier(item: &DebtItem, risk: Option<&RiskAnalysis>) -> f64 {
    match risk {
        Some(r) if r.is_high_churn(&item.location.file) => 1.3,
        _ => 1.0,
    }
}

/// Prioritize debt items by score (pure)
pub fn prioritize_debt(
    items: Vec<DebtItem>,
    context: &ScoringContext,
) -> Vec<PrioritizedDebt> {
    let mut scored: Vec<_> = items
        .into_iter()
        .map(|item| {
            let score = score_debt_item(&item, context);
            PrioritizedDebt { item, score }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_score_for_type() {
        assert_eq!(base_score_for_type(&DebtType::HighComplexity), 50.0);
        assert_eq!(base_score_for_type(&DebtType::DeepNesting), 40.0);
    }

    #[test]
    fn test_severity_multiplier_scales() {
        let item1 = test_debt_item_complexity(15);
        let item2 = test_debt_item_complexity(25);

        let mult1 = severity_multiplier(&item1);
        let mult2 = severity_multiplier(&item2);

        assert!(mult2 > mult1); // Higher complexity = higher multiplier
    }

    #[test]
    fn test_coverage_penalty_untested() {
        let item = test_debt_item();
        let coverage = test_coverage_zero_for_item(&item);

        let penalty = coverage_penalty(&item, Some(&coverage));

        assert_eq!(penalty, 1.5);
    }

    #[test]
    fn test_prioritize_debt_sorts() {
        let items = vec![
            test_debt_item_score_30(),
            test_debt_item_score_70(),
            test_debt_item_score_50(),
        ];
        let context = ScoringContext::default();

        let prioritized = prioritize_debt(items, &context);

        assert_eq!(prioritized[0].score, 70.0);
        assert_eq!(prioritized[1].score, 50.0);
        assert_eq!(prioritized[2].score, 30.0);
    }
}
```

#### Phase 6: Data Transformation Extraction

```rust
// src/pipeline/stages/filtering.rs

/// Filter out test functions (pure)
pub fn filter_test_functions(
    metrics: Vec<FunctionMetrics>,
    test_only: &HashSet<FunctionId>,
) -> Vec<FunctionMetrics> {
    metrics
        .into_iter()
        .filter(|m| !m.is_test && !test_only.contains(&function_id(m)))
        .collect()
}

/// Filter by minimum score threshold (pure)
pub fn filter_by_score(
    items: Vec<PrioritizedDebt>,
    min_score: f64,
) -> Vec<PrioritizedDebt> {
    items
        .into_iter()
        .filter(|item| item.score >= min_score)
        .collect()
}

/// Take top N items (pure)
pub fn take_top_n(
    items: Vec<PrioritizedDebt>,
    n: usize,
) -> Vec<PrioritizedDebt> {
    items.into_iter().take(n).collect()
}
```

```rust
// src/pipeline/stages/aggregation.rs

/// Aggregate debt items by file (pure)
pub fn aggregate_by_file(
    items: Vec<DebtItem>,
) -> HashMap<PathBuf, Vec<DebtItem>> {
    items.into_iter().fold(HashMap::new(), |mut acc, item| {
        acc.entry(item.location.file.clone())
            .or_default()
            .push(item);
        acc
    })
}

/// Calculate total impact metrics (pure)
pub fn calculate_total_impact(
    items: &[PrioritizedDebt],
) -> ImpactMetrics {
    ImpactMetrics {
        total_score: items.iter().map(|i| i.score).sum(),
        item_count: items.len(),
        avg_score: items.iter().map(|i| i.score).sum::<f64>() / items.len() as f64,
        high_priority_count: items.iter().filter(|i| i.score > 70.0).count(),
    }
}

/// Group by debt type (pure)
pub fn group_by_type(
    items: Vec<DebtItem>,
) -> HashMap<DebtType, Vec<DebtItem>> {
    items.into_iter().fold(HashMap::new(), |mut acc, item| {
        acc.entry(item.debt_type).or_default().push(item);
        acc
    })
}
```

### Architecture Changes

**Before**:
```
src/builders/unified_analysis.rs (2,500+ lines)
  ├─ perform_unified_analysis_computation (453 lines, mixed I/O + logic)
  ├─ create_unified_analysis_with_exclusions (192 lines, mutation-heavy)
  └─ Various helper functions (also mixed concerns)
```

**After**:
```
src/pipeline/
  ├─ effects/          # I/O operations (Spec 207)
  └─ stages/           # Pure transformations (this spec)
      ├─ call_graph.rs     (~150 lines, all pure)
      ├─ purity.rs         (~120 lines, all pure)
      ├─ debt.rs           (~100 lines, all pure)
      ├─ scoring.rs        (~130 lines, all pure)
      ├─ filtering.rs      (~50 lines, all pure)
      └─ aggregation.rs    (~60 lines, all pure)

src/builders/unified_analysis.rs (< 100 lines)
  └─ Thin wrapper calling pipeline stages
```

### APIs and Interfaces

#### Public API

```rust
pub mod pipeline {
    pub mod stages {
        // Call graph
        pub use call_graph::{
            build_call_graph,
            resolve_trait_calls,
            find_test_only_functions,
        };

        // Purity
        pub use purity::{
            analyze_local_purity,
            propagate_purity,
        };

        // Debt detection
        pub use debt::{
            detect_all_debt,
            detect_complexity_debt,
            detect_nesting_debt,
            detect_length_debt,
        };

        // Scoring
        pub use scoring::{
            score_debt_item,
            prioritize_debt,
        };

        // Transformations
        pub use filtering::{filter_test_functions, filter_by_score};
        pub use aggregation::{aggregate_by_file, calculate_total_impact};
    }
}
```

## Dependencies

- **Prerequisites**: Spec 207 (Stillwater Effects Integration)
- **Affected Components**:
  - `src/builders/unified_analysis.rs` - Major refactoring
  - `src/priority/unified_scorer.rs` - Extract scoring logic
  - `src/analysis/purity.rs` - Extract purity logic
- **External Dependencies**: None (uses existing Rust stdlib + rayon)

## Testing Strategy

### Unit Tests

Every pure function gets comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Example-based tests
    #[test]
    fn test_build_call_graph_empty() { ... }

    #[test]
    fn test_build_call_graph_simple() { ... }

    #[test]
    fn test_build_call_graph_complex() { ... }

    // Property-based tests
    proptest! {
        #[test]
        fn test_call_graph_node_count_matches_metrics(
            metrics in prop::collection::vec(arbitrary_metric(), 0..100)
        ) {
            let graph = build_call_graph(&metrics);
            assert_eq!(graph.node_count(), metrics.len());
        }

        #[test]
        fn test_scoring_never_negative(
            item in arbitrary_debt_item(),
            context in arbitrary_scoring_context(),
        ) {
            let score = score_debt_item(&item, &context);
            assert!(score >= 0.0);
        }
    }
}
```

### Integration Tests

Validate composed pipeline produces correct results:

```rust
#[test]
fn test_full_pure_pipeline() {
    let metrics = test_metrics_fixture();
    let thresholds = Thresholds::default();

    // Pure pipeline composition
    let graph = build_call_graph(&metrics);
    let purity = analyze_purity(&metrics, &graph);
    let test_only = find_test_only_functions(&graph, &test_roots());
    let filtered = filter_test_functions(metrics, &test_only);
    let debt_items = filtered
        .iter()
        .flat_map(|m| detect_all_debt(m, &thresholds))
        .collect();
    let context = ScoringContext { coverage: None, risk: None };
    let prioritized = prioritize_debt(debt_items, &context);

    assert_eq!(prioritized.len(), 5);
    assert!(prioritized[0].score > prioritized[1].score);
}
```

### Performance Tests

```rust
#[bench]
fn bench_build_call_graph(b: &mut Bencher) {
    let metrics = large_metrics_fixture(1000);
    b.iter(|| build_call_graph(&metrics));
}

#[bench]
fn bench_detect_all_debt(b: &mut Bencher) {
    let metric = complex_metric_fixture();
    let thresholds = Thresholds::default();
    b.iter(|| detect_all_debt(&metric, &thresholds));
}

#[bench]
fn bench_score_debt_item(b: &mut Bencher) {
    let item = test_debt_item();
    let context = ScoringContext::default();
    b.iter(|| score_debt_item(&item, &context));
}
```

## Documentation Requirements

### Code Documentation

Each pure function documented with:
```rust
/// Build call graph from function metrics.
///
/// Creates a directed graph where nodes are functions and edges are
/// function calls. This is a pure transformation that never performs I/O.
///
/// # Arguments
///
/// * `metrics` - Slice of function metrics containing call information
///
/// # Returns
///
/// Call graph with all functions and their relationships
///
/// # Examples
///
/// ```
/// let metrics = vec![
///     FunctionMetrics { name: "foo", calls: vec!["bar"], ... },
///     FunctionMetrics { name: "bar", calls: vec![], ... },
/// ];
///
/// let graph = build_call_graph(&metrics);
/// assert_eq!(graph.node_count(), 2);
/// assert_eq!(graph.edge_count(), 1);
/// ```
pub fn build_call_graph(metrics: &[FunctionMetrics]) -> CallGraph {
    // ...
}
```

### Architecture Documentation

Update `ARCHITECTURE.md`:
```markdown
## Pure Function Organization

All business logic is organized into pure functions in `src/pipeline/stages/`:

- **call_graph.rs**: Graph construction and analysis
- **purity.rs**: Function purity detection and propagation
- **debt.rs**: Technical debt detection algorithms
- **scoring.rs**: Debt scoring and prioritization
- **filtering.rs**: Data filtering operations
- **aggregation.rs**: Data aggregation and summarization

Pure functions follow strict rules:
- No I/O operations (no file/network/database access)
- No side effects (no logging, no progress reporting)
- Deterministic (same input → same output)
- Small (< 20 lines, single responsibility)
- Testable (unit tests without mocks)
```

## Implementation Notes

### Best Practices

1. **Iterator Chains Over Loops**
   ```rust
   // Good: Functional pipeline
   metrics
       .iter()
       .filter(|m| !m.is_test)
       .map(|m| detect_debt(m, thresholds))
       .flatten()
       .collect()

   // Avoid: Imperative loop with mutation
   let mut debt = vec![];
   for m in metrics {
       if !m.is_test {
           if let Some(d) = detect_debt(m, thresholds) {
               debt.push(d);
           }
       }
   }
   ```

2. **Small Functions**
   ```rust
   // Good: Compose small functions
   fn score_with_penalties(item: &DebtItem, ctx: &Context) -> f64 {
       let base = base_score(item);
       let cov = coverage_penalty(item, ctx.coverage);
       let risk = risk_multiplier(item, ctx.risk);
       base * cov * risk
   }

   // Avoid: One large function doing everything
   fn score(item: &DebtItem, ctx: &Context) -> f64 {
       let base = match item.debt_type { ... }; // 50 lines
       let cov = if let Some(c) = ctx.coverage { ... }; // 30 lines
       let risk = if ctx.risk.is_high_churn() { ... }; // 20 lines
       base * cov * risk
   }
   ```

3. **Type-Driven Development**
   ```rust
   // Let types guide implementation
   fn prioritize(items: Vec<DebtItem>) -> Vec<PrioritizedDebt> {
       // Type signature tells us we need to:
       // 1. Score each item
       // 2. Sort by score
       // 3. Return prioritized items
       items
           .into_iter()
           .map(|item| score_item(item))
           .sorted_by_score()
           .collect()
   }
   ```

### Common Pitfalls

1. **Don't Sneak in I/O**
   ```rust
   // Bad: Hidden I/O
   fn build_graph(metrics: &[FunctionMetrics]) -> CallGraph {
       log::info!("Building graph..."); // Side effect!
       // ...
   }

   // Good: Pure
   fn build_graph(metrics: &[FunctionMetrics]) -> CallGraph {
       // No side effects
       // ...
   }
   ```

2. **Don't Over-Clone**
   ```rust
   // Bad: Unnecessary cloning
   fn filter_metrics(metrics: Vec<FunctionMetrics>) -> Vec<FunctionMetrics> {
       metrics.clone().into_iter().filter(|m| !m.is_test).collect()
   }

   // Good: Take ownership or borrow
   fn filter_metrics(metrics: Vec<FunctionMetrics>) -> Vec<FunctionMetrics> {
       metrics.into_iter().filter(|m| !m.is_test).collect()
   }
   ```

## Migration and Compatibility

### Migration Steps

1. **Extract one stage at a time**
   - Start with simplest (e.g., filtering)
   - Validate with tests
   - Move to next stage

2. **Keep old code working**
   - Old functions call new pure functions
   - Gradual migration, no big bang

3. **Update incrementally**
   - One pull request per stage
   - Each PR is independently reviewable

### Backward Compatibility

Old API continues to work:
```rust
// Old function (kept for compatibility)
pub fn perform_unified_analysis(...) -> Result<UnifiedAnalysis> {
    // Internally uses new pure functions
    let graph = stages::build_call_graph(&metrics);
    let purity = stages::analyze_purity(&metrics, &graph);
    // ...
}
```

## Success Metrics

- [ ] Average function length reduced from 50+ to < 15 lines
- [ ] 100% of business logic is pure (no I/O in pure functions)
- [ ] 100% test coverage for all pure functions
- [ ] Zero mocking required in unit tests
- [ ] Performance within 5% of current implementation
- [ ] All 450+ existing tests pass

## References

- [Spec 207: Stillwater Effects Integration](./207-stillwater-effects-integration.md)
- [Stillwater Philosophy](../stillwater/PHILOSOPHY.md)
- [Functional Core, Imperative Shell](https://www.destroyallsoftware.com/screencasts/catalog/functional-core-imperative-shell)
