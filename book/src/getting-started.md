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

## Troubleshooting

### Installation Issues

- **Binary not in PATH**: Add `~/.cargo/bin` or `~/.local/bin` to your PATH
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"  # Add to ~/.bashrc or ~/.zshrc
  ```
- **Permission issues**: Run the install script with your current user (don't use sudo)
- **Cargo not found**: Install Rust from https://rustup.rs

### First Run Issues

- **Empty output or no items found**: Check that your project contains Rust source files (`.rs`). Verify with `debtmap analyze . -vvv` for debug output.

- **Parser failures**: If analysis fails with parsing errors:
  ```bash
  # Run with verbose output to identify problematic files
  debtmap analyze . -vv
  # Exclude problematic files temporarily
  debtmap init  # Creates .debtmap.toml
  # Edit .debtmap.toml to add ignore patterns
  ```

- **Performance issues**: For large codebases (10,000+ files):
  ```bash
  # Limit parallel jobs to reduce memory usage
  debtmap analyze . --jobs 4
  # Or analyze specific directories
  debtmap analyze ./src
  ```

## Quick Start

Here are the most common commands to get you started:

```bash
# Analyze current directory (simplest command)
debtmap analyze .

# Analyze with coverage data for risk scoring (recommended)
# Note: --lcov is a shorthand alias for --coverage-file
debtmap analyze . --lcov target/coverage/lcov.info

# Generate coverage first (for Rust projects)
cargo tarpaulin --out lcov --output-dir target/coverage
debtmap analyze . --lcov target/coverage/lcov.info

# Analyze with custom thresholds
# Note: threshold-duplication specifies minimum lines of duplicated code to detect
debtmap analyze ./src --threshold-complexity 15 --threshold-duplication 50

# Output as JSON (for CI/CD integration)
debtmap analyze ./src --format json --output report.json

# Show only top 10 high-priority issues
debtmap analyze . --top 10

# Initialize configuration file for project-specific settings
debtmap init

# Validate against thresholds (CI/CD integration)
debtmap validate ./src --max-debt-density 5.0

# Validate using config file settings
# Note: CLI flags override config file values when both are specified
debtmap validate . --config .debtmap.toml --max-debt-density 5.0

# Compare before/after to track improvements
debtmap analyze . --format json --output before.json
# ... make improvements ...
debtmap analyze . --format json --output after.json
debtmap compare --before before.json --after after.json

# Advanced comparison: focus on specific function
debtmap compare --before before.json --after after.json --target-location src/main.rs:main:10

# Extract target from implementation plan
debtmap compare --before before.json --after after.json --plan IMPLEMENTATION_PLAN.md
```

### Advanced Options

Debtmap provides many powerful options to customize your analysis:

**Verbosity Levels:**
```bash
# Show main factors contributing to scores
debtmap analyze . -v

# Show detailed calculations
debtmap analyze . -vv

# Show all debug information
debtmap analyze . -vvv
```

**Filtering and Prioritization:**
```bash
# Only show high-priority items
debtmap analyze . --min-priority high

# Filter by specific categories
debtmap analyze . --filter Architecture,Testing

# Group results by debt category
debtmap analyze . --group-by-category
```

**Performance Control:**
```bash
# Limit parallel jobs
debtmap analyze . --jobs 4

# Disable parallel processing
debtmap analyze . --no-parallel
```

**Output Control:**
```bash
# Plain output (no colors/emoji, for CI/CD)
debtmap analyze . --plain

# Compact summary output
debtmap analyze . --summary

# Control aggregation behavior
debtmap analyze . --aggregate-only          # Show only aggregated results
debtmap analyze . --no-aggregation          # Skip aggregation entirely
debtmap analyze . --aggregation-method sum  # Choose aggregation method

# Adjust detail level in output
debtmap analyze . --detail-level high       # More detailed output
```

**Expert Options:**

These advanced options are available for power users and specialized use cases:

```bash
# Analysis behavior
--semantic-off              # Disable semantic analysis
--no-context-aware          # Disable context-aware analysis
--no-multi-pass             # Disable multi-pass analysis (for performance-critical scenarios)
--validate-loc              # Validate lines of code calculations

# Rust-specific options
--verbose-macro-warnings    # Show detailed macro expansion warnings
--show-macro-stats          # Display macro usage statistics

# Filtering and thresholds
--threshold-preset <name>   # Use predefined threshold preset
--min-problematic <count>   # Minimum problematic items to report
--max-files <count>         # Limit analysis to N files
--no-god-object             # Disable god object detection

# Advanced reporting
--attribution               # Include code attribution information
```

For detailed documentation of these options, run `debtmap analyze --help`.

## First Analysis

Let's run your first analysis! Navigate to a project directory and run:

```bash
debtmap analyze .
```

**What happens during analysis:**

1. **File Discovery** - Debtmap scans your project for Rust source files (`.rs`)
2. **Parsing** - Each file is parsed into an Abstract Syntax Tree (AST)
3. **Metrics Calculation** - Complexity, debt patterns, and risk scores are computed
4. **Prioritization** - Results are ranked by priority (CRITICAL, HIGH, MEDIUM, LOW)
5. **Output** - Results are displayed in your chosen format

**Expected timing**: Analyzing a 10,000 LOC project typically takes 2-5 seconds.

## Language Support

Debtmap is focused exclusively on Rust for comprehensive code analysis:

### Rust (Full Support)
All analysis features available:
- Complexity metrics (cyclomatic, cognitive, nesting, lines)
- Technical debt detection (code smells, anti-patterns)
- Test gap analysis with coverage integration
- Advanced features:
  - Trait detection and analysis
  - Function purity analysis
  - Call graph generation
  - Macro expansion tracking

**Note**: Support for additional languages may be added in future releases based on community feedback and demand.

## Example Output

When you run `debtmap analyze .`, you'll see output similar to this:

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

**Note:** These are default priority thresholds. You can customize them in `.debtmap.toml` under the `[tiers]` section to match your team's standards.

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

### Organizing Results

When analyzing large codebases, you can organize and filter results to focus on specific areas:

**Group by Debt Category:**
```bash
debtmap analyze . --group-by-category
```

This organizes results by type: Architecture, Testing, Performance, CodeQuality

**Filter by Priority:**
```bash
# Show only high and critical priority items
debtmap analyze . --min-priority high

# Combine with --top to limit results
debtmap analyze . --min-priority high --top 10
```

**Filter by Category:**
```bash
# Focus on specific debt types
debtmap analyze . --filter Architecture,Testing

# Available categories: Architecture, Testing, Performance, CodeQuality
```

These filtering options help you focus on specific types of technical debt, making it easier to plan targeted improvements.

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
# JSON output uses unified format with consistent structure
debtmap analyze . --format json --output report.json
```

The JSON format uses a unified structure with a `type` field for all items, providing consistent parsing for CI/CD integrations.

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

**Example Configuration File:**

Here's a typical `.debtmap.toml` with common settings:

```toml
# Debtmap Configuration File
# Generated by: debtmap init

# Analysis thresholds
[thresholds]
complexity = 10        # Flag functions with cyclomatic complexity > 10
duplication = 40       # Minimum lines for duplicate code detection
file_size = 500        # Warn on files exceeding 500 lines

# Priority tier boundaries (for unified priority scores 0-10)
[tiers]
critical = 9.0         # Score >= 9.0 = CRITICAL priority
high = 7.0             # Score >= 7.0 = HIGH priority
medium = 5.0           # Score >= 5.0 = MEDIUM priority
# Anything below 5.0 = LOW priority

# Risk scoring weights (must sum to 1.0)
[weights]
coverage = 0.4         # Weight for test coverage gaps
complexity = 0.35      # Weight for complexity metrics
dependencies = 0.25    # Weight for call graph dependencies

# Files and directories to ignore
[ignore]
patterns = [
    "**/target/**",     # Rust build artifacts
    "**/tests/**",      # Test directories (optional)
]

# God object detection thresholds
[god_object]
methods = 20           # Warn on classes/modules with > 20 methods
lines = 500            # Warn on classes/modules with > 500 lines

# Entropy-based complexity detection
[entropy]
enabled = true         # Enable entropy analysis
threshold = 0.7        # Flag functions with entropy > 0.7
```

**Key Configuration Options:**

The configuration file allows you to customize:
- **Threshold customization** - Adjust complexity, duplication, and file size thresholds
- **Scoring weights** - Fine-tune how coverage, complexity, and dependencies are weighted
- **Ignore patterns** - Exclude test files or generated code from analysis
- **God object thresholds** - Configure what constitutes a "god object" anti-pattern
- **Entropy analysis** - Control entropy-based complexity detection
- **Priority tiers** - Customize CRITICAL/HIGH/MEDIUM/LOW threshold ranges

See the Configuration chapter for complete documentation of all available options.

### Try Analysis with Coverage

For more accurate risk assessment, run analysis with coverage data. Coverage helps Debtmap identify **truly risky code** - functions that are both complex AND untested.

#### Generating Coverage Data

**Rust Projects:**
```bash
# Option 1: Using cargo-tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin --out lcov --output-dir target/coverage
debtmap analyze . --lcov target/coverage/lcov.info

# Option 2: Using cargo-llvm-cov (recommended for faster builds)
cargo install cargo-llvm-cov
cargo llvm-cov --lcov --output-path target/coverage/lcov.info
debtmap analyze . --lcov target/coverage/lcov.info
```

**Note**: The `--lcov` flag is a shorthand alias for `--coverage-file`. Both accept LCOV format coverage reports.

---

**Need help?** Report issues at https://github.com/iepathos/debtmap/issues
