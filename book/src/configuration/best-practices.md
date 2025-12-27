# Configuration Best Practices

This subsection provides practical configuration guidance for different project types and development contexts. Use these recommendations as starting points, then tune based on your specific codebase.

## Overview

Effective Debtmap configuration depends on your project's characteristics:

- **Maturity level** - New projects vs legacy codebases
- **Quality standards** - Strict quality gates vs gradual improvement
- **Team context** - Open source library vs internal tool
- **Workflow integration** - CI/CD pipeline vs local development

The key is choosing thresholds that surface actionable issues without overwhelming noise.

## Configuration for Strict Quality Standards

Use strict configurations for new projects, libraries with public APIs, or teams committed to maintaining low technical debt.

**Source:** `src/config/thresholds.rs:120-196` (ValidationThresholds)

### Complete Strict Configuration

```toml
# Strict quality configuration for new/greenfield projects
# Prioritize coverage and early debt prevention

[scoring]
coverage = 0.60         # Emphasize test coverage
complexity = 0.30       # Moderate complexity weight
dependency = 0.10       # Low dependency weight

[thresholds]
complexity = 8                    # Lower cyclomatic threshold
max_function_length = 30          # Enforce smaller functions
minimum_debt_score = 3.0          # Higher bar for flagging issues

[thresholds.validation]
max_average_complexity = 8.0      # Strict complexity limits
max_debt_density = 30.0           # Low debt density tolerance
max_codebase_risk_score = 6.0     # Stricter risk tolerance
min_coverage_percentage = 80.0    # Require 80% coverage

[complexity_thresholds]
minimum_cyclomatic_complexity = 3
minimum_cognitive_complexity = 7
minimum_function_length = 15
```

### Key Principles for Strict Standards

1. **Start strict, loosen if needed** - It's easier to relax thresholds than tighten them later
2. **Enforce coverage early** - Set `min_coverage_percentage` from day one
3. **Use validation in CI** - Block merges that exceed thresholds
4. **Review high-scoring items weekly** - Prevent debt accumulation

### CLI Example

```bash
# Use strict preset for quick strictness
debtmap analyze . --threshold-preset strict

# Validate with strict configuration
debtmap validate . --config .debtmap.toml
```

## Configuration for Legacy Codebases

Legacy codebases need gradual improvement rather than strict enforcement. Focus on identifying the highest-priority items without overwhelming the team.

**Source:** `src/config/thresholds.rs:83-118` (ThresholdsConfig)

### Complete Legacy Configuration

```toml
# Legacy codebase configuration
# Focus on high-priority issues, reduce noise from moderate complexity

[scoring]
coverage = 0.30         # Reduce coverage weight (legacy often lacks tests)
complexity = 0.50       # Focus on complexity
dependency = 0.20       # Higher dependency weight for coupling issues

[thresholds]
minimum_debt_score = 5.0              # Only show highest priority items
minimum_cyclomatic_complexity = 10    # Filter out moderate complexity
minimum_cognitive_complexity = 15     # Focus on worst offenders
minimum_risk_score = 4.0              # High-risk items only

[thresholds.validation]
max_debt_density = 100.0              # Accommodate existing debt
max_average_complexity = 15.0         # Start lenient
max_total_debt_score = 5000           # Higher limits for legacy code
max_codebase_risk_score = 8.0         # More tolerant risk threshold

[complexity_thresholds]
minimum_total_complexity = 15
minimum_function_length = 50
```

### Gradual Threshold Tightening Strategy

1. **Week 1-2**: Run analysis with lenient thresholds, establish baseline
2. **Month 1**: Lower `minimum_debt_score` from 5.0 to 4.5, address top 10 items
3. **Month 2**: Lower `max_debt_density` by 10%, continue addressing high-priority items
4. **Quarterly**: Reduce thresholds by 10-15% until reaching target

**Example progression:**

```toml
# Month 1
[thresholds.validation]
max_debt_density = 100.0

# Month 3
[thresholds.validation]
max_debt_density = 85.0

# Month 6
[thresholds.validation]
max_debt_density = 70.0

# Target (Year 1)
[thresholds.validation]
max_debt_density = 50.0
```

### CLI Example

```bash
# Focus on high-priority items only
debtmap analyze . --min-score 5.0 --top 20

# Use lenient preset for legacy code
debtmap analyze . --threshold-preset lenient
```

## Configuration for Open Source Libraries

Open source libraries require high test coverage for public APIs and clear documentation of complexity.

### Complete Open Source Configuration

```toml
# Open source library configuration
# Prioritize test coverage and public API detection

[scoring]
coverage = 0.55         # High coverage weight (public API focus)
complexity = 0.30       # Moderate complexity weight
dependency = 0.15       # Standard dependency weight

[analysis]
detect_external_api = true       # Flag untested public APIs
public_api_threshold = 0.8       # 80% threshold for public API detection

[thresholds]
max_function_length = 40         # Moderate function size limit
minimum_debt_score = 2.5         # Surface more issues for thorough review

[thresholds.validation]
min_coverage_percentage = 90.0   # High coverage for public API
max_high_complexity_count = 20   # Keep complexity low
max_average_complexity = 9.0     # Strict complexity average
max_debt_density = 40.0          # Low debt tolerance

# Coverage expectations by function role
[coverage_expectations.pure]
min = 95.0
target = 98.0
max = 100.0

[coverage_expectations.io_operations]
min = 70.0
target = 80.0
max = 90.0
```

### Key Principles for Open Source

1. **Prioritize public API coverage** - Users depend on your public interface
2. **Document complexity trade-offs** - Explain why complex functions exist
3. **Use semantic classification** - Apply role-based scoring for accurate prioritization
4. **Enable pattern detection** - Identify boilerplate for macro opportunities

### CLI Example

```bash
# Analyze with public API detection
debtmap analyze . --no-public-api-detection=false --public-api-threshold 0.8

# Generate markdown report for documentation
debtmap analyze . --format markdown --output TECH_DEBT.md
```

## CI/CD Integration Best Practices

Integrate Debtmap into your CI/CD pipeline to enforce quality gates automatically.

**Source:** `.github/workflows/debtmap.yml` (example workflow)

### GitHub Actions Configuration

```yaml
name: Technical Debt Validation

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Debtmap
        run: cargo install debtmap

      - name: Generate coverage data
        run: |
          cargo install cargo-llvm-cov
          cargo llvm-cov --lcov --output-path coverage.lcov

      - name: Run Debtmap validation
        run: |
          debtmap validate . \
            --coverage-file coverage.lcov \
            --format json \
            --output debtmap-report.json

      - name: Upload report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: debtmap-report
          path: debtmap-report.json
```

### GitLab CI Configuration

```yaml
stages:
  - test
  - quality

coverage:
  stage: test
  script:
    - cargo install cargo-llvm-cov
    - cargo llvm-cov --lcov --output-path coverage.lcov
  artifacts:
    paths:
      - coverage.lcov

debtmap:
  stage: quality
  needs: [coverage]
  script:
    - cargo install debtmap
    - debtmap validate . --coverage-file coverage.lcov --format json --output report.json
  artifacts:
    paths:
      - report.json
    reports:
      codequality: report.json
  allow_failure: false  # Block merge on validation failure
```

### Generic Pipeline Configuration

For any CI system, the key steps are:

1. **Install Debtmap** - `cargo install debtmap`
2. **Generate coverage** - Use cargo-llvm-cov, tarpaulin, or your coverage tool
3. **Run validation** - `debtmap validate . --coverage-file coverage.lcov`
4. **Check exit code** - Non-zero exit indicates threshold violation
5. **Archive report** - Store JSON output for trend analysis

### CI/CD-Specific Thresholds

Use scale-independent metrics for CI/CD validation:

```toml
[thresholds.validation]
# Scale-independent metrics - no adjustment needed as codebase grows
max_debt_density = 50.0             # Debt per 1000 LOC
max_average_complexity = 10.0       # Per-function average
max_codebase_risk_score = 7.0       # Overall risk level

# Enable coverage gate if you have coverage data
min_coverage_percentage = 75.0
```

### Exit Codes

The `validate` command returns specific exit codes for CI integration:

| Exit Code | Meaning |
|-----------|---------|
| 0 | All thresholds passed |
| 1 | One or more thresholds exceeded |
| 2 | Configuration or runtime error |

## Common Configuration Anti-Patterns

Avoid these configuration mistakes:

### Absolute Count Thresholds in CI

**Problem:** Absolute counts punish healthy codebase growth.

```toml
# BAD - Will fail as codebase grows
[thresholds.validation]
max_debt_items = 100
max_high_complexity_count = 50
```

**Solution:** Use density-based metrics instead.

```toml
# GOOD - Scales with codebase size
[thresholds.validation]
max_debt_density = 50.0
max_average_complexity = 10.0
```

### Ignoring Coverage in Scoring

**Problem:** High-complexity well-tested code scores same as untested code.

```toml
# BAD - Ignores coverage entirely
[scoring]
coverage = 0.0
complexity = 0.85
dependency = 0.15
```

**Solution:** Include coverage weight proportional to your testing culture.

```toml
# GOOD - Balanced scoring
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
```

### Over-Suppression

**Problem:** Too many suppressions hide real issues.

**Solution:** Use minimum thresholds to filter noise instead of inline suppressions:

```toml
[thresholds]
minimum_debt_score = 3.0           # Filter low-priority items
minimum_cyclomatic_complexity = 5  # Filter simple functions
```

### Inconsistent Team Configuration

**Problem:** Different developers use different thresholds.

**Solution:** Commit `.debtmap.toml` to version control and reference it explicitly:

```bash
# Always reference committed config
debtmap validate . --config .debtmap.toml
```

## Related Topics

- [Thresholds Configuration](thresholds.md) - Detailed threshold reference
- [Scoring Configuration](scoring.md) - Scoring weight configuration
- [Validation and Quality Gates](../validation-gates.md) - CI/CD integration guide
- [CLI Reference](../cli-reference.md) - Command-line options
