# Validation and Quality Gates

The `validate` command enforces quality gates in your development workflow, making it ideal for CI/CD integration. Unlike the `analyze` command which focuses on exploration and reporting, `validate` checks your codebase against configured thresholds and returns appropriate exit codes for automated workflows.

## Table of Contents

- [Validate vs Analyze](#validate-vs-analyze)
- [Quick Start](#quick-start)
- [Understanding Density-Based Validation](#understanding-density-based-validation)
- [Configuration Setup](#configuration-setup)
- [Validation Metrics](#validation-metrics)
- [Exit Codes and CI Integration](#exit-codes-and-ci-integration)
- [Coverage Integration](#coverage-integration)
- [Context-Aware Validation](#context-aware-validation)
- [CI/CD Examples](#cicd-examples)
- [Migrating from Deprecated Thresholds](#migrating-from-deprecated-thresholds)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Validate vs Analyze

Understanding when to use each command is crucial:

| Aspect | `validate` | `analyze` |
|--------|-----------|-----------|
| **Purpose** | Enforce quality gates | Explore and understand debt |
| **Exit Codes** | Returns non-zero on failure | Always returns 0 (unless error) |
| **Thresholds** | From `.debtmap.toml` config | Command-line flags |
| **Use Case** | CI/CD pipelines, pre-commit hooks | Interactive analysis, reports |
| **Output Focus** | Pass/fail with violation details | Comprehensive metrics and insights |
| **Configuration** | Requires `.debtmap.toml` | Works without config file |

**Rule of thumb:** Use `validate` for automation and `analyze` for investigation.

## Quick Start

1. **Initialize configuration:**
   ```bash
   debtmap init
   ```

2. **Edit `.debtmap.toml` to set thresholds:**
   ```toml
   [thresholds.validation]
   max_debt_density = 50.0              # Debt items per 1000 LOC
   max_average_complexity = 10.0        # Average cyclomatic complexity
   max_codebase_risk_score = 7.0        # Overall risk level (1-10)
   ```

3. **Run validation:**
   ```bash
   debtmap validate .
   ```

4. **Check exit code:**
   ```bash
   echo $?  # 0 = pass, non-zero = fail
   ```

## Understanding Density-Based Validation

Debtmap uses **density-based metrics** as the primary quality measure. This approach provides several advantages over traditional absolute count metrics.

### Why Density Matters

Traditional metrics like "maximum 50 high-complexity functions" fail as your codebase grows:

```
Scenario: Your team adds 10,000 LOC of high-quality code
- Old metric: "max 50 complex functions" → FAILS (now 55 total)
- Density metric: "max 50 per 1000 LOC" → PASSES (density improved)
```

**Scale-dependent metrics** (absolute counts):
- Grow linearly with codebase size
- Require constant threshold adjustments
- Punish healthy growth
- Don't reflect actual code quality

**Density metrics** (per 1000 LOC):
- Remain stable as codebase grows
- Measure true quality ratios
- No adjustment needed for growth
- Directly comparable across projects

### Calculating Debt Density

```
Debt Density = (Total Debt Items / Total LOC) × 1000
```

**Example:**
- 25 debt items in 5,000 LOC project
- Density = (25 / 5000) × 1000 = **5.0 debt items per 1000 LOC**

This density remains meaningful whether your codebase is 5,000 or 500,000 LOC.

### Recommended Density Thresholds

| Project Type | max_debt_density | Rationale |
|--------------|------------------|-----------|
| **New/Greenfield** | 20.0 | High quality bar for new code |
| **Active Development** | 50.0 | Balanced quality/velocity (default) |
| **Legacy Modernization** | 100.0 | Prevent regression during refactoring |
| **Mature/Critical** | 30.0 | Maintain quality in stable systems |

## Configuration Setup

### Creating Configuration File

The `debtmap init` command generates a `.debtmap.toml` with sensible defaults:

```bash
debtmap init
```

This creates:
```toml
[thresholds.validation]
# Primary quality metrics (scale-independent)
max_average_complexity = 10.0
max_debt_density = 50.0
max_codebase_risk_score = 7.0

# Optional metrics
min_coverage_percentage = 0.0  # Disabled by default

# Safety net (high ceiling for extreme cases)
max_total_debt_score = 10000
```

### Editing Thresholds

Edit the `[thresholds.validation]` section to match your quality requirements:

```toml
[thresholds.validation]
# Enforce stricter quality for new project
max_debt_density = 30.0              # Tighter density requirement
max_average_complexity = 8.0         # Lower complexity tolerance
max_codebase_risk_score = 6.0        # Reduced risk threshold
min_coverage_percentage = 80.0       # Require 80% test coverage
```

### Override via Command Line

You can override the density threshold from the command line:

```bash
# Temporarily use stricter threshold
debtmap validate . --max-debt-density 40.0
```

## Validation Metrics

Debtmap organizes validation metrics into three categories:

### Primary Metrics (Scale-Independent)

These are the core quality measures that every project should monitor:

1. **`max_average_complexity`** (default: 10.0)
   - Average cyclomatic complexity per function
   - Measures typical function complexity across codebase
   - Lower values indicate simpler, more maintainable code

   ```toml
   max_average_complexity = 10.0
   ```

2. **`max_debt_density`** (default: 50.0) - **PRIMARY METRIC**
   - Debt items per 1000 lines of code
   - Scale-independent quality measure
   - Remains stable as codebase grows

   ```toml
   max_debt_density = 50.0
   ```

3. **`max_codebase_risk_score`** (default: 7.0)
   - Overall risk level combining complexity, coverage, and criticality
   - Score ranges from 1 (low risk) to 10 (high risk)
   - Considers context-aware analysis when enabled

   ```toml
   max_codebase_risk_score = 7.0
   ```

### Optional Metrics

Configure these when you want additional quality enforcement:

4. **`min_coverage_percentage`** (default: 0.0 - disabled)
   - Minimum required test coverage percentage
   - Only enforced when coverage data is provided via `--coverage-file`
   - Set to 0.0 to disable coverage requirements

   ```toml
   min_coverage_percentage = 75.0  # Require 75% coverage
   ```

### Safety Net Metrics

High ceilings to catch extreme cases:

5. **`max_total_debt_score`** (default: 10000)
   - Absolute ceiling on total technical debt
   - Prevents runaway growth even if density stays low
   - Rarely triggers in normal operation

   ```toml
   max_total_debt_score = 10000
   ```

### Metric Priority

**Validation uses AND logic:** All primary metrics must pass for validation to succeed. If any check fails, the entire validation fails with a non-zero exit code.

When validation fails, fix issues in this order:

1. **Critical:** `max_debt_density` violations (core quality metric)
2. **High:** `max_average_complexity` violations (function-level quality)
3. **High:** `max_codebase_risk_score` violations (overall risk)
4. **Medium:** `min_coverage_percentage` violations (test coverage)
5. **Low:** `max_total_debt_score` violations (extreme cases only)

The priority list above is for remediation order when validation fails, not for which checks are enforced. All configured thresholds are enforced equally.

## Exit Codes and CI Integration

The `validate` command uses exit codes to signal success or failure:

### Exit Code Behavior

```bash
debtmap validate .
echo $?
```

**Exit codes:**
- **`0`** - Success: All thresholds passed
- **Non-zero** - Failure: One or more thresholds exceeded or errors occurred

### Using Exit Codes in CI

Exit codes integrate naturally with CI/CD systems:

**GitHub Actions:**
```yaml
- name: Validate code quality
  run: debtmap validate .
  # Step fails automatically if exit code is non-zero
```

**GitLab CI:**
```yaml
script:
  - debtmap validate .
  # Pipeline fails if exit code is non-zero
```

**Shell scripts:**
```bash
#!/bin/bash
if debtmap validate .; then
    echo "✅ Validation passed"
else
    echo "❌ Validation failed"
    exit 1
fi
```

### Understanding Validation Output

**Success output:**
```
✅ Validation PASSED

Metrics:
  Average Complexity: 7.2 / 10.0 ✓
  Debt Density: 32.5 / 50.0 ✓
  Codebase Risk: 5.8 / 7.0 ✓
  Total Debt Score: 1250 / 10000 ✓
```

**Failure output:**
```
❌ Validation FAILED

Metrics:
  Average Complexity: 12.3 / 10.0 ✗ EXCEEDED
  Debt Density: 65.8 / 50.0 ✗ EXCEEDED
  Codebase Risk: 5.2 / 7.0 ✓
  Total Debt Score: 2100 / 10000 ✓

Failed checks: 2
```

### Summary Output Format

For compact output suitable for CI logs, use the `--summary` or `-s` flag:

```bash
debtmap validate . --summary
# or
debtmap validate . -s
```

**Summary format output:**
```
✅ Validation PASSED

Priority Tiers:
  P0 (Critical): 2 items
  P1 (High): 8 items
  P2 (Medium): 15 items
  P3 (Low): 23 items

Top Issues:
  1. Complex authentication logic (complexity: 28)
  2. Database connection pool (risk: 9.2)
  3. Untested error handler (coverage: 0%)
```

The summary format provides:
- Tiered priority counts instead of individual item details
- Top violating functions for quick triage
- Compact format ideal for CI/CD logs
- Same pass/fail determination as standard format

## Coverage Integration

Integrate test coverage data to enable risk-based validation:

### Generating Coverage Data

**For Rust projects with `cargo-tarpaulin`:**
```bash
cargo tarpaulin --out Lcov --output-dir target/coverage
```

**For Python projects with `pytest-cov`:**
```bash
pytest --cov --cov-report=lcov:coverage/lcov.info
```

**For JavaScript projects with Jest:**
```bash
jest --coverage --coverageReporters=lcov
```

### Running Validation with Coverage

```bash
debtmap validate . --coverage-file target/coverage/lcov.info
```

### Benefits of Coverage Integration

With coverage data, validation gains additional insights:

1. **Risk-based prioritization** - Identifies untested complex code
2. **Coverage threshold enforcement** - via `min_coverage_percentage`
3. **Enhanced risk scoring** - Combines complexity + coverage + context
4. **Better failure diagnostics** - Shows which untested areas need attention

### Coverage-Enhanced Output

```bash
debtmap validate . --coverage-file coverage/lcov.info -vv
```

Output includes:
- Overall coverage percentage
- High-risk uncovered functions
- Coverage-adjusted risk scores
- Prioritized remediation recommendations

## Context-Aware Validation

Enable context-aware analysis for deeper risk insights:

### Available Context Providers

1. **`critical_path`** - Analyzes call graph to find execution bottlenecks
2. **`dependency`** - Identifies highly-coupled modules
3. **`git_history`** - Detects frequently-changed code (churn)

### Enabling Context Providers

**Enable all providers:**
```bash
debtmap validate . --enable-context
```

**Select specific providers:**
```bash
debtmap validate . --enable-context --context-providers critical_path,git_history
```

**Disable specific providers:**
```bash
debtmap validate . --enable-context --disable-context dependency
```

### Context-Aware Configuration

Add context settings to `.debtmap.toml`:

```toml
[analysis]
enable_context = true
context_providers = ["critical_path", "git_history"]
```

Then run validation:
```bash
debtmap validate .  # Uses config settings
```

### Context Benefits for Validation

Context-aware analysis improves risk scoring by:
- Prioritizing frequently-called functions
- Weighting high-churn code more heavily
- Identifying architectural bottlenecks
- Surfacing critical code paths

## CI/CD Examples

### GitHub Actions

Complete workflow with coverage generation and validation:

```yaml
name: Code Quality Validation

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  validate:
    name: Technical Debt Validation
    runs-on: ubuntu-latest

    steps:
    - name: Checkout repository
      uses: actions/checkout@v5
      with:
        fetch-depth: 0  # Full history for git context

    - name: Setup Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy

    - name: Cache cargo dependencies
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/bin/
          ~/.cargo/registry/index/
          ~/.cargo/registry/cache/
          ~/.cargo/git/db/
          target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ runner.os }}-cargo-

    - name: Install cargo-tarpaulin
      run: |
        if ! command -v cargo-tarpaulin &> /dev/null; then
          cargo install cargo-tarpaulin
        fi

    - name: Build debtmap
      run: cargo build --release

    - name: Generate coverage data
      run: cargo tarpaulin --out Lcov --output-dir target/coverage --timeout 300

    - name: Run debtmap validation with coverage
      run: |
        if [ -f "target/coverage/lcov.info" ]; then
          ./target/release/debtmap validate . \
            --coverage-file target/coverage/lcov.info \
            --enable-context \
            --format json \
            --output debtmap-report.json
        else
          echo "Warning: LCOV file not found, running without coverage"
          ./target/release/debtmap validate . \
            --format json \
            --output debtmap-report.json
        fi

    - name: Upload debtmap report
      if: always()
      uses: actions/upload-artifact@v4
      with:
        name: debtmap-analysis-artifacts
        path: |
          debtmap-report.json
          target/coverage/lcov.info
        retention-days: 7
```

### GitLab CI

```yaml
stages:
  - test
  - quality

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo

debtmap:
  stage: quality
  image: rust:latest

  cache:
    paths:
      - .cargo/
      - target/

  before_script:
    # Install debtmap and coverage tools
    - cargo install debtmap
    - cargo install cargo-tarpaulin

  script:
    # Generate coverage
    - cargo tarpaulin --out Lcov --output-dir coverage

    # Validate with debtmap
    - debtmap validate . --coverage-file coverage/lcov.info -v

  artifacts:
    when: always
    paths:
      - coverage/
    reports:
      coverage_report:
        coverage_format: cobertura
        path: coverage/cobertura.xml
```

### CircleCI

```yaml
version: 2.1

jobs:
  validate:
    docker:
      - image: cimg/rust:1.75

    steps:
      - checkout

      - restore_cache:
          keys:
            - cargo-{{ checksum "Cargo.lock" }}

      - run:
          name: Install tools
          command: |
            cargo install debtmap
            cargo install cargo-tarpaulin

      - run:
          name: Generate coverage
          command: cargo tarpaulin --out Lcov

      - run:
          name: Validate code quality
          command: debtmap validate . --coverage-file lcov.info

      - save_cache:
          key: cargo-{{ checksum "Cargo.lock" }}
          paths:
            - ~/.cargo
            - target

workflows:
  version: 2
  quality:
    jobs:
      - validate
```

## Migrating from Deprecated Thresholds

Debtmap version 0.3.0 deprecated scale-dependent absolute count metrics in favor of density-based metrics.

### Deprecated Metrics

The following metrics will be **removed in v1.0**:

| Deprecated Metric | Migration Path |
|-------------------|----------------|
| `max_high_complexity_count` | Use `max_debt_density` |
| `max_debt_items` | Use `max_debt_density` |
| `max_high_risk_functions` | Use `max_debt_density` + `max_codebase_risk_score` |

### Migration Example

**Old configuration (deprecated):**
```toml
[thresholds.validation]
max_high_complexity_count = 50    # ❌ Scale-dependent
max_debt_items = 100               # ❌ Scale-dependent
max_high_risk_functions = 20       # ❌ Scale-dependent
```

**New configuration (recommended):**
```toml
[thresholds.validation]
max_debt_density = 50.0            # ✅ Scale-independent
max_average_complexity = 10.0      # ✅ Quality ratio
max_codebase_risk_score = 7.0      # ✅ Risk level
```

### Calculating Equivalent Density Threshold

Convert your old absolute thresholds to density:

```
Old: max_debt_items = 100 in 10,000 LOC codebase
New: max_debt_density = (100 / 10000) × 1000 = 10.0
```

### Deprecation Warnings

When you run `validate` with deprecated metrics, you'll see:

```
⚠️  DEPRECATION WARNING:
   The following validation thresholds are deprecated:
   - max_high_complexity_count
   - max_debt_items

   These scale-dependent metrics will be removed in v1.0.
   Please migrate to density-based validation:
     - Use 'max_debt_density' instead of absolute counts
     - Density metrics remain stable as your codebase grows
```

### Migration Timeline

- **v0.3.0** - Density metrics introduced, old metrics deprecated
- **v0.4.0 - v0.9.x** - Deprecation warnings shown
- **v1.0.0** - Deprecated metrics removed

## Troubleshooting

### Debugging Validation Failures

Use verbosity flags to understand why validation failed:

**Level 1: Basic details (`-v`)**
```bash
debtmap validate . -v
```
Shows which thresholds failed, by how much, and timing breakdown:
- Call graph building time
- Trait resolution time
- Coverage loading time
- Individual analysis phase durations

**Level 2: Detailed breakdown (`-vv`)**
```bash
debtmap validate . -vv
```
Shows everything from `-v` plus:
- Score calculation factors and weights
- Top violating functions with metrics
- Detailed phase timing information
- Risk score component breakdown

**Level 3: Full diagnostic output (`-vvv`)**
```bash
debtmap validate . -vvv
```
Shows complete debug information:
- All debt items with full details
- Complete risk calculations for each function
- All timing information including sub-phases
- File-level and function-level analysis data
- Context provider outputs (if enabled)

### Common Issues

**Issue: Validation fails but output unclear**
```bash
# Solution: Increase verbosity
debtmap validate . -vv
```

**Issue: Want to see only the worst problems**
```bash
# Solution: Use --top flag
debtmap validate . --top 10 -v
```

**Issue: Output is too verbose for CI logs**
```bash
# Solution: Use --summary flag for compact tiered output
debtmap validate . --summary
# or
debtmap validate . -s
```
This provides a condensed view focused on priority tiers rather than individual items.

**Issue: Validation passes locally but fails in CI**
```bash
# Possible causes:
# 1. Different code (stale local branch)
# 2. Different config file (check .debtmap.toml in CI)
# 3. Missing coverage data (check LCOV generation in CI)

# Debug in CI:
debtmap validate . -vvv  # Maximum verbosity
```

**Issue: Coverage threshold fails unexpectedly**
```bash
# Check if coverage file is being read
debtmap validate . --coverage-file coverage/lcov.info -v

# Verify coverage file exists and is valid
ls -lh coverage/lcov.info
```

**Issue: Context providers causing performance issues**
```bash
# Disable expensive providers
debtmap validate . --enable-context --disable-context git_history
```

**Issue: Semantic analysis causing errors or unexpected behavior**

Debtmap uses semantic analysis by default, powered by tree-sitter for deep AST (Abstract Syntax Tree) analysis. This provides accurate understanding of code structure, control flow, and complexity patterns.

However, semantic analysis may encounter issues with:
- Unsupported or experimental language features
- Malformed or incomplete syntax
- Complex macro expansions
- Very large files that timeout during parsing

```bash
# Solution: Disable semantic analysis with fallback mode
debtmap validate . --semantic-off
```

When semantic analysis is disabled with `--semantic-off`, debtmap falls back to basic syntax analysis, which is faster but less accurate for complexity calculations. Use this flag if:
- Encountering parsing errors or timeouts
- Working with bleeding-edge language features
- Need faster validation at the cost of precision

### Validation Report Generation

Generate detailed reports for debugging:

**JSON format for programmatic analysis:**
```bash
debtmap validate . --format json --output validation-report.json
cat validation-report.json | jq '.validation_details'
```

**Markdown format for documentation:**
```bash
debtmap validate . --format markdown --output validation-report.md
```

**Terminal format with filtering:**
```bash
debtmap validate . --format terminal --top 20 -vv
```

## Best Practices

### Setting Initial Thresholds

**1. Establish baseline:**
```bash
# Run analysis to see current metrics
debtmap analyze . --format json > baseline.json
cat baseline.json | jq '.unified_analysis.debt_density'
```

**2. Set pragmatic thresholds:**
```toml
[thresholds.validation]
# Start slightly above current values to prevent regression
max_debt_density = 60.0  # Current: 55.0
max_average_complexity = 12.0  # Current: 10.5
```

**3. Gradually tighten:**
```toml
# After 1 month of cleanup
max_debt_density = 50.0
max_average_complexity = 10.0
```

### Progressive Threshold Tightening

**Month 1-2: Prevent regression**
```toml
max_debt_density = 60.0  # Above current baseline
```

**Month 3-4: Incremental improvement**
```toml
max_debt_density = 50.0  # Industry standard
```

**Month 5-6: Quality leadership**
```toml
max_debt_density = 30.0  # Best-in-class
```

### Project-Specific Recommendations

**Greenfield projects:**
```toml
# Start with high quality bar
max_debt_density = 20.0
max_average_complexity = 8.0
min_coverage_percentage = 80.0
```

**Active development:**
```toml
# Balanced quality/velocity
max_debt_density = 50.0
max_average_complexity = 10.0
min_coverage_percentage = 70.0
```

**Legacy modernization:**
```toml
# Prevent regression during refactoring
max_debt_density = 100.0
max_average_complexity = 15.0
min_coverage_percentage = 50.0
```

### Pre-Commit Hook Integration

Add validation as a pre-commit hook:

```bash
# .git/hooks/pre-commit
#!/bin/bash
echo "Running debtmap validation..."
if debtmap validate . -v; then
    echo "✅ Validation passed"
    exit 0
else
    echo "❌ Validation failed - commit blocked"
    exit 1
fi
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

### Performance Optimization

**Enable parallel processing:**
Validation uses parallel processing by default for fast execution on multi-core systems.

**Disable for resource-constrained environments:**
```bash
# Limit parallelism
debtmap validate . --jobs 2

# Disable completely
debtmap validate . --no-parallel
```

**Performance characteristics:**
- Parallel call graph construction
- Multi-threaded file analysis
- Same performance as `analyze` command

### Monitoring Trends

Track validation metrics over time:

```bash
# Generate timestamped reports
debtmap validate . --format json --output "reports/validation-$(date +%Y%m%d).json"

# Compare trends
jq -s 'map(.unified_analysis.debt_density)' reports/validation-*.json
```

### Documentation

Document your threshold decisions:

```toml
# .debtmap.toml
[thresholds.validation]
# Rationale: Team agreed 50.0 density balances quality and velocity
# Review: Quarterly (next: 2025-04-01)
max_debt_density = 50.0

# Rationale: Enforces single-responsibility principle
# Review: After 3 months of metrics
max_average_complexity = 10.0
```

## Summary

The `validate` command provides automated quality gates for CI/CD integration:

- **Use density-based metrics** for scale-independent quality measurement
- **Configure in `.debtmap.toml`** for consistent, version-controlled thresholds
- **Integrate with CI/CD** using exit codes for automated enforcement
- **Enable coverage and context** for risk-based validation
- **Migrate from deprecated metrics** to density-based approach
- **Debug with verbosity flags** when validation fails unexpectedly
- **Tighten thresholds progressively** as code quality improves

Next steps:
- Review [Configuration Reference](./configuration.md) for detailed threshold options
- See [Examples](./examples.md) for more CI/CD integration patterns
- Check [CLI Reference](./cli-reference.md) for complete command documentation
