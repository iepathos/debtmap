# Quick Fixes

If you're experiencing problems with debtmap, try these quick solutions before diving into detailed troubleshooting.

## Slow Analysis

**Problem**: Analysis takes too long to complete

**Quick Solutions**:

```bash
# Use all available CPU cores (default)
debtmap analyze . --jobs 0

# Disable multi-pass analysis for faster single-pass
debtmap analyze . --no-multi-pass

# Use faster fallback mode (less accurate but much faster)
debtmap analyze . --semantic-off

# Limit files for testing
debtmap analyze . --max-files 100

# Analyze a specific subdirectory only
debtmap analyze src/specific/module
```

**Source**: CLI flags defined in `src/cli/args.rs:227-240`

**When to use each approach**:
- `--jobs 0`: Default, maximum performance on dedicated machine
- `--no-multi-pass`: CI/CD pipelines, large codebases (>100k LOC)
- `--semantic-off`: Quick complexity checks during development
- `--max-files`: Testing configuration before full analysis

See [Debug Mode](debug-mode.md) for performance monitoring options.

## Parse Errors

**Problem**: "Parse error in file:line:column" messages

**Quick Solutions**:

```bash
# Try fallback mode without semantic analysis
debtmap analyze . --semantic-off

# For Rust macro issues, see detailed warnings
debtmap analyze . --verbose-macro-warnings --show-macro-stats

# Exclude problematic files via configuration
# Add to .debtmap/config.toml:
# exclude = ["path/to/problematic/file.rs"]
```

**Source**: Verbose macro flags in `src/cli/args.rs:317-326`

**Common causes**:
- Unsupported language syntax or version
- Complex macro expansions (Rust)
- Type inference edge cases (Python, TypeScript)

See [Error Messages Reference](error-messages.md) for detailed parse error explanations.

## No Output

**Problem**: Running debtmap produces no output or results

**Quick Solutions**:

```bash
# Increase verbosity to see what's happening
debtmap analyze . -v

# Lower the minimum priority threshold
debtmap analyze . --min-priority 0

# Lower the minimum score threshold
debtmap analyze . --min-score 0

# Check if files are being analyzed
debtmap analyze . -vv 2>&1 | grep "Processing"

# Use lenient threshold preset
debtmap analyze . --threshold-preset lenient

# Show all items without filtering
debtmap analyze . --min-priority 0 --top 100
```

**Source**: Filtering options in `src/cli/args.rs:143-163`

**Common causes**:
- Threshold preset too strict (try `--threshold-preset lenient`)
- Category filters excluding all results
- Min-priority too high
- Min-score too high

## Inconsistent Results

**Problem**: Results differ between runs

**Quick Solutions**:

```bash
# Check if coverage file changed
debtmap analyze . --coverage-file coverage.info -v

# Disable context providers for consistent baseline
debtmap analyze . --no-context-aware

# Compare runs with debug output
debtmap analyze . -vv > run1.txt
debtmap analyze . -vv > run2.txt
diff run1.txt run2.txt
```

**Source**: Context-aware flag in `src/cli/args.rs:202-204`

**Common causes**:
- Coverage file changed between runs
- Context providers enabled/disabled (`--context`)
- Git history changes affecting git_history provider
- Different threshold settings

## Coverage Data Not Matching Functions

**Problem**: Coverage data not being applied to functions

**Quick Solutions**:

```bash
# Debug coverage data parsing for a specific function
debtmap explain-coverage . \
  --coverage-file coverage.lcov \
  --function "process_file"

# See detailed function matching diagnostics
debtmap explain-coverage . \
  --coverage-file coverage.lcov \
  --function "process_file" \
  -v

# Narrow search to specific file
debtmap explain-coverage . \
  --coverage-file coverage.lcov \
  --function "calculate_score" \
  --file src/scoring.rs
```

**Source**: `explain-coverage` command in `src/commands/explain_coverage/mod.rs`

**Example output**:
```
Coverage Detection Explanation
==============================

Function: process_file
File: src/processor.rs

Matched: suffix_match
Coverage: 87.5%
```

See [Context Provider Issues](context-providers.md) for detailed coverage troubleshooting.

## Too Many Low-Priority Warnings

**Problem**: Results overwhelmed with low-priority items

**Quick Solutions**:

```bash
# Increase minimum score threshold
debtmap analyze . --min-score 5.0

# Use stricter threshold preset
debtmap analyze . --threshold-preset strict

# Filter by specific debt categories
debtmap analyze . --filter "complexity,debt"

# Limit output to top N items
debtmap analyze . --top 20

# Use summary mode for compact output
debtmap analyze . --summary
```

**Source**: Filtering flags in `src/cli/args.rs:147-159`

**Min-score filter behavior** (spec 193, 205):
- T1 Critical Architecture and T2 Complex Untested items bypass the min-score filter
- T3 and T4 items are filtered by min-score threshold
- Default min-score is 3.0

## Call Graph Resolution Failures

**Problem**: Call graph shows incomplete or missing relationships

**Quick Solutions**:

```bash
# Enable debug output to see graph construction
debtmap analyze . --debug-call-graph -vv

# Validate the call graph consistency
debtmap analyze . --validate-call-graph

# Include external dependencies if relevant
debtmap analyze . --show-external-calls --show-std-lib-calls

# Trace specific functions to see their relationships
debtmap analyze . --trace-function "my_function" -vv

# Show only statistics, not the full graph
debtmap analyze . --call-graph-stats
```

**Source**: Call graph debug flags in `src/cli/args.rs:332-350`

See [Advanced Analysis Issues](advanced-analysis.md) for detailed call graph debugging.

## God Object False Positives

**Problem**: God object detection flagging legitimate large files

**Quick Solutions**:

```bash
# Disable god object detection entirely
debtmap analyze . --no-god-object

# See god object analysis with responsibility metrics
debtmap analyze . -vv

# Check specific file for god object patterns
debtmap analyze path/to/large/file.rs -vv
```

**Source**: God object flag in `src/cli/args.rs:242-244`

**When to disable**:
- Framework files with intentionally large aggregator classes
- Generated code files
- Files that are legitimately large due to single responsibility

See [Advanced Analysis Issues](advanced-analysis.md#god-object-detection) for god object configuration options.

## JSON Format Parsing Errors

**Problem**: JSON output parsing errors or unexpected structure

**Quick Solutions**:

```bash
# Use unified JSON format (consistent structure, recommended)
debtmap analyze . --format json --output-format unified

# Validate JSON output with jq
debtmap analyze . --format json | jq .

# Write to file for easier inspection
debtmap analyze . --format json --output results.json
```

**Source**: Output format options from CLI help

**Understanding the two formats**:

| Format | Structure | Recommended For |
|--------|-----------|-----------------|
| Legacy (default) | `{"File": {...}}` | Backwards compatibility |
| Unified | `{"type": "File", ...}` | Tool integration, parsing |

See [Output and Command Issues](output-issues.md) for detailed JSON format documentation.

## Context Provider Errors

**Problem**: Errors with critical_path, dependency, or git_history providers

**Quick Solutions**:

```bash
# Disable all context providers
debtmap analyze . --no-context-aware

# Disable specific problematic provider
debtmap analyze . --context --disable-context git_history

# Enable specific providers only
debtmap analyze . --context --context-providers critical_path,dependency

# Check context provider details
debtmap analyze . --context -vvv
```

**Source**: Context provider flags in `src/cli/args.rs:119-125`

**Provider-specific issues**:
- **git_history**: Requires git repository, fails outside git repos
- **dependency**: Complex import structures may not resolve
- **critical_path**: Requires valid call graph

See [Context Provider Issues](context-providers.md) for detailed troubleshooting.

## Quick Reference Table

| Problem | Quick Fix |
|---------|-----------|
| Slow analysis | `--no-multi-pass` or `--semantic-off` |
| Parse errors | `--semantic-off` or exclude files |
| No output | `--min-priority 0` or `-v` |
| Coverage issues | `explain-coverage` command |
| Too many warnings | `--min-score 5.0` or `--top 20` |
| Call graph issues | `--debug-call-graph -vv` |
| God object false positives | `--no-god-object` |
| JSON parsing | `--output-format unified` |
| Context errors | `--no-context-aware` |

## When Quick Fixes Don't Work

If these quick fixes don't resolve your issue, consult:

- [Debug Mode](debug-mode.md) - Detailed debugging options
- [Context Provider Issues](context-providers.md) - Provider-specific troubleshooting
- [Advanced Analysis Issues](advanced-analysis.md) - Complex analysis problems
- [Error Messages Reference](error-messages.md) - Error message explanations
- [FAQ](faq.md) - Common questions and answers

For issues not covered in this documentation, consider:

1. Running with maximum verbosity: `debtmap analyze . -vvv 2>&1 | tee debug.log`
2. Checking the [GitHub issues](https://github.com/anthropics/debtmap/issues)
3. Filing a bug report with the debug output
