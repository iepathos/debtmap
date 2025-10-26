# Migration Guide: Scale-Dependent to Density-Based Validation

## Why Migrate?

Traditional validation metrics like maximum complexity, total dependencies, or test count suffer from a fundamental flaw: they scale with codebase size. A small project with 1,000 lines might have 50 functions and 10 dependencies, while a large project with 100,000 lines might have 5,000 functions and 1,000 dependencies. Using absolute thresholds makes it impossible to set meaningful limits that work across different project sizes.

**Density-based validation solves this problem** by normalizing metrics to per-line or per-function rates. This allows you to set consistent quality thresholds regardless of project size.

### Key Benefits

1. **Size-Independent Quality Standards**: The same thresholds work for small and large projects
2. **Early Detection**: Catch quality degradation before it becomes a major issue
3. **Predictable CI/CD**: No need to adjust thresholds as your project grows
4. **Meaningful Comparisons**: Compare quality across different projects and teams

## Understanding Density Metrics

### Complexity Density

**Traditional (Scale-Dependent)**:
```bash
# Fails when total complexity > 1000
# Problem: Threshold needs constant adjustment as code grows
debtmap analyze --max-complexity 1000
```

**Density-Based (Scale-Independent)**:
```bash
# Fails when average complexity per function > 10
# Works for any project size
debtmap analyze --max-complexity-density 10.0
```

**Interpretation**:
- `complexity_density = total_complexity / total_functions`
- A density of 10.0 means each function has an average complexity of 10
- Recommended thresholds:
  - **Excellent**: < 5.0 (simple, well-decomposed functions)
  - **Good**: 5.0 - 10.0 (acceptable complexity)
  - **Warning**: 10.0 - 15.0 (needs refactoring)
  - **Critical**: > 15.0 (high technical debt)

### Dependency Density

**Traditional (Scale-Dependent)**:
```bash
# Fails when total dependencies > 100
debtmap analyze --max-dependencies 100
```

**Density-Based (Scale-Independent)**:
```bash
# Fails when dependencies per 1000 lines > 5.0
debtmap analyze --max-dependency-density 5.0
```

**Interpretation**:
- `dependency_density = (total_dependencies / total_lines) * 1000`
- A density of 5.0 means 5 dependencies per 1,000 lines of code
- Recommended thresholds:
  - **Excellent**: < 3.0 (minimal external dependencies)
  - **Good**: 3.0 - 5.0 (reasonable dependency usage)
  - **Warning**: 5.0 - 8.0 (dependency-heavy)
  - **Critical**: > 8.0 (excessive dependencies)

### Test Coverage Density

**Traditional (Scale-Dependent)**:
```bash
# Requires at least 100 tests
debtmap analyze --min-tests 100
```

**Density-Based (Scale-Independent)**:
```bash
# Requires at least 2 tests per 100 lines
debtmap analyze --min-test-density 2.0
```

**Interpretation**:
- `test_density = (total_tests / total_lines) * 100`
- A density of 2.0 means 2 tests per 100 lines of code
- Recommended thresholds:
  - **Excellent**: > 3.0 (comprehensive test coverage)
  - **Good**: 2.0 - 3.0 (solid test coverage)
  - **Warning**: 1.0 - 2.0 (minimal testing)
  - **Critical**: < 1.0 (insufficient tests)

## Step-by-Step Migration Process

### Step 1: Analyze Current State

Run debtmap with the new `--density-metrics` flag to see your current density values:

```bash
debtmap analyze --density-metrics > current-state.json
```

Examine the output to understand your baseline:

```json
{
  "summary": {
    "total_complexity": 4520,
    "total_functions": 387,
    "total_lines": 15234,
    "total_dependencies": 42,
    "total_tests": 284
  },
  "density_metrics": {
    "complexity_density": 11.68,
    "dependency_density": 2.76,
    "test_density": 1.86
  }
}
```

### Step 2: Set Initial Thresholds

Based on your current state, set realistic initial thresholds that won't immediately fail:

```bash
# Current values from above:
# complexity_density: 11.68
# dependency_density: 2.76
# test_density: 1.86

# Set thresholds slightly above current values
debtmap analyze \
  --max-complexity-density 13.0 \
  --max-dependency-density 3.5 \
  --min-test-density 1.5
```

**Rationale**: Start with achievable targets to avoid immediate CI failures.

### Step 3: Gradual Improvement Plan

Create a roadmap to improve density metrics over time:

**Quarter 1**: Stabilize at current levels
```bash
--max-complexity-density 13.0
--max-dependency-density 3.5
--min-test-density 1.5
```

**Quarter 2**: Incremental improvement
```bash
--max-complexity-density 12.0  # Reduce by ~8%
--max-dependency-density 3.2   # Reduce by ~9%
--min-test-density 1.7         # Increase by ~13%
```

**Quarter 3**: Approach best practices
```bash
--max-complexity-density 10.0  # Reduce by ~17%
--max-dependency-density 3.0   # Reduce by ~6%
--min-test-density 2.0         # Increase by ~18%
```

**Quarter 4**: Maintain excellence
```bash
--max-complexity-density 8.0   # Reduce by ~20%
--max-dependency-density 2.5   # Reduce by ~17%
--min-test-density 2.5         # Increase by ~25%
```

### Step 4: Update CI/CD Configuration

Replace absolute thresholds with density-based thresholds in your CI configuration.

**Before (GitHub Actions)**:
```yaml
- name: Check code quality
  run: |
    debtmap analyze \
      --max-complexity 1000 \
      --max-dependencies 100 \
      --min-tests 200
```

**After**:
```yaml
- name: Check code quality
  run: |
    debtmap analyze \
      --max-complexity-density 13.0 \
      --max-dependency-density 3.5 \
      --min-test-density 1.5
```

### Step 5: Monitor and Adjust

Track your density metrics over time to ensure they're improving:

```bash
# Weekly monitoring
debtmap analyze --density-metrics --output-format json > "metrics-$(date +%Y-%m-%d).json"

# Compare trends
jq '.density_metrics' metrics-*.json
```

## Threshold Selection Guidelines

### For New Projects

Start with industry best practices:

```bash
debtmap analyze \
  --max-complexity-density 8.0 \
  --max-dependency-density 3.0 \
  --min-test-density 2.5
```

These thresholds encourage:
- Simple, well-decomposed functions
- Minimal external dependencies
- Comprehensive test coverage

### For Legacy Projects

Start with current state + 20% buffer:

```python
# Calculate from current analysis
current_complexity_density = 15.2
current_dependency_density = 6.8
current_test_density = 0.9

# Set thresholds with 20% buffer
max_complexity_density = current_complexity_density * 1.2  # 18.24
max_dependency_density = current_dependency_density * 1.2  # 8.16
min_test_density = current_test_density * 0.8              # 0.72
```

Then gradually tighten over 6-12 months.

### For Multi-Team Projects

Use percentile-based thresholds:

```bash
# Get current distribution
debtmap analyze --density-metrics --per-file

# Set thresholds at 75th percentile
# This allows 25% of files to exceed limits initially
# But prevents new code from making things worse
```

## Example Before/After Configurations

### Small Microservice (5,000 lines)

**Before**:
```yaml
quality_gates:
  max_total_complexity: 500
  max_dependencies: 20
  min_test_count: 50
```

**After**:
```yaml
quality_gates:
  max_complexity_density: 8.0    # 500 / ~60 functions
  max_dependency_density: 4.0    # 20 / 5000 * 1000
  min_test_density: 1.0          # 50 / 5000 * 100
```

### Medium Web Application (50,000 lines)

**Before**:
```yaml
quality_gates:
  max_total_complexity: 3000
  max_dependencies: 150
  min_test_count: 800
```

**After**:
```yaml
quality_gates:
  max_complexity_density: 6.0    # 3000 / ~500 functions
  max_dependency_density: 3.0    # 150 / 50000 * 1000
  min_test_density: 1.6          # 800 / 50000 * 100
```

### Large Enterprise System (500,000 lines)

**Before**:
```yaml
quality_gates:
  max_total_complexity: 50000    # Constantly adjusted
  max_dependencies: 2000         # Frequently exceeded
  min_test_count: 10000          # Hard to maintain
```

**After**:
```yaml
quality_gates:
  max_complexity_density: 10.0   # 50000 / ~5000 functions
  max_dependency_density: 4.0    # 2000 / 500000 * 1000
  min_test_density: 2.0          # 10000 / 500000 * 100
```

## Frequently Asked Questions

### Q: Should I completely remove absolute thresholds?

**A**: Not immediately. Use both during migration:

```bash
debtmap analyze \
  --max-complexity 5000 \           # Absolute ceiling
  --max-complexity-density 10.0 \   # Density target
  --max-dependency-density 3.5 \
  --min-test-density 2.0
```

After 3-6 months, drop absolute thresholds once density metrics are stable.

### Q: What if my density metrics are already high?

**A**: Set thresholds at current levels first, then create improvement plan:

```bash
# Phase 1: Don't make it worse (month 1-3)
--max-complexity-density 18.0  # Current: 17.5

# Phase 2: Start improving (month 4-6)
--max-complexity-density 16.0

# Phase 3: Reach acceptable levels (month 7-12)
--max-complexity-density 12.0
```

### Q: How do I handle different file types?

**A**: Use per-language thresholds if your project is multi-language:

```bash
# Rust (typically lower complexity)
debtmap analyze src/**/*.rs \
  --max-complexity-density 8.0

# Python (typically higher complexity)
debtmap analyze src/**/*.py \
  --max-complexity-density 12.0
```

### Q: What about generated code?

**A**: Exclude generated code from analysis:

```bash
debtmap analyze \
  --exclude "**/generated/**" \
  --exclude "**/*.pb.rs" \
  --max-complexity-density 10.0
```

### Q: Can I use density metrics for PR reviews?

**A**: Yes! Check density delta in PRs:

```bash
# Before PR
debtmap analyze --density-metrics > before.json

# After PR
debtmap analyze --density-metrics > after.json

# Compare
jq -s '.[1].density_metrics.complexity_density - .[0].density_metrics.complexity_density' before.json after.json
```

Fail PR if density increases beyond threshold:

```yaml
- name: Check density delta
  run: |
    DELTA=$(jq -s '...' before.json after.json)
    if (( $(echo "$DELTA > 0.5" | bc -l) )); then
      echo "Complexity density increased by $DELTA"
      exit 1
    fi
```

### Q: What if I'm starting fresh?

**A**: Lucky you! Use strict thresholds from day one:

```bash
debtmap analyze \
  --max-complexity-density 5.0 \   # Excellent target
  --max-dependency-density 2.0 \   # Minimal dependencies
  --min-test-density 3.0           # Comprehensive tests
```

### Q: How do density metrics relate to code quality?

**A**: Direct correlation:

| Complexity Density | Code Quality  | Maintenance Effort |
|-------------------|---------------|-------------------|
| < 5.0             | Excellent     | Low               |
| 5.0 - 8.0         | Good          | Moderate          |
| 8.0 - 12.0        | Fair          | High              |
| > 12.0            | Poor          | Very High         |

### Q: Can I combine density metrics with other quality gates?

**A**: Absolutely! Density metrics work well with:

```bash
debtmap analyze \
  --max-complexity-density 10.0 \
  --max-dependency-density 3.5 \
  --min-test-density 2.0 \
  --max-debt-score 50 \           # Traditional debt scoring
  --require-tests \               # Enforce test presence
  --check-style                   # Code style checks
```

## Troubleshooting

### Issue: Density metrics fluctuate wildly

**Cause**: Small changes in line count or function count
**Solution**: Use rolling averages or require minimum sample size

```bash
# Only enforce if project has > 1000 lines
if [ $(wc -l < src/**/*.rs) -gt 1000 ]; then
  debtmap analyze --max-complexity-density 10.0
fi
```

### Issue: Tests skew density metrics

**Cause**: Test files included in analysis
**Solution**: Exclude test files from production metrics

```bash
debtmap analyze \
  --exclude "**/tests/**" \
  --exclude "**/*_test.rs" \
  --max-complexity-density 10.0
```

### Issue: Legacy code dominates metrics

**Cause**: Old code with high complexity/dependencies
**Solution**: Progressive analysis with path filters

```bash
# Strict for new code
debtmap analyze src/new_features/** \
  --max-complexity-density 8.0

# Lenient for legacy
debtmap analyze src/legacy/** \
  --max-complexity-density 15.0
```

## Additional Resources

- [CI/CD Integration Guide](../README.md#cicd-integration)
- [Score Interpretation Guide](./score-interpretation-guide.md)
- [Tiered Prioritization](./tiered-prioritization.md)
- [Output Format Documentation](./output-format.md)

## Summary

Density-based validation provides:
- **Consistent quality standards** across project sizes
- **Predictable CI/CD** without constant threshold adjustments
- **Meaningful metrics** that drive actual improvements

Start with current state, set realistic targets, and gradually improve over time. Your future self (and team) will thank you!
