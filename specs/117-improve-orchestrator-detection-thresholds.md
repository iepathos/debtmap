---
number: 117
title: Improve Orchestrator Detection with Lenient Thresholds and Cognitive Complexity Weighting
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-16
---

# Specification 117: Improve Orchestrator Detection with Lenient Thresholds and Cognitive Complexity Weighting

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's existing orchestrator detection in `src/priority/semantic_classifier.rs` is too strict, causing false positives where well-designed coordination functions are flagged as high-priority technical debt. The current system requires:

- Cyclomatic complexity ≤ 2-3 for orchestrators
- Simple delegation pattern with minimal branching

**Real-world issue**: Functions with legitimate error handling, conditional orchestration, or 4-5 decision points are misclassified as `PureLogic` when they're actually orchestrators. Example:

```rust
fn coordinate_analysis() -> Result<Report> {
    let data = load_data()?;      // +1 complexity (Result)
    if !validate(data) {           // +1 complexity (branch)
        return Err(...);
    }

    let parsed = parse_data(data)?; // +1 complexity (Result)
    let results = run_analysis(parsed)?; // +1 complexity (Result)
    save_results(results)?;         // +1 complexity (Result)
    Ok(report)
}
// Complexity: 5, currently NOT classified as orchestrator
```

Additionally, the scoring system (`unified_scorer.rs:303-316`) averages cyclomatic and cognitive complexity equally, when cognitive complexity is specifically designed to differentiate orchestration (sequential calls, low cognitive load) from algorithmic complexity (nested logic, high cognitive load).

**Impact**: After functional refactoring, orchestrator functions still get high priority scores, discouraging proper separation of concerns.

## Objective

Enhance existing orchestrator detection with more lenient thresholds and cognitive complexity weighting to reduce false positives by 40-50% for well-designed coordination functions without requiring new infrastructure.

## Requirements

### Functional Requirements

1. **Lenient Orchestrator Thresholds**:
   - Increase cyclomatic complexity threshold from 2-3 to 5 for orchestrators
   - Allow error handling patterns (Result, Option) that add complexity
   - Support conditional orchestration (if/match for routing)
   - Maintain minimum delegation count (≥2 meaningful callees)

2. **Cognitive Complexity Weighting**:
   - Weight cognitive complexity more heavily (70%) vs cyclomatic (30%) for orchestrators
   - Keep equal weighting (50%/50%) for non-orchestrators
   - Apply weighting in `normalize_complexity()` function
   - Preserve existing scoring interface (no breaking changes)

3. **Delegation Ratio Heuristic** (Simple Version):
   - Calculate approximate delegation ratio: `callees.len() / total_statements`
   - Use function length as proxy for statement count
   - Threshold: ≥20% of statements are function calls
   - Filter out standard library calls when counting

4. **Enhanced Detection Logic**:
   - Orchestrator if: (cyclomatic ≤ 5 AND delegation_ratio > 0.2 AND meaningful_callees ≥ 2)
   - OR: name suggests orchestration AND cyclomatic ≤ 5
   - Maintain existing exclusions (adapters, formatters, functional chains)

### Non-Functional Requirements

- **Backward Compatibility**: No changes to public APIs or data structures
- **Performance**: < 5% overhead vs current implementation
- **Configuration**: Make new thresholds configurable via existing config system
- **Testing**: ≥ 90% test coverage for new logic

## Acceptance Criteria

- [ ] Orchestrator cyclomatic threshold increased to 5 in `semantic_classifier.rs`
- [ ] Delegation ratio calculation added to `is_orchestrator()` function
- [ ] Cognitive complexity weighting implemented in `normalize_complexity()`
- [ ] Weighting applied conditionally based on orchestrator detection
- [ ] Configuration options added to `.debtmap.toml`:
  - `orchestrator_max_cyclomatic` (default: 5)
  - `orchestrator_min_delegation_ratio` (default: 0.2)
  - `orchestrator_cognitive_weight` (default: 0.7)
- [ ] Test cases for orchestrators with complexity 4-5 pass
- [ ] Debtmap's own codebase analysis shows reduced false positives
- [ ] Integration tests verify scoring adjustments work correctly
- [ ] Documentation updated with new threshold rationale
- [ ] Performance benchmarks show < 5% overhead

## Technical Details

### Implementation Approach

**Phase 1: Enhance Orchestrator Detection** (1-2 days)
1. Update `is_orchestrator()` in `semantic_classifier.rs`:
   - Change `func.cyclomatic <= 2` to `func.cyclomatic <= 5`
   - Add delegation ratio calculation
   - Combine thresholds with existing logic

2. Add helper function:
```rust
fn calculate_delegation_ratio(
    func: &FunctionMetrics,
    meaningful_callees: &[FunctionId],
) -> f64 {
    if func.length == 0 {
        return 0.0;
    }
    meaningful_callees.len() as f64 / func.length as f64
}
```

**Phase 2: Cognitive Complexity Weighting** (1 day)
1. Modify `normalize_complexity()` in `unified_scorer.rs`:
```rust
fn normalize_complexity(
    cyclomatic: u32,
    cognitive: u32,
    is_orchestrator_candidate: bool,
) -> f64 {
    let combined = if is_orchestrator_candidate {
        // Weight cognitive more heavily for orchestrators
        // Sequential calls = low cognitive, high cyclomatic
        (cyclomatic as f64 * 0.3) + (cognitive as f64 * 0.7)
    } else {
        // Equal weight for pure logic
        (cyclomatic + cognitive) as f64 / 2.0
    };

    // ... rest of normalization logic
}
```

2. Update callers to pass orchestrator flag:
   - Detect orchestrator pattern early in scoring
   - Pass flag through to `normalize_complexity()`

**Phase 3: Configuration** (0.5 days)
1. Add to `src/config.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorDetectionConfig {
    pub max_cyclomatic: u32,
    pub min_delegation_ratio: f64,
    pub min_meaningful_callees: usize,
    pub cognitive_weight: f64,
}

impl Default for OrchestratorDetectionConfig {
    fn default() -> Self {
        Self {
            max_cyclomatic: 5,
            min_delegation_ratio: 0.2,
            min_meaningful_callees: 2,
            cognitive_weight: 0.7,
        }
    }
}
```

2. Integrate with existing config loading

**Phase 4: Testing and Validation** (1 day)
1. Add unit tests for new thresholds
2. Test with debtmap's own codebase
3. Validate false positive reduction
4. Performance benchmarks

### Architecture Changes

**Modified Files**:
- `src/priority/semantic_classifier.rs` - Enhanced orchestrator detection
- `src/priority/unified_scorer.rs` - Cognitive complexity weighting
- `src/config.rs` - New configuration options

**No New Files Required**: Enhances existing modules

### Data Structures

No new public data structures. Internal additions:

```rust
// In semantic_classifier.rs
struct DelegationMetrics {
    meaningful_callees: usize,
    total_statements: usize,
    delegation_ratio: f64,
}

// In config.rs
pub struct OrchestratorDetectionConfig {
    pub max_cyclomatic: u32,
    pub min_delegation_ratio: f64,
    pub min_meaningful_callees: usize,
    pub cognitive_weight: f64,
}
```

### APIs and Interfaces

**Internal API Changes** (no breaking changes):

```rust
// Enhanced signature (internal only)
fn is_orchestrator(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    config: &OrchestratorDetectionConfig,
) -> bool

// Updated normalization (internal only)
fn normalize_complexity(
    cyclomatic: u32,
    cognitive: u32,
    is_orchestrator_candidate: bool,
) -> f64
```

**Configuration API** (new):
```toml
[orchestrator_detection]
max_cyclomatic = 5
min_delegation_ratio = 0.20
min_meaningful_callees = 2
cognitive_weight = 0.7
```

## Dependencies

- **Prerequisites**: None (enhances existing functionality)
- **Affected Components**:
  - `src/priority/semantic_classifier.rs` - Core detection logic
  - `src/priority/unified_scorer.rs` - Complexity normalization
  - `src/config.rs` - Configuration management
- **External Dependencies**: None (uses existing infrastructure)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_orchestrator_with_error_handling() {
        // Function with complexity 5 from Result handling
        let func = create_test_metrics("coordinate_tasks", 5, 3, 20);
        let mut graph = CallGraph::new();

        // Add 3 meaningful callees (60% calls in 5 statements)
        add_test_callees(&mut graph, &func_id, 3);

        let config = OrchestratorDetectionConfig::default();
        assert!(is_orchestrator(&func, &func_id, &graph, &config));
    }

    #[test]
    fn test_delegation_ratio_calculation() {
        let func = create_test_metrics("orchestrator", 4, 2, 20);
        let callees = vec![/* 4 callees */];

        let ratio = calculate_delegation_ratio(&func, &callees);
        assert_eq!(ratio, 0.2); // 4 calls / 20 lines = 20%
    }

    #[test]
    fn test_cognitive_weighting_for_orchestrators() {
        // High cyclomatic (5), low cognitive (2) = orchestrator pattern
        let complexity = normalize_complexity(5, 2, true);
        let expected = (5.0 * 0.3) + (2.0 * 0.7); // 1.5 + 1.4 = 2.9
        assert!((complexity - expected).abs() < 0.1);
    }

    #[test]
    fn test_equal_weighting_for_pure_logic() {
        // Both high = pure logic pattern
        let complexity = normalize_complexity(8, 10, false);
        let expected = (8.0 + 10.0) / 2.0; // 9.0
        assert!((complexity - expected).abs() < 0.1);
    }
}
```

### Integration Tests

1. **Debtmap Self-Analysis**:
   - Run analysis on debtmap's own codebase
   - Verify known orchestrators (`create_unified_analysis_with_exclusions`) get lower scores
   - Confirm pure logic functions maintain high scores
   - Document false positive reduction percentage

2. **Configuration Tests**:
   - Test with different threshold values
   - Verify config loading and application
   - Test edge cases (threshold = 0, very high thresholds)

3. **Regression Tests**:
   - Ensure existing classifications remain stable
   - No unintended reclassifications of pure logic
   - Adapter and formatter exclusions still work

### Performance Tests

```rust
#[bench]
fn bench_orchestrator_detection_with_delegation_ratio(b: &mut Bencher) {
    let func = create_large_test_case();
    let graph = create_large_call_graph();
    let config = OrchestratorDetectionConfig::default();

    b.iter(|| {
        is_orchestrator(&func, &func_id, &graph, &config)
    });
}
```

Target: < 5% overhead vs baseline

## Documentation Requirements

### Code Documentation

```rust
/// Enhanced orchestrator detection with lenient thresholds.
///
/// Orchestrators are coordination functions that delegate to other functions
/// with minimal local logic. This implementation uses:
///
/// - **Cyclomatic threshold**: ≤5 (allows error handling)
/// - **Delegation ratio**: ≥20% of statements are function calls
/// - **Meaningful callees**: ≥2 non-stdlib functions
///
/// # Rationale
///
/// Real-world orchestrators often have complexity 4-5 from:
/// - Result/Option error propagation (`?` operator)
/// - Conditional routing (if/match on input)
/// - Guard clauses for validation
///
/// These patterns are fundamentally different from algorithmic complexity
/// (nested loops, complex conditionals) and should be scored differently.
///
/// # Example
///
/// ```rust
/// // Orchestrator: complexity 5, but simple coordination
/// fn process_workflow(input: Input) -> Result<Output> {
///     let validated = validate(input)?;      // +1
///     if validated.needs_preprocessing() {    // +1
///         validated = preprocess(validated)?; // +1
///     }
///     let result = analyze(validated)?;      // +1
///     save_results(result)?;                 // +1
///     Ok(result)
/// }
/// ```
pub fn is_orchestrator(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
    config: &OrchestratorDetectionConfig,
) -> bool
```

### User Documentation

Add to configuration documentation:

```markdown
## Orchestrator Detection

Debtmap automatically identifies orchestrator functions (coordination logic)
and applies reduced priority scores to avoid false positives.

### Configuration

```toml
[orchestrator_detection]
# Maximum cyclomatic complexity for orchestrators (default: 5)
# Allows for error handling and conditional routing
max_cyclomatic = 5

# Minimum delegation ratio (default: 0.20)
# Percentage of statements that are function calls
min_delegation_ratio = 0.20

# Minimum meaningful callees (default: 2)
# Must coordinate at least 2 non-stdlib functions
min_meaningful_callees = 2

# Cognitive complexity weight (default: 0.7)
# Weight given to cognitive vs cyclomatic complexity
cognitive_weight = 0.7
```

### Tuning Thresholds

**More Strict Detection**:
```toml
max_cyclomatic = 3
min_delegation_ratio = 0.30
```

**More Lenient Detection**:
```toml
max_cyclomatic = 7
min_delegation_ratio = 0.15
```

### Understanding the Scores

Orchestrators receive:
- 0.8x role multiplier (vs 1.2x for pure logic)
- Cognitive complexity weighted 70% (vs 50% normally)
- Result: ~30-40% score reduction for coordination functions
```

### Architecture Documentation

Add section to ARCHITECTURE.md:

```markdown
## Orchestrator Detection Enhancement (Spec 117)

The semantic classifier uses lenient thresholds and cognitive complexity
weighting to distinguish coordination logic from algorithmic complexity:

### Detection Algorithm

1. **Threshold Check**: Cyclomatic ≤ 5 (allows error handling)
2. **Delegation Check**: ≥20% of statements are function calls
3. **Meaningful Callees**: ≥2 non-stdlib functions coordinated
4. **Name Heuristics**: Orchestration keywords in function name

### Complexity Scoring

For orchestrators, cognitive complexity receives 70% weight:
- **Orchestrator pattern**: Low cognitive (2), high cyclomatic (5) = 2.9
- **Pure logic pattern**: Both high (8, 10) = 9.0

This reflects that sequential delegation has lower mental overhead than
nested algorithmic logic, even with similar cyclomatic scores.
```

## Implementation Notes

### Key Design Decisions

1. **Why ≤5 Cyclomatic Threshold?**
   - Empirical observation: most orchestrators have 4-5 complexity
   - Accounts for error handling (Result chains)
   - Allows conditional routing (if/match for different paths)
   - Still excludes deeply nested logic (>5)

2. **Why 20% Delegation Ratio?**
   - Conservative threshold: 1 call per 5 statements
   - Avoids false positives from incidental calls
   - Higher than pure logic functions (typically <10%)
   - Lower than pure orchestrators (typically 30-50%)

3. **Why 70% Cognitive Weight?**
   - Cognitive complexity designed for this use case
   - Sequential calls = low cognitive load
   - Nested logic = high cognitive load
   - 70/30 split provides meaningful differentiation

4. **Why Not Full Spec 109 Approach?**
   - 80% of value from simple enhancements
   - No new infrastructure required
   - Faster to implement and validate
   - Can extend later if needed

### Edge Cases

1. **Orchestrators with High Cognitive Complexity**:
   - Weighted score still penalizes properly
   - May indicate mixed concerns (orchestration + logic)
   - User should consider refactoring

2. **Pure Logic with Many Calls**:
   - High cyclomatic + high cognitive = not orchestrator
   - Delegation ratio alone doesn't override other signals
   - Multiple heuristics prevent misclassification

3. **Empty Functions**:
   - Length 0 → delegation ratio 0
   - Not classified as orchestrator (correct)

4. **Adapter Pattern (Single Callee)**:
   - Requires ≥2 meaningful callees
   - Single delegation excluded (existing behavior)
   - Maintains adapter/wrapper distinction

### Performance Optimizations

1. **Lazy Evaluation**:
   - Calculate delegation ratio only if cyclomatic ≤ 5
   - Skip if meaningful_callees < 2
   - Early exit for obvious non-orchestrators

2. **Caching**:
   - Meaningful callee filtering done once
   - Reuse filtered list for delegation calculation
   - No additional call graph traversal

3. **Simple Heuristics**:
   - Use function length as statement count (no AST parsing)
   - Count callees directly from call graph
   - Division operation only (no complex math)

## Migration and Compatibility

### Backward Compatibility

- **No Breaking Changes**: All modifications internal to existing modules
- **Config Optional**: Default values maintain reasonable behavior
- **API Stability**: No public API changes
- **Output Format**: No changes to JSON output structure

### Rollout Strategy

**Phase 1**: Deploy with defaults (1 week)
- Enable new thresholds by default
- Monitor false positive reduction
- Gather user feedback

**Phase 2**: Tune if needed (1 week)
- Adjust thresholds based on real-world data
- Document edge cases found
- Update configuration guidance

**Phase 3**: Document best practices (ongoing)
- Create examples of common patterns
- Provide tuning guidance
- Collect success stories

### Migration Path

Users automatically get improved detection with defaults. To customize:

```toml
# Add to .debtmap.toml
[orchestrator_detection]
max_cyclomatic = 5  # Adjust as needed
```

No action required for existing configurations.

## Success Metrics

- **False Positive Reduction**: 40-50% fewer orchestrators flagged as high priority
- **Classification Accuracy**: ≥85% correct orchestrator identification on test set
- **Performance**: < 5% overhead vs baseline
- **User Adoption**: Positive feedback on reduced false positives
- **Maintenance**: Zero critical bugs in first 3 months

### Validation Plan

1. **Before/After Analysis**:
   - Run debtmap on its own codebase (baseline)
   - Apply spec 117 changes
   - Compare top 20 priority items
   - Document reclassifications

2. **Test Set Creation**:
   - Hand-label 50 functions from debtmap
   - 25 orchestrators, 25 pure logic
   - Validate classification accuracy

3. **Performance Benchmark**:
   - Measure analysis time before/after
   - Test on large codebase (1000+ functions)
   - Verify < 5% overhead

## Future Enhancements

If this approach proves insufficient, consider:

1. **Delegation Pattern Sophistication** (Spec 109 revisited):
   - Full AST-based statement counting
   - Distinguish call types (async, sync, closure)
   - Track call depth and patterns

2. **Machine Learning Classification**:
   - Train on hand-labeled dataset
   - Multi-factor model beyond heuristics
   - Continuous learning from user feedback

3. **User Annotations**:
   - Allow `#[orchestrator]` attribute
   - Manual override for edge cases
   - Per-function configuration

These can be implemented incrementally based on need.
