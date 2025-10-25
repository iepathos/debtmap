# debtmap

[![CI](https://github.com/iepathos/debtmap/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/ci.yml)
[![Security](https://github.com/iepathos/debtmap/actions/workflows/security.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/security.yml)
[![Release](https://github.com/iepathos/debtmap/actions/workflows/release.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/release.yml)
[![Debtmap](https://github.com/iepathos/debtmap/actions/workflows/debtmap.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/debtmap.yml)
[![Crates.io](https://img.shields.io/crates/v/debtmap)](https://crates.io/crates/debtmap)
[![License](https://img.shields.io/badge/license-MIT)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/debtmap)](https://crates.io/crates/debtmap)

> 🚧 **Early Prototype** - This project is under active development and APIs may change

A fast code complexity and technical debt analyzer written in Rust. Debtmap identifies which code to refactor for maximum cognitive debt reduction and which code to test for maximum risk reduction, providing data-driven prioritization for both.

📚 **[Read the full documentation](https://iepathos.github.io/debtmap/)** for detailed guides, examples, and API reference.

## Why Debtmap?

Unlike traditional static analysis tools that simply flag complex code, debtmap answers two critical questions:

1. **"What should I refactor to reduce cognitive burden?"** - Identifies overly complex code that slows down development
2. **"What should I test first to reduce the most risk?"** - Pinpoints untested complex code that threatens stability

**Unique Capabilities:**
- **Entropy-Based Complexity Analysis** - Uses information theory to distinguish genuinely complex code from pattern-based repetitive code, reducing false positives by up to 70%
- **Coverage-Risk Correlation** - The only tool that combines complexity metrics with test coverage to identify genuinely risky code
- **Actionable Recommendations** - Provides specific guidance with quantified impact metrics instead of generic warnings
- **Blazing Fast** - Written in Rust for 10-100x faster analysis than Java/Python-based competitors
- **Free & Open Source** - MIT licensed, no enterprise pricing required

📖 **Read more:** [Why Debtmap?](https://iepathos.github.io/debtmap/why-debtmap.html)

## What Makes Debtmap Different

| Capability | Debtmap Approach |
|-----------|------------------|
| **Complexity Analysis** | Entropy-based analysis distinguishes genuine complexity from repetitive patterns |
| **Risk Prioritization** | Correlates complexity with test coverage to identify truly risky code |
| **Recommendations** | Quantified impact metrics ("Add 6 tests, -3.7 risk reduction") |
| **Speed** | Rust-based parallel processing for 10-100x faster analysis |
| **Coverage Integration** | Works with any LCOV-compatible coverage tool |
| **Cost** | Free, open source, MIT licensed |

**Key Differentiator**: Debtmap is the only tool that combines entropy-based complexity analysis with coverage-risk correlation to reduce false positives and prioritize testing efforts.

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
```

### Analyze
```bash
# Basic analysis
debtmap analyze .

# With test coverage (recommended)
cargo tarpaulin --out lcov --output-dir target/coverage
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

- **Entropy-Based Complexity Analysis** - Reduces false positives by 70% using information theory
- **Coverage-Risk Correlation** - The only tool combining complexity with test coverage
- **Actionable Recommendations** - Specific guidance with quantified impact metrics
- **Multi-language Support** - Full Rust support, partial Python/JavaScript/TypeScript
- **Blazing Fast** - 10-100x faster than Java/Python-based competitors (written in Rust)
- **Language-Agnostic Coverage** - Works with any tool generating LCOV format
- **Context-Aware Analysis** - Intelligently reduces false positives by 70%
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
just coverage    # Generate test coverage report

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
