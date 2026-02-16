# debtmap

[![Crates.io](https://img.shields.io/crates/v/debtmap)](https://crates.io/crates/debtmap)
[![Downloads](https://img.shields.io/crates/d/debtmap)](https://crates.io/crates/debtmap)
[![CI](https://github.com/iepathos/debtmap/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Debtmap analyzes your codebase and ranks technical debt by risk. Know exactly where to focus your refactoring effort - whether you're fixing it yourself or handing it to an AI assistant.

## Why Debtmap?

Large codebases accumulate complexity. You know there's debt, but where do you start?

Debtmap answers that question by combining multiple signals into a single priority score:

- **Complexity** - cyclomatic, cognitive, nesting depth
- **Coverage gaps** - untested code with high complexity
- **Git history** - files with high churn and bug fix rates
- **Coupling** - functions with many dependencies
- **Purity** - side effects that make code harder to test

The result: a ranked list of what to fix first, with the context needed to understand why.

## Quick Start

```bash
# Install
cargo install debtmap

# Explore issues interactively (default)
debtmap analyze .

# Terminal output for scripts and CI
debtmap analyze . --format terminal

# JSON for programmatic access
debtmap analyze . --format json --top 10 > debt.json

# Pipe to an LLM for automated fixes
debtmap analyze . --format markdown --top 1 | claude "Fix this"
```

## How It Works

Debtmap combines static analysis with git history to score technical debt:

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| **Complexity** | Cyclomatic, cognitive, nesting depth | How hard code is to understand |
| **Coverage** | Test coverage percentage per function | How risky changes are |
| **Git History** | Change frequency, bug fix rate, author count | Which code keeps breaking |
| **Coupling** | Dependencies, call graph depth | How changes ripple through the codebase |
| **Purity** | Side effects, I/O operations | How testable and predictable code is |
| **Entropy** | Pattern consistency | Reduces false positives from intentional complexity |

These signals combine into a **severity score** (0-10). High scores mean high-complexity, poorly-tested, frequently-broken code.

## Supported Languages

| Language | Complexity | Functions | Async Patterns | Call Graph |
|----------|:----------:|:---------:|:--------------:|:----------:|
| **Rust** | Full | Full | Full | Full |
| **TypeScript** | Full | Full | Full | Full |
| **JavaScript** | Full | Full | Full | Full |

**Rust** — Full AST analysis with syn, including macro expansion and trait detection.

**TypeScript/JavaScript** — Tree-sitter parsing with support for:
- ES6+ syntax (arrow functions, classes, async/await)
- JSX/TSX for React components
- TypeScript-specific patterns (`any` usage, type assertions)
- Promise chains and callback nesting detection

```bash
# Analyze specific languages
debtmap analyze . --languages rust,typescript

# All supported languages are enabled by default
debtmap analyze .
```

## Interactive TUI

Run `debtmap analyze .` to explore results interactively:

![List view](docs/images/debtmap%20list%20view.png)

![Detail overview](docs/images/debtmap%20detail%20overview.png)

![Score breakdown](docs/images/debtmap%20detail%20score%20breakdown.png)

![Context for AI](docs/images/debtmap%20detail%20context.png)

Features:
- Browse debt items sorted by severity
- Drill into score breakdowns to understand why code ranks high
- View git history, dependencies, and test coverage per function
- Copy context to clipboard for AI assistants
- Jump to code in your editor

## LLM Integration

The `--format markdown` output is designed for AI coding assistants. It provides:

- **Context suggestions** - Specific file ranges the LLM should read to understand the problem
- **Structured metadata** - All scoring factors exposed so the LLM can reason about priorities
- **Minimal tokens** - Compact format that fits more context into the LLM's window
- **Deterministic output** - Same input produces same output for reproducible workflows

```bash
# Pipe directly to Claude Code
debtmap analyze . --format markdown --top 1 | claude "Fix this technical debt"
```

See [LLM Integration Guide](https://iepathos.github.io/debtmap/llm-integration.html) for details.

## Output Formats

```bash
# Markdown (recommended for AI workflows)
debtmap analyze . --format markdown

# JSON for programmatic access
debtmap analyze . --format json

# Terminal for human exploration
debtmap analyze . --format terminal
```

## Visual Dashboard

Explore your results interactively with the **[Online Dashboard](https://iepathos.github.io/debtmap/dashboard/)**:

```bash
# Generate JSON output
debtmap analyze . --format json -o debtmap.json --lcov coverage.lcov --context

# Open dashboard and load your JSON file
open https://iepathos.github.io/debtmap/dashboard/
```

**Local development**: If you have the repo cloned, open `viz-dev/dashboard.html` directly in your browser.

The dashboard provides:
- **Risk Quadrant** - Functions plotted by complexity vs coverage gap
- **Top Debt Items** - Sortable table of highest priority issues
- **Module Flow** - Chord diagram showing debt relationships
- **Risk Radar** - Multi-dimensional comparison of top files

All processing happens client-side - your data never leaves your browser.

## With Coverage Data

```bash
# Generate coverage first
cargo llvm-cov --lcov --output-path coverage.lcov

# Analyze with coverage integration
debtmap analyze . --lcov coverage.lcov
```

Coverage data enables accurate risk assessment - complex code with good tests ranks lower than simple code with no tests.

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

## Documentation

**[Full Documentation](https://iepathos.github.io/debtmap/)** — guides, examples, configuration reference

Quick links:
- [Getting Started](https://iepathos.github.io/debtmap/getting-started.html)
- [Visual Dashboard](https://iepathos.github.io/debtmap/dashboard/)
- [LLM Integration](https://iepathos.github.io/debtmap/llm-integration.html)
- [Configuration](https://iepathos.github.io/debtmap/configuration.html)
- [Metrics Reference](https://iepathos.github.io/debtmap/metrics-reference.html)

## Performance Profiling

Debtmap includes built-in profiling to identify performance bottlenecks.

### Built-in Timing

```bash
# Show timing breakdown for each analysis phase
debtmap analyze . --profile

# Write detailed timing data to JSON
debtmap analyze . --profile --profile-output timing.json
```

Example output:
```
=== Profiling Report ===
Total analysis time: 38.55s

Phase breakdown:
Operation                                    Duration        %      Count
------------------------------------------------------------------------
analyze_project                                27.94s    72.5%          1
  duplication_detection                        24.94s    64.7%          1
  parsing                                       2.57s     6.7%          1
unified_analysis                               10.75s    27.9%          1
  call_graph_building                           8.05s    20.9%          1
  debt_scoring                                  1.89s     4.9%          1
```

### External Profilers

For CPU-level profiling, use sampling profilers with debug builds:

**macOS (samply)**
```bash
# Install samply
cargo install samply

# Build with debug symbols
cargo build --profile dev

# Profile debtmap
samply record ./target/debug/debtmap analyze /path/to/project

# Opens Firefox Profiler with flame graphs
```

**macOS (Instruments)**
```bash
# Build with debug symbols
cargo build --profile dev

# Profile with Instruments
xcrun xctrace record --template 'Time Profiler' --launch ./target/debug/debtmap analyze .
```

**Linux (perf)**
```bash
# Build with debug symbols
RUSTFLAGS="-C debuginfo=2" cargo build --release

# Record profile
perf record -g ./target/release/debtmap analyze /path/to/project

# View results
perf report
```

**Tip:** The `--profile` flag identifies *what* is slow; sampling profilers show *why* it's slow at the code level.

## Roadmap

**Current focus:** Rust analysis excellence + AI workflow integration

- [x] Cognitive + cyclomatic complexity
- [x] Test coverage correlation
- [x] Pattern-based false positive reduction
- [x] LLM-optimized output format
- [x] Context suggestions for AI
- [x] Multi-language support (Rust, TypeScript, JavaScript)
- [ ] Streaming output for large codebases
- [ ] Python language support
- [ ] Go language support

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
