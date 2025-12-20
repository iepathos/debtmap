# Metrics Reference

Comprehensive guide to all signals measured by Debtmap and how to interpret them.

Debtmap acts as a **sensor**, providing quantified signals about code complexity and risk. These signals are designed for consumption by AI coding tools and developers alike.

## Metric Categories (Spec 118)

Debtmap distinguishes between two fundamental categories of metrics:

### Measured Metrics

**Definition**: Metrics directly computed from the Abstract Syntax Tree (AST) through precise analysis.

These metrics are:
- **Deterministic**: Same code always produces the same metric value
- **Precise**: Exact counts from syntax analysis, not estimates
- **Suitable for thresholds**: Reliable for CI/CD quality gates
- **Language-specific**: Computed using language parsers (syn for Rust, tree-sitter for others)

**Measured metrics include:**

| Metric | Description | Example |
|--------|-------------|---------|
| `cyclomatic_complexity` | Count of decision points (if, match, while, for, etc.) | Function with 3 if statements = complexity 4 |
| `cognitive_complexity` | Weighted measure of code understandability | Nested loops increase cognitive load |
| `nesting_depth` | Maximum levels of nested control structures | 3 nested if statements = depth 3 |
| `loc` | Lines of code in the function | Physical line count |
| `parameter_count` | Number of function parameters | `fn foo(a: i32, b: String)` = 2 |

### Estimated Metrics

**Definition**: Heuristic approximations calculated using formulas, not direct AST measurements.

These metrics are:
- **Heuristic**: Based on mathematical formulas and assumptions
- **Approximate**: Close estimates, not exact counts
- **Useful for prioritization**: Help estimate effort and risk
- **Not suitable for hard thresholds**: Use for relative comparisons, not absolute gates

**Estimated metrics include:**

| Metric | Formula | Purpose | Example |
|--------|---------|---------|---------|
| `est_branches` | `max(nesting, 1) × cyclomatic ÷ 3` | Estimate test cases needed for branch coverage | Complexity 12, nesting 3 → ~12 branches |

**Important**: The `est_branches` metric was previously called `branches`. It was renamed in Spec 118 to make it explicit that this is an **estimate**, not a precise count from the AST.

## Why the Distinction Matters

### For Code Quality Gates

```bash
# GOOD: Use measured metrics for CI/CD thresholds
debtmap validate . --threshold-complexity 15

# AVOID: Don't use estimated metrics for hard gates
# (est_branches is not exposed as a threshold flag)
```

**Rationale**: Measured metrics are deterministic and precise, making them suitable for build-breaking quality gates.

### For Prioritization

```bash
# GOOD: Use est_branches for prioritization
debtmap analyze . --top 10  # Sorts by est_branches (among other factors)

# GOOD: Estimated metrics help understand testing effort
debtmap analyze . --lcov coverage.info --verbose
```

**Rationale**: Estimated metrics provide useful heuristics for understanding where to focus testing and refactoring efforts.

### For Comparison Across Codebases

- **Measured metrics**: Comparable across projects (cyclomatic 10 means the same everywhere)
- **Estimated metrics**: Project-specific heuristics (est_branches depends on nesting patterns)

## Detailed Metric Descriptions

### Cyclomatic Complexity (Measured)

**What it measures**: The number of linearly independent paths through a function's control flow.

**How it's calculated**:
- Start with a base of 1
- Add 1 for each decision point:
  - `if`, `else if`
  - `match` arms
  - `while`, `for`, `loop`
  - `&&`, `||` in conditions
  - `?` operator (early return)

**Example**:
```rust
fn example(x: i32, y: i32) -> bool {
    if x > 0 {        // +1
        if y > 0 {    // +1
            true
        } else {      // implicit in if/else
            false
        }
    } else if x < 0 { // +1
        false
    } else {
        y == 0        // no additional branches
    }
}
// Cyclomatic complexity = 1 + 3 = 4
```

**Thresholds**:
- **1-5**: Simple, easy to test
- **6-10**: Moderate, manageable complexity
- **11-20**: Complex, consider refactoring
- **21+**: Very complex, high maintenance cost

### Cognitive Complexity (Measured)

**What it measures**: How difficult the code is for humans to understand.

**How it differs from cyclomatic**:
- Weights nested structures more heavily (nested if is worse than sequential if)
- Ignores shorthand structures (early returns, guard clauses)
- Focuses on readability, not just logic paths

**Example**:
```rust
fn cyclomatic_low_cognitive_low(status: Status) -> bool {
    match status {  // Cyclomatic: 4, Cognitive: 1
        Status::Active => true,
        Status::Pending => false,
        Status::Closed => false,
        Status::Error => false,
    }
}

fn cyclomatic_low_cognitive_high(x: i32, y: i32, z: i32) -> bool {
    if x > 0 {
        if y > 0 {      // Nested: +2 cognitive penalty
            if z > 0 {  // Deeply nested: +3 cognitive penalty
                return true;
            }
        }
    }
    false
}
// Cyclomatic: 4, Cognitive: 7 (nesting penalty applied)
```

**Thresholds**:
- **1-5**: Easy to understand
- **6-10**: Moderate mental load
- **11-15**: Difficult to follow
- **16+**: Refactor recommended

### Estimated Branches (Estimated)

**What it estimates**: Approximate number of execution paths that would need test coverage.

**Formula**:
```
est_branches = max(nesting_depth, 1) × cyclomatic_complexity ÷ 3
```

**Why this formula**:
- **Nesting multiplier**: Deeper nesting creates more combinations
- **Cyclomatic base**: Higher complexity → more paths
- **÷ 3 adjustment**: Empirical factor to align with typical branch coverage needs

**Example scenarios**:

| Cyclomatic | Nesting | est_branches | Interpretation |
|------------|---------|--------------|----------------|
| 3 | 1 | 1 | Simple linear code |
| 12 | 1 | 4 | Multiple sequential branches |
| 12 | 3 | 12 | Nested conditions, many paths |
| 20 | 5 | 33 | Complex nested logic |

**Use cases**:
- Estimating test case requirements
- Prioritizing untested complex code
- Understanding coverage gaps

**Limitations**:
- **Not a precise count**: This is a heuristic approximation
- **Don't use for coverage percentage calculation**: Use actual coverage tools
- **Varies by code style**: Heavily nested code scores higher

## Terminology Change (Spec 118)

### Before: `branches`
Previously, this metric was displayed as `branches=X`, which was confusing because:
1. Users thought it was a precise count from AST analysis
2. It was mistaken for cyclomatic complexity (actual branch count)
3. The estimation nature was not obvious

### After: `est_branches`
Now displayed as `est_branches=X` to:
1. **Make estimation explicit**: "est_" prefix indicates this is approximate
2. **Avoid confusion**: Clearly different from cyclomatic complexity
3. **Set correct expectations**: Users know this is a heuristic, not a measurement

### Migration Guide

**Terminal Output**:
- Old: `COMPLEXITY: cyclomatic=12, branches=8, cognitive=15`
- New: `COMPLEXITY: cyclomatic=12, est_branches=8, cognitive=15`

**Code**:
- Internal variable names updated from `branches` to `est_branches`
- Comments added explaining the estimation formula

**JSON Output**:
- No change: The ComplexityMetrics struct does not include this field
- `est_branches` is calculated on-demand for display purposes only

## Practical Usage Examples

### Example 1: Code Quality Gate

```bash
# Fail build if any function exceeds cyclomatic complexity 15
debtmap validate . --threshold-complexity 15 --max-high 0

# Why: Cyclomatic is measured, precise, and repeatable
```

### Example 2: Prioritize Testing Effort

```bash
# Show top 10 functions by risk (uses est_branches in scoring)
debtmap analyze . --lcov coverage.info --top 10

# Functions with high est_branches and low coverage appear first
```

### Example 3: Understanding Test Requirements

```bash
# Verbose output shows est_branches for each function
debtmap analyze . --verbose

# Output:
# └─ COMPLEXITY: cyclomatic=12, est_branches=8, cognitive=15, nesting=2
#
# Interpretation: ~8 test cases likely needed for good branch coverage
```

### Example 4: Explaining Metrics to Team

```bash
# Display comprehensive metric definitions
debtmap analyze --explain-metrics

# Shows:
# - Measured vs Estimated categories
# - Formulas and thresholds
# - When to use each metric
```

## Metric Selection Guide

### When to Use Cyclomatic Complexity

✅ **Use for:**
- CI/CD quality gates
- Code review guidelines
- Consistent cross-project comparison
- Identifying refactoring candidates

❌ **Don't use for:**
- Estimating test effort (use est_branches)
- Readability assessment (use cognitive complexity)

### When to Use Cognitive Complexity

✅ **Use for:**
- Readability reviews
- Identifying hard-to-maintain code
- Onboarding difficulty assessment

❌ **Don't use for:**
- Test coverage planning
- Strict quality gates (more subjective than cyclomatic)

### When to Use est_branches

✅ **Use for:**
- Estimating test case requirements
- Prioritizing test coverage work
- Understanding coverage gaps

❌ **Don't use for:**
- CI/CD quality gates (it's an estimate)
- Calculating coverage percentages (use actual coverage data)
- Cross-project comparison (formula is heuristic)

## Combining Metrics for Insights

### High Complexity, Low Coverage

```
cyclomatic=18, est_branches=12, coverage=0%
```
**Interpretation**: High-risk code needing ~12 test cases for adequate coverage.

**Action**: Prioritize writing tests, consider refactoring.

### High Cyclomatic, Low Cognitive

```
cyclomatic=15, cognitive=5
```
**Interpretation**: Many branches, but simple linear logic (e.g., validation checks).

**Action**: Acceptable pattern, tests should be straightforward.

### Low Cyclomatic, High Cognitive

```
cyclomatic=8, cognitive=18
```
**Interpretation**: Deeply nested logic, hard to understand despite fewer branches.

**Action**: Refactor to reduce nesting, extract functions.

### High est_branches, Low Cyclomatic

```
cyclomatic=9, nesting=5, est_branches=15
```
**Interpretation**: Deep nesting creates many path combinations.

**Action**: Flatten nesting, use early returns, extract nested logic.

## Frequently Asked Questions

### Q: Why is est_branches different from cyclomatic complexity?

**A**: Cyclomatic is the **measured** count of decision points. `est_branches` is an **estimated** number of execution paths, calculated using nesting depth to account for path combinations.

### Q: Can I use est_branches in CI/CD thresholds?

**A**: No. Use measured metrics (cyclomatic_complexity, cognitive_complexity) for quality gates. `est_branches` is a heuristic for prioritization, not a precise measurement.

### Q: Why did the metric name change from "branches" to "est_branches"?

**A**: To make it explicit that this is an **estimate**, not a measured value. Users were confused, thinking it was a precise count from the AST.

### Q: How accurate is est_branches for estimating test cases?

**A**: It's a **rough approximation**. Actual test case requirements depend on:
- Business logic complexity
- Edge cases
- Error handling paths
- Integration points

Use `est_branches` as a starting point, not an exact requirement.

### Q: Should I refactor code with high est_branches?

**A**: Not necessarily. High `est_branches` indicates complex logic that may need thorough testing. If the logic is unavoidable (e.g., state machines, complex business rules), focus on comprehensive test coverage rather than refactoring.

## Signal Categories Summary

| Category | Signals | Purpose |
|----------|---------|---------|
| Complexity | cyclomatic, cognitive, nesting, loc | How hard code is to understand |
| Coverage | line_percent, branch_percent | How risky changes are |
| Coupling | fan_in, fan_out, call_depth | How changes ripple |
| Quality | entropy, purity, dead_code | False positive reduction |

## Using Signals with AI

When piping debtmap output to an AI assistant, signals provide the context needed for intelligent fixes:

```bash
# Get structured signals for AI consumption
debtmap analyze . --format llm-markdown --top 5 | claude "Fix the top item"
```

The AI uses these signals to:
- Understand code complexity before reading it
- Prioritize which files to examine first
- Decide between refactoring vs testing approaches
- Estimate the scope of changes needed

## Further Reading

- [Why Debtmap?](why-debtmap.md) - Sensor model explained
- [LLM Integration](llm-integration.md) - AI workflow patterns
- [Configuration](configuration.md#thresholds) - Threshold customization
- [Scoring Strategies](scoring-strategies.md) - How signals combine
