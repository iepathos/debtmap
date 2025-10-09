# CLI Reference

Complete reference for Debtmap command-line interface.

## Quick Start

```bash
# Basic analysis
debtmap analyze src/

# With coverage integration
debtmap analyze src/ --coverage-file coverage.lcov

# Generate JSON report
debtmap analyze . --format json --output report.json

# Show top 10 priority items only
debtmap analyze . --top 10 --min-priority high

# Initialize configuration and validate
debtmap init
debtmap validate . --config debtmap.toml
```

## Commands

Debtmap provides four main commands:

### `analyze`

Analyze code for complexity and technical debt.

**Usage:**
```bash
debtmap analyze <PATH> [OPTIONS]
```

**Arguments:**
- `<PATH>` - Path to analyze (file or directory)

**Description:**
Primary command for code analysis. Supports multiple output formats (json, markdown, terminal), coverage file integration, caching, parallel processing, context-aware risk analysis, and comprehensive filtering options.

See [Options](#options) section below for all available flags.

### `init`

Initialize a Debtmap configuration file.

**Usage:**
```bash
debtmap init [OPTIONS]
```

**Options:**
- `-f, --force` - Force overwrite existing config

**Description:**
Creates a `debtmap.toml` configuration file in the current directory with default settings. Use `--force` to overwrite an existing configuration file.

### `validate`

Validate code against thresholds defined in configuration file.

**Usage:**
```bash
debtmap validate <PATH> [OPTIONS]
```

**Arguments:**
- `<PATH>` - Path to analyze

**Options:**
- `-c, --config <CONFIG>` - Configuration file path
- `--max-debt-density <N>` - Maximum debt density allowed (per 1000 LOC)
- All analysis, coverage, context, display, and verbosity options from `analyze` command

**Description:**
Similar to `analyze` but enforces thresholds defined in configuration file. Returns non-zero exit code if thresholds are exceeded, making it suitable for CI/CD integration.

**Exit Codes:**
- `0` - Success (no errors, all thresholds passed)
- Non-zero - Failure (errors occurred or thresholds exceeded)

### `compare`

Compare two analysis results and generate a diff report.

**Usage:**
```bash
debtmap compare --before <FILE> --after <FILE> [OPTIONS]
```

**Required Options:**
- `--before <FILE>` - Path to "before" analysis JSON
- `--after <FILE>` - Path to "after" analysis JSON

**Optional Target Location:**
- `--plan <FILE>` - Path to implementation plan (to extract target location)
- `--target-location <LOCATION>` - Target location in format `file:function:line`
  - Conflicts with `--plan` (cannot use both)

**Output Options:**
- `-f, --format <FORMAT>` - Output format: json, markdown, terminal (default: json)
- `-o, --output <OUTPUT>` - Output file (defaults to stdout)

**Description:**
Compares two analysis results and generates a diff showing improvements or regressions in code quality metrics.

## Options

Options are organized by category for clarity. Most options apply to the `analyze` command, with a subset available for `validate`.

### Output Control

Control how analysis results are formatted and displayed.

**Format Options:**
- `-f, --format <FORMAT>` - Output format: json, markdown, terminal (default: terminal for analyze)
- `--output-format <JSON_FORMAT>` - JSON structure format: legacy or unified (default: legacy)
  - `legacy` - Current format with `{File: {...}}` and `{Function: {...}}` wrappers
  - `unified` - New format with consistent structure and 'type' field
- `-o, --output <OUTPUT>` - Output file path (defaults to stdout)
- `--plain` - Plain output mode: ASCII-only, no colors, no emoji, machine-parseable

**Display Filtering:**
- `--top <N>` / `--head <N>` - Show only top N priority items
- `--tail <N>` - Show only bottom N priority items (lowest priority)
- `-s, --summary` - Use summary format with tiered priority display (compact output)
- `--min-priority <PRIORITY>` - Minimum priority to display: low, medium, high, critical
- `--filter <CATEGORIES>` - Filter by debt categories (comma-separated)
- `--aggregate-only` - Show only aggregated file-level scores
- `--group-by-category` - Group output by debt category

### Analysis Control

Configure analysis behavior, thresholds, and language selection.

**Thresholds:**
- `--threshold-complexity <N>` - Complexity threshold (default: 10)
- `--threshold-duplication <N>` - Duplication threshold in lines (default: 50)
- `--threshold-preset <PRESET>` - Complexity threshold preset: strict, balanced, lenient
  - `strict` - Strict thresholds for high code quality standards
  - `balanced` - Balanced thresholds for typical projects (default)
  - `lenient` - Lenient thresholds for legacy or complex domains
- `--max-debt-density <N>` - Maximum debt density allowed per 1000 LOC (validate command)

**Language Selection:**
- `--languages <LANGS>` - Comma-separated list of languages to analyze
  - Example: `--languages rust,python,javascript`
  - Supported: rust, python, javascript, typescript

**Analysis Modes:**
- `--semantic-off` - Disable semantic analysis (fallback mode)
- `--no-context-aware` - Disable context-aware false positive reduction (enabled by default)
- `--multi-pass` - Enable multi-pass analysis with attribution
- `--attribution` - Show complexity attribution details

### Context & Coverage

Enable context-aware risk analysis and integrate test coverage data.

**Context-Aware Risk Analysis:**
- `--context` / `--enable-context` - Enable context-aware risk analysis
- `--context-providers <PROVIDERS>` - Context providers to use (comma-separated)
  - Available: `critical_path`, `dependency`, `git_history`
  - Example: `--context-providers critical_path,git_history`
- `--disable-context <PROVIDERS>` - Disable specific context providers (comma-separated)

**Coverage Integration:**
- `--coverage-file <PATH>` / `--lcov <PATH>` - LCOV coverage file for risk analysis
  - Coverage data dampens debt scores for well-tested code (multiplier = 1.0 - coverage)
  - Surfaces untested complex functions as higher priority
  - Total debt score with coverage ≤ score without coverage
- `--validate-loc` - Validate LOC consistency across analysis modes (with/without coverage)

### Performance & Caching

Optimize analysis performance through parallelization and caching.

**Parallel Processing:**
- `--no-parallel` - Disable parallel call graph construction (enabled by default)
- `-j, --jobs <N>` - Number of threads for parallel processing
  - `0` = use all available CPU cores (default)
  - Specify number to limit thread count

**Caching:**
- `--no-cache` - Disable caching for this run (caching is enabled by default)
- `--clear-cache` - Clear cache before running analysis
- `--force-cache-rebuild` - Force cache rebuild (same as --clear-cache)
- `--cache-stats` - Show cache statistics and location
- `--migrate-cache` - Migrate cache from local to shared location
- `--cache-location <LOCATION>` - Cache location strategy: local, shared, or path
  - Can also be set via `DEBTMAP_CACHE_DIR` environment variable
  - Affects where analysis results are cached for faster subsequent runs

**Other Performance:**
- `--max-files <N>` - Maximum number of files to analyze (0 = no limit)

### Debugging & Verbosity

Control diagnostic output and debugging information.

**Verbosity Levels:**
- `-v, --verbose` - Increase verbosity level (can be repeated: -v, -vv, -vvv)
  - `-v` - Show main score factors
  - `-vv` - Show detailed calculations
  - `-vvv` - Show all debug information

**Specialized Debugging:**
- `--verbose-macro-warnings` - Show verbose macro parsing warnings (Rust analysis)
- `--show-macro-stats` - Show macro expansion statistics at end of analysis
- `--detail-level <LEVEL>` - Detail level for diagnostic reports
  - Options: summary, standard, comprehensive, debug (default: standard)

### Aggregation

Control file-level aggregation and god object detection.

**File Aggregation:**
- `--aggregate-only` - Show only aggregated file-level scores
- `--no-aggregation` - Disable file-level aggregation
- `--aggregation-method <METHOD>` - File aggregation method (default: weighted_sum)
  - Options: sum, weighted_sum, logarithmic_sum, max_plus_average
- `--min-problematic <N>` - Minimum number of problematic functions for file aggregation
- `--no-god-object` - Disable god object detection

### Option Aliases

Common option shortcuts and aliases for convenience:

- `--lcov` is alias for `--coverage-file`
- `--enable-context` is alias for `--context`
- `--head` is alias for `--top`
- `-s` is short form for `--summary`
- `-v` is short form for `--verbose`
- `-f` is short form for `--format`
- `-o` is short form for `--output`
- `-c` is short form for `--config`
- `-j` is short form for `--jobs`

### Deprecated Options

The following options are deprecated and should be migrated:

- `--use-cache` (hidden) - **Deprecated:** caching is now enabled by default
  - **Migration:** Remove this flag, use `--no-cache` to disable if needed
- `--explain-score` (hidden) - **Deprecated:** use `-v` instead
  - **Migration:** Use `-v`, `-vv`, or `-vvv` for increasing verbosity levels

## Configuration

### Configuration File

Created via `debtmap init` command. The configuration file (`debtmap.toml`) is used by the `validate` command for threshold enforcement and default settings.

**Creating Configuration:**
```bash
# Create new config
debtmap init

# Overwrite existing config
debtmap init --force
```

### Environment Variables

- `DEBTMAP_CACHE_DIR` - Override default cache directory location
  - Can also be set via `--cache-location` flag
  - Affects where analysis results are cached for faster subsequent runs

### Getting Help

Get help for any command:
```bash
# General help
debtmap --help

# Command-specific help
debtmap analyze --help
debtmap validate --help
debtmap compare --help
debtmap init --help
```

## Common Workflows

### Basic Analysis

Analyze a project and view results in terminal:
```bash
debtmap analyze src/
```

Generate JSON report for further processing:
```bash
debtmap analyze . --format json --output report.json
```

Generate Markdown report:
```bash
debtmap analyze . --format markdown --output report.md
```

### Coverage-Integrated Analysis

Analyze with test coverage to surface untested complex code:
```bash
# Generate coverage file first (example for Rust)
cargo tarpaulin --out lcov

# Run analysis with coverage
debtmap analyze src/ --coverage-file lcov.info
```

Coverage dampens debt scores for well-tested code, making untested complex functions more visible.

### Context-Aware Analysis

Enable context providers for risk-aware prioritization:
```bash
# Use all context providers
debtmap analyze . --context

# Use specific context providers
debtmap analyze . --context --context-providers critical_path,git_history
```

Context-aware analysis reduces false positives and prioritizes code based on:
- Critical execution paths
- Dependency relationships
- Git history (change frequency)

### Filtered & Focused Analysis

Show only top priority items:
```bash
debtmap analyze . --top 10 --min-priority high
```

Filter by specific debt categories:
```bash
debtmap analyze . --filter complexity,duplication
```

Use summary mode for compact output:
```bash
debtmap analyze . --summary
```

Show only file-level aggregations:
```bash
debtmap analyze . --aggregate-only
```

### Performance Tuning

Control parallelization:
```bash
# Use 8 threads
debtmap analyze . --jobs 8

# Disable parallel processing
debtmap analyze . --no-parallel
```

Manage caching:
```bash
# Use shared cache location
debtmap analyze . --cache-location shared

# Clear cache and rebuild
debtmap analyze . --clear-cache

# Show cache statistics
debtmap analyze . --cache-stats
```

Limit analysis scope:
```bash
# Analyze maximum 100 files
debtmap analyze . --max-files 100

# Analyze specific languages only
debtmap analyze . --languages rust,python
```

### CI/CD Integration

Use the `validate` command in CI/CD pipelines:
```bash
# Initialize configuration (one time)
debtmap init

# Edit debtmap.toml to set thresholds
# ...

# In CI pipeline: validate against thresholds
debtmap validate . --config debtmap.toml --max-debt-density 50
```

The `validate` command returns non-zero exit code if thresholds are exceeded, failing the build.

### Comparison & Tracking

Compare analysis results before and after changes:
```bash
# Before changes
debtmap analyze . --format json --output before.json

# Make code changes...

# After changes
debtmap analyze . --format json --output after.json

# Generate comparison report
debtmap compare --before before.json --after after.json --format markdown
```

With implementation plan:
```bash
debtmap compare --before before.json --after after.json --plan IMPLEMENTATION_PLAN.md
```

### Debugging Analysis

Increase verbosity to understand scoring:
```bash
# Show main score factors
debtmap analyze src/ -v

# Show detailed calculations
debtmap analyze src/ -vv

# Show all debug information
debtmap analyze src/ -vvv
```

Show macro expansion statistics (Rust):
```bash
debtmap analyze . --show-macro-stats --verbose-macro-warnings
```

Use detailed diagnostic reports:
```bash
debtmap analyze . --detail-level comprehensive
```

## Examples

### Basic Analysis
```bash
# Analyze current directory
debtmap analyze .

# Analyze specific directory
debtmap analyze src/

# Generate JSON output
debtmap analyze . --format json --output report.json
```

### With Coverage
```bash
# Analyze with LCOV coverage file
debtmap analyze src/ --coverage-file coverage.lcov

# Alternative alias
debtmap analyze src/ --lcov coverage.lcov
```

### Context-Aware Analysis
```bash
# Enable all context providers
debtmap analyze . --context

# Use specific context providers
debtmap analyze . --context --context-providers critical_path,git_history

# Disable specific providers
debtmap analyze . --context --disable-context dependency
```

### Filtered Output
```bash
# Top 10 priority items only
debtmap analyze . --top 10

# High priority and above
debtmap analyze . --min-priority high

# Specific categories
debtmap analyze . --filter complexity,duplication

# Summary format
debtmap analyze . --summary

# Group by category
debtmap analyze . --group-by-category
```

### Performance Tuning
```bash
# Use 8 threads
debtmap analyze . --jobs 8

# Disable parallelization
debtmap analyze . --no-parallel

# Limit file count
debtmap analyze . --max-files 100

# Shared cache
debtmap analyze . --cache-location shared

# Clear and rebuild cache
debtmap analyze . --clear-cache
```

### Validation
```bash
# Initialize config
debtmap init --force

# Validate against config
debtmap validate . --config debtmap.toml

# With max debt density threshold
debtmap validate . --max-debt-density 50
```

### Comparison
```bash
# Compare two analyses
debtmap compare --before before.json --after after.json

# With markdown output
debtmap compare --before before.json --after after.json --format markdown

# With implementation plan
debtmap compare --before before.json --after after.json --plan IMPLEMENTATION_PLAN.md

# With target location
debtmap compare --before before.json --after after.json --target-location "src/main.rs:process_file:42"
```

### Language Selection
```bash
# Analyze only Rust files
debtmap analyze . --languages rust

# Multiple languages
debtmap analyze . --languages rust,python,javascript
```

### Threshold Configuration
```bash
# Custom complexity threshold
debtmap analyze . --threshold-complexity 15

# Use preset
debtmap analyze . --threshold-preset strict

# Custom duplication threshold
debtmap analyze . --threshold-duplication 100
```

### Plain/Machine-Readable Output
```bash
# Plain output (no colors, no emoji)
debtmap analyze . --plain

# Combine with JSON for CI
debtmap analyze . --format json --plain --output report.json
```

## Command Compatibility Matrix

| Option | analyze | validate | compare | init |
|--------|---------|----------|---------|------|
| `<PATH>` argument | ✓ | ✓ | ✗ | ✗ |
| `--format` | ✓ | ✓ | ✓ | ✗ |
| `--output` | ✓ | ✓ | ✓ | ✗ |
| `--coverage-file` | ✓ | ✓ | ✗ | ✗ |
| `--context` | ✓ | ✓ | ✗ | ✗ |
| `--threshold-*` | ✓ | ✓ | ✗ | ✗ |
| `--top / --tail` | ✓ | ✓ | ✗ | ✗ |
| `--cache-*` | ✓ | ✓ | ✗ | ✗ |
| `--jobs` | ✓ | ✓ | ✗ | ✗ |
| `--verbose` | ✓ | ✓ | ✗ | ✗ |
| `--config` | ✗ | ✓ | ✗ | ✗ |
| `--before / --after` | ✗ | ✗ | ✓ | ✗ |
| `--force` | ✗ | ✗ | ✗ | ✓ |

## Troubleshooting

### Performance Issues

**Problem:** Analysis is slow on large codebases

**Solutions:**
```bash
# Use more threads (if you have CPU cores available)
debtmap analyze . --jobs 16

# Enable caching (on by default, but ensure it's not disabled)
debtmap analyze . # caching is automatic

# Use shared cache for team
debtmap analyze . --cache-location shared

# Limit analysis scope
debtmap analyze . --max-files 500 --languages rust
```

### Memory Issues

**Problem:** Analysis runs out of memory

**Solutions:**
```bash
# Disable parallelization
debtmap analyze . --no-parallel

# Limit file count
debtmap analyze . --max-files 100

# Analyze in batches by language
debtmap analyze . --languages rust
debtmap analyze . --languages python
```

### Output Issues

**Problem:** Terminal output has garbled characters

**Solution:**
```bash
# Use plain mode
debtmap analyze . --plain
```

**Problem:** Want machine-readable output

**Solution:**
```bash
# Use JSON with plain mode
debtmap analyze . --format json --plain --output report.json
```

### Cache Issues

**Problem:** Stale cached results

**Solutions:**
```bash
# Clear cache
debtmap analyze . --clear-cache

# Check cache statistics
debtmap analyze . --cache-stats

# Disable cache temporarily
debtmap analyze . --no-cache
```

### Threshold Issues

**Problem:** Too many items flagged

**Solutions:**
```bash
# Use lenient preset
debtmap analyze . --threshold-preset lenient

# Increase threshold
debtmap analyze . --threshold-complexity 20

# Filter to high priority only
debtmap analyze . --min-priority high
```

**Problem:** Not enough items flagged

**Solutions:**
```bash
# Use strict preset
debtmap analyze . --threshold-preset strict

# Lower threshold
debtmap analyze . --threshold-complexity 5

# Show all items
debtmap analyze . --min-priority low
```

## Best Practices

### Regular Analysis

Run analysis regularly to track code quality trends:
```bash
# Daily in CI
debtmap validate . --config debtmap.toml

# Weekly deep analysis with coverage
debtmap analyze . --coverage-file coverage.lcov --format json --output weekly-report.json
```

### Team Workflows

Use shared cache for consistent team experience:
```bash
# Set environment variable for all team members
export DEBTMAP_CACHE_DIR=/shared/team/debtmap-cache

# Or use flag
debtmap analyze . --cache-location shared
```

### Performance Optimization

For large codebases:
```bash
# Use maximum parallelization
debtmap analyze . --jobs 0  # 0 = all cores

# Cache aggressively
debtmap analyze . --cache-location shared

# Focus on changed files in CI
# (implement via custom scripts to analyze git diff)
```

### Integration with Coverage

Always analyze with coverage when available:
```bash
# Rust example
cargo tarpaulin --out lcov
debtmap analyze src/ --coverage-file lcov.info

# Python example
pytest --cov --cov-report=lcov
debtmap analyze . --coverage-file coverage.lcov
```

Coverage integration helps prioritize untested complex code.

## Additional Tools

### prodigy-validate-debtmap-improvement

Specialized validation tool for Prodigy workflow integration.

**Description:**
This binary is part of the Prodigy workflow system and provides specialized validation for Debtmap improvement workflows.

**Usage:**
See Prodigy documentation for detailed usage instructions.

## See Also

- [Configuration Format](./configuration.md) - Detailed configuration file format
- [Output Formats](./output-formats.md) - Understanding JSON, Markdown, and Terminal output
- [Coverage Integration](./coverage.md) - Integrating test coverage data
- [Context Providers](./context-providers.md) - Understanding context-aware analysis
- [Examples](./examples.md) - More comprehensive usage examples
