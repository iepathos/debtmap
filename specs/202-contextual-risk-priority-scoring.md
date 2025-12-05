---
number: 202
title: Contextual Risk Integration for Priority Scoring
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-12-04
---

# Specification 202: Contextual Risk Integration for Priority Scoring

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The risk analysis flow (`validate` command with coverage data) successfully integrates git history context providers to amplify risk scores based on empirical volatility metrics (change frequency, bug density, file age, author count). However, the priority scoring flow (`analyze` command) does not utilize these context providers, even when `--context` is enabled.

Currently:
- `FunctionRisk` struct includes `contextual_risk` field (fully functional)
- `UnifiedDebtItem` struct includes `contextual_risk` field (structure only)
- Context flags flow through CLI but providers never run
- Git history data would provide valuable prioritization signals

The unified analysis pipeline is complex with multiple code paths:
- Parallel vs sequential execution
- With/without coverage data
- With/without god object detection
- Various aggregation strategies

## Objective

Enable contextual risk analysis in the priority scoring flow so that `debtmap analyze . --context` enriches priority recommendations with git history and other context provider data, displaying volatility metrics alongside complexity scores.

## Requirements

### Functional Requirements

1. **Context Provider Initialization**
   - Build `ContextAggregator` when `enable_context=true` in unified analysis
   - Support provider selection via `--context-providers` flag
   - Support provider exclusion via `--disable-context` flag
   - Use same provider factory as risk analysis (`utils/risk_analyzer.rs`)

2. **Risk Analysis Integration**
   - Create `RiskAnalyzer` with context aggregator when context enabled
   - Call `analyze_function_with_context()` for each function during scoring
   - Attach returned `ContextualRisk` to `UnifiedDebtItem.contextual_risk`
   - Preserve existing risk analysis behavior when context disabled

3. **Pipeline Integration**
   - Thread context aggregator through unified analysis call chain
   - Support both parallel and sequential execution paths
   - Handle edge cases (no git repo, git command failures)
   - Maintain performance characteristics

4. **Display Integration**
   - Show contextual risk breakdown in priority formatter output
   - Display git history metrics (change frequency, bug density, age, authors)
   - Show risk multiplier effect (base_risk vs contextual_risk)
   - Include provider contributions in verbose output

### Non-Functional Requirements

1. **Performance**
   - Context analysis should add <10% overhead to total analysis time
   - Git operations must be cached to avoid redundant subprocess calls
   - Parallel execution must remain efficient

2. **Backward Compatibility**
   - Analysis without `--context` must behave identically to current
   - JSON output format remains compatible
   - No breaking changes to existing APIs

3. **Error Handling**
   - Gracefully handle missing git repositories
   - Continue analysis if git commands fail
   - Log provider failures without crashing

## Acceptance Criteria

- [ ] `debtmap analyze . --context` runs git history provider successfully
- [ ] Terminal output shows git history metrics for high-priority functions
- [ ] Output displays: change frequency, bug density, file age, author count
- [ ] Base risk vs contextual risk comparison visible in output
- [ ] JSON output includes `contextual_risk` object when `--context` enabled
- [ ] Analysis without `--context` shows no git history data (backward compatible)
- [ ] Provider selection works: `--context-providers git_history`
- [ ] Provider exclusion works: `--disable-context git_history`
- [ ] Performance overhead is <10% compared to non-context analysis
- [ ] Parallel execution path works with context providers
- [ ] Sequential execution path works with context providers
- [ ] Missing git repo handled gracefully (no crash)
- [ ] Git command failures logged but don't stop analysis
- [ ] Debug logging shows provider contributions: `RUST_LOG=debtmap=debug`

## Technical Details

### Implementation Approach

**Phase 1: Context Aggregator Creation**
```rust
// In perform_unified_analysis_computation()
let context_aggregator = if enable_context {
    Some(build_context_aggregator(
        project_path,
        enable_context,
        context_providers,
        disable_context,
    ))
} else {
    None
};
```

**Phase 2: Risk Analyzer Initialization**
```rust
let mut risk_analyzer = risk::RiskAnalyzer::default()
    .with_debt_context(debt_score, debt_threshold);

if let Some(aggregator) = context_aggregator {
    risk_analyzer = risk_analyzer.with_context_aggregator(aggregator);
}
```

**Phase 3: Function Analysis**
```rust
// In create_debt_item() or similar
let contextual_risk = if risk_analyzer.has_context() {
    let (_, ctx_risk) = risk_analyzer.analyze_function_with_context(
        func.file.clone(),
        func.name.clone(),
        (func.line, func.line + func.length),
        &complexity_metrics,
        coverage,
        func.is_test,
        project_path.to_path_buf(),
    );
    ctx_risk
} else {
    None
};

// Attach to UnifiedDebtItem
item.contextual_risk = contextual_risk;
```

### Architecture Changes

**Modified Files:**
1. `src/builders/unified_analysis.rs`
   - Add context aggregator creation logic
   - Thread through call chain to item creation

2. `src/priority/scoring/construction.rs`
   - Accept optional risk analyzer parameter
   - Call `analyze_function_with_context()` when available
   - Populate `contextual_risk` field

3. `src/priority/formatter/writer.rs` or similar
   - Display contextual risk data in priority output
   - Format git history metrics

4. `src/priority/formatter_verbosity/body.rs`
   - Add verbose context provider details

### Data Structures

**No new structures needed** - reuse existing:
- `risk::context::ContextualRisk`
- `risk::context::ContextAggregator`
- `risk::RiskAnalyzer`

### APIs and Interfaces

**New Function Signatures:**
```rust
// In construction.rs
pub fn create_unified_debt_item_with_context(
    func: &FunctionMetrics,
    // ... existing params ...
    risk_analyzer: Option<&mut risk::RiskAnalyzer>,
    project_path: &Path,
) -> Option<UnifiedDebtItem>
```

**Modified Function Signatures:**
```rust
// Update signatures to thread risk_analyzer through:
- create_unified_analysis_with_exclusions_and_timing()
- create_unified_analysis_parallel()
- create_debt_item()
```

## Dependencies

### Prerequisites
- Git history provider implementation (already complete)
- Risk analyzer with context support (already complete)
- FunctionRisk contextual_risk field (already complete)
- UnifiedDebtItem contextual_risk field (already complete)

### Affected Components
- `builders/unified_analysis.rs` - Main analysis pipeline
- `priority/scoring/construction.rs` - Debt item creation
- `priority/formatter/*.rs` - Output formatting
- `utils/risk_analyzer.rs` - Provider factory (reuse existing)

### External Dependencies
- Git binary (already required)
- No new crate dependencies

## Testing Strategy

### Unit Tests
- Test context aggregator creation with various flag combinations
- Test risk analyzer integration with mock providers
- Test contextual_risk population in UnifiedDebtItem
- Test backward compatibility (no context vs with context)

### Integration Tests
- Test full analyze command with `--context` flag
- Test provider selection and exclusion
- Test missing git repo handling
- Test git command failure handling

### Performance Tests
- Benchmark analysis time with vs without context
- Ensure <10% overhead for context analysis
- Verify git caching works (no redundant calls)

### User Acceptance
- Verify contextual risk appears in terminal output
- Check JSON output includes contextual_risk field
- Confirm git metrics are accurate (change frequency, bug density)
- Validate risk multiplier calculations

## Documentation Requirements

### Code Documentation
- Document new parameters in function signatures
- Add inline comments explaining context flow
- Document error handling for git failures

### User Documentation
- Update `book/src/context-providers.md` with analyze command examples
- Add examples showing contextual risk output
- Document performance characteristics

### Architecture Updates
- Update ARCHITECTURE.md with context provider flow diagram
- Document integration between priority scoring and risk analysis

## Implementation Notes

### Performance Optimization
- Reuse existing git history cache from risk analysis
- Consider lazy initialization of context aggregator
- Batch git operations where possible

### Error Handling Strategy
```rust
// Graceful degradation pattern
let contextual_risk = match risk_analyzer.analyze_function_with_context(...) {
    Ok((_, ctx_risk)) => ctx_risk,
    Err(e) => {
        log::warn!("Context analysis failed for {}: {}", func.name, e);
        None
    }
};
```

### Threading Through Call Chain

The challenge is threading `risk_analyzer` through multiple layers:
1. `perform_unified_analysis_computation()` creates it
2. `create_unified_analysis_with_exclusions_and_timing()` receives it
3. `create_unified_analysis_parallel()` or sequential path gets it
4. `create_debt_item()` uses it

**Recommendation**: Add as optional parameter at each level, defaulting to `None` for backward compatibility.

### Display Format

**Terminal Output Example:**
```
#1 SCORE: 55.6 [CRITICAL]
├─ LOCATION: ./src/main.rs:393 main()
├─ IMPACT: +50% function coverage, -11 complexity, -13.2 risk
├─ COMPLEXITY: cyclomatic=23, cognitive=39
├─ COVERAGE: 0.0% coverage
├─ GIT HISTORY: 3.5 changes/month, 25.0% bugs, 45 days old, 3 authors
│  └─ Risk Impact: base_risk=6.6 → contextual_risk=13.2 (2.0x multiplier)
├─ WHY THIS MATTERS: Function has high churn and bug density...
└─ RECOMMENDED ACTION: Add 8 tests for untested branches
```

**Verbose Output (-v flag):**
```
Context Provider Contributions:
  └─ git_history: +1.0 impact (weight: 1.0)
     - Change frequency: 3.5/month (moderately unstable)
     - Bug density: 25.0% (high)
     - File age: 45 days (young)
     - Author count: 3
```

## Migration and Compatibility

### Breaking Changes
None - this is a pure addition.

### Migration Requirements
None - existing workflows unaffected.

### Compatibility Considerations
- JSON schema gains optional `contextual_risk` field
- Terminal output gains optional git history section
- Both changes are additive and backward compatible

## Implementation Plan

### Phase 1: Core Wiring (2-3 hours)
1. Add context aggregator creation in `perform_unified_analysis_computation()`
2. Create risk analyzer with context aggregator
3. Thread through to `create_debt_item()` signature
4. Populate `contextual_risk` field when available

### Phase 2: Display Integration (1-2 hours)
1. Add git history display in priority formatter
2. Format contextual risk metrics
3. Add verbose output for provider details
4. Test output formatting

### Phase 3: Testing & Polish (1-2 hours)
1. Test with real git repos
2. Test edge cases (no git, git failures)
3. Verify performance overhead
4. Update documentation

### Total Estimated Effort: 4-7 hours

## Success Metrics

- Context providers run during `analyze` command
- Git history metrics visible in output
- Performance overhead <10%
- No regressions in non-context workflows
- User feedback positive on contextual insights
