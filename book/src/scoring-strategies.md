# Scoring Strategies

Debtmap provides two complementary scoring approaches: **file-level** and **function-level**. Understanding when to use each approach helps you make better refactoring decisions and prioritize work effectively.

## Overview

Different refactoring scenarios require different levels of granularity:
- **File-level scoring**: Identifies architectural issues and planning major refactoring initiatives
- **Function-level scoring**: Pinpoints specific hot spots for targeted improvements

This chapter explains both approaches, when to use each, and how to interpret the results.

## File-Level Scoring

File-level scoring aggregates metrics across all functions in a file to identify architectural problems and module-level refactoring opportunities.

### Formula

```
File Score = Size × Complexity × Coverage × Density × GodObject × FunctionScores
```

Where each factor is calculated as:
- **Size** = `sqrt(total_lines / 100)`
- **Complexity** = `(avg_complexity / 5.0) × sqrt(total_complexity / 50.0)`
- **Coverage** = `(1 - coverage_percent) × 2 + 1`
- **Density** = `max(1.0, function_count / 50)` if function_count > 50
- **GodObject** = `2.0 + god_object_score` if detected
- **FunctionScores** = `sum(function_scores) / 10`

### Factors

**Size Factor**: `sqrt(total_lines / 100)`
- Larger files have higher impact
- Square root dampens the effect to avoid over-penalizing large files
- Rationale: Refactoring a 1000-line file affects more code than a 100-line file

**Complexity Factor**: Combines average and total complexity
- `(average_cyclomatic + total_cyclomatic / function_count) / 2`
- Balances per-function and aggregate complexity
- Rationale: Both concentrated complexity and spread-out complexity matter

**Coverage Factor**: `(1 - coverage_percent) × 2 + 1`
- Lower coverage increases score multiplicatively
- Range: 1.0 (100% coverage) to 3.0 (0% coverage)
- Rationale: Untested files are riskier to refactor

**Density Factor**: Penalizes files with excessive function count
- Triggers when function count > 50
- `max(1.0, function_count / 50)`
- Rationale: Files with 100+ functions likely violate single responsibility

**God Object Multiplier**: `2.0 + god_object_score` when detected
- Applies when god object detection flags the file
- Range: 2.0 (borderline) to 3.0 (severe god object)
- Rationale: God objects need immediate architectural attention

**Function Scores**: `sum(all_function_scores) / 10`
- Normalized sum of individual function debt scores
- Provides baseline before modifiers

### Use Cases

**1. Planning Major Refactoring Initiatives**

```bash
# Show top 10 files needing architectural refactoring
debtmap analyze . --aggregate-only --top 10
```

Use when:
- Planning sprint or quarterly refactoring work
- Deciding which modules to split
- Prioritizing architectural improvements
- Allocating team resources

**Note**: File-level scoring is enabled with the `--aggregate-only` flag, which changes output to show only file-level metrics instead of function-level details.

**2. Identifying Architectural Issues**

File-level scoring excels at finding:
- God objects with too many responsibilities
- Files with poor cohesion
- Modules that should be split
- Files with too many functions

```bash
# Focus on architectural problems
debtmap analyze . --aggregate-only --filter Architecture
```

**3. Breaking Up Monolithic Modules**

```bash
# Find files with excessive function counts
debtmap analyze . --aggregate-only --min-problematic 50
```

**4. Evaluating Overall Codebase Health**

```bash
# Generate file-level report for executive summary
debtmap analyze . --aggregate-only --format markdown -o report.md
```

### Aggregation Methods

Debtmap supports multiple aggregation methods for file-level scores, configurable via CLI or configuration file.

**Weighted Sum (Default)**
```bash
debtmap analyze . --aggregation-method weighted_sum
```

Or via configuration:
```toml
[aggregation]
method = "weighted_sum"
```
- Weights functions by complexity and coverage
- Emphasizes high-impact functions
- Best for most use cases

**Simple Sum**
```toml
[aggregation]
method = "sum"
```
- Adds all function scores directly
- Treats all functions equally
- Useful for broad overview

**Logarithmic Sum**
```toml
[aggregation]
method = "logarithmic_sum"
```
- Dampens impact of many small issues
- `log(1 + sum_of_scores)`
- Useful for legacy codebases with many minor issues

**Max Plus Average**
```toml
[aggregation]
method = "max_plus_average"
```
- Considers worst function plus average of others
- `max_score × 0.6 + average_score × 0.4`
- Balances worst-case and typical-case analysis

### Configuration

```toml
[aggregation]
method = "weighted_sum"
min_problematic = 3              # Need 3+ problematic functions for file-level score

[god_object_detection]
enabled = true
max_methods = 20
max_fields = 15
max_responsibilities = 5
```

## Function-Level Scoring

Function-level scoring identifies specific functions needing attention for targeted improvements.

### Formula

```
Function Score = (Complexity × 0.35) + (Coverage × 0.50) + (Dependency × 0.15)
Final Score = Base Score × Role Multiplier
```

**Note**: These weights are configurable via the `[scoring]` section in `.debtmap.toml`. See [Configuration](#configuration) below.

### Metrics

**Cyclomatic Complexity**
- Counts decision points (if, match, loops)
- Guides test case count

**Cognitive Complexity**
- Measures understanding difficulty
- Accounts for nesting depth

**Coverage Percentage**
- Direct line coverage from LCOV
- 0% coverage = maximum urgency

**Dependency Count**
- Upstream callers + downstream callees
- Higher dependencies = higher impact

**Role Multiplier**

Functions are classified by role, and each role receives a multiplier based on its architectural importance:

- **Entry points**: 1.5x - Public APIs, main functions, HTTP handlers
- **Pure logic / Business logic**: 1.2x-1.3x - Core business rules and algorithms (multiplier increases with complexity)
- **Orchestrator**: 0.8x - Functions that coordinate other functions
- **IO wrapper**: 0.5x - Simple file/network I/O wrappers
- **Pattern match**: 0.6x - Functions primarily doing pattern matching
- **Utility**: 0.5x - Helper functions and utilities

**Note**: Role multipliers are configurable via the `[role_multipliers]` section in `.debtmap.toml`.

### Use Cases

**1. Identifying Specific Hot Spots**

```bash
# Show top 20 functions needing attention
debtmap analyze . --top 20
```

Use when:
- Planning individual developer tasks
- Assigning specific refactoring work
- Identifying functions to test first
- Code review focus

**2. Sprint Planning for Developers**

```bash
# Get function-level tasks for this sprint
debtmap analyze . --top 10 --format json -o sprint-tasks.json
```

**3. Writing Unit Tests**

```bash
# Find untested complex functions
debtmap analyze . --lcov coverage.lcov --filter Testing --top 15
```

**4. Targeted Performance Optimization**

```bash
# Find complex hot paths
debtmap analyze . --filter Performance --context --top 10
```

### Configuration

```toml
[scoring]
coverage = 0.50              # Weight for coverage gaps (default)
complexity = 0.35            # Weight for complexity (default)
dependency = 0.15            # Weight for dependency impact (default)

[role_multipliers]
entry_point = 1.5
business_logic = 1.2
pure_logic = 1.2
orchestrator = 0.8
io_wrapper = 0.7
utility = 0.5
```

## When to Use Each Approach

### Use File-Level Scoring When:

✅ Planning architectural refactoring
✅ Quarterly or annual planning
✅ Deciding which modules to split
✅ Executive summaries and high-level reports
✅ Team capacity planning
✅ Identifying god objects
✅ Module reorganization

**Command:**
```bash
debtmap analyze . --aggregate-only
```

### Use Function-Level Scoring When:

✅ Sprint planning
✅ Individual developer task assignment
✅ Writing specific unit tests
✅ Code review preparation
✅ Pair programming sessions
✅ Daily or weekly development work
✅ Targeted hot spot fixes

**Command:**
```bash
debtmap analyze . --top 20
```

### Use Both Together:

Many workflows benefit from both views:

```bash
# Step 1: Identify problematic files
debtmap analyze . --aggregate-only --top 5 -o files.json

# Step 2: Drill into specific file
debtmap analyze src/problematic/module.rs --format terminal
```

## Comparison Examples

### Example 1: God Object Detection

**File-Level View:**
```
src/services/user_service.rs - Score: 245.8
  - 850 lines, 45 methods
  - God Object: 78% score
  - Action: Split into UserAuth, UserProfile, UserNotifications
```

**Function-Level View:**
```
src/services/user_service.rs:142 - authenticate_user() - Score: 8.5
src/services/user_service.rs:298 - update_profile() - Score: 7.2
src/services/user_service.rs:456 - send_notification() - Score: 6.8
```

**Decision**: File-level score (245.8) correctly identifies architectural issue. Individual functions aren't exceptionally complex, but the file has too many responsibilities. **Solution**: Split the file.

### Example 2: Targeted Function Fix

**File-Level View:**
```
src/parsers/expression.rs - Score: 45.2
  - 320 lines, 12 functions
  - No god object detected
```

**Function-Level View:**
```
src/parsers/expression.rs:89 - parse_complex_expression() - Score: 9.1
  - Cyclomatic: 22, Cognitive: 35
  - Coverage: 0%
  - Action: Add tests and refactor
```

**Decision**: File as a whole is acceptable, but one function needs attention. **Solution**: Focus on that specific function.

### Example 3: Balanced Refactoring

**File-Level View:**
```
src/analysis/scoring.rs - Score: 125.6
  - 580 lines, 18 functions
  - High complexity, low coverage
```

**Function-Level View:**
```
calculate_score() - Score: 8.8 (15% coverage)
apply_weights() - Score: 8.2 (10% coverage)
normalize_results() - Score: 7.5 (0% coverage)
```

**Decision**: Both file and functions need work. **Solution**: Add tests first (function-level), then consider splitting if complexity persists (file-level).

## Score Normalization

Both scoring approaches normalize to a 0-10 scale for consistency.

### Normalization Strategy

```rust
score_normalized = if raw_score < 10.0 {
    raw_score  // Linear below 10
} else if raw_score < 100.0 {
    sqrt(raw_score) × 3.33  // Square root 10-100
} else {
    log10(raw_score) × 10.0  // Logarithmic above 100
}
```

### Configuration

```toml
[normalization]
linear_threshold = 10.0       # Scores below this use linear scaling (1:1 mapping)
logarithmic_threshold = 100.0 # Scores above this use logarithmic dampening
sqrt_multiplier = 3.33        # Applied to scores between linear and log thresholds
log_multiplier = 10.0         # Applied to scores above logarithmic threshold
show_raw_scores = true        # Display both normalized (0-10) and raw scores in output
```

**Explanation**:
- **linear_threshold**: Scores below this value are mapped 1:1 (no scaling)
- **logarithmic_threshold**: Scores above this value are dampened logarithmically to prevent extreme values
- **sqrt_multiplier**: Square root scaling applied to mid-range scores (between linear and logarithmic thresholds)
- **log_multiplier**: Logarithmic dampening factor for very high scores
- **show_raw_scores**: When enabled, output includes both the normalized 0-10 score and the raw calculated score

## Best Practices

### Workflow Integration

**Week 1: File-Level Assessment**
```bash
# Identify architectural problems
debtmap analyze . --aggregate-only --top 10
```

**Week 2-4: Function-Level Work**
```bash
# Work through specific functions
debtmap analyze src/target/module.rs
```

**Monthly: Compare Progress**
```bash
debtmap compare --before baseline.json --after current.json
```

### Team Collaboration

- **Architects**: Use file-level scores for strategic planning
- **Tech Leads**: Use both for sprint planning
- **Developers**: Use function-level for daily work
- **QA**: Use function-level for test prioritization

### CI/CD Integration

```bash
# Gate: No new file-level regressions
debtmap analyze . --aggregate-only --format json -o file-scores.json

# Gate: No new critical function-level issues
debtmap analyze . --min-priority critical --format json -o critical-items.json
```

## Troubleshooting

**Issue**: File-level scores seem too high

**Solution**: Check aggregation method:
```toml
[aggregation]
method = "logarithmic_sum"  # Dampen scores
```

**Issue**: Function-level scores all similar

**Solution**: Adjust scoring weights to emphasize different factors:
```toml
[scoring]
coverage = 0.60    # Emphasize testing gaps more
complexity = 0.30
dependency = 0.10
```

**Issue**: Too many low-priority items

**Solution**: Use minimum thresholds:
```toml
[thresholds]
minimum_debt_score = 3.0
```

## See Also

- [Tiered Prioritization](./tiered-prioritization.md) - Understanding tier-based classification
- [Configuration](./configuration.md) - Scoring and aggregation configuration
- [Analysis Guide](./analysis-guide.md) - Detailed metric explanations
