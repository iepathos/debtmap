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
- **Rust projects**: `cargo-tarpaulin` for coverage data
- **JavaScript/TypeScript**: Jest or other tools generating LCOV format
- **Python**: pytest with coverage plugin

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

**Common installation issues:**

- **Binary not in PATH**: Add `~/.cargo/bin` or `~/.local/bin` to your PATH
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"  # Add to ~/.bashrc or ~/.zshrc
  ```
- **Permission issues**: Run the install script with your current user (don't use sudo)
- **Cargo not found**: Install Rust from https://rustup.rs

## Quick Start

Here are the most common commands to get you started:

```bash
# Analyze current directory (simplest command)
debtmap analyze .

# Analyze with coverage data for risk scoring (recommended)
debtmap analyze . --lcov target/coverage/lcov.info

# Generate coverage first (for Rust projects)
cargo tarpaulin --out lcov --output-dir target/coverage
debtmap analyze . --lcov target/coverage/lcov.info

# Analyze with custom thresholds
debtmap analyze ./src --threshold-complexity 15 --threshold-duplication 50

# Output as JSON (for CI/CD integration)
debtmap analyze ./src --format json --output report.json

# Show only top 10 high-priority issues
debtmap analyze . --top 10

# Initialize configuration file for project-specific settings
debtmap init
```

## First Analysis

Let's run your first analysis! Navigate to a project directory and run:

```bash
debtmap analyze .
```

**What happens during analysis:**

1. **File Discovery** - Debtmap scans your project for supported source files (Rust, Python, JavaScript, TypeScript)
2. **Parsing** - Each file is parsed into an Abstract Syntax Tree (AST)
3. **Metrics Calculation** - Complexity, debt patterns, and risk scores are computed
4. **Prioritization** - Results are ranked by priority (CRITICAL, HIGH, MEDIUM, LOW)
5. **Output** - Results are displayed in your chosen format

**Expected timing**: Analyzing a 10,000 LOC project typically takes 2-5 seconds. The first run may be slightly slower as Debtmap builds its cache.

**Language support**:
- **Rust**: Full support with advanced features (trait detection, purity analysis, call graphs)
- **Python**: Partial support (complexity metrics, basic debt detection)
- **JavaScript/TypeScript**: Partial support (complexity metrics, basic debt detection)

### Example Output

When you run `debtmap analyze .`, you'll see output like this:

```
════════════════════════════════════════════
    PRIORITY TECHNICAL DEBT FIXES
════════════════════════════════════════════

🎯 TOP 3 RECOMMENDATIONS (by unified priority)

#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/analyzers/rust_call_graph.rs:38 add_function_to_graph()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: Full test coverage, -3.7 risk
├─ COMPLEXITY: cyclomatic=6, branches=6, cognitive=8, nesting=2, lines=32
├─ DEPENDENCIES: 0 upstream, 11 downstream
└─ WHY: Business logic with 0% coverage, manageable complexity (cyclo=6, cog=8)

#2 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/debt/smells.rs:196 detect_data_clumps()
├─ ACTION: Add 5 unit tests for full coverage
├─ IMPACT: Full test coverage, -3.7 risk
├─ COMPLEXITY: cyclomatic=5, branches=5, cognitive=11, nesting=5, lines=31
├─ DEPENDENCIES: 0 upstream, 4 downstream
└─ WHY: Business logic with 0% coverage, manageable complexity (cyclo=5, cog=11)

#3 SCORE: 8.6 [CRITICAL]
├─ TEST GAP: ./src/risk/context/dependency.rs:247 explain()
├─ ACTION: Add 5 unit tests for full coverage
├─ IMPACT: Full test coverage, -3.6 risk
├─ COMPLEXITY: cyclomatic=5, branches=5, cognitive=9, nesting=1, lines=24
├─ DEPENDENCIES: 0 upstream, 1 downstream
└─ WHY: Business logic with 0% coverage, manageable complexity (cyclo=5, cog=9)


📊 TOTAL DEBT SCORE: 4907
📈 OVERALL COVERAGE: 67.12%
```

## Understanding the Output

Let's break down what this output means:

### Priority Levels

- **CRITICAL** (9.0-10.0): Immediate action required - high complexity with no test coverage
- **HIGH** (7.0-8.9): Should be addressed soon - moderate-high complexity with poor coverage
- **MEDIUM** (5.0-6.9): Plan for next sprint - moderate complexity or partial coverage gaps
- **LOW** (3.0-4.9): Nice to have - well-tested or simple functions

### Key Metrics

- **Unified Score** (0-10 scale): Overall priority combining complexity, coverage, and dependencies
  - Higher score = higher priority
  - Takes into account multiple risk factors

- **Debt Type**: Category of the issue
  - `TestGap`: Missing test coverage
  - `Complexity`: Exceeds complexity thresholds
  - `Duplication`: Repeated code blocks
  - `CodeSmell`: Anti-patterns and bad practices

- **Complexity Metrics**:
  - **Cyclomatic**: Number of decision points (branches, loops)
  - **Cognitive**: How difficult the code is to understand
  - **Nesting**: Maximum indentation depth
  - **Lines**: Function length

- **Dependencies**:
  - **Upstream callers**: Functions that call this function
  - **Downstream callees**: Functions this function calls
  - More dependencies = higher impact when this code breaks

### Recommendation Structure

Each recommendation shows:

- **ACTION**: What you should do (e.g., "Add 6 unit tests")
- **IMPACT**: Expected improvement (e.g., "Full test coverage, -3.7 risk")
- **WHY**: The reasoning behind this recommendation

### Summary Statistics

- **Total Debt Score**: Sum of all debt scores across your codebase
  - Lower is better
  - Track over time to measure improvement

- **Overall Coverage**: Percentage of code covered by tests
  - Only shown when coverage data is provided

### Output Formats

Debtmap supports multiple output formats:

- **Terminal** (default): Human-readable colored output with tables
- **JSON**: Machine-readable format for CI/CD integration
- **Markdown**: Documentation-friendly format for reports

Example JSON output:
```bash
debtmap analyze . --format json --output report.json
```

Example Markdown output:
```bash
debtmap analyze . --format markdown --output report.md
```

## What's Next?

Now that you've run your first analysis, explore these topics:

- **[Analysis Guide](./analysis-guide.md)** - Deep dive into complexity metrics, debt patterns, and risk scoring
- **[Output Formats](./output-formats.md)** - Detailed guide to JSON schema and integration options
- **Configuration** - Customize thresholds and filters with `.debtmap.toml`
- **CI/CD Integration** - Use the `validate` command to enforce quality gates

### Generate a Configuration File

Create a project-specific configuration:

```bash
debtmap init
```

This creates a `.debtmap.toml` file with sensible defaults that you can customize for your project.

### Try Analysis with Coverage

For more accurate risk assessment, run analysis with coverage data:

```bash
# For Rust projects
cargo tarpaulin --out lcov --output-dir target/coverage
debtmap analyze . --lcov target/coverage/lcov.info

# For Python projects
pytest --cov --cov-report=lcov
debtmap analyze . --lcov coverage.lcov

# For JavaScript/TypeScript projects
jest --coverage --coverageReporters=lcov
debtmap analyze . --lcov coverage/lcov.info
```

Coverage data helps Debtmap identify **truly risky code** - functions that are both complex AND untested.

---

**Need help?** Report issues at https://github.com/iepathos/debtmap/issues
