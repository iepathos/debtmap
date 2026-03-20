# debtmap

[![Crates.io](https://img.shields.io/crates/v/debtmap)](https://crates.io/crates/debtmap)
[![Downloads](https://img.shields.io/crates/d/debtmap)](https://crates.io/crates/debtmap)
[![CI](https://github.com/iepathos/debtmap/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Debtmap analyzes your codebase and ranks technical debt by risk. Use it directly in the terminal, TUI, or dashboard to understand where to focus refactoring effort, or pipe the results into an AI assistant when that fits your workflow.

## Why Debtmap?

Large codebases accumulate complexity. You know there's debt, but where do you start?

Debtmap combines static analysis with git history to score technical debt across multiple signals:

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| **Complexity** | Cyclomatic, cognitive, nesting depth | How hard code is to understand |
| **Coverage** | Test coverage percentage per function | How risky changes are |
| **Git History** | Change frequency, bug fix rate, author count | Which code keeps breaking |
| **Coupling** | Dependencies, call graph depth | How changes ripple through the codebase |
| **Purity** | Side effects, I/O operations | How testable and predictable code is |
| **Entropy** | Pattern consistency within a codebase | Reduces false positives from intentional complexity |

These signals combine into a **severity score** (0-10). The result: a ranked list of what to fix first, with the context needed to understand why, whether you're reviewing it yourself or handing it off to automation.

## Quick Start

```bash
# Install
cargo install debtmap

# Explore issues interactively (default)
debtmap analyze .

# JSON for programmatic access
debtmap analyze . --format json --top 10 --output debt.json

# Optional: pipe to an LLM for automated fixes
debtmap analyze . --format markdown --top 1 | claude "Fix this"
```

## Supported Languages

- **Rust** — Full AST analysis with syn, including macro expansion and trait detection
- **Python** — Tree-sitter-based analysis for functions, classes, decorators, comprehensions, and Python-specific complexity patterns
- **JavaScript** — Tree-sitter parsing with ES modules, React/JSX patterns, and async workflow analysis
- **TypeScript** — Tree-sitter parsing with TS/TSX support, type-aware patterns, and modern frontend/server syntax

```bash
# Analyze specific languages
debtmap analyze . --languages rust,python,javascript,typescript

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
- Copy context to clipboard for AI assistants when needed
- Jump to code in your editor

## LLM Integration

Debtmap works well on its own, and the `--format markdown` output is available when you want to feed findings into AI coding assistants. It provides:

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
# Markdown (good for AI workflows)
debtmap analyze . --format markdown

# JSON for programmatic access
debtmap analyze . --format json

# Terminal/TUI for direct exploration
debtmap analyze .
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
# Rust coverage
cargo llvm-cov --lcov --output-path coverage.lcov

# Analyze with coverage integration
debtmap analyze . --lcov coverage.lcov
```

```bash
# Python coverage
pytest --cov=. --cov-report=lcov:coverage.lcov

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

**Current focus:** Rust, Python, and JS/TS analysis quality + AI workflow integration

- [x] Cognitive + cyclomatic complexity
- [x] Test coverage correlation
- [x] Pattern-based false positive reduction
- [x] LLM-optimized output format
- [x] Context suggestions for AI
- [x] Rust language support
- [x] Python language support
- [x] JavaScript language support
- [x] TypeScript language support
- [ ] Streaming output for large codebases
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
