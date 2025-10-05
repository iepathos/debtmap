---
number: 108
title: Optional Coverage Data Handling
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-05
---

# Specification 108: Optional Coverage Data Handling

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently displays "[🔴 UNTESTED]" for all high-priority items when no coverage data (lcov file) is provided on the command line. This is misleading because:

1. Coverage data is optional - users may analyze code without having coverage reports
2. The "UNTESTED" label implies the code lacks tests, when in reality we simply don't have coverage data
3. When no coverage data is supplied, the scoring system should focus on other metrics (complexity, code smells, dependencies) rather than penalizing functions for missing coverage information

The current implementation in `unified_scorer.rs:248` sets `coverage_factor: (1.0 - coverage_pct) * 10.0`, which evaluates to `10.0` when no coverage data exists. This maximum coverage factor is then interpreted by the formatter as "UNTESTED" status.

## Objective

Adjust debtmap's behavior when no coverage data is supplied to:
1. Exclude coverage from scoring and ranking calculations
2. Remove misleading "UNTESTED" labels from output
3. Emit a one-time warning suggesting users supply coverage data for more comprehensive analysis
4. Focus scoring on available metrics: complexity, code smells, dependencies, and architectural patterns

## Requirements

### Functional Requirements

1. **Coverage Data Detection**
   - Detect whether lcov coverage data was provided via CLI arguments
   - Distinguish between "no coverage data provided" and "0% coverage for a specific function"
   - Pass this state through the analysis pipeline

2. **Conditional Scoring**
   - When coverage data is NOT provided:
     - Set `coverage_factor` to a neutral value (not 0, not 10)
     - Exclude coverage from weighted scoring calculation
     - Rebalance weights to focus on complexity (50%), dependencies (25%), and debt patterns (25%)
   - When coverage data IS provided:
     - Use existing coverage-based scoring
     - Display coverage status indicators (UNTESTED, LOW COVERAGE, etc.)

3. **Output Formatting**
   - When coverage data is NOT provided:
     - Remove all "[🔴 UNTESTED]", "[🟠 LOW COVERAGE]", etc. indicators
     - Do not display coverage-related information in item details
     - Remove "OVERALL COVERAGE" from summary statistics
   - When coverage data IS provided:
     - Keep all existing coverage indicators and displays

4. **User Warning**
   - When coverage data is NOT provided:
     - Emit a single warning at the start of analysis (not per-file or per-function)
     - Suggest: "💡 Tip: Provide coverage data with --lcov-file for more comprehensive analysis including test gap detection"
     - Do not repeat this warning for subsequent operations

### Non-Functional Requirements

1. **Backward Compatibility**
   - All existing behavior when coverage data IS provided must remain unchanged
   - Existing tests with coverage data should continue to pass

2. **Performance**
   - No performance degradation from coverage detection logic
   - Conditional scoring should not introduce significant overhead

3. **Maintainability**
   - Clear separation between "coverage provided" and "no coverage" code paths
   - Functional approach with pure functions for scoring calculation
   - Well-documented rationale for scoring adjustments

## Acceptance Criteria

- [ ] Running `debtmap analyze` without `--lcov-file` shows NO "[🔴 UNTESTED]" indicators
- [ ] Running `debtmap analyze` without `--lcov-file` displays a one-time tip about providing coverage data
- [ ] Running `debtmap analyze --lcov-file coverage.info` shows coverage indicators as before
- [ ] Scoring without coverage data focuses on complexity, dependencies, and code smells
- [ ] Top 10 recommendations without coverage data are ranked by non-coverage factors only
- [ ] Summary statistics without coverage data omit "OVERALL COVERAGE" line
- [ ] All existing tests continue to pass
- [ ] New tests verify correct behavior with and without coverage data
- [ ] Documentation updated to explain coverage as optional enhancement

## Technical Details

### Implementation Approach

#### Phase 1: Coverage Data State Propagation

1. Add `has_coverage_data: bool` field to:
   - `UnifiedAnalysis` struct
   - Analysis builder structs
   - Relevant configuration structures

2. Detect coverage data presence in CLI parsing:
```rust
// In CLI argument processing
let has_coverage_data = lcov_file_path.is_some();
```

3. Thread this state through the analysis pipeline:
```rust
pub struct UnifiedAnalysis {
    pub items: Vec<DebtItem>,
    pub has_coverage_data: bool,
    // ... existing fields
}
```

#### Phase 2: Conditional Scoring Logic

Modify `unified_scorer.rs::calculate_unified_priority_with_debt`:

```rust
pub fn calculate_unified_priority_with_debt(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    coverage: Option<&LcovData>,
    _organization_issues: Option<f64>,
    debt_aggregator: Option<&DebtAggregator>,
    has_coverage_data: bool,  // NEW PARAMETER
) -> UnifiedScore {
    // ... existing setup code ...

    let coverage_factor = if has_coverage_data {
        // Existing coverage calculation
        if func.is_test {
            0.1 // Test functions get minimal coverage factor
        } else {
            calculate_coverage_factor(coverage_pct)
        }
    } else {
        // No coverage data provided - use neutral value
        0.0 // Will be excluded from weighted calculation
    };

    let base_score = if has_coverage_data {
        // Existing weighted calculation (50% coverage, 35% complexity, 15% deps)
        calculate_base_score(coverage_factor, complexity_factor, dependency_factor)
    } else {
        // Adjusted weights when no coverage data
        // 50% complexity, 25% dependencies, 25% debt patterns
        calculate_base_score_no_coverage(complexity_factor, dependency_factor, debt_adjustment)
    };

    // ... rest of scoring calculation ...
}
```

Add new scoring function:

```rust
/// Calculate base score without coverage data
/// Uses adjusted weights: 50% complexity, 25% dependencies, 25% debt patterns
pub fn calculate_base_score_no_coverage(
    complexity_factor: f64,
    dependency_factor: f64,
    debt_adjustment: f64,
) -> f64 {
    let complexity_weight = 0.50;
    let dependency_weight = 0.25;
    let debt_weight = 0.25;

    (complexity_factor * complexity_weight * 100.0)
        + (dependency_factor * dependency_weight * 100.0)
        + (debt_adjustment * debt_weight * 100.0)
}
```

#### Phase 3: Output Formatting Adjustments

Modify `formatter_verbosity.rs`:

```rust
// Pure function to get coverage indicator - now takes has_coverage_data flag
fn get_coverage_indicator(item: &UnifiedDebtItem, has_coverage_data: bool) -> &'static str {
    if !has_coverage_data {
        return ""; // No coverage data, no indicator
    }

    // Existing logic for when coverage data exists
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        classify_coverage_percentage(coverage_pct).0
    } else if item.unified_score.coverage_factor >= 10.0 {
        " [🔴 UNTESTED]"
    } else {
        ""
    }
}

fn format_coverage_factor_description(
    item: &UnifiedDebtItem,
    _weights: &crate::config::ScoringWeights,
    has_coverage_data: bool,  // NEW PARAMETER
) -> Option<String> {
    if !has_coverage_data {
        return None; // Don't show coverage info when not available
    }

    // Existing coverage formatting logic
    // ...
}
```

Modify `formatter.rs`:

```rust
fn format_default_with_config(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
    config: FormattingConfig,
) -> String {
    // ... existing header code ...

    // Only show overall coverage if coverage data was provided
    if analysis.has_coverage_data {
        if let Some(coverage) = analysis.overall_coverage {
            writeln!(
                output,
                "{} {}",
                formatter.emoji("📈", "[CHART]"),
                format!("OVERALL COVERAGE: {:.2}%", coverage).bright_green()
            )
            .unwrap();
        }
    }

    output
}
```

#### Phase 4: User Warning

Add warning emission in main analysis flow:

```rust
pub fn run_analysis(config: &AnalysisConfig) -> Result<UnifiedAnalysis> {
    let has_coverage_data = config.lcov_file.is_some();

    if !has_coverage_data {
        eprintln!(
            "{} Coverage data not provided. Analysis will focus on complexity and code smells.",
            "💡 TIP:".bright_yellow()
        );
        eprintln!(
            "   For test gap detection, provide coverage with: {}",
            "--lcov-file coverage.info".bright_cyan()
        );
        eprintln!();
    }

    // ... rest of analysis ...
}
```

### Architecture Changes

**Before:**
```
CLI Args → Load Coverage → Calculate Scores (always uses coverage) → Format Output (always shows UNTESTED)
```

**After:**
```
CLI Args → Detect Coverage Presence → Load Coverage (if provided)
                                    ↓
                          Calculate Scores (conditional logic)
                                    ↓
                          Format Output (conditional indicators)
```

### Data Structures

```rust
// Add to existing structures
pub struct UnifiedAnalysis {
    pub items: Vec<DebtItem>,
    pub has_coverage_data: bool,  // NEW
    // ... existing fields
}

pub struct AnalysisConfig {
    pub lcov_file: Option<PathBuf>,
    // ... existing fields
}
```

### Modified Files

1. `src/cli.rs` - Detect coverage data presence from arguments
2. `src/priority/unified_scorer.rs` - Conditional scoring logic
3. `src/priority/scoring/calculation.rs` - New scoring function without coverage
4. `src/priority/formatter.rs` - Conditional coverage display in summary
5. `src/priority/formatter_verbosity.rs` - Conditional coverage indicators
6. `src/priority/mod.rs` - Thread has_coverage_data through types
7. `src/commands/analyze.rs` - Emit one-time warning when no coverage data

## Dependencies

**Prerequisites**: None - this is a standalone improvement

**Affected Components**:
- Scoring system (unified_scorer.rs)
- Output formatters (formatter.rs, formatter_verbosity.rs)
- CLI argument parsing (cli.rs)
- Main analysis command (commands/analyze.rs)

**External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Scoring Tests** (in `unified_scorer.rs`):
```rust
#[test]
fn test_scoring_without_coverage_data() {
    let func = create_test_function(cyclomatic: 10, cognitive: 12);
    let call_graph = CallGraph::new();

    let score = calculate_unified_priority_with_debt(
        &func,
        &call_graph,
        None,  // No coverage data
        None,
        None,
        false,  // has_coverage_data = false
    );

    // Coverage factor should be neutral (0.0)
    assert_eq!(score.coverage_factor, 0.0);

    // Score should be based on complexity and dependencies only
    assert!(score.final_score > 0.0);
}

#[test]
fn test_scoring_with_zero_coverage() {
    let func = create_test_function(cyclomatic: 10, cognitive: 12);
    let call_graph = CallGraph::new();
    let lcov = create_lcov_with_zero_coverage(&func);

    let score = calculate_unified_priority_with_debt(
        &func,
        &call_graph,
        Some(&lcov),
        None,
        None,
        true,  // has_coverage_data = true
    );

    // Should show UNTESTED (coverage_factor = 10.0)
    assert_eq!(score.coverage_factor, 10.0);
}
```

2. **Formatter Tests** (in `formatter.rs`):
```rust
#[test]
fn test_format_without_coverage_data() {
    let mut analysis = UnifiedAnalysis::new(CallGraph::new());
    analysis.has_coverage_data = false;
    analysis.add_item(create_high_complexity_item());

    let output = format_priorities(&analysis, OutputFormat::Default);

    // Should NOT contain UNTESTED indicator
    assert!(!output.contains("UNTESTED"));
    assert!(!output.contains("🔴"));

    // Should NOT show overall coverage
    assert!(!output.contains("OVERALL COVERAGE"));
}

#[test]
fn test_format_with_coverage_data() {
    let mut analysis = UnifiedAnalysis::new(CallGraph::new());
    analysis.has_coverage_data = true;
    analysis.add_item(create_untested_item());

    let output = format_priorities(&analysis, OutputFormat::Default);

    // SHOULD contain UNTESTED indicator
    assert!(output.contains("UNTESTED") || output.contains("🔴"));
}
```

### Integration Tests

1. **End-to-End Without Coverage**:
```rust
#[test]
fn test_analysis_without_lcov_file() {
    let result = run_command(&["debtmap", "analyze", "test-project/"]);

    // Should succeed
    assert!(result.success);

    // Should show tip about coverage
    assert!(result.stderr.contains("Coverage data not provided"));
    assert!(result.stderr.contains("--lcov-file"));

    // Should NOT show UNTESTED
    assert!(!result.stdout.contains("UNTESTED"));
}
```

2. **End-to-End With Coverage**:
```rust
#[test]
fn test_analysis_with_lcov_file() {
    let result = run_command(&[
        "debtmap",
        "analyze",
        "test-project/",
        "--lcov-file",
        "coverage.info"
    ]);

    // Should succeed
    assert!(result.success);

    // Should NOT show coverage tip
    assert!(!result.stderr.contains("Coverage data not provided"));

    // SHOULD show coverage indicators
    assert!(result.stdout.contains("COVERAGE") || result.stdout.contains("TESTED"));
}
```

### Performance Tests

No specific performance tests required - scoring overhead should be negligible.

## Documentation Requirements

### Code Documentation

1. Add doc comments to `calculate_base_score_no_coverage`:
```rust
/// Calculate base score when coverage data is not available.
///
/// Uses adjusted weights focusing on observable code quality metrics:
/// - 50% complexity (cyclomatic and cognitive complexity)
/// - 25% dependencies (upstream callers indicating change risk)
/// - 25% debt patterns (code smells, duplication, resource issues)
///
/// This provides meaningful prioritization even without test coverage data.
```

2. Update `UnifiedAnalysis` struct documentation:
```rust
/// Unified technical debt analysis results.
///
/// Fields:
/// - `has_coverage_data`: Indicates if lcov coverage data was provided.
///   When false, scoring excludes coverage and focuses on complexity/smells.
```

### User Documentation

Update README.md and user guide:

```markdown
## Coverage Data (Optional)

Coverage data enhances debtmap's analysis but is not required.

### Without Coverage Data

```bash
debtmap analyze src/
```

Focuses on:
- Code complexity (cyclomatic, cognitive)
- Architectural issues (god objects, feature envy)
- Code smells (duplication, magic values)
- Dependency risk (highly coupled functions)

### With Coverage Data

```bash
# Generate coverage first
cargo tarpaulin --out Lcov --output-path coverage.info

# Analyze with coverage
debtmap analyze src/ --lcov-file coverage.info
```

Includes all of the above PLUS:
- Test gap detection
- Coverage-based prioritization
- Untested code identification
```

### Architecture Updates

Update ARCHITECTURE.md to document the conditional scoring approach:

```markdown
## Scoring System

The unified scoring system adapts based on available data:

### With Coverage Data
- 50% coverage gap weight
- 35% complexity weight
- 15% dependency weight

### Without Coverage Data
- 50% complexity weight
- 25% dependency weight
- 25% debt pattern weight
```

## Implementation Notes

### Edge Cases

1. **Empty Coverage File**: Treat as "has coverage data" but all functions are 0%
2. **Partial Coverage**: Some functions covered, some not - still "has coverage data"
3. **Invalid Coverage File**: Show error, fall back to "no coverage data" mode

### Gotchas

- Must propagate `has_coverage_data` through entire pipeline consistently
- Don't confuse "no coverage data" with "0% coverage" - these are different states
- Warning should only appear once, not repeated in verbose mode

### Best Practices

- Use pure functions for conditional scoring logic
- Keep coverage-aware and coverage-agnostic code paths clearly separated
- Test both paths independently to ensure neither breaks the other

## Migration and Compatibility

### Breaking Changes

None - this is purely additive functionality.

### Migration Requirements

None - existing workflows continue to work unchanged.

### Compatibility Considerations

- JSON output format unchanged (coverage fields may be null/absent)
- Markdown output format adjusted only when no coverage data
- API consumers relying on "UNTESTED" labels may need updates (low risk)

## Success Metrics

1. **User Confusion Reduction**: No more "why is everything UNTESTED?" support requests
2. **Adoption**: Users can analyze code immediately without setting up coverage
3. **Quality**: Meaningful rankings even without coverage data
4. **Clarity**: Clear messaging about what coverage data enables

## Alternative Approaches Considered

### Alternative 1: Default to 50% Coverage
**Rejected** - Misleading to assume any specific coverage level

### Alternative 2: Disable All Scoring Without Coverage
**Rejected** - Still valuable to analyze complexity and code smells

### Alternative 3: Show "NO DATA" Instead of "UNTESTED"
**Considered but rejected** - Better to omit confusing labels entirely when not applicable
