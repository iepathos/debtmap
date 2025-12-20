# Getting Started

This guide will help you install Debtmap and run your first analysis in just a few minutes.

## Prerequisites

Before installing Debtmap, you'll need:

- **For pre-built binaries**: No prerequisites! The install script handles everything.
- **For cargo install or building from source**:
  - Rust toolchain (rustc and cargo)
  - Supported platforms: Linux, macOS, Windows
  - Rust edition 2021 or later

**Optional** (for coverage-based risk analysis):
- **Rust projects**: `cargo-tarpaulin` or `cargo-llvm-cov` for coverage data

## Installation

### Quick Install (Recommended)

Install the latest release with a single command:

```bash
curl -sSL https://raw.githubusercontent.com/iepathos/debtmap/master/install.sh | bash
```

Or with wget:
```bash
wget -qO- https://raw.githubusercontent.com/iepathos/debtmap/master/install.sh | bash
```

This will:
- Automatically detect your OS and architecture
- Download the appropriate pre-built binary from the latest GitHub release
- Install debtmap to `~/.cargo/bin` if it exists, otherwise `~/.local/bin`
- Offer to automatically add the install directory to your PATH if needed

### Using Cargo

If you have Rust installed:

```bash
cargo install debtmap
```

### From Source

For the latest development version:

```bash
# Clone the repository
git clone https://github.com/iepathos/debtmap.git
cd debtmap

# Build and install
cargo install --path .
```

### Verify Installation

After installation, verify Debtmap is working:

```bash
# Check version
debtmap --version

# See available commands
debtmap --help
```

## Quick Start

Here are the most common commands to get you started:

```bash
# Basic analysis (simplest command)
debtmap analyze .

# LLM-optimized output for AI workflows (recommended)
debtmap analyze . --format llm-markdown

# Pipe directly to Claude Code
debtmap analyze . --format llm-markdown --top 3 | claude "Fix the top item"

# JSON output for programmatic access
debtmap analyze . --format json --top 10 > debt.json

# With coverage data for accurate risk assessment
cargo llvm-cov --lcov --output-path coverage.lcov
debtmap analyze . --lcov coverage.lcov

# Show only critical/high priority items
debtmap analyze . --min-priority high --top 10

# Terminal output for human exploration
debtmap analyze . --format terminal
```

## First Analysis

Let's run your first analysis! Navigate to a project directory and run:

```bash
debtmap analyze .
```

**What happens during analysis:**

1. **File Discovery** - Debtmap scans your project for Rust source files (`.rs`)
2. **Parsing** - Each file is parsed into an Abstract Syntax Tree (AST)
3. **Metric Extraction** - Complexity, coverage gaps, and coupling are measured
4. **Prioritization** - Results are ranked by severity (CRITICAL, HIGH, MEDIUM, LOW)
5. **Context Generation** - File ranges are suggested for each debt item
6. **Output** - Results are displayed in your chosen format

**Expected timing**: Analyzing a 10,000 LOC project typically takes 2-5 seconds.

## Example Output

When you run `debtmap analyze . --format llm-markdown`, you'll see output like this:

```markdown
# Technical Debt Analysis

## Summary
- Total items: 47
- Critical: 3, High: 12, Moderate: 20, Low: 12

## #1 [CRITICAL] parse_complex_input
**Location:** src/parser.rs:38-85
**Score:** 8.9/10

**Signals:**
| Metric | Value |
|--------|-------|
| Cyclomatic | 12 |
| Cognitive | 18 |
| Coverage | 0% |
| Nesting | 4 |

**Context:**
- Primary: src/parser.rs:38-85
- Caller: src/handler.rs:100-120
- Test: tests/parser_test.rs:50-75
```

## Understanding the Output

### Priority Tiers

| Tier | Score | Meaning |
|------|-------|---------|
| CRITICAL | 8.0-10.0 | High complexity with no test coverage |
| HIGH | 5.0-7.9 | Moderate complexity with coverage gaps |
| MODERATE | 2.0-4.9 | Lower risk, monitor |
| LOW | 0.0-1.9 | Acceptable state |

### Key Signals

**Complexity signals:**
- **Cyclomatic**: Decision points (if, match, loop)
- **Cognitive**: How hard code is to understand
- **Nesting**: Indentation depth
- **Lines**: Function length

**Coverage signals:**
- **Line coverage**: % of lines executed by tests
- **Branch coverage**: % of branches taken

**Coupling signals:**
- **Fan-in**: Functions that call this function
- **Fan-out**: Functions this function calls

### Context Suggestions

Each debt item includes file ranges the AI should read:

```
Context:
├─ Primary: src/parser.rs:38-85 (the debt item)
├─ Caller: src/handler.rs:100-120 (usage context)
└─ Test: tests/parser_test.rs:50-75 (expected behavior)
```

These suggestions help AI assistants understand the code before making changes.

## Output Formats

### LLM Markdown (for AI workflows)

```bash
debtmap analyze . --format llm-markdown
```

Optimized for minimal token usage while providing all necessary context.

### JSON (for programmatic access)

```bash
debtmap analyze . --format json --output debt.json
```

Structured data for CI/CD integration and custom tooling.

### Terminal (for human exploration)

```bash
debtmap analyze . --format terminal
```

Color-coded, interactive output for manual review.

## Adding Coverage Data

Coverage data enables accurate risk assessment:

```bash
# Generate coverage with cargo-llvm-cov
cargo llvm-cov --lcov --output-path coverage.lcov

# Or with cargo-tarpaulin
cargo tarpaulin --out lcov --output-dir target/coverage

# Analyze with coverage
debtmap analyze . --lcov coverage.lcov
```

With coverage data:
- Complex code with good tests = lower priority
- Simple code with no tests = higher priority
- Untested error paths are identified

## AI Workflow Examples

### Claude Code

```bash
# Direct piping
debtmap analyze . --format llm-markdown --top 3 | claude "Fix the top item"

# With coverage
cargo llvm-cov --lcov --output-path coverage.lcov
debtmap analyze . --format llm-markdown --lcov coverage.lcov --top 1 | \
  claude "Add tests for this function"
```

### Cursor

```bash
# Generate report for Cursor to reference
debtmap analyze . --format llm-markdown --top 10 > debt-report.md

# In Cursor: @debt-report.md Fix the top critical item
```

### Custom Pipelines

```bash
# Get JSON for programmatic processing
debtmap analyze . --format json --top 5 | \
  jq '.items[0].context.primary' | \
  xargs -I {} echo "Read {}"
```

## Configuration

Create a project-specific configuration:

```bash
debtmap init
```

This creates `.debtmap.toml`:

```toml
[thresholds]
complexity = 10
duplication = 40

[tiers]
critical = 9.0
high = 7.0
medium = 5.0

[ignore]
patterns = ["**/target/**", "**/tests/**"]
```

## CLI Options Reference

### Analysis Options

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: terminal, json, markdown, llm-markdown |
| `--output <FILE>` | Write to file instead of stdout |
| `--lcov <FILE>` | LCOV coverage file for risk analysis |
| `--top <N>` | Show only top N priority items |
| `--min-priority <TIER>` | Filter by minimum priority (low, medium, high, critical) |
| `--min-score <N>` | Filter items below score N |

### Verbosity Options

| Option | Description |
|--------|-------------|
| `-v` | Show main score factors |
| `-vv` | Show detailed calculations |
| `-vvv` | Show all debug information |
| `--quiet` | Suppress progress output |

### Performance Options

| Option | Description |
|--------|-------------|
| `--jobs <N>` | Number of threads (0 = all cores) |
| `--no-parallel` | Disable parallel processing |
| `--max-files <N>` | Limit analysis to N files |

## Troubleshooting

### Installation Issues

- **Binary not in PATH**: Add `~/.cargo/bin` or `~/.local/bin` to your PATH
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"  # Add to ~/.bashrc or ~/.zshrc
  ```
- **Permission issues**: Run the install script with your current user (don't use sudo)
- **Cargo not found**: Install Rust from https://rustup.rs

### Analysis Issues

- **Empty output**: Check that your project contains Rust source files (`.rs`)
- **Parser failures**: Run with `-vv` for debug output
- **Performance issues**: Limit parallel jobs with `--jobs 4`

### Coverage Issues

- **Coverage not applied**: Verify LCOV file path is correct
- **Low coverage detected**: Ensure tests actually run during coverage generation

## What's Next?

Now that you've run your first analysis:

- **Integrate with AI**: See [LLM Integration](llm-integration.md) for AI workflow patterns
- **Understand metrics**: See [Metrics Reference](metrics-reference.md) for signal definitions
- **Configure thresholds**: See [Configuration](configuration.md) for customization
- **CI/CD integration**: See [Prodigy Integration](prodigy-integration.md) for automation

---

**Need help?** Report issues at https://github.com/iepathos/debtmap/issues
