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

**Cache Management:**
```bash
# Skip cache for fresh analysis
debtmap analyze . --no-cache

# Clear cache and rebuild
debtmap analyze . --clear-cache

# View cache statistics
debtmap analyze . --cache-stats

# Specify custom cache location
debtmap analyze . --cache-location /custom/path

# Migrate cache from local to shared location
debtmap analyze . --migrate-cache
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
--multi-pass                # Enable multi-pass analysis for deeper insights
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

1. **File Discovery** - Debtmap scans your project for supported source files (Rust, Python, JavaScript, TypeScript)
2. **Parsing** - Each file is parsed into an Abstract Syntax Tree (AST)
3. **Metrics Calculation** - Complexity, debt patterns, and risk scores are computed
4. **Prioritization** - Results are ranked by priority (CRITICAL, HIGH, MEDIUM, LOW)
5. **Output** - Results are displayed in your chosen format

**Expected timing**: Analyzing a 10,000 LOC project typically takes 2-5 seconds. The first run may be slightly slower as Debtmap builds its cache.

**About Caching:**
Debtmap caches parsed ASTs and computed metrics to speed up subsequent analyses:
- **Cache location**: `XDG_CACHE_HOME/debtmap` on Linux, `~/Library/Caches/debtmap` on macOS, `%LOCALAPPDATA%/debtmap` on Windows
- **What's cached**: Parsed ASTs and computed metrics for each file
- **Invalidation**: Cache is automatically invalidated when files are modified

### Example Output

When you run `debtmap analyze .`, you'll see output like this:

```
════════════════════════════════════════════
    PRIORITY TECHNICAL DEBT FIXES
════════════════════════════════════════════

📊 ANALYSIS SUMMARY
   Files analyzed: 47
   Total functions: 234
   Debt items found: 12
   Analysis time: 3.2s

🎯 TOP 5 RECOMMENDATIONS

#1 SCORE: 8.9 [CRITICAL]
├─ TEST GAP: ./src/parser.rs:38 parse_input()
├─ ACTION: Add 6 unit tests for full coverage
├─ IMPACT: -3.7 risk reduction
├─ WHY: Complex logic (cyclo=12) with 0% coverage
└─ DETAILS: Function has 12 decision points with no test coverage.
            Untested error paths could cause production failures.

#2 SCORE: 7.2 [HIGH]
├─ TEST GAP: ./src/analyzer/complexity.rs:145 calculate_entropy()
├─ ACTION: Add 4 unit tests for edge cases
├─ IMPACT: -2.8 risk reduction
├─ WHY: Medium complexity (cyclo=8) with 35% coverage
└─ DETAILS: Missing tests for error conditions and boundary cases.

#3 SCORE: 5.8 [HIGH]
├─ COMPLEXITY: ./src/io/formatter.rs:89 format_output()
├─ ACTION: Extract 3 functions from 16 branches
├─ IMPACT: -2.1 risk reduction
├─ WHY: High complexity (cyclo=16, entropy=0.75)
└─ DETAILS: Function handles multiple output formats with nested conditionals.
            Consider extracting format-specific logic.

════════════════════════════════════════════
```

With coverage integration (`--lcov`), you'll see more detailed test gap analysis:

```bash
debtmap analyze . --lcov target/coverage/lcov.info --top 3
```

```
════════════════════════════════════════════
    PRIORITY TECHNICAL DEBT FIXES
    (Coverage-Integrated Analysis)
════════════════════════════════════════════

📊 ANALYSIS SUMMARY
   Files analyzed: 47
   Coverage: 68.5% lines, 54.2% branches
   Critical gaps: 3
   High-priority gaps: 7

🎯 TOP 3 CRITICAL GAPS

#1 SCORE: 9.1 [CRITICAL]
├─ TEST GAP: ./src/core/state_machine.rs:67 transition_state()
├─ COVERAGE: 0% (0/8 branches covered)
├─ COMPLEXITY: 15 (high entropy: 0.82)
├─ ACTION: Add 8 unit tests covering all state transitions
├─ IMPACT: -4.5 risk reduction
├─ WHY: Core business logic with complex branching, completely untested
└─ CALL GRAPH: Called from 12 different code paths
                5 of those callers are also untested

#2 SCORE: 8.4 [CRITICAL]
├─ TEST GAP: ./src/parser.rs:38 parse_input()
├─ COVERAGE: 25% (2/8 branches covered)
├─ COMPLEXITY: 12 (medium entropy: 0.61)
├─ ACTION: Add 6 tests for untested error paths
├─ IMPACT: -3.2 risk reduction
└─ MISSING BRANCHES:
    • Line 45: Error path for invalid syntax
    • Line 52: Error path for unexpected EOF
    • Line 58: Error path for malformed input
    • Line 63: Edge case: empty input
    • Line 68: Edge case: very large input
    • Line 72: Edge case: special characters

════════════════════════════════════════════
```

**Reading the output:**
- **Score**: Risk score from 0-10 (higher = more urgent)
- **Tier**: CRITICAL (8.0+), HIGH (5.0-7.9), MODERATE (2.0-4.9), LOW (<2.0)
- **Location**: File path, line number, and function name
- **Action**: Specific steps to address the issue
- **Impact**: Estimated risk reduction from taking action
- **Why**: Rationale explaining why this is flagged
- **Management**: Use `--clear-cache` to clear, `--no-cache` to skip, or `--cache-stats` to view statistics

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
# By default, JSON uses legacy format
debtmap analyze . --format json --output report.json

# For the new unified format (with consistent structure and type field):
debtmap analyze . --format json --output-format unified --output report.json
```

**JSON Format Options:**
- **legacy** (default): Uses `{File: {...}}` and `{Function: {...}}` wrappers for backward compatibility with existing tools
- **unified**: New format (spec 108) with consistent structure and `type` field for all items

Recommendation: Use `unified` for new integrations, `legacy` only for compatibility with existing tooling.

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

**Key Configuration Options:**

The configuration file allows you to customize:
- **Threshold customization** - Adjust complexity, duplication, and file size thresholds
- **Scoring weights** - Fine-tune how coverage, complexity, and dependencies are weighted
- **Language selection** - Enable/disable specific language analyzers
- **Ignore patterns** - Exclude test files or generated code from analysis
- **God object thresholds** - Configure what constitutes a "god object" anti-pattern
- **Entropy analysis** - Control entropy-based complexity detection
- **Priority tiers** - Customize CRITICAL/HIGH/MEDIUM/LOW threshold ranges

See the Configuration chapter for complete documentation of all available options.

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
