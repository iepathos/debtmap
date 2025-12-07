---
number: 248
title: File-Level Git Context Analysis for God Objects
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-07
---

# Specification 248: File-Level Git Context Analysis for God Objects

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Currently, god objects in the TUI detail view often show blank "Git Context" pages. This occurs because:

1. **God objects aggregate contextual risk from member functions** (`src/priority/god_object_aggregation.rs:156-190`)
2. **Member functions only get contextual risk if**:
   - They pass complexity/coverage thresholds to become technical debt items
   - A risk analyzer was provided during analysis
3. **If members lack contextual risk, aggregation returns `None`** (line 163-164)
4. **Result**: TUI displays "No git context data available"

### Current Data Flow

```
God Object Detection
  → Extract member functions (only debt items)
    → Aggregate contextual_risk from members
      → If no members or no contextual_risk → None
        → TUI shows "No git context data available"
```

### The Problem

God objects are **file-level** architectural issues, not just aggregations of complex functions. A god object file may contain many simple functions that don't individually become debt items, yet the **file itself** has important git history:

- High change frequency indicating instability
- Many bug fixes showing quality issues
- Multiple contributors suggesting unclear ownership
- Recent modifications indicating active churn

This file-level git context is **critical** for understanding the risk and priority of refactoring the god object, but it's currently missing.

## Objective

Implement direct file-level git context analysis for god objects to ensure the Git Context page in TUI always displays relevant data (when in a git repository).

**Success Criteria**:
- God objects display git context data even when member functions don't have contextual risk
- File-level git metrics (change frequency, bug fixes, stability) are shown for god objects
- Direct file analysis takes precedence over member aggregation
- No performance degradation during analysis

## Requirements

### Functional Requirements

1. **Direct File Analysis**
   - Analyze git history for the god object's file path directly
   - Use the risk analyzer's `ContextAggregator` to gather file-level git context
   - Store result as `ContextualRisk` for the god object debt item

2. **Risk Analyzer Propagation**
   - Pass `risk_analyzer` parameter through to `apply_file_analysis_results`
   - Make risk analyzer available during god object item creation
   - Maintain optional nature (work without risk analyzer)

3. **Fallback Strategy**
   - **Primary**: Use direct file-level git context analysis
   - **Secondary**: Fall back to member aggregation if direct analysis fails
   - **Tertiary**: Use `None` if both approaches fail

4. **Data Flow Integration**
   - Integrate with existing `GitHistoryProvider` via `ContextAggregator`
   - Reuse batched git history for performance (no extra git subprocess calls)
   - Store contextual risk in `UnifiedDebtItem.contextual_risk` field

### Non-Functional Requirements

1. **Performance**
   - Leverage existing batched git history (O(1) HashMap lookup)
   - No additional git subprocess calls per file
   - Minimal overhead during god object creation

2. **Maintainability**
   - Keep pure function signatures clean
   - Minimize parameter passing complexity
   - Follow existing functional patterns

3. **Compatibility**
   - Work seamlessly when git repository is not available
   - Handle missing risk analyzer gracefully
   - Maintain backward compatibility with existing aggregation

## Acceptance Criteria

- [ ] God objects display git context in TUI detail view when in git repository
- [ ] File-level git context shows: change frequency, bug density, age, author count
- [ ] Direct file analysis is used as primary source for god object contextual risk
- [ ] Member aggregation still works as fallback when direct analysis unavailable
- [ ] No performance regression in analysis pipeline
- [ ] Git Context page shows "No git context data available" only when:
  - Not in a git repository, OR
  - Risk analyzer not provided, OR
  - Git history provider failed to initialize
- [ ] Integration tests verify git context displays for god objects
- [ ] Existing god object tests continue to pass

## Technical Details

### Implementation Approach

#### 1. Modify Function Signatures

**File**: `src/builders/unified_analysis.rs`

**Change 1**: Update `apply_file_analysis_results` to accept `risk_analyzer`

```rust
// Line ~1688
fn apply_file_analysis_results(
    unified: &mut UnifiedAnalysis,
    processed_files: Vec<ProcessedFileData>,
    risk_analyzer: Option<&risk::RiskAnalyzer>,  // NEW PARAMETER
) {
    // ... existing code ...
}
```

**Change 2**: Pass `risk_analyzer` from callers

```rust
// Line ~728 in analyze_files_sequential
apply_file_analysis_results(&mut unified, processed_files, risk_analyzer.as_ref());

// Similar changes in parallel analysis path if applicable
```

#### 2. Add File-Level Git Context Analysis

**File**: `src/builders/unified_analysis.rs`

**New Function**: Analyze git context for a file path

```rust
/// Analyze file-level git context for god objects
/// Returns contextual risk based on file's git history
fn analyze_file_git_context(
    file_path: &Path,
    risk_analyzer: &risk::RiskAnalyzer,
    project_root: &Path,
) -> Option<risk::context::ContextualRisk> {
    // Get context aggregator from risk analyzer
    let aggregator = risk_analyzer.context_aggregator()?;

    // Create analysis target for the file
    let target = risk::context::AnalysisTarget {
        file_path: file_path.to_path_buf(),
        function_name: "".to_string(),  // Empty for file-level
        line_range: (0, 0),  // Not used for file-level
        complexity_metrics: Default::default(),  // Not used for file-level
        coverage: None,
        is_test: false,
        project_root: project_root.to_path_buf(),
    };

    // Gather contexts (includes git_history provider)
    let contexts = aggregator.gather(&target).ok()?;

    // Calculate risk contribution
    let base_risk = 0.0;  // File-level, no function complexity
    let contextual_risk = contexts.iter()
        .map(|ctx| ctx.contribution * ctx.weight)
        .sum::<f64>();

    // Generate explanation
    let explanation = format!(
        "File-level git analysis: {} context providers",
        contexts.len()
    );

    Some(risk::context::ContextualRisk {
        base_risk,
        contextual_risk,
        contexts,
        explanation,
    })
}
```

#### 3. Update God Object Creation Flow

**File**: `src/builders/unified_analysis.rs`

**In `apply_file_analysis_results`** (around line 1696-1730):

```rust
for file_data in processed_files {
    if let Some(god_analysis) = &file_data.god_analysis {
        update_function_god_indicators(unified, &file_data.file_path, god_analysis);

        let mut aggregated_metrics = aggregate_from_raw_metrics(&file_data.raw_functions);

        let member_functions = extract_member_functions(unified.items.iter(), &file_data.file_path);
        if !member_functions.is_empty() {
            let item_metrics = aggregate_god_object_metrics(&member_functions);
            // Merge contextual data from debt items
            aggregated_metrics.weighted_coverage = item_metrics.weighted_coverage;
            aggregated_metrics.unique_upstream_callers = item_metrics.unique_upstream_callers;
            aggregated_metrics.unique_downstream_callees = item_metrics.unique_downstream_callees;
            aggregated_metrics.upstream_dependencies = item_metrics.upstream_dependencies;
            aggregated_metrics.downstream_dependencies = item_metrics.downstream_dependencies;

            // CHANGE: Prefer direct file analysis over member aggregation
            aggregated_metrics.aggregated_contextual_risk = risk_analyzer
                .and_then(|analyzer| analyze_file_git_context(
                    &file_data.file_path,
                    analyzer,
                    &file_data.project_root,  // Need to add to ProcessedFileData
                ))
                .or(item_metrics.aggregated_contextual_risk);  // Fallback to member aggregation
        } else {
            // NEW: When no member functions, try direct file analysis
            aggregated_metrics.aggregated_contextual_risk = risk_analyzer
                .and_then(|analyzer| analyze_file_git_context(
                    &file_data.file_path,
                    analyzer,
                    &file_data.project_root,
                ));
        }

        let god_item = create_god_object_debt_item(
            &file_data.file_path,
            &file_data.file_metrics,
            god_analysis,
            aggregated_metrics,
        );
        unified.add_item(god_item);
    }

    // ... rest of function ...
}
```

#### 4. Update ProcessedFileData Structure

**File**: `src/builders/unified_analysis.rs`

**Add `project_root` field** to `ProcessedFileData`:

```rust
struct ProcessedFileData {
    file_path: PathBuf,
    file_metrics: FileDebtMetrics,
    file_context: FileContext,
    god_analysis: Option<crate::organization::GodObjectAnalysis>,
    raw_functions: Vec<FunctionMetrics>,
    project_root: PathBuf,  // NEW FIELD
}
```

Update creation sites to include `project_root` (propagate from function parameters).

### Architecture Changes

**No major architectural changes required**. This builds on existing infrastructure:

- Reuses `GitHistoryProvider` via `ContextAggregator`
- Follows existing `ContextualRisk` data structure
- Integrates with existing god object creation pipeline
- Uses established batched git history for performance

### Data Flow After Changes

```
God Object Detection
  ↓
Try direct file-level git analysis (NEW)
  ├─ Success → Use as contextual_risk
  ├─ Failure → Try member aggregation
  │              ├─ Success → Use aggregated risk
  │              └─ Failure → None
  ↓
Create God Object Debt Item
  ↓
TUI Display (Git Context page)
  → Shows file-level git metrics
```

### Integration Points

1. **Risk Analyzer**
   - Access `ContextAggregator` from `RiskAnalyzer`
   - Use existing `gather()` method with file-level target
   - Reuse batched git history cache

2. **Git History Provider**
   - Already supports file-level analysis via `AnalysisTarget.file_path`
   - Batched history provides O(1) lookups
   - No code changes needed in git provider

3. **TUI Display**
   - No changes needed
   - Already reads `contextual_risk.contexts` for "git_history" provider
   - Will automatically display file-level metrics

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/builders/unified_analysis.rs` - Primary changes
- `src/risk/context/mod.rs` - Use existing AnalysisTarget
- `src/risk/context/git_history.rs` - No changes (already supports file-level)
- `src/tui/results/detail_pages/git_context.rs` - No changes (already reads data correctly)

**External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Test `analyze_file_git_context` function**
   - Mock risk analyzer with git_history provider
   - Verify contextual risk returned for valid file
   - Verify None returned when provider missing
   - Test with various file paths

2. **Test god object creation with file-level context**
   - Create god object with direct git analysis
   - Verify `contextual_risk` field populated
   - Test fallback to member aggregation
   - Test graceful handling of missing risk analyzer

### Integration Tests

**File**: `tests/tui_integration_test.rs` or new file

```rust
#[test]
fn test_god_object_git_context_display() {
    // Setup: Create test project with git history
    let temp_repo = setup_git_repo_with_god_object();

    // Run analysis with git context enabled
    let results = analyze_with_git_context(&temp_repo);

    // Find god object item
    let god_item = results.items.iter()
        .find(|item| matches!(item.debt_type, DebtType::GodObject { .. }))
        .expect("God object should be detected");

    // Verify contextual risk present
    assert!(god_item.contextual_risk.is_some(), "God object should have contextual risk");

    let contextual_risk = god_item.contextual_risk.as_ref().unwrap();

    // Verify git_history context present
    let git_context = contextual_risk.contexts.iter()
        .find(|ctx| ctx.provider == "git_history")
        .expect("Should have git_history context");

    // Verify git metrics
    match &git_context.details {
        ContextDetails::Historical { change_frequency, bug_density, age_days, author_count } => {
            assert!(*change_frequency >= 0.0, "Change frequency should be non-negative");
            assert!(*age_days >= 0, "Age should be non-negative");
            assert!(*author_count >= 1, "Should have at least one author");
        }
        _ => panic!("Expected Historical context details"),
    }
}

#[test]
fn test_god_object_without_git_context() {
    // Run analysis without git context provider
    let results = analyze_without_git_context();

    let god_item = results.items.iter()
        .find(|item| matches!(item.debt_type, DebtType::GodObject { .. }));

    if let Some(item) = god_item {
        // Should work gracefully without git context
        assert!(item.contextual_risk.is_none() ||
                item.contextual_risk.as_ref().unwrap().contexts.is_empty());
    }
}
```

### Manual Testing

1. **TUI Verification**
   - Run `debtmap tui` on a project with god objects
   - Navigate to god object detail view
   - Switch to "Git Context" tab (key: 'g')
   - Verify display shows:
     - Change Frequency: [value] changes/month
     - Stability: [percentage]
     - Bug Density: [value]
     - Age: [days] days
     - Contributors: [count] contributors

2. **Edge Cases**
   - Non-git repository: Verify graceful handling
   - Git provider disabled: Verify fallback to None
   - God object with no member functions: Verify direct analysis works
   - God object with complex members: Verify direct analysis preferred

### Performance Tests

- **Before/After Benchmarks**
  - Measure analysis time for projects with god objects
  - Verify no significant regression
  - Confirm batched git history prevents subprocess overhead

- **Expected Performance**
  - File-level git context: O(1) HashMap lookup
  - No additional git subprocess calls
  - Negligible overhead vs. member aggregation

## Documentation Requirements

### Code Documentation

1. **Inline Documentation**
   - Document `analyze_file_git_context` function with examples
   - Explain fallback strategy in `apply_file_analysis_results`
   - Add comments explaining direct vs. aggregated approach

2. **Architecture Documentation**
   - Update god object section to mention file-level git context
   - Document data flow from git analysis to TUI display
   - Explain integration with `ContextAggregator`

### User Documentation

**Update TUI documentation**:
- Mention that god objects show file-level git context
- Explain git context metrics and what they indicate
- Document when git context may be unavailable

### ARCHITECTURE.md Updates

Add to **God Object Detection** section:

```markdown
### Git Context for God Objects

God objects receive **direct file-level git context analysis** rather than relying solely
on aggregation from member functions. This ensures git history data (change frequency,
bug density, stability) is always available for architectural-level debt items.

**Data Flow**:
1. Direct file analysis via GitHistoryProvider (primary)
2. Member function aggregation (fallback)
3. None (when git unavailable or analysis fails)

This approach recognizes that god objects are file-level issues requiring file-level
historical context for accurate risk assessment.
```

## Implementation Notes

### Key Insights

1. **File-level analysis is the correct abstraction**
   - God objects are architectural issues, not just complex function aggregations
   - File git history directly reflects god object risk
   - Member aggregation is useful but insufficient

2. **Reuse existing infrastructure**
   - `GitHistoryProvider` already supports file-level analysis
   - `ContextAggregator.gather()` works with any `AnalysisTarget`
   - Batched git history prevents performance issues

3. **Graceful degradation**
   - Works without risk analyzer (backward compatible)
   - Falls back to member aggregation if direct analysis fails
   - TUI handles missing data appropriately

### Potential Gotchas

1. **Project root propagation**
   - Need to pass `project_root` through `ProcessedFileData`
   - Ensure it's available at god object creation time
   - May need to trace back through call chain

2. **AnalysisTarget construction**
   - File-level targets have empty function name
   - Line range and complexity metrics not used
   - Ensure git provider handles this correctly (it does)

3. **Risk analyzer availability**
   - Only available when git context enabled
   - Need to handle `Option<&RiskAnalyzer>` throughout
   - Test both with and without analyzer

### Best Practices

1. **Prefer direct analysis**
   - Always try file-level analysis first
   - Only fall back to aggregation when direct fails
   - Document this precedence clearly

2. **Maintain purity where possible**
   - `analyze_file_git_context` is pure given inputs
   - Keep I/O (git calls) inside providers
   - Separate data flow from analysis logic

3. **Test both paths**
   - Test direct analysis path with git provider
   - Test fallback to member aggregation
   - Test graceful handling of failures

## Migration and Compatibility

### Breaking Changes

**None**. This is a purely additive change:
- Existing god object creation still works
- Member aggregation still functions as fallback
- No changes to public APIs or data structures

### Backward Compatibility

- **Without risk analyzer**: God objects work as before (may have no contextual risk)
- **Without git repository**: Gracefully returns `None`, TUI shows appropriate message
- **Existing tests**: Should continue to pass without modification

### Migration Path

**No migration needed**. Changes are transparent to users:
1. Upgrade to new version
2. Run analysis as usual
3. Git context automatically appears for god objects (in git repos)

### Rollback Plan

If issues arise, rollback is straightforward:
1. Revert changes to `apply_file_analysis_results`
2. God objects fall back to member aggregation only
3. No data loss or corruption risk

## Success Metrics

### Quantitative Metrics

- **Coverage**: 100% of god objects in git repos have contextual risk data
- **Performance**: Analysis time increase < 5% (should be ~0% due to batching)
- **Test Coverage**: 95%+ coverage for new code paths

### Qualitative Metrics

- **User Experience**: Git Context page shows data instead of "No git context available"
- **Data Quality**: File-level metrics accurately reflect git history
- **Maintainability**: Code remains clean and follows functional patterns

### Validation Criteria

Before marking this spec as complete:
- [ ] All acceptance criteria met
- [ ] Integration tests passing
- [ ] Manual TUI testing confirms git context displays
- [ ] Performance benchmarks show no regression
- [ ] Code review completed
- [ ] Documentation updated

## Future Enhancements

### Potential Improvements

1. **Enhanced file-level metrics**
   - Code ownership concentration
   - Hotspot analysis (areas of frequent change)
   - Cross-file change correlation

2. **Aggregate metrics**
   - Combine file-level and function-level insights
   - Weight by severity and recency
   - More sophisticated risk scoring

3. **Historical trend analysis**
   - Show git metrics over time
   - Identify acceleration in churn
   - Predict future instability

These are out of scope for this spec but may be valuable future work.

## References

### Related Code

- **God Object Aggregation**: `src/priority/god_object_aggregation.rs:156-190`
- **Git History Provider**: `src/risk/context/git_history.rs:361-410`
- **TUI Git Context Display**: `src/tui/results/detail_pages/git_context.rs:17-154`
- **Risk Analyzer**: `src/utils/risk_analyzer.rs`
- **Context Aggregator**: `src/risk/context/mod.rs`

### Related Specifications

- **Spec 207**: God object TUI display (introduced god object debt items)
- **Spec 133**: God object detection refinement
- **Spec 219**: Context provider progress tracking

### External Resources

- Git history analysis: Uses `git log`, `git rev-list` for file metrics
- Batched git history: Pre-fetches all git data to avoid subprocess overhead
- Functional patterns: Pure functions, immutable data, composition
