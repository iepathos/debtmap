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

### Alignment with Stillwater Philosophy

From `../stillwater/PHILOSOPHY.md` lines 85-116:

> **Result (fail fast)**: Sequential operations where later steps depend on earlier
> **Validation (fail completely)**: Independent validations that should all be checked

Debt types are **independent** - they don't depend on each other. Current architecture incorrectly uses "fail fast" pattern when it should use "fail completely" (accumulation).

## Objective

Transform debt classification from **early-return (single debt)** to **accumulation (multiple debts)** pattern, following functional programming principles for independent validations.

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
   - No side effects or mutations
   - Composable and testable in isolation
   - Follow Stillwater "fail completely" pattern

4. **FR4: Test Function Exception**
   - Test functions remain exclusive (only test-specific debt)
   - Prevents noise from test code complexity
   - Returns early for `func.is_test == true`

### Non-Functional Requirements

1. **NFR1: Performance**
   - No significant performance degradation
   - Parallel processing remains effective
   - Use `flat_map` instead of nested loops

2. **NFR2: Backward Compatibility**
   - Existing debt scoring logic unchanged
   - Same score calculation per debt type
   - Configuration flag for gradual rollout

3. **NFR3: Code Quality**
   - Follow debtmap functional programming guidelines
   - Pure functions under 20 lines
   - Single responsibility per function
   - Comprehensive test coverage

## Acceptance Criteria

- [ ] **AC1**: Function with TestingGap + ComplexityHotspot creates 2 `UnifiedDebtItem` entries
- [ ] **AC2**: Each debt item has correct debt-type-specific score and recommendation
- [ ] **AC3**: Test functions still only report test-specific debt (no mixing)
- [ ] **AC4**: TUI displays all debt items for a function (may appear as multiple entries)
- [ ] **AC5**: All existing tests pass (backward compatibility)
- [ ] **AC6**: New integration test validates multi-debt scenarios
- [ ] **AC7**: Performance impact < 5% on large codebases (10K+ functions)
- [ ] **AC8**: Configuration flag `DEBTMAP_ACCUMULATE_DEBT=true` controls behavior

## Technical Details

### Implementation Approach

#### Phase 1: Remove Grouping Feature (Immediate Performance Fix)

**Files to modify**:
- `src/tui/results/app.rs`: Change `show_grouped: false` (line 149)
- Document removal in commit message

**Expected impact**: 50% TUI rendering speedup with zero functionality loss.

#### Phase 2: Refactor Classification to Accumulation Pattern

**Create new pure function** (`src/priority/scoring/classification.rs`):

```rust
/// Pure function: Check all applicable debt types (Stillwater "fail completely" pattern)
pub fn classify_all_debt_types(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    coverage: Option<&TransitiveCoverage>,
) -> Vec<DebtType> {
    let mut debt_types = Vec::new();

    // Test functions get exclusive test debt (early return)
    if func.is_test {
        debt_types.push(classify_test_debt(func));
        return debt_types;
    }

    // Check all independent debt types
    if let Some(gap) = check_testing_gap(func, coverage) {
        debt_types.push(gap);
    }

    if let Some(hotspot) = check_complexity_hotspot(func) {
        debt_types.push(hotspot);
    }

    if let Some(dead) = check_dead_code_with_exclusions(
        func, call_graph, func_id,
        framework_exclusions, function_pointer_used_functions
    ) {
        debt_types.push(dead);
    }

    // If no specific debt, classify by role (fallback)
    if debt_types.is_empty() {
        let role = classify_function_role(func, func_id, call_graph);
        if let Some(simple) = classify_simple_function_risk(func, &role) {
            debt_types.push(simple);
        }
    }

    debt_types
}
```

**Keep existing function** for backward compatibility:
```rust
pub fn classify_debt_type_with_exclusions(...) -> DebtType {
    if std::env::var("DEBTMAP_ACCUMULATE_DEBT").is_ok() {
        // New behavior: return first of accumulated types
        classify_all_debt_types(...).into_iter().next().unwrap_or(...)
    } else {
        // Legacy behavior: early returns
        // ... existing code ...
    }
}
```

#### Phase 3: Update Debt Item Construction

**Modify** `src/priority/scoring/construction.rs`:

```rust
/// Returns Vec<UnifiedDebtItem> instead of Option<UnifiedDebtItem>
pub fn create_unified_debt_items_with_aggregator_and_data_flow(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    risk_analyzer: Option<&crate::risk::RiskAnalyzer>,
    project_path: &Path,
) -> Vec<UnifiedDebtItem> {
    let func_id = create_function_id(func);

    // Get coverage data once
    let transitive_coverage = calculate_coverage_data(&func_id, func, call_graph, coverage);

    // Get all debt types for this function
    let debt_types = if std::env::var("DEBTMAP_ACCUMULATE_DEBT").is_ok() {
        classify_all_debt_types(
            func, call_graph, &func_id,
            framework_exclusions, function_pointer_used_functions,
            transitive_coverage.as_ref()
        )
    } else {
        // Legacy single-debt behavior
        vec![classify_debt_type_with_exclusions(
            func, call_graph, &func_id,
            framework_exclusions, function_pointer_used_functions,
            transitive_coverage.as_ref()
        )]
    };

    // Create one item per debt type
    debt_types.into_iter()
        .filter_map(|debt_type| {
            analyze_single_debt_type(
                func, &func_id, debt_type, call_graph, coverage,
                debt_aggregator, data_flow, risk_analyzer, project_path,
                transitive_coverage.as_ref()
            )
        })
        .collect()
}

/// Helper: Analyze a single debt type and create item
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

### Configuration

**Environment Variable**:
```bash
# Enable multi-debt accumulation
export DEBTMAP_ACCUMULATE_DEBT=true

# Disable (legacy single-debt behavior)
unset DEBTMAP_ACCUMULATE_DEBT
```

**Rationale**:
- Gradual rollout to production
- Easy A/B testing
- Zero code changes to toggle behavior
- Can be promoted to config file later

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
fn test_multi_debt_end_to_end() {
    std::env::set_var("DEBTMAP_ACCUMULATE_DEBT", "true");

    // Create test project with function that has multiple issues
    let temp_dir = create_test_project_with_multi_debt_function();

    // Run analysis
    let analysis = analyze_project(&temp_dir);

    // Verify multiple debt items for same function
    let items_for_function: Vec<_> = analysis.items
        .iter()
        .filter(|item| item.location.function == "problematic_function")
        .collect();

    assert!(items_for_function.len() >= 2,
            "Expected multiple debt items for problematic_function");

    // Verify different debt types
    let debt_types: HashSet<_> = items_for_function
        .iter()
        .map(|item| std::mem::discriminant(&item.debt_type))
        .collect();

    assert!(debt_types.len() >= 2,
            "Expected different debt types for same function");

    std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");
}
```

### Performance Tests

**Benchmark**: `benches/multi_debt_benchmark.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_single_vs_multi_debt(c: &mut Criterion) {
    let metrics = load_test_metrics(10_000);  // 10K functions

    let mut group = c.benchmark_group("debt_classification");

    group.bench_function("single_debt", |b| {
        std::env::remove_var("DEBTMAP_ACCUMULATE_DEBT");
        b.iter(|| classify_all_metrics(black_box(&metrics)));
    });

    group.bench_function("multi_debt", |b| {
        std::env::set_var("DEBTMAP_ACCUMULATE_DEBT", "true");
        b.iter(|| classify_all_metrics(black_box(&metrics)));
    });

    group.finish();
}

criterion_group!(benches, bench_single_vs_multi_debt);
criterion_main!(benches);
```

**Acceptance threshold**: Multi-debt must be within 5% of single-debt performance.

### User Acceptance

1. **Validation on debtmap codebase**:
   - Run `DEBTMAP_ACCUMULATE_DEBT=true cargo run -- analyze .`
   - Verify functions with multiple issues show multiple entries
   - Compare output count: should increase (more comprehensive)

2. **TUI verification**:
   - Launch TUI with multi-debt enabled
   - Navigate to function with multiple debt types
   - Verify all debt types appear as separate entries
   - Verify each entry has correct score and recommendation

## Documentation Requirements

### Code Documentation

1. **Module-level docs** in `src/priority/scoring/classification.rs`:
   ```rust
   //! Debt classification using functional composition.
   //!
   //! This module implements the "fail completely" pattern from Stillwater
   //! philosophy, accumulating all applicable debt types rather than
   //! stopping at the first match.
   ```

2. **Function docs** for `classify_all_debt_types()`:
   - Document independent debt checks
   - Explain test function exception
   - Show example usage

3. **Configuration docs** in inline comments:
   ```rust
   // Control multi-debt accumulation via environment variable:
   // DEBTMAP_ACCUMULATE_DEBT=true  → Accumulate all debt types
   // (unset)                       → Legacy single-debt behavior
   ```

### User Documentation

1. **Update README.md**:
   ```markdown
   ### Multiple Debt Types Per Function

   By default, debtmap reports the highest-priority debt type per function.
   To see ALL debt types a function suffers from:

   ```bash
   DEBTMAP_ACCUMULATE_DEBT=true debtmap analyze .
   ```

   This will create separate debt items for each issue (e.g., both
   "untested" and "complex" if applicable).
   ```

2. **Update CHANGELOG.md**:
   ```markdown
   ## [Unreleased]

   ### Added
   - Multi-debt type accumulation: Functions can now report multiple
     independent debt types (TestingGap + ComplexityHotspot + DeadCode)
     when `DEBTMAP_ACCUMULATE_DEBT=true`

   ### Removed
   - TUI grouping feature (50% performance improvement, feature never
     worked as intended)
   ```

### Architecture Updates

**Update ARCHITECTURE.md**:

```markdown
## Debt Classification

### Single vs Multi-Debt Mode

**Single-Debt (Legacy)**:
- Uses early-return pattern
- Reports highest-priority debt type only
- Faster for large codebases
- Enabled when `DEBTMAP_ACCUMULATE_DEBT` is unset

**Multi-Debt (Recommended)**:
- Uses accumulation pattern (Stillwater "fail completely")
- Reports all applicable debt types independently
- More comprehensive debt assessment
- Enabled with `DEBTMAP_ACCUMULATE_DEBT=true`

### Debt Type Priority (Single-Debt Mode Only)

1. TestingGap (highest)
2. ComplexityHotspot
3. DeadCode
4. Risk (fallback)

In multi-debt mode, all applicable types are reported without priority filtering.
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

1. **Function with no debt**: Returns empty vec or single `Risk { score: 0.0 }`
2. **Test functions**: Always exclusive, never mixed with production debt
3. **Framework-excluded functions**: Handled by existing exclusion logic
4. **Zero-score debt types**: Filter in construction phase if score < threshold

## Migration and Compatibility

### Breaking Changes

**None**. Multi-debt is opt-in via environment variable.

### Migration Path

1. **Phase 1**: Release with `DEBTMAP_ACCUMULATE_DEBT=false` (default)
   - Users get current behavior
   - No surprises

2. **Phase 2**: Dogfood on debtmap codebase
   - Test with `DEBTMAP_ACCUMULATE_DEBT=true`
   - Validate performance and output quality

3. **Phase 3**: Beta release
   - Announce feature to users
   - Gather feedback
   - Tune thresholds if needed

4. **Phase 4**: Make default (future release)
   - Flip default to `true`
   - Keep opt-out via `DEBTMAP_ACCUMULATE_DEBT=false`

### Compatibility Guarantees

- **CLI output format**: No change (each debt item is independent)
- **JSON schema**: No change (same `UnifiedDebtItem` structure)
- **TUI rendering**: Works with existing code (just more items)
- **Scoring algorithm**: No change per debt type

## Success Metrics

### Quantitative

1. **Coverage improvement**: % increase in total debt items reported
2. **Performance overhead**: < 5% increase in analysis time
3. **False positive rate**: No increase (same predicates)
4. **Test coverage**: > 95% for new code paths

### Qualitative

1. **User feedback**: Positive reception for more comprehensive analysis
2. **Code quality**: Maintains functional programming principles
3. **Maintainability**: No increase in complexity (simpler logic)

## Related Specifications

- **Spec 223** (TUI Grouped Debt Display): Will be deprecated by this spec
- **Spec 180** (Tier-based Filtering): Works with multi-debt (filter per item)
- **Spec 193** (Minimum Score Threshold): Works with multi-debt (filter per item)

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
**Estimated Effort**: 2-3 days (including tests and documentation)
**Risk Level**: Low (backward compatible, opt-in feature)
