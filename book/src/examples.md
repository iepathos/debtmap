# Examples

This chapter provides practical, real-world examples of using Debtmap across different project types and workflows. All examples use current CLI syntax verified against the source code.

> **Quick Navigation**: For detailed explanations of all CLI options, see the [CLI Reference](cli-reference.md) chapter.

## Table of Contents

- [Analyzing Rust Projects](#analyzing-rust-projects)
- [Python Analysis](#python-analysis)
- [JavaScript/TypeScript](#javascripttypescript)
- [CI Integration](#ci-integration)
- [Output Formats](#output-formats)
- [Advanced Usage](#advanced-usage)
- [Configuration Examples](#configuration-examples)
- [Compare Command](#compare-command)

## Analyzing Rust Projects

### Basic Rust Analysis

Start with a simple analysis of your Rust project:

```bash
# Analyze all Rust files in current directory
debtmap analyze .

# Analyze specific directory
debtmap analyze ./src

# Analyze with custom complexity threshold
debtmap analyze ./src --threshold-complexity 15
```

### Coverage Integration with cargo-tarpaulin

Combine complexity analysis with test coverage for risk-based prioritization:

```bash
# Generate LCOV coverage data
cargo tarpaulin --out lcov --output-dir target/coverage

# Analyze with coverage data
debtmap analyze . --lcov target/coverage/lcov.info

# Or use the shorter alias
debtmap analyze . --coverage-file target/coverage/lcov.info
```

**What this does:**
- Functions with 0% coverage and high complexity get marked as `[CRITICAL]`
- Well-tested functions (>80% coverage) are deprioritized
- Shows risk reduction potential for each untested function

### Custom Thresholds

Configure thresholds to match your project standards:

```bash
# Set both complexity and duplication thresholds
debtmap analyze . \
  --threshold-complexity 15 \
  --threshold-duplication 50
```

### God Object Detection

Identify classes and modules with too many responsibilities:

```bash
# Standard analysis includes god object detection
debtmap analyze .

# Disable god object detection for specific run
debtmap analyze . --no-god-object
```

God objects are flagged with detailed metrics:
- Number of methods and fields
- Responsibility count (grouped by naming patterns)
- God object score (0-100%)
- Recommendations for splitting

### Filtering and Focusing

```bash
# Analyze only Rust files
debtmap analyze . --languages rust

# Focus on architecture issues (god objects, complexity)
debtmap analyze . --filter Architecture

# Focus on testing gaps
debtmap analyze . --filter Testing

# Show only top 10 issues
debtmap analyze . --top 10

# Show only high-priority items
debtmap analyze . --min-priority high
```

### Output Formats

```bash
# JSON output for CI integration
debtmap analyze . --format json --output report.json

# Markdown report
debtmap analyze . --format markdown --output DEBT_REPORT.md

# Terminal output (default) - prettified
debtmap analyze .
```

### Multi-Pass Analysis

For deeper analysis with context awareness:

```bash
# Enable context-aware analysis with multiple providers
debtmap analyze . \
  --context \
  --context-providers critical_path,dependency,git_history

# Multi-pass analysis with attribution
debtmap analyze . --multi-pass --attribution
```

### Complete CI Example

This is from Debtmap's own `.github/workflows/debtmap.yml`:

```bash
# 1. Install cargo-tarpaulin
cargo install cargo-tarpaulin

# 2. Build debtmap
cargo build --release

# 3. Generate coverage
cargo tarpaulin --config .tarpaulin.toml --out Lcov --timeout 300

# 4. Run validation with coverage
./target/release/debtmap validate . \
  --coverage-file target/coverage/lcov.info \
  --format json \
  --output debtmap-report.json
```

## Python Analysis

### Basic Python Analysis

```bash
# Analyze Python files only
debtmap analyze . --languages python

# Analyze specific Python directory
debtmap analyze src --languages python
```

### Coverage Integration with pytest

Generate coverage and analyze risk:

```bash
# Generate LCOV coverage with pytest
pytest --cov --cov-report=lcov

# Analyze with coverage data
debtmap analyze . \
  --languages python \
  --lcov coverage.lcov
```

### Python-Specific Patterns

```bash
# Focus on testing gaps in Python code
debtmap analyze . \
  --languages python \
  --filter Testing

# Find god objects in Python modules
debtmap analyze . \
  --languages python \
  --filter Architecture
```

### Example Configuration for Python Projects

Create `.debtmap.toml`:

```toml
[languages]
enabled = ["python"]

[thresholds]
complexity = 12
max_function_lines = 40

[ignore]
patterns = [
  "**/*_test.py",
  "tests/**",
  ".venv/**",
  "**/__pycache__/**",
]

[god_object]
enabled = true
max_methods = 15
max_responsibilities = 4
```

## JavaScript/TypeScript

### Analyzing JS/TS Projects

```bash
# Analyze JavaScript and TypeScript
debtmap analyze . --languages javascript,typescript

# TypeScript only
debtmap analyze . --languages typescript
```

### Coverage Integration with Jest

```bash
# Generate LCOV with Jest
jest --coverage --coverageReporters=lcov

# Analyze with coverage
debtmap analyze . \
  --languages javascript,typescript \
  --lcov coverage/lcov.info
```

### Node.js Project Patterns

```bash
# Exclude node_modules and focus on source
debtmap analyze src --languages javascript,typescript

# With custom complexity thresholds for JS
debtmap analyze . \
  --languages javascript,typescript \
  --threshold-complexity 10
```

### TypeScript Configuration Example

Create `.debtmap.toml`:

```toml
[languages]
enabled = ["typescript", "javascript"]

[thresholds]
complexity = 10
max_function_lines = 50

[ignore]
patterns = [
  "node_modules/**",
  "**/*.test.ts",
  "**/*.spec.ts",
  "dist/**",
  "build/**",
  "**/*.d.ts",
]
```

### Monorepo Analysis

```bash
# Analyze specific package
debtmap analyze packages/api --languages typescript

# Analyze all packages, grouped by category
debtmap analyze packages \
  --languages typescript \
  --group-by-category
```

## CI Integration

### GitHub Actions

Complete workflow example (from `.github/workflows/debtmap.yml`):

```yaml
name: Debtmap

on:
  push:
    branches: [ main, master ]
  pull_request:
    branches: [ main, master ]

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
        fetch-depth: 0

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

    - name: Install cargo-tarpaulin
      run: |
        if ! command -v cargo-tarpaulin &> /dev/null; then
          cargo install cargo-tarpaulin
        fi

    - name: Build debtmap
      run: cargo build --release

    - name: Generate coverage data
      run: cargo tarpaulin --config .tarpaulin.toml --out Lcov --timeout 300

    - name: Run debtmap validation with coverage
      run: |
        if [ -f "target/coverage/lcov.info" ]; then
          ./target/release/debtmap validate . \
            --coverage-file target/coverage/lcov.info \
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
debtmap:
  stage: quality
  image: rust:latest
  script:
    # Install debtmap
    - cargo install debtmap

    # Run tests with coverage
    - cargo install cargo-tarpaulin
    - cargo tarpaulin --out Lcov

    # Validate with debtmap
    - debtmap validate .
        --coverage-file cobertura.xml
        --format json
        --output debtmap-report.json
  artifacts:
    reports:
      coverage_report:
        coverage_format: cobertura
        path: cobertura.xml
    paths:
      - debtmap-report.json
    expire_in: 1 week
```

### CircleCI

```yaml
version: 2.1

jobs:
  debtmap:
    docker:
      - image: cimg/rust:1.75
    steps:
      - checkout

      - run:
          name: Install debtmap
          command: cargo install debtmap

      - run:
          name: Generate coverage
          command: |
            cargo install cargo-tarpaulin
            cargo tarpaulin --out Lcov

      - run:
          name: Run debtmap
          command: |
            debtmap validate . \
              --coverage-file lcov.info \
              --format json \
              --output debtmap.json

      - store_artifacts:
          path: debtmap.json

workflows:
  version: 2
  build:
    jobs:
      - debtmap
```

### Using debtmap validate for PR Gates

```bash
# Fail build if thresholds are exceeded
debtmap validate . --coverage-file lcov.info

# With custom thresholds
debtmap validate . \
  --coverage-file lcov.info \
  --threshold-complexity 15

# Exit code 0 if passing, 1 if failing
```

### Compare Command in CI

Track technical debt trends over time:

```bash
# Generate baseline (on main branch)
debtmap analyze . --format json --output baseline.json

# After PR changes
debtmap analyze . --format json --output current.json

# Compare and fail if regressions detected
debtmap compare \
  --before baseline.json \
  --after current.json \
  --format json
```

## Output Formats

### Terminal Output (Default)

The default terminal output is prettified with colors and priorities:

```bash
debtmap analyze . --lcov coverage.lcov --top 3
```

Example output:
```
════════════════════════════════════════════
    PRIORITY TECHNICAL DEBT FIXES
════════════════════════════════════════════

🎯 TOP 3 RECOMMENDATIONS (by unified priority)

#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/analyzers/rust.rs:38 parse_function()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: Full test coverage, -3.7 risk
├─ COMPLEXITY: cyclomatic=6, cognitive=8, nesting=2, lines=32
├─ DEPENDENCIES: 0 upstream, 11 downstream
└─ WHY: Business logic with 0% coverage, manageable complexity

📊 TOTAL DEBT SCORE: 4907
📈 OVERALL COVERAGE: 67.12%
```

### JSON Output

Machine-readable format for CI/CD integration:

```bash
debtmap analyze . --format json --output report.json
```

Structure:
```json
{
  "items": [
    {
      "location": {
        "file": "src/main.rs",
        "function": "process_data",
        "line": 42
      },
      "debt_type": "TestGap",
      "unified_score": {
        "complexity_factor": 3.2,
        "coverage_factor": 10.0,
        "dependency_factor": 2.5,
        "role_multiplier": 1.2,
        "final_score": 9.4
      },
      "function_role": "BusinessLogic",
      "recommendation": {
        "action": "Add unit tests",
        "details": "Add 6 unit tests for full coverage",
        "effort_estimate": "2-3 hours"
      },
      "expected_impact": {
        "risk_reduction": 3.9,
        "complexity_reduction": 0,
        "coverage_improvement": 100
      }
    }
  ],
  "overall_coverage": 67.12,
  "total_debt_score": 4907
}
```

### Markdown Report

```bash
debtmap analyze . --format markdown --output DEBT_REPORT.md
```

Great for documentation or PR comments.

### Understanding Output Formats

```bash
# Unified JSON format (default)
debtmap analyze . --format json

# Legacy JSON format for backward compatibility
debtmap analyze . --format json --json-format legacy

# Output format options: terminal, json, markdown
debtmap analyze . --format terminal
```

## Advanced Usage

### Context-Aware Analysis

Enable advanced context providers for more accurate prioritization:

```bash
# Enable all context providers
debtmap analyze . \
  --context \
  --context-providers critical_path,dependency,git_history

# Disable specific providers
debtmap analyze . \
  --context \
  --disable-context git_history
```

### Multi-Pass Analysis

```bash
# Multi-pass with attribution tracking
debtmap analyze . --multi-pass --attribution

# Shows which functions contribute to which patterns
```

### Cache Management

```bash
# Show cache statistics
debtmap cache stats

# Clear cache for current project
debtmap cache clear

# Prune old cache entries
debtmap cache prune --max-age-days 7
```

### Aggregation Methods

```bash
# Use logarithmic sum for aggregation
debtmap analyze . --aggregation-method logarithmic_sum

# Standard sum (default)
debtmap analyze . --aggregation-method sum
```

### Filtering and Grouping

```bash
# Group results by debt category
debtmap analyze . --group-by-category

# Filter specific categories
debtmap analyze . --filter Architecture,Testing

# Show only high-priority items
debtmap analyze . --min-priority high --top 10
```

### Verbosity Levels

```bash
# Level 1: Show main score factors
debtmap analyze . -v

# Level 2: Show detailed calculations
debtmap analyze . -vv

# Level 3: Show all debug information
debtmap analyze . -vvv

# Show macro expansion details (Rust)
debtmap analyze . --verbose-macro-warnings --show-macro-stats
```

### Parallel Processing Control

```bash
# Use 8 parallel jobs
debtmap analyze . --jobs 8

# Disable parallel processing
debtmap analyze . --no-parallel
```

## Configuration Examples

### Basic Configuration

Create `.debtmap.toml`:

```toml
[thresholds]
complexity = 15
duplication = 25
max_function_lines = 50
max_nesting_depth = 4

[languages]
enabled = ["rust", "python"]

[ignore]
patterns = [
  "tests/**/*",
  "**/*.test.rs",
  "target/**",
]
```

### Entropy-Based Complexity

```toml
[entropy]
enabled = true
weight = 0.5
use_classification = true
pattern_threshold = 0.7
entropy_threshold = 0.4
branch_threshold = 0.8
max_combined_reduction = 0.3
```

This reduces false positives for repetitive code patterns.

### Custom Scoring Weights

```toml
[scoring]
coverage = 0.40      # Test coverage gaps
complexity = 0.40    # Code complexity
dependency = 0.20    # Dependency criticality
```

### God Object Detection Tuning

```toml
[god_object]
enabled = true
max_methods = 20
max_fields = 15
max_responsibilities = 5
```

### External API Configuration

For libraries (not CLI tools):

```toml
[external_api]
detect_external_api = true

api_functions = [
  "parse",
  "Parser::new",
  "client::connect",
]

api_files = [
  "src/lib.rs",
  "src/api.rs",
  "src/public/*.rs",
]
```

### Complete Multi-Language Configuration

```toml
[thresholds]
complexity = 12
duplication = 30
max_file_lines = 400
max_function_lines = 40
minimum_debt_score = 1.0
minimum_cyclomatic_complexity = 2

[entropy]
enabled = true
weight = 0.5

[scoring]
coverage = 0.40
complexity = 0.40
dependency = 0.20

[languages]
enabled = ["rust", "python", "javascript", "typescript"]

[ignore]
patterns = [
  # Tests
  "tests/**/*",
  "**/*.test.*",
  "**/*_test.*",

  # Build artifacts
  "target/**",
  "dist/**",
  "build/**",
  "node_modules/**",

  # Python
  ".venv/**",
  "**/__pycache__/**",

  # Generated code
  "*.generated.*",
  "*.pb.*",
]

[god_object]
enabled = true
max_methods = 18
max_fields = 12
```

## Compare Command

The `compare` command helps validate that refactoring achieved its goals.

### Basic Comparison Workflow

```bash
# 1. Generate baseline before refactoring
debtmap analyze . --format json --output before.json

# 2. Make your code improvements
#    ... refactor, add tests, etc ...

# 3. Generate new analysis
debtmap analyze . --format json --output after.json

# 4. Compare and verify improvements
debtmap compare --before before.json --after after.json
```

### Target-Specific Comparison

Focus on whether a specific function improved:

```bash
# Target format: file:function:line
debtmap compare \
  --before before.json \
  --after after.json \
  --target src/main.rs:process_data:100
```

### Using with Implementation Plans

Extract target automatically from plan files:

```bash
# If IMPLEMENTATION_PLAN.md contains:
# **Target**: src/parser.rs:parse_expression:45

debtmap compare \
  --before before.json \
  --after after.json \
  --plan IMPLEMENTATION_PLAN.md
```

### Output Formats

```bash
# Terminal output (default)
debtmap compare --before before.json --after after.json

# JSON for CI integration
debtmap compare \
  --before before.json \
  --after after.json \
  --format json \
  --output comparison.json

# Markdown report
debtmap compare \
  --before before.json \
  --after after.json \
  --format markdown \
  --output COMPARISON.md
```

### Interpreting Results

**Target Status:**
- **Resolved**: Function no longer appears (complexity reduced below threshold)
- **Improved**: Metrics improved (complexity down, coverage up)
- **Unchanged**: No significant change
- **Regressed**: Metrics got worse
- **Not Found**: Target not found in baseline

**Overall Trend:**
- **Improving**: More items resolved/improved than regressed
- **Stable**: No significant changes
- **Regressing**: New critical debt introduced

**Example Output:**
```
Target Status: Resolved ✅
- src/parser.rs:parse_expression:45 reduced from complexity 22 to 8
- Coverage improved from 0% to 85%

Overall Trend: Improving
- 3 items resolved
- 2 items improved
- 0 regressions
- Total debt score: 450 → 285 (-37%)
```

### CI Integration

Use in pull request validation:

```bash
# In CI script
debtmap compare \
  --before baseline.json \
  --after current.json \
  --format json | jq -e '.overall_trend == "Improving"'

# Exit code 0 if improving, 1 otherwise
```

## Tips and Best Practices

1. **Start Simple**: Begin with basic analysis, add coverage later
2. **Use Filters**: Focus on one category at a time (Architecture, Testing)
3. **Iterate**: Run analysis, fix top items, repeat
4. **CI Integration**: Automate validation in your build pipeline
5. **Track Progress**: Use `compare` command to validate improvements
6. **Configure Thresholds**: Adjust to match your team's standards
7. **Leverage Coverage**: Always include coverage data for accurate risk assessment

## Next Steps

- [CLI Reference](cli-reference.md) - Complete CLI documentation
- [Analysis Guide](analysis-guide.md) - Understanding analysis results
- [Configuration](configuration.md) - Advanced configuration options
