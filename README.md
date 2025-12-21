# debtmap

[![Crates.io](https://img.shields.io/crates/v/debtmap)](https://crates.io/crates/debtmap)
[![Downloads](https://img.shields.io/crates/d/debtmap)](https://crates.io/crates/debtmap)
[![CI](https://github.com/iepathos/debtmap/actions/workflows/ci.yml/badge.svg)](https://github.com/iepathos/debtmap/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Code complexity sensor for AI-assisted development.**

Debtmap identifies technical debt hotspots and provides the structured data AI coding tools need to understand and fix them. It doesn't tell you what to do - it tells AI agents where to look and what signals matter.

## Why Debtmap?

AI coding assistants (Claude Code, Copilot, Cursor) are transforming how we write code. But they struggle with technical debt:

- They can't see the whole codebase at once
- They don't know which complex code is tested vs untested
- They can't prioritize what to fix first
- They waste context window on irrelevant code

Debtmap solves this by providing:

1. **Prioritized debt items** - What needs attention, ranked by severity
2. **Quantified signals** - Complexity, coverage, coupling metrics
3. **Context suggestions** - Exactly which files/lines the AI should read
4. **Structured output** - JSON and markdown optimized for LLM consumption

## Quick Start

```bash
# Install
cargo install debtmap

# Analyze and pipe to Claude Code
debtmap analyze . --format llm-markdown | claude "Fix the top debt item"

# Get structured data for your AI workflow
debtmap analyze . --format json --top 10 > debt.json

# Interactive exploration
debtmap analyze . --format terminal
```

## How It Works

Debtmap is a **sensor**, not an oracle. It measures:

| Signal | What It Measures | Why It Matters |
|--------|------------------|----------------|
| Complexity | Cyclomatic, cognitive, nesting | How hard code is to understand |
| Coverage | Test coverage gaps | How risky changes are |
| Coupling | Dependencies, call graph | How changes ripple |
| Entropy | Pattern variety | False positive reduction |
| Purity | Side effects | How testable code is |

These signals are combined into a **severity score** that ranks debt items. The AI uses these signals + the actual code to decide how to fix it.

## Example Output

```
#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/parser.rs:38 parse_complex_input()
├─ COMPLEXITY: cyclomatic=12, cognitive=18, nesting=4
├─ COVERAGE: 0% (12 lines untested)
├─ CONTEXT:
│  ├─ Primary: src/parser.rs:38-85
│  ├─ Caller: src/handler.rs:100-120
│  └─ Tests: tests/parser_test.rs:50-75
└─ WHY: High complexity function with zero test coverage
```

## For AI Tool Developers

Debtmap output is designed for machine consumption:

- **Context suggestions** - File ranges the AI should read
- **Deterministic output** - Same input = same output
- **Rich metadata** - All scoring factors exposed
- **Stable IDs** - Reference items across runs
- **LLM-optimized format** - Markdown structured for minimal tokens

See [LLM Integration Guide](https://iepathos.github.io/debtmap/llm-integration.html) for details.

## Output Formats

```bash
# LLM-optimized markdown (recommended for AI)
debtmap analyze . --format llm-markdown

# JSON for programmatic access
debtmap analyze . --format json

# Terminal for human exploration
debtmap analyze . --format terminal

# Standard markdown for reports
debtmap analyze . --format markdown
```

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
- [ ] Streaming output for large codebases
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
