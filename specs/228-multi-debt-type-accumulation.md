---
number: 228
title: Multi-Debt Type Accumulation Per Function
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 228: Multi-Debt Type Accumulation Per Function

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

### Current Limitation

The debt classification system uses early-return pattern, allowing only **one debt type per function**:

```rust
// src/priority/scoring/classification.rs:231-302
pub fn classify_debt_type_with_exclusions(...) -> DebtType {
    if has_testing_gap(...) {
        return DebtType::TestingGap { ... };  // ← Returns immediately
    }
    if is_complexity_hotspot(...) {
        return DebtType::ComplexityHotspot { ... };  // ← Never reached
    }
    if is_dead_code(...) {
        return DebtType::DeadCode { ... };  // ← Never reached
    }
    // ...
}
```

**Problem**: A function with **multiple independent issues** only reports the first:

Example function `calculate_total()`:
- ❌ No test coverage (TestingGap)
- ❌ Cyclomatic complexity 25 (ComplexityHotspot)
- ❌ No callers (DeadCode)

**Current behavior**: Only `TestingGap` is reported, other issues are silently ignored.

**Expected behavior**: All three debt types should be reported as separate `UnifiedDebtItem` entries.

### Relationship to Grouping Feature

The TUI grouping feature (spec 223) was originally built to handle this scenario:

> "Functions with multiple debt types now appear as single entries with combined scores and multi-issue badges."

However:
1. **Grouping never worked** because multi-debt was never implemented
2. **Grouping adds 50% TUI rendering overhead** (duplicate `group_by_location()` calls)
3. **Grouping logic is correct** but has nothing to group (architecture prevents multi-debt)

**Decision**:
- Remove grouping feature immediately (performance fix)
- Implement proper multi-debt accumulation (this spec)
- Reconsider grouping later if needed

### Alignment with Functional Programming Principles

Debt type classification should follow the functional programming principle of **validation accumulation**:

- **Sequential operations (fail fast)**: Use `Result` chains with `?` for dependent steps
- **Independent validations (accumulate)**: Collect all validation results

Debt types are **independent** - a function can simultaneously have testing gaps, high complexity, and no callers. The current early-return architecture treats these as mutually exclusive when they should all be reported.

## Objective

Transform debt classification from **early-return (single debt)** to **accumulation (multiple debts)** pattern, following functional programming principles for independent validations.

**This is a breaking change**: Multi-debt accumulation becomes the default and only behavior. Functions that previously returned single debt items will now return multiple items when applicable.

## Requirements

### Functional Requirements

1. **FR1: Independent Debt Classification**
   - Each debt type check must be independent
   - TestingGap, ComplexityHotspot, DeadCode checks run regardless of others
   - No early returns that prevent subsequent checks

2. **FR2: Multiple Debt Items Per Function**
   - One `UnifiedDebtItem` created per debt type found
   - Each item has same location (file, function, line)
   - Each item has independent score calculation
   - Each item has debt-type-specific recommendations

3. **FR3: Pure Functional Classification**
   - Debt classification functions remain pure
   - Use iterator chains instead of mutable collections
   - Composable and testable in isolation
   - Follow debtmap's functional-first architecture (prefer `map`/`filter`/`fold` over loops)

4. **FR4: Test Function Exception**
   - Test functions remain exclusive (only test-specific debt)
   - Prevents noise from test code complexity
   - Returns early for `func.is_test == true`

### Non-Functional Requirements

1. **NFR1: Performance**
   - Performance overhead < 5% compared to single-debt baseline
   - Parallel processing remains effective
   - Use `flat_map` instead of nested loops

2. **NFR2: Code Quality**
   - Follow debtmap functional programming guidelines
   - Pure functions under 20 lines
   - Single responsibility per function
   - Use iterator chains instead of mutable collections
   - Comprehensive test coverage

3. **NFR3: Score Consistency**
   - Same score calculation per debt type
   - No changes to existing scoring algorithms
   - Each debt item independently scored

## Acceptance Criteria

- [ ] **AC1**: Function with TestingGap + ComplexityHotspot creates 2 `UnifiedDebtItem` entries
- [ ] **AC2**: Each debt item has correct debt-type-specific score and recommendation
- [ ] **AC3**: Test functions still only report test-specific debt (no mixing)
- [ ] **AC4**: TUI displays all debt items for a function (may appear as multiple entries)
- [ ] **AC5**: Updated tests pass with new multi-debt behavior
- [ ] **AC6**: Integration test validates multi-debt scenarios
- [ ] **AC7**: Performance overhead < 5% compared to single-debt baseline
- [ ] **AC8**: All functions using classification are updated to handle `Vec<DebtType>`
- [ ] **AC9**: No clippy warnings introduced by new code
- [ ] **AC10**: Debtmap's own analysis shows increased debt item count (more comprehensive)

## Technical Details

### Implementation Approach

**Note on Grouping Feature**: The TUI grouping feature removal mentioned in the Context section is orthogonal to multi-debt accumulation and should be handled as a **separate spec/commit**. This spec focuses solely on debt type accumulation. If grouping removal is desired, create a separate spec (e.g., "Spec 229: Remove Non-Functional TUI Grouping").

#### Phase 1: Extract Pure Predicates

**Create helper functions** (`src/priority/scoring/classification.rs`):
- `check_testing_gap()` → `Option<DebtType>`
- `check_complexity_hotspot()` → `Option<DebtType>`
- `check_dead_code_with_exclusions()` → `Option<DebtType>` (extract from existing code)

These pure predicates enable functional composition and testing in isolation.

#### Phase 2: Refactor Classification to Accumulation Pattern

**Create new pure function** (`src/priority/scoring/classification.rs`):

```rust
/// Pure function: Check all applicable debt types using functional composition
pub fn classify_all_debt_types(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    coverage: Option<&TransitiveCoverage>,
) -> Vec<DebtType> {
    // Test functions get exclusive test debt (early return preserved for correctness)
    if func.is_test {
        return vec![classify_test_debt(func)];
    }

    // Compose all independent debt checks using iterator chains (functional style)
    let debt_types: Vec<DebtType> = vec![
        check_testing_gap(func, coverage),
        check_complexity_hotspot(func),
        check_dead_code_with_exclusions(
            func, call_graph, func_id,
            framework_exclusions, function_pointer_used_functions
        ),
    ]
    .into_iter()
    .flatten()  // Remove None values, keep Some(debt)
    .collect();

    // If no specific debt, classify by role (fallback)
    if debt_types.is_empty() {
        let role = classify_function_role(func, func_id, call_graph);
        classify_simple_function_risk(func, &role)
            .map(|debt| vec![debt])
            .unwrap_or_default()
    } else {
        debt_types
    }
}

/// Pure predicate: Check for testing gaps
fn check_testing_gap(
    func: &FunctionMetrics,
    coverage: Option<&TransitiveCoverage>,
) -> Option<DebtType> {
    coverage.and_then(|cov| {
        if has_testing_gap(cov.direct, func.is_test)
            || (cov.direct < 0.8 && func.cyclomatic > 5 && !cov.uncovered_lines.is_empty())
        {
            Some(DebtType::TestingGap {
                coverage: cov.direct,
                cyclomatic: func.cyclomatic,
                cognitive: func.cognitive,
            })
        } else {
            None
        }
    })
}

/// Pure predicate: Check for complexity hotspots
fn check_complexity_hotspot(func: &FunctionMetrics) -> Option<DebtType> {
    if is_complexity_hotspot_by_metrics(func.cyclomatic, func.cognitive) {
        Some(DebtType::ComplexityHotspot {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
            adjusted_cyclomatic: func.adjusted_complexity.map(|adj| adj.round() as u32),
        })
    } else {
        None
    }
}
```

**Replace existing function** with new implementation:
```rust
// Old signature (REMOVE):
// pub fn classify_debt_type_with_exclusions(...) -> DebtType

// New signature (REPLACE WITH):
pub fn classify_debt_type_with_exclusions(...) -> Vec<DebtType> {
    // Now returns Vec instead of single DebtType
    classify_all_debt_types(...)
}
```

**Note**: This is a breaking change. The function signature changes from `-> DebtType` to `-> Vec<DebtType>`. All call sites must be updated.

#### Phase 3: Update Debt Item Construction

**Replace existing function** `src/priority/scoring/construction.rs`:

```rust
// Old signature (REMOVE):
// pub fn create_unified_debt_item_with_aggregator_and_data_flow(...) -> Option<UnifiedDebtItem>

// New signature (REPLACE WITH):
/// Returns Vec<UnifiedDebtItem> - one per debt type found
pub fn create_unified_debt_item_with_aggregator_and_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    risk_analyzer: Option<&crate::risk::RiskAnalyzer>,
    project_path: &Path,
) -> Vec<UnifiedDebtItem> {  // Changed from Option<UnifiedDebtItem>
    let func_id = create_function_id(func);

    // Get coverage data once (reused for all debt types)
    let transitive_coverage = calculate_coverage_data(&func_id, func, call_graph, coverage);

    // Get all debt types for this function using new accumulation logic
    let debt_types = classify_all_debt_types(
        func, call_graph, &func_id,
        framework_exclusions, function_pointer_used_functions,
        transitive_coverage.as_ref()
    );

    // Create one UnifiedDebtItem per debt type (functional transformation)
    debt_types
        .into_iter()
        .filter_map(|debt_type| {
            analyze_single_debt_type(
                func, &func_id, debt_type, call_graph, coverage,
                debt_aggregator, data_flow, risk_analyzer, project_path,
                transitive_coverage.as_ref()
            )
        })
        .collect()
}

/// Helper: Analyze a single debt type and create item (pure function)
fn analyze_single_debt_type(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    debt_type: DebtType,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    risk_analyzer: Option<&crate::risk::RiskAnalyzer>,
    project_path: &Path,
    transitive_coverage: Option<&TransitiveCoverage>,
) -> Option<UnifiedDebtItem> {
    // Calculate unified score for this specific debt type
    let unified_score = calculate_score_for_debt_type(
        func, call_graph, coverage, debt_aggregator, data_flow, &debt_type
    );

    // Generate debt-type-specific recommendation
    let recommendation = generate_recommendation_for_debt_type(
        &debt_type, func, call_graph, transitive_coverage
    );

    // Build unified debt item
    Some(build_unified_debt_item_for_type(
        func, func_id, debt_type, unified_score, recommendation, /* ... */
    ))
}
```

**Note**: All call sites that previously expected `Option<UnifiedDebtItem>` must be updated to handle `Vec<UnifiedDebtItem>`.

#### Phase 4: Update Parallel Builder

**Modify** `src/builders/parallel_unified_analysis.rs`:

```rust
fn process_metrics_pipeline(...) -> Vec<UnifiedDebtItem> {
    metrics
        .par_iter()
        .progress_with(progress)
        .flat_map(|metric| {  // ← Changed from filter_map to flat_map
            self.process_single_metric(metric, test_only_functions, context)
        })
        .collect()
}

fn process_single_metric(...) -> Vec<UnifiedDebtItem> {  // ← Returns Vec now
    let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);
    let callee_count = self.call_graph.get_callees(&func_id).len();

    if !predicates::should_process_metric(metric, test_only_functions, callee_count) {
        return Vec::new();  // ← Empty vec instead of None
    }

    self.metric_to_debt_items(metric, context)  // ← Returns Vec
}

fn metric_to_debt_items(...) -> Vec<UnifiedDebtItem> {  // ← Returns Vec
    crate::builders::unified_analysis::create_debt_items_from_metric_with_aggregator(
        metric, context.call_graph, context.coverage_data,
        context.framework_exclusions, context.function_pointer_used_functions,
        context.debt_aggregator, Some(context.data_flow_graph),
        context.risk_analyzer, context.project_path,
    )
}
```

### Architecture Changes

**Before** (Single Debt):
```
FunctionMetrics → classify_debt_type → DebtType (singular)
                                      ↓
                  create_debt_item → Option<UnifiedDebtItem>
                                      ↓
                  Vec<UnifiedDebtItem> (max 1 per function)
```

**After** (Multi-Debt):
```
FunctionMetrics → classify_all_debt_types → Vec<DebtType>
                                            ↓
                  create_debt_items → Vec<UnifiedDebtItem>
                                            ↓
                  flat_map → Vec<UnifiedDebtItem> (multiple per function)
```

### Data Structures

No new data structures required. Existing types support multiple items:
- `Vec<UnifiedDebtItem>` already exists in `UnifiedAnalysis.items`
- Each `UnifiedDebtItem` has independent `debt_type` and `unified_score`
- Location reuse: same `Location { file, function, line }` for multiple items

### Breaking Changes

**Function Signature Changes**:

This is a **breaking change** that modifies existing function signatures:

1. **Classification layer** (`src/priority/scoring/classification.rs`):
   - `classify_debt_type_with_exclusions()` → returns `Vec<DebtType>` (was `DebtType`)
   - New helper: `classify_all_debt_types()` → returns `Vec<DebtType>` (internal implementation)

2. **Construction layer** (`src/priority/scoring/construction.rs`):
   - `create_unified_debt_item_with_aggregator_and_data_flow()` → returns `Vec<UnifiedDebtItem>` (was `Option<UnifiedDebtItem>`)

3. **Builder layer** (`src/builders/parallel_unified_analysis.rs`):
   - `process_single_metric()` → returns `Vec<UnifiedDebtItem>` (was `Option<UnifiedDebtItem>`)
   - Change `filter_map` to `flat_map` in pipeline

**Rationale**:
- Simpler implementation (single code path)
- More comprehensive debt detection (primary goal)
- Cleaner functional composition
- No runtime configuration needed

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/priority/scoring/classification.rs` - Add `classify_all_debt_types()`
- `src/priority/scoring/construction.rs` - Return `Vec<UnifiedDebtItem>`
- `src/builders/parallel_unified_analysis.rs` - Use `flat_map`
- `src/builders/unified_analysis.rs` - Sequential path compatibility
- `src/tui/results/app.rs` - Remove grouping (separate commit)

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**Test file**: `src/priority/scoring/classification.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulates_testing_gap_and_complexity() {
        let func = FunctionMetrics {
            cyclomatic: 25,
            cognitive: 30,
            is_test: false,
            // ...
        };
        let coverage = Some(TransitiveCoverage { direct: 0.0, ... });

        let debt_types = classify_all_debt_types(
            &func, &call_graph, &func_id,
            &exclusions, None, Some(&coverage)
        );

        assert_eq!(debt_types.len(), 2);
        assert!(debt_types.iter().any(|d| matches!(d, DebtType::TestingGap { .. })));
        assert!(debt_types.iter().any(|d| matches!(d, DebtType::ComplexityHotspot { .. })));
    }

    #[test]
    fn test_test_functions_only_get_test_debt() {
        let func = FunctionMetrics {
            is_test: true,
            cyclomatic: 25,  // Would be ComplexityHotspot if not test
            // ...
        };

        let debt_types = classify_all_debt_types(&func, ...);

        assert_eq!(debt_types.len(), 1);
        assert!(matches!(debt_types[0], DebtType::TestComplexityHotspot { .. }));
    }

    #[test]
    fn test_no_false_positives() {
        let func = FunctionMetrics {
            cyclomatic: 3,
            cognitive: 5,
            is_test: false,
            // Clean, simple function
        };
        let coverage = Some(TransitiveCoverage { direct: 0.9, ... });

        let debt_types = classify_all_debt_types(&func, ...);

        // Should only have Risk with 0.0 score or empty
        assert!(debt_types.is_empty() ||
                matches!(debt_types[0], DebtType::Risk { risk_score: 0.0, .. }));
    }
}
```

### Integration Tests

**Test file**: `tests/multi_debt_integration_test.rs`

```rust
#[test]
fn test_multi_debt_accumulation() {
    use std::collections::HashSet;

    // Create test project with function that has multiple issues:
    // - No test coverage (TestingGap)
    // - High complexity (ComplexityHotspot)
    // - No callers (DeadCode)
    let temp_dir = create_test_project_with_multi_debt_function();

    // Run analysis
    let analysis = analyze_project(&temp_dir);

    // Verify multiple debt items for same function
    let items_for_function: Vec<_> = analysis.items
        .iter()
        .filter(|item| item.location.function == "problematic_function")
        .collect();

    assert!(items_for_function.len() >= 2,
            "Expected multiple debt items for problematic_function, got {}",
            items_for_function.len());

    // Verify different debt types
    let debt_types: HashSet<_> = items_for_function
        .iter()
        .map(|item| std::mem::discriminant(&item.debt_type))
        .collect();

    assert!(debt_types.len() >= 2,
            "Expected different debt types for same function, got {}",
            debt_types.len());

    // Verify each debt type has independent score
    for item in &items_for_function {
        assert!(item.unified_score.total > 0.0,
                "Debt item should have non-zero score: {:?}", item.debt_type);
    }
}

#[test]
fn test_no_duplicate_debt_types() {
    let temp_dir = create_test_project();
    let analysis = analyze_project(&temp_dir);

    // Group items by function location
    let mut by_location: HashMap<_, Vec<_>> = HashMap::new();
    for item in &analysis.items {
        by_location.entry(&item.location).or_default().push(item);
    }

    // Verify no duplicates within same function
    for (location, items) in by_location {
        let debt_type_discriminants: Vec<_> = items
            .iter()
            .map(|item| std::mem::discriminant(&item.debt_type))
            .collect();

        let unique_count = debt_type_discriminants.iter().collect::<HashSet<_>>().len();
        assert_eq!(debt_type_discriminants.len(), unique_count,
                   "Found duplicate debt types for {:?}", location);
    }
}
```

### Performance Tests

**Benchmark**: `benches/debt_classification_benchmark.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn bench_debt_classification(c: &mut Criterion) {
    let metrics = load_test_metrics(10_000);  // 10K functions
    let context = create_test_context();

    let mut group = c.benchmark_group("debt_classification");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark multi-debt classification (now the only implementation)
    group.bench_function("classify_all_debt_types", |b| {
        b.iter(|| {
            metrics.iter().flat_map(|m| {
                classify_all_debt_types(
                    black_box(m),
                    &context.call_graph,
                    &create_function_id(m),
                    &context.exclusions,
                    None,
                    None,
                )
            }).collect::<Vec<_>>()
        });
    });

    // Benchmark full analysis pipeline
    group.bench_function("full_analysis_pipeline", |b| {
        b.iter(|| {
            analyze_metrics_with_multi_debt(black_box(&metrics), &context)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_debt_classification);
criterion_main!(benches);
```

**Performance baseline**: Establish baseline for future comparison (not comparing to old implementation).

**Automated performance test**:
```rust
#[test]
fn performance_acceptable() {
    use std::time::Instant;

    let metrics = load_large_codebase_metrics();  // 10K+ functions
    let context = create_test_context();

    // Measure analysis time
    let start = Instant::now();
    let results = analyze_metrics_with_multi_debt(&metrics, &context);
    let elapsed = start.elapsed();

    // Ensure reasonable performance
    let functions_per_second = metrics.len() as f64 / elapsed.as_secs_f64();

    eprintln!("Performance: {:.0} functions/second", functions_per_second);

    // Should process at least 1000 functions per second
    assert!(functions_per_second > 1000.0,
            "Performance too slow: {:.0} functions/second (minimum: 1000)",
            functions_per_second);

    // Verify results are comprehensive
    assert!(results.len() >= metrics.len(),
            "Multi-debt should produce comprehensive results");
}
```

### User Acceptance

1. **Validation on debtmap codebase**:
   - Implement multi-debt changes
   - Update all call sites to handle `Vec<DebtType>` and `Vec<UnifiedDebtItem>`
   - Run `cargo run -- analyze .`
   - Verify functions with multiple issues show multiple entries
   - **Expected**: Debt item count increases 20-50% (more comprehensive analysis)

2. **TUI verification**:
   - Launch TUI after implementation
   - Navigate to functions with multiple debt types
   - Verify all debt types appear as separate entries
   - Verify each entry has correct score and recommendation
   - Verify navigation and filtering still work correctly

3. **Compilation and testing**:
   - All code compiles without warnings: `cargo build --all-features`
   - All tests pass: `cargo test --all-features`
   - No clippy warnings: `cargo clippy --all-targets --all-features`
   - Performance benchmarks complete successfully

4. **Output validation**:
   - JSON output remains valid (schema unchanged)
   - CLI output readable with multiple items per function
   - Sorting and filtering work correctly with multi-debt items

## Documentation Requirements

### Code Documentation

1. **Module-level docs** in `src/priority/scoring/classification.rs`:
   ```rust
   //! Debt classification using functional composition.
   //!
   //! This module accumulates all applicable debt types for each function using
   //! pure predicates and iterator chains, following debtmap's functional-first
   //! architecture.
   //!
   //! # Breaking Change
   //! Functions now return `Vec<DebtType>` instead of single `DebtType`,
   //! enabling comprehensive multi-debt detection per function.
   ```

2. **Function docs** for `classify_all_debt_types()`:
   ```rust
   /// Classify all applicable debt types for a function using functional composition.
   ///
   /// This function accumulates all independent debt classifications rather than
   /// stopping at the first match, providing comprehensive technical debt assessment.
   ///
   /// # Independent Debt Checks
   /// - `check_testing_gap()`: Coverage-based testing debt
   /// - `check_complexity_hotspot()`: Cyclomatic/cognitive complexity
   /// - `check_dead_code_with_exclusions()`: Unused code detection
   ///
   /// # Test Function Exception
   /// Test functions (`func.is_test == true`) only return test-specific debt types
   /// to avoid noise from legitimate test complexity.
   ///
   /// # Returns
   /// A vector of all applicable debt types. May contain 0-3 items depending on
   /// the function's issues.
   ///
   /// # Examples
   /// ```rust
   /// let debt_types = classify_all_debt_types(
   ///     &func, &call_graph, &func_id,
   ///     &exclusions, None, Some(&coverage)
   /// );
   /// // Returns: vec![TestingGap, ComplexityHotspot, DeadCode]
   /// ```
   ```

3. **Breaking change docs** in inline comments:
   ```rust
   // BREAKING CHANGE (Spec 228):
   // This function now returns Vec<DebtType> instead of DebtType.
   // All call sites must be updated to handle multiple debt types.
   ```

### User Documentation

1. **Update CHANGELOG.md**:
   ```markdown
   ## [Unreleased]

   ### Added
   - Multi-debt type accumulation: Functions now report all applicable debt types
   - Pure functional predicates for independent debt classification
   - Comprehensive test coverage for multi-debt scenarios

   ### Changed
   - **BREAKING**: `classify_debt_type_with_exclusions()` now returns `Vec<DebtType>` (was `DebtType`)
   - **BREAKING**: `create_unified_debt_item_with_aggregator_and_data_flow()` now returns `Vec<UnifiedDebtItem>` (was `Option<UnifiedDebtItem>`)
   - Debt classification refactored to use pure predicates and iterator chains
   - Analysis now detects 20-50% more debt items (more comprehensive)

   ### Performance
   - Multi-debt classification maintains high performance (>1000 functions/second)
   - Functional composition enables compiler optimizations

   ### Migration Guide
   - Update call sites from `.map()` to `.flat_map()` when processing debt items
   - Handle `Vec<DebtType>` instead of single `DebtType` in classification
   - Tests may need updating for increased debt item counts
   ```

2. **Update README.md**:
   ```markdown
   ### Comprehensive Debt Analysis

   Debtmap identifies all independent technical debt issues per function:
   - A complex function with no tests reports both `ComplexityHotspot` and `TestingGap`
   - Unused code with high complexity shows both `DeadCode` and `ComplexityHotspot`
   - Each debt type has independent scoring and actionable recommendations

   This multi-debt approach provides more comprehensive technical debt assessment,
   helping teams prioritize fixes based on all issues, not just the most severe.
   ```

### Architecture Updates

**Update ARCHITECTURE.md**:

```markdown
## Debt Classification

### Multi-Debt Architecture (Spec 228)

Debtmap uses a **multi-debt accumulation** approach that identifies all applicable
debt types for each function, providing comprehensive technical debt assessment.

**Key Functions**:
- `classify_all_debt_types()` → `Vec<DebtType>` (internal helper)
- `classify_debt_type_with_exclusions()` → `Vec<DebtType>` (public API)
- `create_unified_debt_item_*()` → `Vec<UnifiedDebtItem>` (construction)

**Architecture Principles**:
- Uses functional composition with iterator chains
- Accumulates all independent debt validations
- Each function can report 0-3+ debt items
- Pure predicates for independent classification

### Functional Composition Pattern

The classification follows debtmap's functional-first architecture:

```rust
// Pure predicates (independent validations)
check_testing_gap()          → Option<DebtType>
check_complexity_hotspot()   → Option<DebtType>
check_dead_code_*()          → Option<DebtType>

// Compose into accumulation using iterator chains
vec![check1(), check2(), check3()]
    .into_iter()
    .flatten()  // Collect all Some values
    .collect()  // → Vec<DebtType>
```

### Data Flow

```
FunctionMetrics
    ↓
classify_all_debt_types() → Vec<DebtType>
    ↓
create_unified_debt_item_*() → Vec<UnifiedDebtItem>
    ↓
flat_map in parallel builder
    ↓
Vec<UnifiedDebtItem> (comprehensive results)
```

### Breaking Change Note

Prior to Spec 228, functions returned single debt items (`DebtType` and
`Option<UnifiedDebtItem>`). Now all functions return vectors to support
multi-debt accumulation.
```

## Implementation Notes

### Functional Programming Principles

1. **Pure Functions**: All classification functions remain pure
   - No side effects
   - Deterministic output
   - Easily testable

2. **Composition**: Build complex behavior from simple predicates
   - `check_testing_gap()` - pure predicate
   - `check_complexity_hotspot()` - pure predicate
   - `check_dead_code_with_exclusions()` - pure predicate
   - Compose into `classify_all_debt_types()`

3. **Immutability**: No mutations in classification logic
   - Build `Vec<DebtType>` through accumulation
   - Each debt type independently determined

### Performance Considerations

1. **Parallel Processing**: Use `flat_map` for natural parallelization
2. **Shared Computations**: Calculate coverage once, reuse for all debt types
3. **Early Exit for Tests**: Maintain test function early return (performance)

### Edge Cases

1. **Function with no debt**:
   - If no specific debt checks match: call `classify_simple_function_risk()`
   - If function is truly clean: returns empty `Vec`
   - If function has minimal risk factors: returns `vec![Risk { score: 0.0, ... }]`
   - This preserves existing behavior while supporting multi-debt pattern

2. **Test functions**:
   - Always exclusive (`if func.is_test { return vec![classify_test_debt(func)]; }`)
   - Never mixed with production debt types
   - Early return preserved for correctness

3. **Framework-excluded functions**:
   - Handled by existing exclusion logic in `check_dead_code_with_exclusions()`
   - Exclusions apply at predicate level, not at accumulation level

4. **Zero-score debt types**:
   - Construction phase filters items with `score < threshold`
   - Uses `.filter_map()` to drop low-scoring items
   - Prevents clutter in output

5. **Duplicate debt types**:
   - Not possible - each predicate checks distinct conditions
   - Testing gap ≠ complexity hotspot ≠ dead code
   - If predicates overlap, fix predicate logic (bug)

## Migration and Compatibility

### Breaking Changes

**This is a breaking change** that modifies existing function signatures:

1. **`classify_debt_type_with_exclusions()`**: Returns `Vec<DebtType>` (was `DebtType`)
2. **`create_unified_debt_item_with_aggregator_and_data_flow()`**: Returns `Vec<UnifiedDebtItem>` (was `Option<UnifiedDebtItem>`)
3. **All call sites** must be updated to handle vectors

### Migration Path

1. **Update classification layer**:
   - Extract pure predicates: `check_testing_gap()`, `check_complexity_hotspot()`, etc.
   - Implement `classify_all_debt_types()` using iterator composition
   - Update `classify_debt_type_with_exclusions()` to return `Vec<DebtType>`

2. **Update construction layer**:
   - Modify `create_unified_debt_item_*()` to return `Vec<UnifiedDebtItem>`
   - Update helper functions to handle multiple debt types
   - Ensure each debt type gets independent scoring

3. **Update builder layer**:
   - Change `.filter_map()` to `.flat_map()` in parallel builder
   - Update sequential builder similarly
   - Handle `Vec<UnifiedDebtItem>` in pipeline

4. **Update tests**:
   - Modify tests expecting single debt items
   - Add tests for multi-debt scenarios
   - Update assertion counts (expect 20-50% more items)

5. **Validate**:
   - Run full test suite
   - Run on debtmap codebase
   - Verify performance benchmarks
   - Check TUI still works correctly

### Compatibility Guarantees

- **Data structures**: No changes to `DebtType` or `UnifiedDebtItem` structs
- **JSON schema**: Unchanged (items array already exists)
- **CLI output format**: Unchanged (each debt item displays independently)
- **TUI rendering**: Works with existing code (just more items)
- **Scoring algorithm**: No change per debt type

### Call Site Update Examples

**Before**:
```rust
// Parallel builder
metrics.par_iter()
    .filter_map(|m| process_single_metric(m))  // Returns Option
    .collect()

// Direct classification
let debt_type = classify_debt_type_with_exclusions(...);  // Single DebtType
match debt_type {
    DebtType::TestingGap { .. } => { /* ... */ }
    _ => {}
}
```

**After**:
```rust
// Parallel builder
metrics.par_iter()
    .flat_map(|m| process_single_metric(m))  // Returns Vec
    .collect()

// Direct classification
let debt_types = classify_debt_type_with_exclusions(...);  // Vec<DebtType>
for debt_type in debt_types {
    match debt_type {
        DebtType::TestingGap { .. } => { /* ... */ }
        _ => {}
    }
}
```

## Success Metrics

### Quantitative

1. **Comprehensiveness**: 20-50% increase in total debt items reported
2. **Performance**: Maintains >1000 functions/second throughput
3. **False positive rate**: No increase (same predicates, better coverage)
4. **Test coverage**: > 95% for refactored code paths
5. **Code quality**: All clippy checks pass

### Qualitative

1. **Code clarity**: Improved through functional composition and pure predicates
2. **Maintainability**: Simpler logic (no early returns, clear separation)
3. **Comprehensiveness**: Users see all issues per function, not just first

## Related Specifications

- **Spec 223** (TUI Grouped Debt Display): Consider removal (separate spec recommended)
- **Spec 180** (Tier-based Filtering): Compatible - filters apply per debt item
- **Spec 193** (Minimum Score Threshold): Compatible - thresholds apply per debt item

## Future Enhancements

### Potential Follow-ups

1. **Smart Grouping** (post multi-debt):
   - Re-implement grouping now that multi-debt exists
   - Use actual location-based grouping
   - Performance-optimized with caching

2. **Debt Interaction Analysis**:
   - Detect when debt types compound (e.g., untested + complex = extra risky)
   - Apply interaction multipliers
   - Prioritize compound debt higher

3. **Configurable Debt Type Weights**:
   - Let users prioritize certain debt types
   - Adjust scores based on project needs
   - Support config file: `debtmap.toml`

---

**Status**: Ready for implementation
**Estimated Effort**: 2-3 days (simpler without backward compatibility)

**Breakdown**:
- Day 1: Extract pure predicates, refactor `classify_all_debt_types()` and `classify_debt_type_with_exclusions()`
- Day 2: Update construction and builder layers, update all call sites (`.filter_map()` → `.flat_map()`)
- Day 3: Write tests, update existing tests, performance validation, documentation

**Risk Level**: Medium (breaking change, but type system enforces correctness)

**Mitigation**:
- Type system will catch all call sites that need updating
- Comprehensive test suite validates behavior
- Performance benchmarks ensure no regression
