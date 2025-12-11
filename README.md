# debtmap

[![Crates.io](https://img.shields.io/crates/v/debtmap)](https://crates.io/crates/debtmap)
[![Downloads](https://img.shields.io/crates/d/debtmap)](https://crates.io/crates/debtmap)
[![CI](https://github.com/iepathos/debtmap/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Stop guessing where bugs hide. Start fixing what matters.**

debtmap finds the Rust functions that are complex, untested, and frequently changed—the places bugs actually live.

<!-- TODO: Add terminal recording GIF here showing `debtmap analyze .` running -->
<!-- Recommended: Use `asciinema` or `vhs` to record, convert to GIF with `agg` -->

## The Problem

Static analysis tools cry wolf. You get hundreds of warnings, most are noise, and you waste time on code that works fine.

**debtmap is different.** It combines 5 signals to find *actual* risk:

| Signal | What it catches |
|--------|-----------------|
| Cognitive complexity | Code that's hard to understand |
| Test coverage gaps | Untested critical paths |
| Git history | Code that breaks repeatedly |
| Pattern recognition | Ignores simple match statements |
| Entropy analysis | Filters repetitive false positives |

The result: a prioritized list of what to fix, with quantified impact.

## Install

```bash
cargo install debtmap
```

## Usage

```bash
# Analyze your project
debtmap analyze .

# With test coverage (recommended)
cargo llvm-cov --lcov --output-path coverage.lcov
debtmap analyze . --lcov coverage.lcov

# Generate HTML report
debtmap analyze . --format html > report.html
```

## What You Get

```
#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/parser.rs:38 parse_complex_input()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: -3.7 risk reduction
├─ DEPENDENCIES:
│  ├─ Called by: validate_input, process_request, handle_api_call
│  └─ Calls: tokenize, validate_syntax
└─ WHY: Complex logic (cyclomatic=6, cognitive=12) with 0% test coverage

STEPS:
1. Add 8 tests for 70% coverage gap [Easy]
   Commands: cargo test parse_complex_input::

2. Extract complex branches into focused functions [Medium]
   Commands: cargo clippy -- -W clippy::cognitive_complexity

3. Verify improvements [Easy]
   Commands: cargo test --all
```

Every item tells you:
- **What** to fix (exact file and line)
- **Why** it matters (the risk signals that triggered it)
- **How** to fix it (concrete steps with commands)
- **Impact** (quantified risk reduction)

## Why debtmap?

### Fewer False Positives

A 100-line `match` statement converting enums to strings? Other tools flag it as complex. debtmap recognizes it as a simple mapping and moves on.

**Five pattern systems** eliminate noise:
- Pure mapping detection (40% complexity reduction for simple matches)
- Entropy analysis (repetitive validation chains aren't complex)
- Framework patterns (Axum handlers, Tokio async, Clap CLI)
- Recursive match detection (context-aware nesting analysis)
- Complexity classification (state machines vs god objects)

### Actually Prioritized

Not alphabetical. Not by file. By **actual risk**:

```
Risk = Complexity × (1 - Coverage) × Change Frequency × Bug History
```

Complex + untested + frequently changed = fix first.

### Fast

10-100x faster than Java/Python tools. Parallel processing, lock-free caching, written in Rust.

```
190K lines analyzed in 3.2 seconds (8 cores)
```

## CI/CD Integration

```yaml
# .github/workflows/quality.yml
name: Code Quality
on: [push, pull_request]

jobs:
  debtmap:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: iepathos/debtmap-action@v1
        with:
          max-complexity-density: '10.0'
          fail-on-violation: 'true'
```

Density-based thresholds work for any codebase size—no adjustment needed as your code grows.

## Documentation

**[Full Documentation](https://iepathos.github.io/debtmap/)** — guides, examples, configuration reference

Quick links:
- [Getting Started](https://iepathos.github.io/debtmap/getting-started.html)
- [Configuration](https://iepathos.github.io/debtmap/configuration.html)
- [CI/CD Integration](https://iepathos.github.io/debtmap/ci-cd-integration.html)
- [Coverage Integration](https://iepathos.github.io/debtmap/coverage-integration.html)

## Roadmap

**Current focus:** Rust analysis excellence

- [x] Cognitive + cyclomatic complexity
- [x] Test coverage correlation
- [x] Git history analysis
- [x] Pattern-based false positive reduction
- [x] Framework detection (Axum, Actix, Tokio, Diesel, Clap)
- [x] Interactive TUI and HTML dashboards
- [ ] Unsafe code analysis
- [ ] Performance pattern detection
- [ ] Multi-language support (Go, Python, TypeScript)

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Good first issues:**
- Improve Rust-specific analysis
- Add new complexity metrics
- Expand test coverage
- Documentation improvements

## License

MIT — see [LICENSE](LICENSE)

---

**Questions?** [Open an issue](https://github.com/iepathos/debtmap/issues) or check the [documentation](https://iepathos.github.io/debtmap/).
