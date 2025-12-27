# Debug Mode

Use verbosity flags and environment variables to diagnose issues and understand analysis behavior.

## Verbosity Levels

Debtmap provides three verbosity levels for progressive debugging:

```bash
# Level 1: Show main score factors
debtmap -v

# Level 2: Show detailed calculations
debtmap -vv

# Level 3: Show all debug information
debtmap -vvv
```

**What each level shows**:

| Level | Flag | Information Displayed |
|-------|------|----------------------|
| 1 | `-v` | Score breakdowns, main contributing factors |
| 2 | `-vv` | Detailed metric calculations, file processing |
| 3 | `-vvv` | Full debug output, context provider details |

### Example Output at Each Level

**Level 1 (`-v`)** - Score factors:
```
[Score] src/main.rs::process_file
  Complexity: 15 (cyclomatic) + 20 (cognitive)
  Coverage: 45%
  Priority: 7.2
```

**Level 2 (`-vv`)** - Detailed calculations:
```
[Score] src/main.rs::process_file
  Base risk: 3.6
  Coverage penalty: 1.5× (40-60% coverage tier)
  Debt factor: 1.2×
  Final: 7.2
  Processing time: 45ms
```

**Level 3 (`-vvv`)** - Full debug:
```
[Debug] Parsing src/main.rs
[Debug] AST nodes: 1,234
[Debug] Functions detected: 15
[Context] critical_path: distance=2 from entry point
[Context] dependency: 8 callers, 3 callees
...
```

## Diagnostic Options

### Macro Debugging (Rust)

```bash
# Show macro parsing warnings
debtmap --verbose-macro-warnings

# Show macro expansion statistics
debtmap --show-macro-stats
```

These flags help diagnose Rust macro-related parse issues:
- `--verbose-macro-warnings`: Shows warnings for each macro that couldn't be fully expanded
- `--show-macro-stats`: Displays statistics about macro expansion success rates

### Semantic Analysis Control

```bash
# Disable semantic analysis (faster fallback mode)
debtmap --semantic-off

# Validate LOC consistency
debtmap --validate-loc
```

Use `--semantic-off` when:
- Encountering parse errors on valid syntax
- Need faster analysis with reduced accuracy
- Testing files with unsupported language constructs

### Explain Metrics

The `--explain-metrics` flag displays detailed explanations of all metrics and scoring formulas:

```bash
debtmap --explain-metrics
```

This shows:
- Metric definitions (cyclomatic complexity, cognitive complexity, etc.)
- Scoring formula breakdowns
- Threshold explanations
- Priority calculation details

**Source**: `src/cli/args.rs:313-314`

## Performance Profiling

### Enable Profiling

Use the `--profile` flag to identify performance bottlenecks (Spec 001):

```bash
# Enable profiling output
debtmap --profile

# Write profiling data to JSON file
debtmap --profile --profile-output profile-data.json
```

**Source**: `src/cli/args.rs:356-368`

**Profiling output includes**:
- Time spent in each analysis phase
- File processing times
- Memory usage statistics
- Bottleneck identification

### Analyze Profiling Results

```bash
# Generate profile and analyze
debtmap --profile --profile-output analysis.json
jq '.phases | to_entries | sort_by(.value.duration_ms) | reverse' analysis.json
```

## Debugging Score Calculations

```bash
# Use verbosity levels to see score breakdown
debtmap -v    # Shows score factors

# See how coverage affects scores
debtmap --coverage-file coverage.info -v

# See how context affects scores
debtmap --context --context-providers critical_path -v
```

### Debug Session Example

```bash
# Step 1: Run with verbosity to see what's happening
debtmap -vv

# Step 2: Try without semantic analysis
debtmap --semantic-off -v

# Step 3: Check specific file
debtmap path/to/file.rs -vvv

# Step 4: Validate results
debtmap --validate-loc
```

## Environment Variables

Debtmap supports various environment variables for debugging and diagnostics without command-line flags.

### Debug Environment Variables

| Variable | Purpose | Source |
|----------|---------|--------|
| `DEBTMAP_COVERAGE_DEBUG` | Enable detailed coverage matching diagnostics | `src/risk/lcov/diagnostics.rs:4` |
| `DEBTMAP_TIMING` | Show timing information for analysis phases | `src/analyzers/effects.rs:98` |
| `DEBTMAP_DEBUG_SCORING` | Enable detailed score calculation output | `src/priority/mod.rs:551` |
| `DEBTMAP_LOG_FILE` | Write tracing output to a log file | `src/observability/tracing.rs:63` |
| `DEBTMAP_SHOW_FILTER_STATS` | Show filter statistics summary | `src/priority/mod.rs:649` |
| `DEBTMAP_ENTROPY_ENABLED` | Enable entropy-based pattern dampening | `src/complexity/entropy.rs` |
| `DEBTMAP_FILE_TIMEOUT` | Override per-file analysis timeout (seconds) | `src/analysis_utils.rs:206` |
| `DEBTMAP_NO_TIMEOUT` | Disable file analysis timeouts entirely | `src/analysis_utils.rs:220` |

### Coverage Debugging

Enable detailed coverage matching diagnostics when troubleshooting coverage file issues:

```bash
# Enable coverage debug mode
DEBTMAP_COVERAGE_DEBUG=1 debtmap analyze .

# Combine with explain-coverage for detailed matching
DEBTMAP_COVERAGE_DEBUG=1 debtmap explain-coverage . \
  --coverage-file coverage.lcov \
  --function "my_function" \
  -v
```

**Source**: `src/risk/lcov/diagnostics.rs:4-18`

When enabled, this shows:
- Coverage file parsing statistics
- Function name matching attempts and results
- Path normalization details
- Match success rates by strategy (exact, suffix, method name, etc.)

**Example output**:
```
=== Coverage Matching Statistics ===
Total functions analyzed: 150
Functions with coverage: 123 (82.0%)
Functions without coverage: 27 (18.0%)

Match Strategy Breakdown:
  exact_match: 95 (77.2%)
  suffix_match: 20 (16.3%)
  method_name_match: 8 (6.5%)
```

### Timing Information

Enable timing output to identify slow files or analysis phases:

```bash
# Enable timing output
DEBTMAP_TIMING=1 debtmap analyze .
```

**Source**: `src/analyzers/effects.rs:81-99`, `src/analyzers/rust.rs:211-213`

**Example output**:
```
[TIMING] analyze_rust_file src/main.rs: total=0.45s (analysis=0.32s, debt=0.08s, deps=0.05s)
[TIMING] analyze_rust_file src/lib.rs: total=0.12s (analysis=0.09s, debt=0.02s, deps=0.01s)
```

### Score Calculation Debugging

For detailed score calculation output:

```bash
DEBTMAP_DEBUG_SCORING=1 debtmap analyze .
```

**Source**: `src/priority/mod.rs:551-553`

**Example output**:
```
=== Score Calculation Debug ===
Function-level items count: 150
Base risk calculation:
  complexity_component: 0.27
  coverage_component: 0.45
  base_risk: 3.6
...
```

### Filter Statistics

Show statistics about filtering operations:

```bash
DEBTMAP_SHOW_FILTER_STATS=1 debtmap analyze .
```

**Source**: `src/priority/mod.rs:649-653`

**Example output**:
```
=== Filter Statistics ===
Items before filtering: 500
Items after priority filter: 150
Items after category filter: 120
Items in final output: 100
```

### File Logging

Write tracing output to a file instead of stderr (useful for TUI debugging):

```bash
DEBTMAP_LOG_FILE=debtmap.log debtmap analyze .
```

**Source**: `src/observability/tracing.rs:41-42`, `src/observability/tracing.rs:63-65`

This is particularly useful when:
- Using the TUI mode (prevents display corruption)
- Need to capture debug output for later analysis
- Debugging issues that require verbose output

### File Timeout Control

Control per-file analysis timeouts:

```bash
# Set custom timeout (in seconds, default varies by file size)
DEBTMAP_FILE_TIMEOUT=30 debtmap analyze .

# Disable timeouts entirely (use with caution)
DEBTMAP_NO_TIMEOUT=1 debtmap analyze .
```

**Source**: `src/analysis_utils.rs:206-221`

**When to adjust timeouts**:
- `DEBTMAP_FILE_TIMEOUT`: When analyzing very large or complex files that need more time
- `DEBTMAP_NO_TIMEOUT`: When debugging timeout issues or analyzing files that consistently timeout

**Warning**: Disabling timeouts can cause the analysis to hang on problematic files.

### RUST_LOG for Tracing

Control the structured tracing verbosity:

```bash
# Default: warnings and errors only
debtmap analyze .

# Show phase-level progress
RUST_LOG=info debtmap analyze .

# Detailed debugging output
RUST_LOG=debug debtmap analyze .

# Debug only debtmap crate
RUST_LOG=debtmap=debug debtmap analyze .

# Trace-level (very verbose)
RUST_LOG=trace debtmap analyze .
```

**Source**: `src/observability/tracing.rs:17-31`

### Analysis Feature Flags

```bash
# Enable context-aware analysis by default
export DEBTMAP_CONTEXT_AWARE=true

# Enable functional analysis by default
export DEBTMAP_FUNCTIONAL_ANALYSIS=true

# Single-pass analysis (disable multi-pass)
export DEBTMAP_SINGLE_PASS=1
```

### Output Customization

```bash
# Disable emoji in output
export NO_EMOJI=1

# Force plain text output (no colors)
export NO_COLOR=1
```

### CI/CD Variables

```bash
# Enable automation-friendly output
export PRODIGY_AUTOMATION=true

# Enable validation mode (stricter checks)
export PRODIGY_VALIDATION=true
```

## Precedence Rules

When both environment variables and CLI flags are present:

1. **CLI flags take precedence** over environment variables
2. **Environment variables override** config file defaults
3. **Config file settings override** built-in defaults

```bash
# CLI flag overrides environment variable
DEBTMAP_CONTEXT_AWARE=false debtmap --context  # Flag wins, context enabled
```

## Performance Tips

### Parallel Processing

```bash
# Use all CPU cores (default)
debtmap --jobs 0

# Limit to 4 threads
debtmap --jobs 4

# Disable parallel processing (debugging)
debtmap --no-parallel
```

**When to adjust parallelism**:
- **Use `--jobs 0`** (default): Maximum performance on dedicated machine
- **Use `--jobs N`**: Limit resource usage while other tasks run
- **Use `--no-parallel`**: Debugging concurrency issues

### Analysis Optimizations

```bash
# Faster: disable multi-pass analysis (single-pass mode)
debtmap --no-multi-pass

# Fast mode: disable semantic analysis
debtmap --semantic-off

# Plain output: faster terminal rendering
debtmap --plain

# Limit files for testing
debtmap --max-files 100

# Analyze subdirectory only
debtmap src/specific/module

# Reduce output with filters
debtmap --min-priority 4 --top 20
```

### Performance Comparison

| Configuration | Speed | Accuracy |
|--------------|-------|----------|
| Default (multi-pass) | Fast | Highest |
| `--no-multi-pass` | Faster | High |
| `--semantic-off` | Fastest | Medium |
| `--no-parallel` | Slowest | High |
| `--jobs 4` | Medium | High |

### Monitoring Performance

```bash
# Time analysis
time debtmap

# Profile with verbosity
debtmap -vv 2>&1 | grep "processed in"

# Use built-in profiling
debtmap --profile
```

## Troubleshooting Environment Variables

```bash
# Test with specific environment
env DEBTMAP_CONTEXT_AWARE=true debtmap -v

# See all debtmap-related environment variables
env | grep -i debtmap
env | grep -i prodigy

# Combine multiple debug variables
DEBTMAP_COVERAGE_DEBUG=1 DEBTMAP_TIMING=1 debtmap analyze . -v
```

## Related Documentation

- [Common Issues](../troubleshooting.md#common-issues): Quick fixes for common problems
- [Error Messages](../troubleshooting.md#error-messages-reference): Understanding error messages
- [CLI Reference](../cli-reference.md): Complete command-line documentation
- [Configuration Guide](../configuration.md): Configure debtmap behavior
