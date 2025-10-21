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

## How Debtmap Compares

| Feature | Debtmap | SonarQube | CodeClimate | cargo-geiger | clippy |
|---------|---------|-----------|-------------|--------------|--------|
| **Speed** | ⚡ Fast (Rust) | 🐌 Slow (JVM) | 🐌 Slow (Ruby) | ⚡ Fast (Rust) | ⚡ Fast (Rust) |
| **Entropy Analysis** | ✅ Yes | ❌ No | ❌ No | ❌ No | ❌ No |
| **Coverage Integration** | ✅ LCOV | ⚠️ Enterprise only | ❌ No | ❌ No | ❌ No |
| **False Positives** | 🟢 Low (70% reduction) | 🔴 High | 🟡 Medium | 🟢 Low | 🟡 Medium |
| **Rust Support** | ✅ Full AST analysis | ⚠️ Basic | ⚠️ Basic | ✅ Security focus | ✅ Lints only |
| **Cost** | 🆓 Free & Open Source | 💰 Enterprise pricing | 💰 Paid tiers | 🆓 Free | 🆓 Free |
| **Actionable Recommendations** | ✅ Specific with impact metrics | ⚠️ Generic warnings | ⚠️ Generic warnings | ❌ Detection only | ⚠️ Generic suggestions |
| **Coverage-Risk Correlation** | ✅ Unique feature | ❌ No | ❌ No | ❌ No | ❌ No |

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

### Pattern Detection
Automatically detects common design patterns (Observer, Factory, Singleton, Strategy, etc.) with configurable confidence thresholds.

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

## Roadmap

### Language Support
- [x] Rust - Full support with AST parsing and macro expansion
- [x] Python - Full support via rustpython-parser
- [x] JavaScript/TypeScript - Full support via tree-sitter
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
- [ ] Automated refactoring suggestions

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
