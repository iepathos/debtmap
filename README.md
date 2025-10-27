# debtmap

[![CI](https://github.com/iepathos/debtmap/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/ci.yml)
[![Security](https://github.com/iepathos/debtmap/actions/workflows/security.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/security.yml)
[![Release](https://github.com/iepathos/debtmap/actions/workflows/release.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/release.yml)
[![Debtmap](https://github.com/iepathos/debtmap/actions/workflows/debtmap.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/debtmap.yml)
[![Crates.io](https://img.shields.io/crates/v/debtmap)](https://crates.io/crates/debtmap)
[![License](https://img.shields.io/badge/license-MIT)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/debtmap)](https://crates.io/crates/debtmap)

> 🚧 **Early Prototype** - This project is under active development and APIs may change

A code complexity and technical debt analyzer that identifies which code to refactor for maximum cognitive debt reduction and which code to test for maximum risk reduction.

📚 **[Read the full documentation](https://iepathos.github.io/debtmap/)** for detailed guides, examples, and API reference.

## Why Debtmap?

Unlike traditional static analysis tools that simply flag complex code, debtmap answers two critical questions:

1. **"What should I refactor to reduce cognitive burden?"** - Identifies overly complex code that slows down development
2. **"What should I test first to reduce the most risk?"** - Pinpoints untested complex code that threatens stability

**Unique Capabilities:**
- **Coverage-Risk Correlation** - Combines complexity metrics with test coverage to identify genuinely risky code (high complexity + low coverage = critical risk)
- **Reduced False Positives** - Uses entropy analysis and pattern detection to distinguish genuinely complex code from repetitive patterns, reducing false positives by up to 70%
- **Actionable Recommendations** - Provides specific guidance with quantified impact metrics instead of generic warnings
- **Multi-Factor Analysis** - Analyzes complexity, coverage, dependencies, and call graphs for comprehensive prioritization
- **Fast & Open Source** - Written in Rust for 10-100x faster analysis, MIT licensed with no enterprise pricing

📖 **Read more:** [Why Debtmap?](https://iepathos.github.io/debtmap/why-debtmap.html)

## What Makes Debtmap Different

| Capability | Debtmap Approach |
|-----------|------------------|
| **Risk Prioritization** | Correlates complexity with test coverage to identify truly risky code |
| **False Positive Reduction** | Uses entropy analysis to distinguish genuine complexity from repetitive patterns |
| **Recommendations** | Quantified impact metrics ("Add 6 tests, -3.7 risk reduction") |
| **Multi-Factor Scoring** | Combines complexity, coverage, dependencies, and call graphs |
| **Speed** | Rust-based parallel processing for 10-100x faster analysis |
| **Coverage Integration** | Works with any LCOV-compatible coverage tool |
| **Cost** | Free, open source, MIT licensed |

**Key Differentiator**: Debtmap combines coverage-risk correlation with multi-factor analysis (complexity, dependencies, call graphs) and entropy-adjusted scoring to reduce false positives and prioritize testing efforts effectively.

## Documentation

📚 **[Full Documentation](https://iepathos.github.io/debtmap/)** - Complete guides, tutorials, and API reference

### Quick Links
- [Getting Started](https://iepathos.github.io/debtmap/getting-started.html) - Installation and first analysis
- [CLI Reference](https://iepathos.github.io/debtmap/cli-reference.html) - Complete command documentation
- [Configuration](https://iepathos.github.io/debtmap/configuration.html) - Customize thresholds and behavior
- [Analysis Guide](https://iepathos.github.io/debtmap/analysis-guide.html) - Understanding metrics and scoring
- [Coverage & Risk](https://iepathos.github.io/debtmap/coverage-integration.html) - Integrate test coverage data
- [Examples](https://iepathos.github.io/debtmap/examples.html) - Common workflows and use cases

## Quick Start (3 Minutes)

### Install
```bash
curl -sSL https://raw.githubusercontent.com/iepathos/debtmap/master/install.sh | bash

# For test coverage analysis (optional)
cargo install cargo-llvm-cov
```

### Analyze
```bash
# Basic analysis
debtmap analyze .

# With test coverage (recommended)
cargo llvm-cov --lcov --output-path target/coverage/lcov.info
debtmap analyze . --lcov target/coverage/lcov.info

# Or using just command
just coverage
debtmap analyze . --lcov target/coverage/lcov.info

# Generate JSON report
debtmap analyze . --format json --output report.json
```

### Review Results
Debtmap shows you exactly what to fix first with actionable recommendations:

```
#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/parser.rs:38 parse_complex_input()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: -3.7 risk reduction
└─ WHY: Complex logic (cyclomatic=6) with 0% test coverage
```

📖 See the [Getting Started Guide](https://iepathos.github.io/debtmap/getting-started.html) for detailed installation, examples, and next steps.

## Key Features

- **Coverage-Risk Correlation** - Combines complexity with test coverage to prioritize genuinely risky code
- **Multi-Factor Analysis** - Analyzes complexity, coverage, dependencies, and call graphs for comprehensive scoring
- **Reduced False Positives** - Uses entropy analysis and pattern detection to distinguish genuine complexity from repetitive patterns (reduces false positives by up to 70%)
- **Actionable Recommendations** - Specific guidance with quantified impact metrics
- **Multi-language Support** - Full Rust support, partial Python/JavaScript/TypeScript
- **Fast Performance** - 10-100x faster than Java/Python-based competitors (written in Rust with parallel processing)
- **Language-Agnostic Coverage** - Works with any tool generating LCOV format
- **Context-Aware Analysis** - Understands entry points, call graphs, and testing patterns
- **Free & Open Source** - MIT licensed, no enterprise pricing required

📖 See the [Getting Started Guide](https://iepathos.github.io/debtmap/getting-started.html) for complete feature documentation and examples.

## Advanced Features

### God Object Detection
Debtmap identifies classes and modules with too many responsibilities using purity-weighted scoring that rewards functional programming patterns.

📖 **Read more:** [God Object Detection](https://iepathos.github.io/debtmap/god-object-detection.html)

#### Understanding GOD OBJECT vs GOD MODULE

Debtmap distinguishes between two different organizational anti-patterns:

**GOD OBJECT** - A single struct/class with too many methods and fields:
- Classification: >20 methods AND >5 fields on one struct/class
- Problem: One class doing too much, methods share mutable state
- Example output: `GOD OBJECT: UserController (52 methods, 8 fields)`
- Fix: Extract responsibilities into focused classes

**GOD MODULE** - A file with too many diverse functions:
- Classification: >20 module-level functions, but NOT a god object
- Problem: Module lacks cohesion, contains unrelated utilities
- Example output: `GOD MODULE (47 module functions)`
- Fix: Split into cohesive submodules by domain

**How to interpret the output:**

When debtmap detects a god object, you'll see:
```
#3 SCORE: 7.5 [HIGH]
├─ GOD OBJECT: src/controller.rs
├─ TYPE: UserController (52 methods, 8 fields)
├─ ACTION: Extract responsibilities into focused classes
└─ WHY: Single class with too many methods and fields
```

The key indicators:
- **Methods**: Number of methods on the dominant struct
- **Fields**: Number of fields in that struct
- This means refactor the specific struct, not the whole file

When debtmap detects a god module, you'll see:
```
#5 SCORE: 6.8 [HIGH]
├─ GOD MODULE: src/utils.rs
├─ TYPE: Module with 47 diverse functions
├─ ACTION: Split into cohesive submodules by domain
└─ WHY: Module lacks focus, contains unrelated utilities
```

The key indicators:
- **Module Functions**: Total count of module-level functions
- This means reorganize the file's functions into multiple focused modules

**Quick Decision Guide:**
- See "GOD OBJECT"? Extract that specific class into smaller classes
- See "GOD MODULE"? Split the file's functions into multiple focused modules
- Both can appear in the same codebase for different files

### Pattern Detection
Automatically detects common design patterns (Observer, Factory, Singleton, Strategy, etc.) with configurable confidence thresholds.

📖 **Read more:** [Analysis Guide](https://iepathos.github.io/debtmap/analysis-guide.html)

### Pure Mapping Pattern Detection
Reduces false positives from exhaustive match expressions that are actually simple and maintainable. Debtmap recognizes pure mapping patterns - match statements that transform input to output without side effects - and adjusts complexity scores accordingly.

**What's a pure mapping pattern?**

```rust
fn status_to_string(status: Status) -> &'static str {
    match status {
        Status::Success => "success",
        Status::Pending => "pending",
        Status::Failed => "failed",
        Status::Cancelled => "cancelled",
        // ... many more cases
    }
}
```

This function has high cyclomatic complexity (one branch per case), but it's simple to maintain because:
- Each branch is independent and straightforward
- No mutation or side effects occur
- The pattern is predictable and easy to understand
- Adding new cases requires minimal changes

**Impact**: By recognizing these patterns, debtmap reduces complexity scores by up to 30% for pure mapping functions, preventing them from incorrectly appearing as high-priority refactoring targets.

**Configuration**: Customize detection thresholds in `.debtmap.toml`:
```toml
[mapping_patterns]
enabled = true                      # Enable mapping pattern detection
complexity_reduction = 0.30         # Reduce complexity by 30%
min_branches = 3                    # Minimum match arms to consider
```

📖 **Read more:** [Configuration Guide](https://iepathos.github.io/debtmap/configuration.html#pure-mapping-pattern-detection)

### Role-Based Coverage Expectations
Debtmap recognizes that different types of functions have different testing priorities. Instead of applying a uniform 80% coverage target to all code, it uses role-specific expectations that reflect real-world testing best practices.

**Default Coverage Expectations by Role:**

| Function Role | Target | Why |
|--------------|--------|-----|
| **Pure Logic** | 90-100% | Easy to test, high ROI |
| **Business Logic** | 80-95% | Critical functionality |
| **Validation** | 85-98% | Must be correct |
| **State Management** | 75-90% | Complex behavior |
| **Error Handling** | 70-90% | Important paths |
| **I/O Operations** | 60-80% | Often integration tested |
| **Configuration** | 60-80% | Lower risk |
| **Orchestration** | 65-85% | Coordinating functions |
| **Utilities** | 75-95% | Should be reliable |
| **Initialization** | 50-75% | Lower priority |
| **Performance** | 40-60% | Optimization code |
| **Debug/Development** | 20-40% | Development-only code |

**How it works:**

When debtmap identifies a function with low coverage, it considers the function's role:
- A pure function with 70% coverage gets flagged (below 90% target)
- A debug function with 70% coverage is fine (above 30% target)

**Example output:**
```
#2 SCORE: 7.2 [HIGH]
├─ TEST GAP: ./src/calc.rs:42 compute_price()
├─ COVERAGE: 65% (expected: 90% for Pure functions) 🟠
├─ ACTION: Add 8 unit tests to reach target
└─ WHY: Pure logic is easy to test and high-value
```

**Customize expectations in `.debtmap.toml`:**
```toml
[coverage_expectations]
pure = { min = 90.0, target = 95.0, max = 100.0 }
business_logic = { min = 80.0, target = 90.0, max = 95.0 }
debug = { min = 20.0, target = 30.0, max = 40.0 }
```

**Manual role override:**

You can override automatic role detection using doc comments:
```rust
/// Calculate user discount
/// @debtmap-role: BusinessLogic
fn calculate_discount(user: &User) -> f64 {
    // debtmap will use BusinessLogic expectations (80-95%)
}
```

**Coverage gap severity indicators:**
- 🟢 Meets or exceeds target
- 🟡 Between min and target (minor gap)
- 🟠 Below min but above 50% of min (moderate gap)
- 🔴 Critically low (below 50% of min)

📖 **Read more:** [Coverage Integration Guide](https://iepathos.github.io/debtmap/coverage-integration.html#role-based-expectations)

### Complexity Scoring

Debtmap uses **weighted complexity scoring** that combines cyclomatic and cognitive complexity metrics with configurable weights. This approach provides more accurate prioritization by emphasizing cognitive complexity, which research shows correlates better with bug density and maintenance difficulty.

**Why cognitive complexity matters:**
- Cyclomatic complexity counts control flow branches (if, while, for, etc.)
- Cognitive complexity measures how hard code is to understand (nested conditions, breaks in linear flow)
- A function can have high cyclomatic but low cognitive complexity (e.g., a simple switch statement with many cases)
- Conversely, deeply nested conditionals have high cognitive complexity even with few branches

**Default weights:**
- **70% cognitive complexity** - Emphasizes human understanding difficulty
- **30% cyclomatic complexity** - Still considers control flow complexity
- Weights must sum to 1.0 and can be customized per project

**Weighted score calculation:**
1. Normalize both metrics to 0-100 scale (default: cyclomatic max=50, cognitive max=100)
2. Apply weights: `score = (0.3 × normalized_cyclomatic) + (0.7 × normalized_cognitive)`
3. Display as: `cyclomatic=15, cognitive=3 → weighted=11.1 (cognitive-driven)`

**Configuration** in `.debtmap.toml`:
```toml
[complexity_weights]
# Customize weights (must sum to 1.0)
cyclomatic = 0.3
cognitive = 0.7

# Adjust normalization based on your codebase
max_cyclomatic = 50.0
max_cognitive = 100.0
```

**Benefits:**
- Reduces false positives from simple repetitive patterns (e.g., mapping functions)
- Prioritizes deeply nested logic that's truly hard to understand
- Transparent scoring shows all metrics and the dominant driver
- Configurable for different project needs

📖 **Read more:** [Analysis Guide](https://iepathos.github.io/debtmap/analysis-guide.html)

### Cache Management
Intelligent cache system with automatic pruning and configurable strategies (LRU, LFU, FIFO, age-based).

📖 **Read more:** [Cache Management](https://iepathos.github.io/debtmap/cache-management.html)

### Suppression Patterns
Flexible suppression via inline comments or configuration files.

📖 **Read more:** [Suppression Patterns](https://iepathos.github.io/debtmap/suppression-patterns.html)

## Contributing

We welcome contributions! This is an early-stage project, so there's plenty of room for improvement.

📖 **See the [Contributing Guide](CONTRIBUTING.md)** for detailed development setup and contribution guidelines.

Please note that this project is released with a [Code of Conduct](CODE_OF_CONDUCT.md). By participating in this project you agree to abide by its terms.

### Areas for Contribution
- **Language support** - Add analyzers for Go, Java, etc.
- **New metrics** - Implement additional complexity or quality metrics
- **Speed** - Optimize analysis algorithms
- **Documentation** - Improve docs and add examples
- **Testing** - Expand test coverage

## Development

This project uses [Just](https://github.com/casey/just) for task automation.

```bash
# Common development tasks
just test        # Run all tests
just fmt         # Format code
just lint        # Run clippy linter
just check       # Quick syntax check
just dev         # Run in development mode
just watch       # Run with hot reloading

# CI and quality checks
just ci          # Run all CI checks locally
just coverage    # Generate test coverage report (uses cargo-llvm-cov)

# See all available commands
just --list
```

### Automated Technical Debt Reduction

📖 **See the [Prodigy Integration Guide](https://iepathos.github.io/debtmap/prodigy-integration.html)** for detailed information on using Prodigy and Claude Code for automated debt reduction.

We use [prodigy](https://github.com/iepathos/prodigy) for automated technical debt reduction through AI-driven workflows:

```bash
# Run automated debt reduction (5 iterations)
prodigy run workflows/debtmap.yml -yn 5
```

This command creates an isolated git worktree, runs iterations of automated improvements, validates changes, and commits with detailed metrics.

## License

MIT License - see [LICENSE](LICENSE) file for details

### Dependency Licensing Note

Debtmap includes Python parsing functionality via `rustpython-parser`, which depends on `malachite` (LGPL-3.0 licensed) for arbitrary-precision arithmetic. This LGPL dependency is used only for Python AST parsing and does not affect the MIT licensing of debtmap itself. For use cases requiring strict MIT-only dependencies, Python support can be disabled or replaced with an alternative parser.

## Debugging Call Graph Issues

Debtmap includes powerful debugging and diagnostic tools for troubleshooting call graph analysis and understanding function relationship detection.

### Debug Call Graph Resolution

View detailed information about how functions are resolved and linked in the call graph:

```bash
# Enable debug mode for call graph analysis
debtmap analyze . --debug-call-graph

# Output debug information in JSON format
debtmap analyze . --debug-call-graph --debug-format json

# Trace specific functions to see their resolution details
debtmap analyze . --debug-call-graph --trace-function my_function --trace-function other_function
```

**Debug output includes:**
- Resolution statistics (success rate, failure reasons)
- Strategy performance (exact match, fuzzy matching, etc.)
- Timing percentiles (p50, p95, p99) for performance analysis
- Failed resolutions with detailed candidate information
- Recommendations for improving resolution accuracy

### Validate Call Graph Structure

Check the structural integrity and health of the generated call graph:

```bash
# Run validation checks on call graph
debtmap analyze . --validate-call-graph

# Combine validation with debug output
debtmap analyze . --validate-call-graph --debug-call-graph
```

**Validation checks:**
- **Structural Issues**: Detects dangling edges, orphaned nodes, and duplicate functions
- **Heuristic Warnings**: Identifies suspicious patterns like unusually high fan-in/fan-out
- **Health Score**: Overall graph quality score (0-100) based on detected issues
- **Detailed Reports**: Shows specific issues with file locations and function names

### View Call Graph Statistics

Get quick statistics about call graph size and structure:

```bash
# Show call graph statistics only (fast, minimal output)
debtmap analyze . --call-graph-stats-only
```

**Statistics include:**
- Total number of functions analyzed
- Total number of function calls detected
- Average calls per function (graph density)

### Common Use Cases

**Debugging unresolved function calls:**
```bash
# See why specific functions aren't being linked
debtmap analyze . --debug-call-graph --trace-function problematic_function
```

**Validating analysis quality:**
```bash
# Check for structural problems in call graph
debtmap analyze . --validate-call-graph
```

**Performance profiling:**
```bash
# See timing breakdown of call resolution
debtmap analyze . --debug-call-graph --debug-format json
```

**Combining with normal analysis:**
```bash
# Run full analysis with debugging enabled
debtmap analyze . --lcov coverage.info --debug-call-graph --validate-call-graph
```

### Interpreting Debug Output

**Health Score:**
- **95-100**: Excellent - Very few unresolved calls
- **85-94**: Good - Acceptable resolution rate
- **<85**: Needs attention - High number of unresolved calls

**Resolution Strategies:**
- **Exact**: Exact function name match (highest confidence)
- **Fuzzy**: Qualified name match (e.g., `Module::function`)
- **NameOnly**: Base name match (lowest confidence, may have ambiguity)

**Common Issues:**
- **Dangling Edges**: References to non-existent functions (potential parser bugs)
- **Orphaned Nodes**: Functions with no connections (may indicate missed calls)
- **High Fan-Out**: Functions calling many others (potential god objects)
- **High Fan-In**: Functions called by many others (potential bottlenecks)

### Performance Considerations

Debug and validation modes add minimal overhead (<20% typically) and can be used in CI/CD pipelines. For large codebases (>1000 files), consider:
- Using `--call-graph-stats-only` for quick health checks
- Limiting `--trace-function` to specific problem areas
- Running full debug analysis periodically rather than on every build

## Viewing Dependency Information

Debtmap displays caller/callee relationships for each technical debt item, helping you understand the impact and reach of functions that need attention.

### Dependency Display in Output

When running analysis with default verbosity (`-v`), each debt item includes a DEPENDENCIES section showing:

```
#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/parser.rs:38 parse_complex_input()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: -3.7 risk reduction
├─ DEPENDENCIES:
|  |- Called by (3):
|       ⬆ validate_input
|       ⬆ process_request
|       ⬆ handle_api_call
|  |- Calls (2):
|       ⬇ tokenize
|       ⬇ validate_syntax
└─ WHY: Complex logic (cyclomatic=6) with 0% test coverage
```

**What the dependency information shows:**
- **Called by (callers)**: Functions that depend on this function (upward arrow ⬆)
- **Calls (callees)**: Functions this function depends on (downward arrow ⬇)
- Counts are shown in parentheses (e.g., "(3)" means 3 callers)

### Configuring Dependency Display

Control how many dependencies are shown using CLI flags:

```bash
# Limit callers and callees displayed (default: 5 each)
debtmap analyze . --max-callers 10 --max-callees 10

# Show external crate calls (hidden by default)
debtmap analyze . --show-external-calls

# Show standard library calls (hidden by default)
debtmap analyze . --show-std-lib-calls

# Hide all dependency information
debtmap analyze . --no-dependencies
```

### Configuration File

Add dependency display settings to `.debtmap.toml`:

```toml
[output.dependencies]
max_callers = 10        # Maximum callers to display (default: 5)
max_callees = 10        # Maximum callees to display (default: 5)
show_external = false   # Show external crate calls (default: false)
show_std_lib = false    # Show stdlib calls (default: false)
```

### Understanding Dependency Impact

Dependency information helps prioritize refactoring:
- **High caller count** → Changes affect many parts of codebase (higher refactoring risk)
- **High callee count** → Function has many dependencies (higher complexity)
- **Entry points** (few/no callers) → Good starting points for testing
- **Leaf functions** (few/no callees) → Easier to test in isolation

## CI/CD Integration with Density-Based Validation

Debtmap supports **density-based validation metrics** that work consistently across projects of any size. Unlike traditional absolute thresholds (e.g., "max complexity of 1000"), density metrics normalize by codebase size, making them ideal for CI/CD automation.

### Why Density-Based Metrics?

Traditional metrics fail across different project sizes:
- A 1,000-line project with complexity 500 → 50% of threshold
- A 100,000-line project with complexity 5,000 → 500% of threshold

Density metrics solve this by measuring per-line or per-function rates:
- Complexity density = total_complexity / total_functions
- Same threshold works for any project size
- Quality standards remain consistent as code grows

### Available Density Metrics

| Metric | Formula | Good Threshold | Description |
|--------|---------|----------------|-------------|
| **Complexity Density** | `total_complexity / total_functions` | < 10.0 | Average complexity per function |
| **Dependency Density** | `(dependencies / lines) * 1000` | < 5.0 | Dependencies per 1,000 lines |
| **Test Density** | `(tests / lines) * 100` | > 2.0 | Tests per 100 lines |

### Quick Start: GitHub Actions

Add density-based validation to your CI pipeline:

```yaml
name: Code Quality

on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install debtmap
        run: curl -sSL https://raw.githubusercontent.com/iepathos/debtmap/master/install.sh | bash

      - name: Validate code quality
        run: |
          debtmap analyze . \
            --max-complexity-density 10.0 \
            --max-dependency-density 5.0 \
            --min-test-density 2.0
```

**Benefits:**
- No threshold adjustments needed as your codebase grows
- Catches quality degradation early
- Consistent standards across all projects
- Predictable CI/CD behavior

### Setting Appropriate Thresholds

#### For New Projects

Start with industry best practices:

```bash
debtmap analyze . \
  --max-complexity-density 8.0 \    # Excellent: simple functions
  --max-dependency-density 3.0 \    # Minimal dependencies
  --min-test-density 2.5            # Comprehensive tests
```

#### For Existing Projects

1. **Baseline analysis** - Understand current state:
```bash
debtmap analyze . --density-metrics > baseline.json
```

2. **Set initial thresholds** - Current values + 20% buffer:
```bash
# Example: Current complexity density is 12.5
debtmap analyze . --max-complexity-density 15.0
```

3. **Gradual improvement** - Tighten thresholds quarterly:
```yaml
# Q1: Stabilize
--max-complexity-density 15.0

# Q2: Improve
--max-complexity-density 13.0

# Q3: Approach best practices
--max-complexity-density 10.0

# Q4: Maintain excellence
--max-complexity-density 8.0
```

### CI/CD Configuration Examples

#### GitHub Actions - Pull Request Validation

```yaml
name: PR Quality Check

on: pull_request

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for delta comparison

      - name: Install debtmap
        run: curl -sSL https://raw.githubusercontent.com/iepathos/debtmap/master/install.sh | bash

      - name: Analyze base branch
        run: |
          git checkout ${{ github.base_ref }}
          debtmap analyze . --density-metrics --format json > base.json

      - name: Analyze PR branch
        run: |
          git checkout ${{ github.head_ref }}
          debtmap analyze . --density-metrics --format json > pr.json

      - name: Check density delta
        run: |
          BASE_DENSITY=$(jq '.density_metrics.complexity_density' base.json)
          PR_DENSITY=$(jq '.density_metrics.complexity_density' pr.json)
          DELTA=$(echo "$PR_DENSITY - $BASE_DENSITY" | bc)

          if (( $(echo "$DELTA > 0.5" | bc -l) )); then
            echo "❌ Complexity density increased by $DELTA"
            exit 1
          fi

          echo "✅ Complexity density change: $DELTA"

      - name: Enforce absolute limits
        run: |
          debtmap analyze . \
            --max-complexity-density 10.0 \
            --max-dependency-density 5.0 \
            --min-test-density 2.0
```

#### GitLab CI - Multi-Stage Validation

```yaml
stages:
  - analyze
  - validate

code_analysis:
  stage: analyze
  script:
    - curl -sSL https://raw.githubusercontent.com/iepathos/debtmap/master/install.sh | bash
    - debtmap analyze . --density-metrics --format json > metrics.json
  artifacts:
    paths:
      - metrics.json
    expire_in: 1 week

quality_gates:
  stage: validate
  dependencies:
    - code_analysis
  script:
    - debtmap analyze . --max-complexity-density 10.0 --max-dependency-density 5.0 --min-test-density 2.0
  only:
    - merge_requests
    - master
```

#### CircleCI - Density Tracking

```yaml
version: 2.1

jobs:
  quality_check:
    docker:
      - image: cimg/rust:1.75
    steps:
      - checkout
      - run:
          name: Install debtmap
          command: curl -sSL https://raw.githubusercontent.com/iepathos/debtmap/master/install.sh | bash

      - run:
          name: Analyze and validate
          command: |
            debtmap analyze . \
              --density-metrics \
              --max-complexity-density 10.0 \
              --max-dependency-density 5.0 \
              --min-test-density 2.0 \
              --format json > /tmp/metrics.json

      - store_artifacts:
          path: /tmp/metrics.json
          destination: code-metrics

workflows:
  version: 2
  build:
    jobs:
      - quality_check
```

### Advanced CI/CD Patterns

#### Progressive Tightening

Automatically adjust thresholds based on historical data:

```bash
#!/bin/bash
# progressive-quality.sh

CURRENT_DENSITY=$(debtmap analyze . --density-metrics --format json | jq '.density_metrics.complexity_density')
HISTORICAL_AVG=12.5  # From last 30 days

if (( $(echo "$CURRENT_DENSITY < $HISTORICAL_AVG" | bc -l) )); then
  # Quality improved - tighten threshold
  NEW_THRESHOLD=$(echo "$CURRENT_DENSITY * 1.1" | bc)
  echo "✅ Quality improved! New threshold: $NEW_THRESHOLD"
else
  # Use current average
  NEW_THRESHOLD=$HISTORICAL_AVG
fi

debtmap analyze . --max-complexity-density "$NEW_THRESHOLD"
```

#### Multi-Environment Thresholds

Different standards for different branches:

```yaml
- name: Validate code quality
  run: |
    if [ "${{ github.ref }}" == "refs/heads/main" ]; then
      # Strict for production
      debtmap analyze . --max-complexity-density 8.0
    elif [ "${{ github.ref }}" == "refs/heads/develop" ]; then
      # Moderate for development
      debtmap analyze . --max-complexity-density 10.0
    else
      # Lenient for feature branches
      debtmap analyze . --max-complexity-density 12.0
    fi
```

#### Team-Specific Thresholds

Different teams, different standards:

```yaml
- name: Validate code quality
  run: |
    # Detect which team owns the changed files
    TEAM=$(git diff --name-only ${{ github.base_ref }} | xargs dirname | sort -u | head -1)

    case "$TEAM" in
      "src/core")
        # Core team: strict standards
        debtmap analyze src/core --max-complexity-density 6.0
        ;;
      "src/features")
        # Feature teams: moderate standards
        debtmap analyze src/features --max-complexity-density 10.0
        ;;
      *)
        # Default standards
        debtmap analyze . --max-complexity-density 8.0
        ;;
    esac
```

### Monitoring Density Trends

Track density metrics over time to identify trends:

```bash
# Store metrics with timestamp
DATE=$(date +%Y-%m-%d)
debtmap analyze . --density-metrics --format json > "metrics-$DATE.json"

# Plot trend (requires jq and gnuplot)
for file in metrics-*.json; do
  DATE=$(echo "$file" | sed 's/metrics-\(.*\)\.json/\1/')
  DENSITY=$(jq '.density_metrics.complexity_density' "$file")
  echo "$DATE $DENSITY"
done | gnuplot -e "
  set terminal png;
  set output 'density-trend.png';
  plot '-' using 1:2 with lines title 'Complexity Density'
"
```

### Troubleshooting CI/CD Integration

#### Issue: Thresholds fail on small codebases

**Cause:** Small projects have high variance in density metrics
**Solution:** Require minimum codebase size:

```bash
LINES=$(find . -name "*.rs" | xargs wc -l | tail -1 | awk '{print $1}')
if [ "$LINES" -gt 1000 ]; then
  debtmap analyze . --max-complexity-density 10.0
else
  echo "⚠️  Codebase too small for density validation (${LINES} lines)"
fi
```

#### Issue: Density metrics fluctuate wildly

**Cause:** Including/excluding test files inconsistently
**Solution:** Always exclude test files from production metrics:

```bash
debtmap analyze . \
  --exclude "**/tests/**" \
  --exclude "**/*_test.rs" \
  --max-complexity-density 10.0
```

#### Issue: Legacy code dominates metrics

**Cause:** Old code with high complexity affects overall density
**Solution:** Analyze new and legacy code separately:

```bash
# Strict for new code
debtmap analyze src/new_features --max-complexity-density 8.0

# Lenient for legacy
debtmap analyze src/legacy --max-complexity-density 15.0
```

### Migration Guide

For detailed information on migrating from scale-dependent to density-based validation, see the [Validation Migration Guide](docs/validation-migration.md).

The guide includes:
- Why migrate and key benefits
- Step-by-step migration process
- Threshold selection guidelines
- Example configurations for different project sizes
- Common migration questions and troubleshooting

### Benefits of Density-Based Metrics in Automation

✅ **Size-independent:** Same thresholds work for 1K or 1M lines
✅ **Predictable:** No surprise CI failures as code grows
✅ **Meaningful:** Measures actual code quality, not just size
✅ **Actionable:** Clear signals for refactoring priorities
✅ **Maintainable:** Set once, rarely need adjustment

## Roadmap

### Language Support
- [x] Rust - Full support with AST parsing and macro expansion
- [ ] Python - Full support via rustpython-parser
- [ ] JavaScript/TypeScript - Full support via tree-sitter
- [ ] Go - Planned
- [ ] C/C++ - Planned
- [ ] C# - Planned
- [ ] Java - Planned

### Core Features
- [x] Inline suppression comments
- [x] LCOV coverage integration with risk analysis
- [x] Risk-based testing prioritization
- [x] Comprehensive debt detection (20+ pattern types)
- [x] Security vulnerability detection
- [x] Resource management analysis
- [x] Code organization assessment
- [x] Testing quality evaluation
- [ ] Historical trend tracking

### Integrations
- [ ] GitHub Actions marketplace
- [ ] GitLab CI integration
- [ ] VSCode extension
- [ ] IntelliJ plugin
- [ ] Pre-commit hooks

## Acknowledgments

Built with excellent Rust crates including:
- [syn](https://github.com/dtolnay/syn) for Rust AST parsing
- [rustpython-parser](https://github.com/RustPython/RustPython) for Python parsing
- [tree-sitter](https://github.com/tree-sitter/tree-sitter) for JavaScript/TypeScript parsing
- [rayon](https://github.com/rayon-rs/rayon) for parallel processing
- [clap](https://github.com/clap-rs/clap) for CLI parsing

---

**Note**: This is a prototype tool under active development. Please report issues and feedback on [GitHub](https://github.com/iepathos/debtmap/issues). For detailed documentation, visit [iepathos.github.io/debtmap](https://iepathos.github.io/debtmap/).
