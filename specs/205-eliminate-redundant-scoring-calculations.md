---
number: 205
title: Eliminate Redundant Scoring Calculations in Debt Item Construction
category: performance
priority: high
status: draft
dependencies: []
created: 2025-12-15
---

# Specification 205: Eliminate Redundant Scoring Calculations in Debt Item Construction

**Category**: performance
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Performance profiling of debt scoring on large codebases (Zed: ~10k functions) shows that `create_unified_debt_item_with_aggregator_and_data_flow` contains significant redundant computation. The same expensive calculations are performed multiple times for functions that have multiple debt types.

### Current Problem

When a function has multiple debt types (e.g., both `TestingGap` AND `ComplexityHotspot`), the code iterates through each debt type and calls `analyze_single_debt_type()` for EACH. However:

1. **Unified score calculation** (`calculate_unified_priority_with_debt()`) is **independent of debt type** but called once per debt type
2. **Function role classification** (`classify_function_role()`) is called 2-3x per function:
   - In `classify_all_debt_types()` fallback path
   - In `analyze_single_debt_type()`
   - Inside `calculate_unified_priority_with_debt()`
3. **Entropy details** calculated twice:
   - In `calculate_unified_priority_with_debt()`
   - In `build_unified_debt_item()`

### Impact

For Zed codebase with ~10k functions:
- ~30% of functions have 2+ debt types → 3k redundant score calculations
- `classify_function_role()` called ~20k times when ~10k would suffice
- Each `calculate_unified_priority_with_debt()` does ~15 sub-calculations

## Objective

Eliminate redundant calculations by computing shared values once and reusing them across all debt types for the same function.

## Requirements

### Functional Requirements

1. **Compute unified score once per function** - Calculate the score before iterating through debt types
2. **Cache function role** - Compute once and pass to all dependent functions
3. **Compute entropy once** - Calculate at the start and reuse
4. **Preserve correctness** - All debt items must produce identical results to current implementation

### Non-Functional Requirements

1. **Performance improvement** - Target 30-50% reduction in scoring phase time
2. **No breaking changes** - Public APIs unchanged
3. **Memory neutral** - Additional caching shouldn't significantly increase memory

## Acceptance Criteria

- [ ] `classify_function_role()` called exactly once per function
- [ ] `calculate_unified_priority_with_debt()` called exactly once per function
- [ ] `calculate_entropy_details()` called exactly once per function
- [ ] All tests pass
- [ ] Benchmark shows measurable improvement on large codebase

## Technical Details

### Current Flow (Problematic)

```
for each function:
    debt_types = classify_debt_type_with_exclusions(...)  // may call classify_function_role

    for each debt_type:
        // REDUNDANT: Same for all debt_types!
        score = calculate_unified_priority_with_debt(...)  // calls classify_function_role inside
        role = classify_function_role(...)                 // called again
        recommendation = generate_recommendation(...)
        impact = calculate_expected_impact(...)

        // ALSO REDUNDANT: entropy computed twice
        build_unified_debt_item(...)  // computes entropy_details again
```

### Proposed Flow (Optimized)

```
for each function:
    // === COMPUTE SHARED VALUES ONCE ===
    func_id = create_function_id(func)
    coverage = calculate_coverage_data(...)
    role = classify_function_role(func, func_id, call_graph)  // ONCE
    score = calculate_unified_priority_with_role(func, ..., role)  // ONCE, pass role
    deps = extract_dependency_metrics(...)
    entropy = calculate_entropy_details(func)  // ONCE
    context_analysis = context_detector.detect_context(...)  // ONCE

    // === DEBT TYPE ITERATION (minimal work) ===
    debt_types = classify_debt_types_with_role(func, ..., role)  // pass pre-computed role

    for each debt_type:
        // Only debt-type-specific work
        recommendation = generate_recommendation(func, debt_type, role, score, ...)
        impact = calculate_expected_impact(func, debt_type, score)

        item = create_debt_item_from_precomputed(
            func, debt_type,
            score,    // reuse
            role,     // reuse
            deps,     // reuse
            entropy,  // reuse
            context_analysis,  // reuse
            ...
        )
```

### Implementation Changes

#### 1. Extract Pre-computed Context Struct

```rust
/// Pre-computed values shared across all debt types for a function
struct FunctionScoringContext {
    func_id: FunctionId,
    role: FunctionRole,
    unified_score: UnifiedScore,
    transitive_coverage: Option<TransitiveCoverage>,
    deps: DependencyMetrics,
    entropy_details: Option<EntropyDetails>,
    context_analysis: ContextAnalysis,
}

impl FunctionScoringContext {
    /// Compute all shared values once
    fn compute(
        func: &FunctionMetrics,
        call_graph: &CallGraph,
        coverage: Option<&LcovData>,
        debt_aggregator: &DebtAggregator,
        data_flow: Option<&DataFlowGraph>,
        context_detector: &ContextDetector,
    ) -> Self {
        let func_id = create_function_id(func);
        let role = classify_function_role(func, &func_id, call_graph);

        // Calculate score with pre-computed role
        let unified_score = calculate_unified_priority_with_role(
            func, call_graph, coverage, debt_aggregator, data_flow, &role
        );

        // Other shared computations
        let transitive_coverage = calculate_coverage_data(&func_id, func, call_graph, coverage);
        let deps = extract_dependency_metrics(func, &func_id, call_graph);
        let entropy_details = calculate_entropy_details(func);
        let context_analysis = context_detector.detect_context(func, &func.file);

        Self { func_id, role, unified_score, transitive_coverage, deps, entropy_details, context_analysis }
    }
}
```

#### 2. Modify `calculate_unified_priority_with_debt` to Accept Role

```rust
/// New function that accepts pre-computed role
pub fn calculate_unified_priority_with_role(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    organization_issues: Option<f64>,
    debt_aggregator: Option<&DebtAggregator>,
    has_coverage_data: bool,
    role: &FunctionRole,  // NEW: Accept pre-computed role
) -> UnifiedScore {
    // Skip the classify_function_role() call, use passed role
    // ... rest of implementation
}
```

#### 3. Simplify Main Construction Function

```rust
pub fn create_unified_debt_item_with_aggregator_and_data_flow(
    func: &FunctionMetrics,
    // ... other params
) -> Vec<UnifiedDebtItem> {
    // Step 1: Compute all shared values once
    let ctx = FunctionScoringContext::compute(
        func, call_graph, coverage, debt_aggregator, data_flow, context_detector
    );

    // Step 2: Get debt types (using pre-computed role if needed)
    let debt_types = classify_debt_type_with_exclusions(
        func, call_graph, &ctx.func_id,
        framework_exclusions, function_pointer_used_functions,
        ctx.transitive_coverage.as_ref(),
    );

    // Step 3: Create items (minimal per-debt-type work)
    debt_types
        .into_iter()
        .filter_map(|debt_type| {
            create_debt_item_from_context(func, debt_type, &ctx, ...)
        })
        .collect()
}
```

### Files to Modify

1. **`src/priority/scoring/construction.rs`** - Main refactoring
2. **`src/priority/unified_scorer.rs`** - Add `calculate_unified_priority_with_role()`
3. **`src/priority/scoring/classification.rs`** - Ensure role can be passed in

## Testing Strategy

- **Unit Tests**: Verify identical outputs before/after
- **Integration Tests**: Run full analysis on test codebases
- **Benchmark**: Compare scoring time on Zed before/after
- **Regression**: Compare full debt report output

## Documentation Requirements

- **Code Comments**: Document the caching strategy
- **Spec Updates**: Reference this spec in modified files

## Implementation Notes

- Start by adding the `FunctionScoringContext` struct without changing behavior
- Then incrementally move computations into it
- Run tests after each step to verify correctness
- Consider making the old functions delegate to the new optimized path

## Risk Assessment

- **Low Risk**: Pure refactoring with clear correctness criteria
- **Mitigation**: Comprehensive test coverage exists

## Estimated Impact

For Zed codebase (~10k functions):
- Current: ~3 role classifications per function × 10k = 30k calls
- After: 1 role classification per function × 10k = 10k calls
- Similar savings for score calculation, entropy, etc.

Expected speedup: **30-50% reduction in scoring phase time**
