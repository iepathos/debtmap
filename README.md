# debtmap

[![CI](https://github.com/iepathos/debtmap/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/ci.yml)
[![Coverage](https://github.com/iepathos/debtmap/actions/workflows/coverage.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/coverage.yml)
[![Security](https://github.com/iepathos/debtmap/actions/workflows/security.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/security.yml)
[![Release](https://github.com/iepathos/debtmap/actions/workflows/release.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/release.yml)
[![Debtmap](https://github.com/iepathos/debtmap/actions/workflows/debtmap.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/debtmap.yml)
[![Crates.io](https://img.shields.io/crates/v/debtmap)](https://crates.io/crates/debtmap)
[![License](https://img.shields.io/badge/license-MIT)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/debtmap)](https://crates.io/crates/debtmap)

Debtmap is a Rust code analyzer that combines coverage-risk correlation with multi-factor analysis (complexity, dependencies, call graphs) and entropy-adjusted scoring to reduce false positives and prioritize testing efforts effectively.

📚 **[Read the full documentation](https://iepathos.github.io/debtmap/)** for detailed guides, examples, and API reference.

## Why Debtmap?

Debtmap answers two critical questions:

1. **"What should I refactor to reduce cognitive burden?"** - Identifies overly complex code that slows down development
2. **"What should I test first to reduce the most risk?"** - Pinpoints untested complex code that threatens stability

**Unique Capabilities:**
- **Rust-First Design** - Deep Rust analysis with macro expansion, trait resolution, and lifetime awareness
- **Coverage-Risk Correlation** - Combines complexity metrics with test coverage to identify genuinely risky code (high complexity + low coverage = critical risk)
- **Reduced False Positives** - Uses entropy analysis and pattern detection to distinguish genuinely complex code from repetitive patterns, reducing false positives by up to 70%
- **Actionable Recommendations** - Provides specific guidance with quantified impact metrics instead of generic warnings
- **Multi-Factor Analysis** - Analyzes complexity, coverage, dependencies, and call graphs for comprehensive prioritization
- **Fast & Open Source** - Written in Rust for 10-100x faster analysis, MIT licensed with no enterprise pricing

📖 **Read more:** [Why Debtmap?](https://iepathos.github.io/debtmap/why-debtmap.html)

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

# Generate JSON report
debtmap analyze . --format json --output report.json

# Generate interactive HTML dashboard
debtmap analyze . --format html > report.html
```

### Review Results

#### Progress Visualization

Debtmap features a beautiful **zen minimalist TUI** (Terminal User Interface) that provides real-time visualization of the entire analysis pipeline:

**TUI Features:**
- **Full pipeline visibility**: All 9 analysis stages displayed at once
- **Hierarchical progress**: Active stages expand to show sub-tasks
- **Rich context**: Live counts, percentages, and real-time statistics
- **Smooth animations**: 60 FPS rendering with progress bars and visual indicators
- **Automatic detection**: TUI automatically disables in CI/pipes for clean logs

**Analysis Pipeline (9 Stages):**

1. **files** - Scans your codebase and discovers files to analyze
2. **parse** - Parses each file and extracts AST information
3. **call graph** - Builds function call relationships
4. **trait resolution** - Resolves Rust trait method calls
5. **coverage** - Loads test coverage data (if provided)
6. **purity analysis** - Analyzes function side effects and purity
7. **context** - Loads project context and metadata
8. **debt scoring** - Calculates technical debt scores
9. **prioritization** - Ranks items by impact and priority

**TUI Controls:**
```bash
# TUI enabled by default (auto-detects TTY)
debtmap analyze .

# Disable TUI (fallback to simple progress bars)
debtmap analyze . --no-tui

# Quiet mode (no progress output)
debtmap analyze . --quiet
```

#### Analysis Results

Debtmap shows you exactly what to fix first with actionable recommendations:

```
#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/parser.rs:38 parse_complex_input()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: -3.7 risk reduction
└─ WHY: Complex logic (cyclomatic=6) with 0% test coverage
```

### Concise Actionable Recommendations

Debtmap provides step-by-step recommendations with clear impact estimates and difficulty levels. Each recommendation includes:

- **Maximum 5 high-level steps** - Focused, actionable tasks
- **Impact estimates** - Quantified improvements for each step
- **Difficulty indicators** - Easy/Medium/Hard classifications
- **Executable commands** - Concrete commands to run
- **Estimated effort** - Time estimates in hours

**Before (Legacy format):**
```
ACTION: Add tests and refactor
WHY: High complexity with low coverage
STEPS: Write tests, reduce complexity, verify improvements
```

**After (Concise format):**
```
PRIMARY ACTION: Add 8 tests for untested branches
ESTIMATED EFFORT: 2.5 hours

STEPS:
1. Add 8 tests for 70% coverage gap [Easy]
   Impact: +8 tests, reduce risk
   Commands: cargo test parse_complex_input::
            # Write focused tests covering critical paths

2. Extract complex branches into focused functions [Medium]
   Impact: -15 complexity
   Commands: cargo clippy -- -W clippy::cognitive_complexity

3. Verify tests pass and coverage improved [Easy]
   Impact: Confirmed +70% coverage
   Commands: cargo test --all
            # Run coverage tool to verify improvement
```

The new format helps you:
- **Prioritize** which step to do first (ordered by impact)
- **Estimate** how long the work will take
- **Execute** with specific commands to run
- **Verify** improvements with measurable impact

### HTML Dashboard Output

Debtmap can generate an interactive HTML dashboard for visualizing code quality metrics, complexity analysis, and technical debt:

```bash
# Generate HTML dashboard
debtmap analyze . --format html > debtmap-report.html

# With test coverage integration
debtmap analyze . --lcov target/coverage/lcov.info --format html > report.html
```

**Dashboard Features:**

- **Interactive Charts** - Visual representations of issue distribution, root causes, and complexity patterns
- **Sortable Tables** - Filter and sort complex functions by cyclomatic complexity, cognitive complexity, or nesting depth
- **Search Functionality** - Quick search across all detected issues and functions
- **Metrics Overview** - Summary cards showing critical/high/medium/low priority items
- **Complexity Analysis** - Scatter plot showing relationship between cyclomatic and cognitive complexity
- **God Object Detection** - Dedicated table for architectural debt items
- **Responsive Design** - Works on desktop, tablet, and mobile devices

**Opening the Dashboard:**

Simply open the generated HTML file in any modern web browser:

```bash
# macOS
open debtmap-report.html

# Linux
xdg-open debtmap-report.html

# Windows
start debtmap-report.html
```

The dashboard is a single self-contained HTML file with no external dependencies (uses CDN for Chart.js and Tailwind CSS), making it easy to share with your team or include in CI/CD artifacts.

📖 See the [Getting Started Guide](https://iepathos.github.io/debtmap/getting-started.html) for detailed installation, examples, and next steps.

## GitHub Actions Integration

Automate debtmap analysis in your CI/CD pipeline with the [debtmap GitHub Action](https://github.com/iepathos/debtmap-action).

### Quick Setup

Add debtmap analysis to your workflow:

```yaml
name: Code Quality

on: [push, pull_request]

jobs:
  debtmap:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: iepathos/debtmap-action@v1
        with:
          format: 'json'
          output: 'debtmap-report.json'
```

### With Coverage Analysis

Combine with test coverage for comprehensive risk assessment:

```yaml
name: Code Quality with Coverage

on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Generate coverage
        run: |
          cargo tarpaulin --out lcov --output-dir target/coverage

      - uses: iepathos/debtmap-action@v1
        with:
          coverage-file: 'target/coverage/lcov.info'
          format: 'json'
          output: 'debtmap-report.json'
```

#### Coverage Matching for Trait Methods (Rust)

Debtmap automatically handles coverage matching for Rust trait implementation methods, which can have different names between the AST analysis and LCOV coverage data.

**How it works:**

When analyzing Rust code, debtmap stores trait methods with their full qualified names (e.g., `RecursiveMatchDetector::visit_expr`), but LCOV often stores them with just the method name (e.g., `visit_expr`) after symbol demangling.

Debtmap tries multiple name variants automatically:
1. Full qualified name: `RecursiveMatchDetector::visit_expr`
2. Method name only: `visit_expr`
3. Trait-qualified name: `Visit::visit_expr`

**Benefits:**
- ✓ No false-positive "no coverage data" reports for trait methods
- ✓ Correctly reports coverage for `syn::visit::Visit`, `std::fmt::Display`, and other trait implementations
- ✓ Works automatically - no configuration needed
- ✓ Minimal performance impact (<2% overhead)

**Example:**

```bash
# Generate coverage
cargo llvm-cov --lcov --output-path target/coverage/lcov.info

# Analyze with coverage - trait methods automatically matched
debtmap analyze . --coverage-file target/coverage/lcov.info

# Verify specific trait method coverage
debtmap explain-coverage . \
  --coverage-file target/coverage/lcov.info \
  --function visit_expr \
  --file src/complexity/recursive_detector.rs
```

See [Spec 181](specs/181-trait-method-coverage-matching.md) for technical details.

### Enforce Quality Gates

Fail builds when quality thresholds are exceeded:

```yaml
- uses: iepathos/debtmap-action@v1
  with:
    max-complexity-density: '10.0'
    max-dependency-density: '5.0'
    min-test-density: '2.0'
    fail-on-violation: 'true'
```

📖 **See the [debtmap-action repository](https://github.com/iepathos/debtmap-action)** for complete documentation and configuration options.

## Key Features

- **Rust-First Analysis** - Deep Rust analysis with macro expansion, trait resolution, and lifetime awareness
- **Coverage-Risk Correlation** - Combines complexity with test coverage to prioritize genuinely risky code
- **Multi-Factor Analysis** - Analyzes complexity, coverage, dependencies, and call graphs for comprehensive scoring
- **Reduced False Positives** - Uses entropy analysis and pattern detection to distinguish genuine complexity from repetitive patterns (reduces false positives by up to 70%)
- **Test File Detection** - Automatically identifies test files and applies context-aware scoring adjustments
- **Actionable Recommendations** - Specific guidance with quantified impact metrics
- **Fast Performance** - 10-100x faster than Java/Python-based competitors (written in Rust with parallel processing)
- **Language-Agnostic Coverage** - Works with any tool generating LCOV format
- **Context-Aware Analysis** - Understands entry points, call graphs, and testing patterns
- **Free & Open Source** - MIT licensed, no enterprise pricing required

📖 See the [Getting Started Guide](https://iepathos.github.io/debtmap/getting-started.html) for complete feature documentation and examples.

## Parallel Batch Analysis

Debtmap uses the **traverse pattern** from functional programming for efficient parallel file analysis. This approach provides:

- **Parallel processing** - Analyze multiple files concurrently using all CPU cores
- **Error accumulation** - Collect ALL errors instead of failing at the first one
- **Configurable batching** - Control batch sizes for large codebases

### Basic Usage

```rust
use debtmap::analyzers::batch::{analyze_files_effect, validate_files};
use debtmap::effects::{run_effect, run_validation};

// Parallel analysis with Effect pattern
let files = vec!["src/main.rs".into(), "src/lib.rs".into()];
let results = run_effect(
    analyze_files_effect(files),
    config,
)?;

// Process results
for result in results {
    println!("{}: {} functions analyzed",
        result.path.display(),
        result.metrics.complexity.functions.len()
    );
}
```

### Validation with Error Accumulation

When you need comprehensive error reporting (showing ALL issues, not just the first):

```rust
use debtmap::analyzers::batch::validate_files;
use debtmap::effects::run_validation;

let paths = vec!["file1.rs".into(), "file2.rs".into(), "file3.rs".into()];
match run_validation(validate_files(&paths)) {
    Ok(validated_files) => {
        // All files are valid, proceed with analysis
        println!("Validated {} files", validated_files.len());
    }
    Err(all_errors) => {
        // Show ALL validation errors to user
        eprintln!("Found {} validation errors:", all_errors.len());
        for error in all_errors {
            eprintln!("  - {}", error);
        }
    }
}
```

### Configuration

Control parallel processing behavior in `.debtmap.toml`:

```toml
[batch_analysis]
# Enable/disable parallel processing
parallelism.enabled = true

# Batch size for chunked processing (default: auto-detected based on CPU count)
parallelism.batch_size = 50

# Collect timing information for each file
collect_timing = true
```

Or via CLI:

```bash
# Analyze with timing information
debtmap analyze . --collect-timing

# Force sequential processing (useful for debugging)
debtmap analyze . --sequential
```

### Performance Characteristics

| File Count | Speedup (8 cores) | Memory |
|------------|-------------------|--------|
| 10 files   | ~2x               | Minimal |
| 50 files   | ~4x               | Low |
| 100+ files | ~6-7x             | Moderate |

The traverse pattern batches files to balance parallelism with memory usage. For very large codebases (1000+ files), files are processed in configurable chunks.

### When to Use Each Pattern

| Pattern | Use Case |
|---------|----------|
| `analyze_files_effect` | Normal analysis - fail fast on errors |
| `validate_files` | Pre-validation - collect ALL errors for user feedback |
| `validate_and_analyze_files` | Combined - validate first, then analyze valid files |

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

#### Smart Refactoring Recommendations

Debtmap provides tailored recommendations based on your file's characteristics:

**Struct-Heavy Modules** (many type definitions):
- **Detection criteria**: 5+ structs with 3+ semantic domains, struct-to-function ratio > 0.3
- **Recommendation style**: Domain-based organization
- **Example**: A `config.rs` file with `ScoreConfig`, `ThresholdConfig`, `DetectionConfig` will be recommended to split into:
  - `config/scoring.rs` - Score-related structures
  - `config/thresholds.rs` - Threshold-related structures
  - `config/detection.rs` - Detection-related structures
- **Why**: Groups related types together for better semantic cohesion

**Method-Heavy Modules** (many functions):
- **Detection criteria**: Does not meet struct-heavy criteria
- **Recommendation style**: Responsibility-based organization
- **Example**: A utility file with diverse functions will be recommended to split by responsibility:
  - `parsing.rs` - Input parsing functions
  - `formatting.rs` - Output formatting functions
  - `validation.rs` - Validation functions
- **Why**: Separates different functional concerns for clarity

**Severity Levels**:
- **Critical**: God object with cross-domain mixing (immediate action recommended)
- **High**: Significant complexity or size issues (priority refactoring)
- **Medium**: Proactive improvement opportunity (approaching thresholds)
- **Low**: Informational suggestions (minor improvements)

#### Domain Diversity Analysis

For struct-heavy modules, debtmap performs domain diversity analysis to identify cross-domain mixing patterns that violate the single responsibility principle.

**How It Works**:
- Analyzes struct naming patterns to identify semantic domains (e.g., "Config", "Error", "Handler")
- Calculates domain diversity scores based on struct distribution across domains
- Assigns severity levels from OK to CRITICAL based on diversity

**Severity Levels**:
- **OK**: Single domain or closely related domains (diversity ≤ 0.4)
- **MODERATE**: Some domain mixing (0.4 < diversity ≤ 0.6)
- **HIGH**: Significant cross-domain concerns (0.6 < diversity ≤ 0.75)
- **CRITICAL**: Severe domain mixing (diversity > 0.75)

**Example Output**:
```
WHY THIS MATTERS: This module contains 12 structs across 4 distinct domains.
Cross-domain mixing (Severity: CRITICAL) violates single responsibility
principle and increases maintenance complexity.

DOMAIN DIVERSITY ANALYSIS (Spec 140):
Severity: CRITICAL - 12 structs across 4 domains

Domain Distribution:
  - Configuration: 5 structs (42%)
    Examples: AppConfig, DatabaseConfig, CacheConfig
  - Error Handling: 4 structs (33%)
    Examples: ParseError, ValidationError, NetworkError
  - Request Processing: 2 structs (17%)
    Examples: HttpRequest, ApiResponse
  - Caching: 1 structs (8%)
    Examples: CacheEntry

Recommendation: Split into domain-focused modules for better cohesion
```

This analysis helps you understand exactly why a module should be split and provides clear guidance on how to organize the extracted modules by domain.

**Example recommendation output**:
```
GOD OBJECT DETECTED: src/config.rs (10 structs across 3 domains)
  Recommendation: Split by semantic domain
  Severity: High

  Suggested splits:
    1. config/scoring.rs
       Structs: ScoreConfig, ScoreCalculator, ScoreValidator
       Estimated lines: ~150

    2. config/thresholds.rs
       Structs: ThresholdConfig, ThresholdValidator, ThresholdManager, ThresholdFactory
       Estimated lines: ~200
```

#### Semantic Module Naming

When splitting god objects, debtmap uses intelligent semantic naming to generate descriptive, meaningful module names based on the methods in each split. This eliminates generic names like `utils`, `misc`, or `helpers` and ensures each split has a clear, specific identity.

**How It Works**:
- **Domain Term Extraction**: Analyzes method names to find common domain terms (e.g., "coverage", "metrics", "config")
- **Behavioral Pattern Recognition**: Identifies behavioral patterns like "formatting", "validation", "parsing", "computation"
- **Specificity Scoring**: Ensures names are descriptive, rejecting generic terms
- **Uniqueness Validation**: Guarantees no filename collisions across splits

**Naming Strategies**:
1. **Domain Terms**: Extracts dominant terms from method names
   - Methods: `format_coverage_status`, `format_coverage_factor`, `calculate_coverage_percentage`
   - Generated name: `coverage` (confidence: 0.85)

2. **Behavioral Patterns**: Recognizes common software patterns
   - Methods: `validate_index`, `validate_data`, `validate_config`
   - Generated name: `validation` (confidence: 0.75)

3. **Descriptive Fallback**: When no clear pattern emerges, generates meaningful placeholders
   - Methods: `do_something`, `handle_stuff`
   - Generated name: `needs_review_group_1` (confidence: 0.4)

**Confidence Scoring**:
- **High (0.7-1.0)**: Clear, unambiguous pattern detected
- **Medium (0.5-0.7)**: Reasonable pattern with some uncertainty
- **Low (0.4-0.5)**: Fallback name, manual review recommended
- **Rejected (<0.4)**: Name too generic, alternative generated

**Example Output**:
```
GOD OBJECT DETECTED: src/data_manager.rs (24 methods)

  Suggested splits:
    1. data_manager/formatting.rs (confidence: 0.85)
       Methods: format_output, format_summary, format_report
       Responsibility: Output formatting operations

    2. data_manager/validation.rs (confidence: 0.78)
       Methods: validate_index, validate_data, validate_config
       Responsibility: Input validation

    3. data_manager/parsing.rs (confidence: 0.72)
       Methods: parse_input, parse_config, parse_json
       Responsibility: Data parsing operations
```

**Alternative Names**: Each split includes up to 3 name candidates ranked by confidence, allowing you to choose the most appropriate name for your codebase conventions.

### Framework Pattern Detection
Debtmap identifies framework-specific code patterns in Rust, improving the accuracy of responsibility classification and helping distinguish framework boilerplate from application logic.

**Supported Frameworks:**

- **Rust**: Axum, Actix-Web, Tokio, Diesel, Clap

**How It Works:**

Framework patterns are detected using a combination of:
- Import/require statements
- Decorators and attributes
- Function signatures and parameters
- Return types and naming conventions
- File path patterns

**Example Detection:**

```rust
// Axum Web Handler - Detected as "HTTP Request Handler"
async fn get_user(Path(user_id): Path<u32>) -> Json<User> {
    // ...
}
```

**Custom Pattern Configuration:**

You can add custom framework patterns by creating a `framework_patterns.toml` file in your project root:

```toml
[rust.web.your_framework]
name = "Your Framework"
category = "HTTP Request Handler"
patterns = [
    { type = "import", pattern = "your_framework::" },
    { type = "parameter", pattern = "Request<" },
    { type = "return_type", pattern = "Response" },
]
```

Pattern types available:
- `import` - Match import/use statements
- `decorator` - Match Python/TypeScript decorators
- `attribute` - Match Rust attributes (#[...])
- `derive` - Match Rust derive macros
- `parameter` - Match function parameter types
- `return_type` - Match function return types
- `name` - Match function names (regex supported)
- `call` - Match function calls in body
- `file_path` - Match file paths (regex supported)

**Benefits:**

- **Better Responsibility Classification**: Framework handlers are correctly categorized instead of being flagged as generic "I/O" operations
- **Reduced False Positives**: Test functions and framework boilerplate are properly identified
- **Context-Aware Analysis**: Understanding framework patterns helps debtmap provide more accurate complexity assessments

### Test File Detection and Context-Aware Scoring

Debtmap automatically identifies test files and test functions in Rust, then applies context-aware scoring adjustments to reduce false positives from test-specific patterns.

**Rust Test Detection:**

Debtmap detects test files using Rust-specific patterns:

- **Rust**: `#[test]`, `#[cfg(test)]`, files in `tests/` directory, `_test.rs` suffix
- **General**: Files in `tests/`, `test/` directories

**Context-Aware Scoring:**

When debtmap identifies a test file or test function, it automatically:

1. **Reduces complexity penalties** - Test code often has high cyclomatic complexity (many branches for edge cases) but is maintainable
2. **Adjusts priority levels** - Test debt is scored lower priority than production code debt
3. **Changes coverage expectations** - Test files don't need test coverage themselves
4. **Provides test-specific recommendations** - Suggests test refactoring patterns instead of production refactoring patterns

**Example Output:**

```
#7 SCORE: 4.2 [MEDIUM]
├─ TEST CODE: ./tests/integration_test.rs:125 test_complex_workflow()
├─ COMPLEXITY: cyclomatic=12, cognitive=8 (test-adjusted)
├─ ACTION: Extract test helper functions for reusability
└─ WHY: Test complexity is acceptable but helpers improve maintainability
```

**Benefits:**

- **Fewer false positives** - Test code complexity doesn't dominate production priorities
- **Better recommendations** - Test-specific refactoring guidance
- **Automatic detection** - No configuration needed for standard Rust test patterns

📖 **Read more:** [Testing Guide](https://iepathos.github.io/debtmap/testing-guide.html)

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

### Suppression Patterns
Flexible suppression via inline comments or configuration files.

📖 **Read more:** [Suppression Patterns](https://iepathos.github.io/debtmap/suppression-patterns.html)

## Contributing

We welcome contributions! This is an early-stage project, so there's plenty of room for improvement.

📖 **See the [Contributing Guide](CONTRIBUTING.md)** for detailed development setup and contribution guidelines.

Please note that this project is released with a [Code of Conduct](CODE_OF_CONDUCT.md). By participating in this project you agree to abide by its terms.

### Areas for Contribution
- **Rust-specific analysis** - Enhance macro expansion, trait resolution, lifetime analysis
- **New metrics** - Implement additional complexity or quality metrics for Rust
- **Speed** - Optimize analysis algorithms and parallel processing
- **Documentation** - Improve docs and add examples
- **Testing** - Expand test coverage, especially property-based tests

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

Debtmap has no restrictive dependencies - all dependencies are MIT, Apache-2.0,
or similarly permissive licenses.

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

## Multi-Signal Responsibility Classification

Debtmap uses multi-signal aggregation to accurately classify function responsibilities, achieving **~88% accuracy** compared to ~50% with name-based classification alone.

### Signals

The classification system combines multiple independent signals:

| Signal | Weight | Purpose |
|--------|--------|---------|
| **I/O Detection** | 35% | Identifies file, network, and database operations |
| **Call Graph Analysis** | 25% | Detects orchestration and coordination patterns |
| **Type Signatures** | 15% | Infers responsibility from parameter and return types |
| **Name Heuristics** | 15% | Uses function naming conventions |
| **Purity Analysis** | 5% | Identifies pure computation functions |
| **Framework Patterns** | 5% | Detects framework-specific patterns (web handlers, tests, CLI) |

### Classification Categories

The system classifies functions into these responsibility categories:

**I/O Operations:**
- File I/O, Network I/O, Database I/O, Configuration I/O

**Handlers:**
- HTTP Request Handler, WebSocket Handler, CLI Handler, Database Handler

**Computation:**
- Pure Computation, Validation, Transformation, Parsing, Formatting

**Coordination:**
- Orchestration, Coordination, Error Handling

**Testing:**
- Test Function

### Accuracy & Validation

- **Baseline (name-only):** ~50% accuracy
- **Multi-signal:** **~88% accuracy** (+38% improvement)
- **Validated against:** 15+ manually classified test cases across all categories
- **Configuration:** Tunable weights in `aggregation_config.toml`

### Explainability

Each classification includes:
- **Primary category** with confidence score
- **Evidence** from each signal that contributed
- **Alternative classifications** with their scores
- **Clear reasoning** for troubleshooting misclassifications

Example output:
```json
{
  "primary": "FileIO",
  "confidence": 0.82,
  "evidence": [
    {"signal": "io_detection", "contribution": 0.35, "description": "2 file ops"},
    {"signal": "name_heuristics", "contribution": 0.11, "description": "Name pattern: read_config"}
  ],
  "alternatives": [
    {"category": "ConfigurationIO", "score": 0.24}
  ]
}
```

### Benefits

✅ **Higher accuracy:** 88% vs 50% name-based alone
✅ **Reduced false positives:** Multiple signals must agree
✅ **Explainable:** Clear evidence trail for each classification
✅ **Configurable:** Adjust weights for your codebase's patterns
✅ **Performance:** <3% overhead with parallel processing

## Roadmap

### Rust Analysis (Primary Focus)
- [x] Rust AST parsing with syn
- [x] Macro expansion tracking
- [x] Trait resolution and analysis
- [x] Lifetime and ownership analysis
- [x] Async/await pattern detection
- [x] Property-based test detection
- [ ] Unsafe code analysis and scoring
- [ ] Performance pattern detection (clone vs borrow)
- [ ] Error propagation analysis (Result/Option chains)
- [ ] Type state pattern detection
- [ ] Cargo workspace analysis
- [ ] Proc macro complexity tracking

**Strategic Direction**: Debtmap is currently focused exclusively on perfecting Rust code analysis. Support for other languages is on the long term goals. If you want support for a specific language please open an issue, slide me a dm, or shoot me an email to discuss further. Languages that are high on my hit list are Go, Python, Javascript/Typescript, and C++. 

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
- [rayon](https://github.com/rayon-rs/rayon) for parallel processing
- [clap](https://github.com/clap-rs/clap) for CLI parsing

---

**Note**: This is a prototype tool under active development. Please report issues and feedback on [GitHub](https://github.com/iepathos/debtmap/issues). For detailed documentation, visit [iepathos.github.io/debtmap](https://iepathos.github.io/debtmap/).
