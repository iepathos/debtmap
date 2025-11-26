# Troubleshooting

Common issues and solutions for using debtmap effectively.

## Quick Fixes for Common Issues

If you're experiencing problems, try these first:

1. **Analysis is slow**: Adjust threads with `--jobs` or use `--semantic-off` for faster fallback mode
2. **Parse errors**: Use `--semantic-off` for faster fallback mode or exclude problematic files
3. **No output**: Increase verbosity with `-v` or lower `--min-priority`
4. **Inconsistent results**: Check if coverage file changed or context providers are enabled

## Common Issues

### Parse Errors

**Problem**: Encountering "Parse error in file:line:column" messages

**Causes**:
- Unsupported language syntax or version
- Complex macro expansions (Rust)
- Type inference edge cases (Python, TypeScript)

**Solutions**:
```bash
# Try fallback mode without semantic analysis
debtmap --semantic-off

# For Rust macro issues, see detailed warnings
debtmap --verbose-macro-warnings --show-macro-stats

# Exclude specific problematic files
# Add to .debtmap/config.toml:
# exclude = ["path/to/problematic/file.rs"]
```

### Out of Memory Errors

**Problem**: Analysis crashes or runs out of memory on large codebases

**Solutions**:
```bash
# Limit parallel processing
debtmap --jobs 2

# Disable parallel processing entirely
debtmap --no-parallel

# Test with limited files first
debtmap --max-files 100

# Analyze subdirectories separately
debtmap path/to/subset
```

### Performance Issues

**Problem**: Analysis takes too long to complete

**Solutions**:
```bash
# Use all available CPU cores
debtmap --jobs 0

# Try faster fallback mode (less accurate)
debtmap --semantic-off

# Use plain output for faster terminal rendering
debtmap --plain
```

See [Performance Tips](#performance-tips) for detailed optimization strategies.

### File Permission Errors

**Problem**: "File system error" when accessing files

**Solutions**:
- Ensure you have read permissions for all source files
- Check that the project directory is accessible

### Git History Errors

**Problem**: Errors when using `git_history` context provider

**Causes**:
- Not running in a git repository
- Git history not available for files
- Insufficient git permissions

**Solutions**:
```bash
# Disable git_history context provider
debtmap --context --disable-context git_history

# Disable all context providers
debtmap --no-context-aware

# Check if in git repository
git status
```

### Coverage File Issues

**Problem**: Coverage file not being processed or causing errors

**Causes**:
- Non-LCOV format coverage file
- Malformed coverage data
- Path mismatches between coverage and source files

**Solutions**:
```bash
# Verify coverage file format (must be LCOV)
head coverage.info

# Check coverage file path
debtmap --coverage-file path/to/coverage.info -v

# Ensure paths in coverage file match source paths
# Coverage paths are relative to project root
```

### Threshold and Preset Confusion

**Problem**: Unexpected filtering or priority levels

**Solutions**:
```bash
# Check what threshold preset does
debtmap --threshold-preset strict --help

# Override specific thresholds
debtmap --min-priority 3

# See all items regardless of thresholds
debtmap --min-priority 0

# Use category filters instead
debtmap --filter "complexity,debt"
```

### JSON Format Issues

**Problem**: JSON output parsing errors or unexpected structure

**Understanding the Two Formats**:

**Legacy format** wraps items in variant-specific objects:
```json
{"File": {"path": "src/main.rs", "score": 7.5, ...}}
{"Function": {"name": "parse", "score": 8.2, ...}}
```

**Unified format** uses consistent structure with `type` field:
```json
{"type": "File", "path": "src/main.rs", "score": 7.5, ...}
{"type": "Function", "name": "parse", "score": 8.2, ...}
```

The unified format is **recommended** for parsing and tool integration as it provides a consistent structure across all item types.

**Solutions**:
```bash
# Use unified JSON format (consistent structure, recommended)
debtmap --format json --output-format unified

# Legacy format (default, uses {File: {...}} structure)
debtmap --format json --output-format legacy

# Validate JSON output
debtmap --format json | jq .

# Write to file for easier inspection
debtmap --format json --output results.json
```

See the [Configuration/Output Formats](./configuration.md#output-formats) chapter for detailed JSON structure documentation.

### Context Provider Errors

**Problem**: Errors with critical_path, dependency, or git_history providers

**Solutions**:
```bash
# Enable specific providers only
debtmap --context --context-providers critical_path,dependency

# Disable problematic provider
debtmap --context --disable-context git_history

# Disable context-aware filtering
debtmap --no-context-aware

# Check context provider details
debtmap --context -vvv
```

See [Context Provider Troubleshooting](#context-provider-troubleshooting) for details.

## Debug Mode

Use verbosity flags to diagnose issues and understand analysis behavior.

### Verbosity Levels

```bash
# Level 1: Show main score factors
debtmap -v

# Level 2: Show detailed calculations
debtmap -vv

# Level 3: Show all debug information
debtmap -vvv
```

**What each level shows**:
- `-v`: Score breakdowns, main contributing factors
- `-vv`: Detailed metric calculations, file processing
- `-vvv`: Full debug output, context provider details

### Diagnostic Options

```bash
# Show macro parsing warnings (Rust)
debtmap --verbose-macro-warnings

# Show macro expansion statistics (Rust)
debtmap --show-macro-stats

# Disable semantic analysis (fallback mode)
debtmap --semantic-off

# Validate LOC consistency
debtmap --validate-loc
```

**Note**: The `--explain-score` flag is deprecated and hidden. Use `-v`, `-vv`, or `-vvv` for verbosity levels instead to see score breakdowns.

### Debugging Score Calculations

```bash
# Use verbosity levels to see score breakdown
debtmap -v    # Shows score factors

# See how coverage affects scores
debtmap --coverage-file coverage.info -v

# See how context affects scores
debtmap --context --context-providers critical_path -v
```

### Example Debug Session

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

## Performance Tips

Optimize debtmap analysis speed and resource usage.

### Parallel Processing

```bash
# Use all CPU cores (default)
debtmap --jobs 0

# Limit to 4 threads
debtmap --jobs 4

# Disable parallel processing (debugging)
# Note: --no-parallel is equivalent to --jobs 1 (single-threaded)
debtmap --no-parallel
```

**When to adjust parallelism**:
- **Use `--jobs 0`** (default): Maximum performance on dedicated machine
- **Use `--jobs N`**: Limit resource usage while other tasks run
- **Use `--no-parallel`**: Debugging concurrency issues

### Analysis Optimizations

```bash
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
| Default | Fast | High |
| `--semantic-off` | Fastest | Medium |
| `--no-parallel` | Slowest | High |
| `--jobs 4` | Medium | High |

### Monitoring Performance

```bash
# Time analysis
time debtmap

# Profile with verbosity
debtmap -vv 2>&1 | grep "processed in"
```

## Environment Variables

Debtmap supports various environment variables for configuring behavior without command-line flags.

### Analysis Feature Flags

```bash
# Enable context-aware analysis by default
export DEBTMAP_CONTEXT_AWARE=true

# Enable functional analysis by default
export DEBTMAP_FUNCTIONAL_ANALYSIS=true
```

### Automation and CI/CD Variables

```bash
# Enable automation-friendly output (used by Prodigy)
export PRODIGY_AUTOMATION=true

# Enable validation mode (stricter checks)
export PRODIGY_VALIDATION=true
```

### Output Customization

```bash
# Disable emoji in output
export NO_EMOJI=1

# Force plain text output (no colors)
export NO_COLOR=1
```

### Usage Examples

```bash
# Enable context-aware analysis by default
echo 'export DEBTMAP_CONTEXT_AWARE=true' >> ~/.bashrc

# CI/CD environment setup
export NO_EMOJI=1
export NO_COLOR=1
export PRODIGY_AUTOMATION=true

# Run analysis with environment settings
debtmap

# Override environment with flags
DEBTMAP_CONTEXT_AWARE=false debtmap --context  # Flag takes precedence
```

### Precedence Rules

When both environment variables and CLI flags are present:

1. **CLI flags take precedence** over environment variables
2. **Environment variables override** config file defaults
3. **Config file settings override** built-in defaults

### Troubleshooting Environment Variables

```bash
# Test with specific environment
env DEBTMAP_CONTEXT_AWARE=true debtmap -v

# See all debtmap-related environment variables
env | grep -i debtmap
env | grep -i prodigy
```

## Context Provider Troubleshooting

Diagnose and fix issues with context providers (critical_path, dependency, git_history).

### Enable Context Analysis

```bash
# Enable with default providers
debtmap --context

# Or use explicit flag
debtmap --enable-context

# Specify specific providers
debtmap --context --context-providers critical_path,dependency,git_history
```

### Disable Specific Providers

```bash
# Disable git_history only
debtmap --context --disable-context git_history

# Disable multiple providers
debtmap --context --disable-context git_history,dependency

# Disable context-aware filtering
debtmap --no-context-aware
```

### Git History Provider Issues

**Problem**: "Git history error" when running analysis

**Causes**:
- Not in a git repository
- No git history for files
- Git not installed or accessible

**Solutions**:
```bash
# Verify in git repository
git status

# Disable git_history provider
debtmap --context --disable-context git_history

# Initialize git repo if needed
git init
```

### Dependency Provider Issues

**Problem**: "Dependency error" or incomplete dependency graph

**Causes**:
- Complex import structures
- Circular dependencies
- Unsupported dependency patterns

**Solutions**:
```bash
# Disable dependency provider
debtmap --context --disable-context dependency

# Try with verbosity to see details
debtmap --context -vvv

# Use without context
debtmap
```

### Critical Path Provider Issues

**Problem**: Critical path analysis fails or produces unexpected results

**Causes**:
- Invalid call graph
- Missing function definitions
- Complex control flow

**Solutions**:
```bash
# Disable critical_path provider
debtmap --context --disable-context critical_path

# Try with semantic analysis disabled
debtmap --context --semantic-off

# Debug with verbosity
debtmap --context --context-providers critical_path -vvv
```

### Context Impact on Scoring

Context providers add additional risk factors to scoring:

```bash
# See context contribution to scores
debtmap --context -v

# Compare with and without context
debtmap --output baseline.json
debtmap --context --output with_context.json
debtmap compare --before baseline.json --after with_context.json
```

### Performance Impact

Context analysis adds processing overhead:

```bash
# Faster: no context
debtmap

# Slower: with all context providers
debtmap --context --context-providers critical_path,dependency,git_history

# Medium: selective providers
debtmap --context --context-providers dependency
```

### Debug Context Providers

```bash
# See detailed context provider output
debtmap --context -vvv

# Check which providers are active
debtmap --context -v | grep "context provider"
```

## Advanced Analysis Troubleshooting

Advanced CLI flags for specialized analysis scenarios.

### Multi-Pass Analysis

**Flag**: `--multi-pass`

Multi-pass analysis performs multiple iterations to refine results.

```bash
# Enable multi-pass analysis
debtmap --multi-pass

# Useful for complex projects with intricate dependencies
# May increase analysis time but improve accuracy
```

**When to use**:
- Complex dependency graphs
- Large codebases with deep nesting
- When single-pass results seem incomplete

### Attribution Output

**Flag**: `--show-attribution`

Shows attribution information for detected issues.

```bash
# Enable attribution output
debtmap --show-attribution

# Combine with verbosity for details
debtmap --show-attribution -v
```

**Troubleshooting**:
- Requires git history provider for author information
- May slow down analysis
- Use `--disable-context git_history` if causing errors

### Aggregation Methods

**Flag**: `--aggregation-method <method>`

Controls how results are aggregated across files.

```bash
# Available aggregation methods:
debtmap --aggregation-method weighted_sum  # (default)
debtmap --aggregation-method sum
debtmap --aggregation-method logarithmic_sum
debtmap --aggregation-method max_plus_average
```

**Common issues**:
- Different methods produce different result structures
- Choose method based on your reporting needs
- Use consistent method for comparison over time

### Minimum Problematic Threshold

**Flag**: `--min-problematic <number>`

Sets the minimum score for an item to be considered problematic.

```bash
# Default threshold
debtmap --min-problematic 3

# More strict (show more issues)
debtmap --min-problematic 1

# Less strict (show only serious issues)
debtmap --min-problematic 5
```

**Relationship to other filters**:
- Works alongside `--min-priority`
- Filters at analysis level vs display level
- Lower values = more issues shown

### God Object Detection

**Flag**: `--no-god-object`

Disables god object (large class/module) detection.

**God Object Types**:

Debtmap distinguishes three types of god objects:

1. **god_class**: Files with excessive complexity excluding test functions
   - Focuses on production code complexity
   - Ignores test helper functions and test cases
   - Best indicator of production code quality issues

2. **god_file**: Files with excessive complexity including all functions
   - Considers both production and test code
   - Useful for understanding total file complexity
   - Alias: `god_module` (same as god_file)

3. **god_module**: Alias for god_file
   - Module-level view of complexity
   - Includes all functions regardless of purpose

**Responsibility Analysis Metrics** (Spec 140):

Modern god object detection includes domain responsibility analysis:

```bash
# See detailed god object metrics
debtmap -vv 2>&1 | grep "god_object\|domain"
```

**Additional Metrics**:
- **domain_count**: Number of distinct responsibility domains in file
- **domain_diversity**: Measure of how varied the responsibilities are (0.0-1.0)
- **struct_ratio**: Ratio of structs to total file size
- **cross_domain_severity**: How badly domains are mixed (0.0-1.0)
- **module_splits**: Suggested number of modules to split into

**Configuration**:

```toml
# In .debtmap.toml
[god_object]
# Thresholds for god object detection
complexity_threshold = 100
loc_threshold = 500
function_count_threshold = 20

# Responsibility analysis thresholds
domain_diversity_threshold = 0.7  # High diversity = mixed responsibilities
cross_domain_threshold = 0.6       # High value = poor separation
```

**Usage**:

```bash
# Disable god object detection entirely
debtmap --no-god-object

# See god object analysis with responsibility metrics
debtmap -vv

# Check specific file for god object patterns
debtmap path/to/large/file.rs -vv
```

**When to use**:
- False positives on framework files
- Intentional large aggregator classes
- Reducing noise in results
- Files that are legitimately large due to generated code

**Understanding the Metrics**:

```bash
# Example output interpretation:
# domain_count = 5          → File handles 5 different concerns
# domain_diversity = 0.8    → Very mixed responsibilities (bad)
# cross_domain_severity = 0.7 → Poor separation of concerns
# module_splits = 3         → Suggest splitting into 3 modules

# High domain_diversity + high cross_domain_severity = strong god object
# Recommended: refactor into separate modules per domain
```

### Detail Level Control

**Flag**: `--detail-level <level>`

Controls the level of detail in analysis output.

```bash
# Available detail levels:
debtmap --detail-level summary        # High-level overview only
debtmap --detail-level standard       # (default) Balanced detail
debtmap --detail-level comprehensive  # Detailed analysis
debtmap --detail-level debug         # Full debug information
```

**When to use**:
- `summary`: Quick overview for large codebases
- `standard`: Default, appropriate for most use cases
- `comprehensive`: Deep dive into specific issues
- `debug`: Troubleshooting analysis behavior

### Aggregation Control

**Flags**: `--aggregate-only`, `--no-aggregation`

Control file-level score aggregation.

```bash
# Show only aggregated file-level scores
debtmap --aggregate-only

# Disable file-level aggregation entirely
debtmap --no-aggregation

# Default: show both individual items and file aggregates
debtmap
```

**Use cases**:
- `--aggregate-only`: Focus on file-level technical debt
- `--no-aggregation`: See individual functions/classes only
- Default: Full picture with both levels

### Call Graph Debugging

**Overview**: Debug call graph generation and analysis for dependency tracking.

**Available Flags**:

```bash
# Enable call graph debug output
debtmap --debug-call-graph

# Trace specific functions through call graph
debtmap --trace-functions "function_name,another_function"

# Show only call graph statistics (no detailed graph)
debtmap --call-graph-stats-only

# Control debug output format (text or json)
debtmap --debug-call-graph --debug-format text
debtmap --debug-call-graph --debug-format json

# Validate call graph consistency
debtmap --validate-call-graph
```

**Dependency Control Flags**:

```bash
# Show dependency information in results
debtmap --show-dependencies

# Hide dependency information (default in some contexts)
debtmap --no-dependencies

# Limit number of callers shown per function
debtmap --max-callers 10

# Limit number of callees shown per function
debtmap --max-callees 10

# Include external crate calls in call graph
debtmap --show-external

# Include standard library calls in call graph
debtmap --show-std-lib
```

**Common Issues**:

**Q: Call graph shows incomplete or missing relationships?**

A: Try these debugging steps:
```bash
# Enable debug output to see graph construction
debtmap --debug-call-graph -vv

# Validate the call graph consistency
debtmap --validate-call-graph

# Include external dependencies if relevant
debtmap --show-external --show-std-lib

# Trace specific functions to see their relationships
debtmap --trace-functions "my_function" -vv
```

**Q: Call graph output is overwhelming?**

A: Use filtering options:
```bash
# Show only statistics, not the full graph
debtmap --call-graph-stats-only

# Limit callers and callees shown
debtmap --max-callers 5 --max-callees 5

# Hide dependencies from main output
debtmap --no-dependencies

# Export to JSON for external processing
debtmap --debug-call-graph --debug-format json --output call-graph.json
```

**When to use call graph debugging**:
- Investigating missing critical path detection
- Understanding dependency relationships
- Debugging context provider issues
- Analyzing architectural coupling
- Validating function relationship detection

### Tiered Prioritization Issues

**Overview**: Debtmap uses a 4-tier system to classify and **sort** technical debt items by architectural importance. Tiers affect result ordering but do not multiply scores.

**Tier Classification**:
- **Tier 1 (Critical Architecture)**: High complexity, low coverage, high dependencies, entry points, or file-level architectural debt
- **Tier 2 (Complex Untested)**: Significant complexity or coverage gaps
- **Tier 3 (Testing Gaps)**: Moderate issues that need attention
- **Tier 4 (Maintenance)**: Low-priority items, routine maintenance

**Result Ordering**:
Results are sorted first by tier (T1 > T2 > T3 > T4), then by score within each tier. This ensures architecturally critical items appear at the top regardless of their absolute score.

**Note**: Tier weights (1.5×, 1.0×, 0.7×, 0.3×) exist in the configuration but are currently not applied as score multipliers. Tiers control sort order instead.

**Configuration**:
```toml
# In .debtmap.toml
[tiers]
# Tier 2 requires EITHER high complexity OR high dependencies
t2_complexity_threshold = 15
t2_dependency_threshold = 10

# Tier 3 requires moderate complexity
t3_complexity_threshold = 8

# Control Tier 4 visibility in main report
show_t4_in_main_report = false
```

**Common Issues**:

**Q: Why is my item in Tier 3 instead of Tier 2?**

A: Check if it meets Tier 2 thresholds:
```bash
# See tier classification with verbosity
debtmap -v

# Check current thresholds
cat .debtmap.toml | grep -A 5 "\[tiers\]"

# Lower thresholds to promote more items to Tier 2
# In .debtmap.toml:
# t2_complexity_threshold = 10  (default: 15)
# t2_dependency_threshold = 5   (default: 10)
```

**Q: How do I hide Tier 4 items from the main report?**

A: Use the `show_t4_in_main_report` configuration:
```toml
# In .debtmap.toml
[tiers]
show_t4_in_main_report = false
```

Tier 4 items will still appear in detailed output but won't clutter the main summary.

### File-Level Scoring Issues

**Overview**: Debtmap aggregates function/class scores into file-level scores using configurable aggregation methods.

**Note**: The exact aggregation formula depends on the selected method (see `--aggregation-method` flag). File-level scores combine individual item scores with file-level characteristics.

**Aggregation Methods**:
```bash
# Weighted sum (default) - considers complexity weights
debtmap --aggregation-method weighted_sum

# Simple sum - adds all function scores
debtmap --aggregation-method sum

# Logarithmic sum - dampens very high scores
debtmap --aggregation-method logarithmic_sum

# Max plus average - highlights worst function + context
debtmap --aggregation-method max_plus_average
```

**When to use each method**:
- **weighted_sum**: Default, balances individual and collective impact
- **sum**: When you want raw cumulative debt
- **logarithmic_sum**: For very large files to prevent score explosion
- **max_plus_average**: Focus on worst offender while considering overall file health

**Aggregation Control Flags**:
```bash
# Show only aggregated file-level scores
debtmap --aggregate-only

# Disable file-level aggregation entirely
debtmap --no-aggregation

# Default: show both individual items and file aggregates
debtmap
```

**Troubleshooting High File Scores**:

**Q: Why does this file have such a high score?**

A: Check contributing factors with verbosity:
```bash
# See file-level score breakdown
debtmap path/to/file.rs -vv

# Look for:
# - High function count (density_factor kicks in at 50+)
# - God object detection (1.5× multiplier)
# - Low coverage (high coverage_factor)
# - Large file size (size_factor)
# - Multiple high-complexity functions

# Disable god object detection if false positive
debtmap --no-god-object path/to/file.rs
```

### Functional Analysis Issues

**Overview**: Functional analysis detects violations of functional programming principles like impure functions, excessive mutation, and side effects.

**Enable Functional Analysis**:

```bash
# Enable AST-based functional analysis
debtmap --ast-functional-analysis

# Use different strictness profiles
debtmap --ast-functional-analysis --functional-analysis-profile strict
debtmap --ast-functional-analysis --functional-analysis-profile balanced  # (default)
debtmap --ast-functional-analysis --functional-analysis-profile lenient
```

**Analysis Profiles**:

- **strict**: Flag most functional violations, enforce pure functions
- **balanced**: Default, reasonable middle ground for mixed codebases
- **lenient**: Allow more pragmatic deviations from pure functional style

**Common Issues**:

**Q: Too many false positives for legitimate imperative code?**

A: Adjust the profile or disable for specific areas:
```bash
# Use lenient profile for pragmatic codebases
debtmap --ast-functional-analysis --functional-analysis-profile lenient

# Disable functional analysis if not using FP style
debtmap  # (functional analysis is opt-in via --ast-functional-analysis)
```

**Q: What violations does functional analysis detect?**

A: Functional analysis flags:
- Mutation of variables (reassignment)
- Side effects in functions (I/O, global state)
- Impure functions (non-deterministic behavior)
- Excessive mutable state
- Missing const/immutability annotations

```bash
# See detailed functional analysis results
debtmap --ast-functional-analysis -vv

# Focus on functional purity issues
debtmap --ast-functional-analysis --filter "functional"
```

**When to use functional analysis**:
- Projects following functional programming principles
- Codebases using immutable data structures
- When refactoring to reduce side effects
- For detecting hidden mutation bugs
- In functional-first languages (Rust with functional style)

**When to disable**:
- Imperative codebases where mutation is expected
- Performance-critical code requiring in-place updates
- When false positives overwhelm actual issues

### Pattern Detection Issues

**Overview**: Pattern detection identifies repetitive code structures, anti-patterns, and common debt patterns.

**Control Pattern Detection**:

```bash
# Disable pattern detection entirely
debtmap --no-pattern-detection

# Specify specific patterns to detect
debtmap --patterns "god_object,long_function,complex_conditional"

# Adjust pattern detection sensitivity
debtmap --pattern-threshold 0.8  # Higher = stricter matching (0.0-1.0)

# Show pattern detection warnings
debtmap --show-pattern-warnings
```

**Common Issues**:

**Q: Pattern detection causes too many false positives?**

A: Adjust threshold or disable specific patterns:
```bash
# Increase threshold for stricter matching (fewer false positives)
debtmap --pattern-threshold 0.9

# Disable pattern detection for exploratory analysis
debtmap --no-pattern-detection

# See which patterns are triggering with warnings
debtmap --show-pattern-warnings -v
```

**Q: Missing patterns I expect to see?**

A: Lower threshold or check pattern names:
```bash
# Lower threshold to catch more patterns
debtmap --pattern-threshold 0.6

# Specify patterns explicitly
debtmap --patterns "god_object,long_function,deep_nesting"

# Use verbosity to see pattern detection process
debtmap --show-pattern-warnings -vv
```

**Detected Patterns**:
- `god_object`: Classes/modules with too many responsibilities
- `long_function`: Functions exceeding length thresholds
- `complex_conditional`: Nested or complex branching logic
- `deep_nesting`: Excessive indentation depth
- `parameter_overload`: Too many function parameters
- `duplicate_code`: Repetitive code structures

**When to adjust pattern threshold**:
- **Higher (0.8-1.0)**: Reduce noise, only flag clear violations
- **Lower (0.5-0.7)**: Catch subtle patterns, more comprehensive detection
- **Default (0.7)**: Balanced detection for most codebases

### Public API Detection Issues

**Overview**: Public API detection identifies functions and types that form your crate's public interface, affecting scoring and priority.

**Control Public API Detection**:

```bash
# Disable public API detection
debtmap --no-public-api-detection

# Adjust public API detection threshold
debtmap --public-api-threshold 0.5  # Lower = more items marked as public (0.0-1.0)
```

**Common Issues**:

**Q: Private functions being marked as public API?**

A: Increase the threshold for stricter detection:
```bash
# Higher threshold = only clearly public items
debtmap --public-api-threshold 0.8

# Disable public API detection if not useful
debtmap --no-public-api-detection

# See what's being detected as public
debtmap -vv 2>&1 | grep "public API"
```

**Q: Public functions not being detected?**

A: Lower the threshold or check visibility:
```bash
# Lower threshold to detect more public items
debtmap --public-api-threshold 0.3

# Verify function is actually public (pub keyword in Rust)
debtmap path/to/file.rs -vv
```

**How Public API Detection Works**:
- Checks for `pub` visibility in Rust
- Identifies exported functions in Python (`__all__`)
- Detects exported symbols in JavaScript/TypeScript
- Considers call graph entry points
- Factors in documentation presence

**Impact on Scoring**:
- Public API items get higher priority scores (1.1× multiplier)
- Entry point detection uses public API information
- Critical path analysis considers public boundaries

**When to disable**:
- Internal tools or scripts (no public API)
- When API detection causes confusion
- Libraries where everything is intentionally public

### Combining Advanced Flags

```bash
# Comprehensive analysis with all features
debtmap --multi-pass --attribution --context -vv

# Minimal filtering for exploration
debtmap --min-problematic 1 --min-priority 0 --no-god-object

# Performance-focused advanced analysis
debtmap --multi-pass --jobs 8

# Summary view with aggregated scores
debtmap --detail-level summary --aggregate-only
```

## Error Messages Reference

Understanding common error messages and how to resolve them.

### File System Errors

**Message**: `File system error: Permission denied`

**Meaning**: Cannot read file or directory due to permissions

**Solutions**:
- Check file permissions: `ls -la <file>`
- Ensure user has read access

---

**Message**: `File system error: No such file or directory`

**Meaning**: File or directory does not exist

**Solutions**:
- Verify path is correct
- Check current working directory: `pwd`
- Use absolute paths if needed
- Ensure files weren't moved or deleted

### Parse Errors

**Message**: `Parse error in file.rs:line:column: unexpected token`

**Meaning**: Syntax debtmap cannot parse

**Solutions**:
```bash
# Try fallback mode
debtmap --semantic-off

# For Rust macros
debtmap --verbose-macro-warnings --show-macro-stats

# Exclude problematic file
# In .debtmap/config.toml:
# exclude = ["path/to/file.rs"]
```

### Analysis Errors

**Message**: `Analysis error: internal analysis failure`

**Meaning**: Internal error during analysis phase

**Solutions**:
```bash
# Try fallback mode
debtmap --semantic-off

# Report with debug info
debtmap -vvv 2>&1 | tee error.log

# Isolate problem file
debtmap --max-files 1 path/to/suspected/file
```

### Configuration Errors

**Message**: `Configuration error: invalid config value`

**Meaning**: Invalid configuration in `.debtmap/config.toml` or CLI

**Solutions**:
- Check `.debtmap/config.toml` syntax
- Validate TOML format: `cat .debtmap/config.toml`
- Review CLI flag values
- Check for typos in flag names

### Validation Errors

**Message**: `Validation error: threshold validation failed`

**Meaning**: Threshold configuration is invalid

**Solutions**:
- Check threshold values in config
- Ensure `--min-priority` is in valid range (0-10)
- Verify threshold preset exists
- Use `--threshold-preset` with valid preset name

### Dependency Errors

**Message**: `Dependency error: cannot resolve dependency graph`

**Meaning**: Cannot build dependency relationships

**Solutions**:
```bash
# Disable dependency provider
debtmap --context --disable-context dependency

# Try without context
debtmap

# Debug with verbosity
debtmap -vvv
```

### Concurrency Errors

**Message**: `Concurrency error: parallel processing failure`

**Meaning**: Error during parallel execution

**Solutions**:
```bash
# Disable parallel processing
debtmap --no-parallel

# Reduce thread count
debtmap --jobs 1

# Report issue with debug output
debtmap -vvv 2>&1 | tee error.log
```

### Unsupported Errors

**Message**: `Unsupported: feature not available for <language>`

**Meaning**: Language or construct not supported

**Solutions**:
- Use supported languages: Rust, Python, JavaScript, TypeScript
- Check if language is enabled in config
- Some advanced features may not be available for all languages
- Try `--semantic-off` for basic analysis

### Pattern Errors

**Message**: `Pattern error: invalid glob pattern`

**Meaning**: Invalid glob pattern in configuration or CLI

**Solutions**:
- Check glob pattern syntax
- Escape special characters if needed
- Test pattern with shell glob: `ls <pattern>`
- Use simpler patterns or path prefixes

## Language-Specific Issues

### Rust

**Macro Expansion Issues**

```bash
# See macro warnings
debtmap --verbose-macro-warnings

# Show macro statistics
debtmap --show-macro-stats

# Common issue: Complex macros may not expand correctly
# Solution: Use --semantic-off for faster fallback
```

**Trait and Generic Complexity**

Complex trait bounds and generic constraints may affect analysis accuracy:

```bash
# Full semantic analysis (default)
debtmap

# Fallback mode for edge cases
debtmap --semantic-off
```

### Python

**Type Inference Limitations**

Dynamic typing makes some analysis challenging:

```bash
# Best effort type inference (default)
debtmap

# Fallback mode if issues
debtmap --semantic-off
```

**Import Resolution**

Complex import structures may not resolve fully:
- Relative imports usually work
- Dynamic imports may not be detected
- `__init__.py` packages are supported

### JavaScript/TypeScript

**JSX/TSX Parsing**

Ensure files have correct extensions:
- `.jsx` for JavaScript + JSX
- `.tsx` for TypeScript + JSX
- Configure extensions in `.debtmap/config.toml` if needed

**Type Resolution**

TypeScript type resolution in complex projects:
```bash
# Full type checking (default for .ts files)
debtmap

# Fallback if type issues
debtmap --semantic-off
```

### Mixed Language Projects

```bash
# Analyze all supported languages (default)
debtmap

# Filter specific languages
# In .debtmap/config.toml:
# languages = ["rust", "python"]
```

### Unsupported Language Constructs

Some advanced language features may show as "Unsupported":
- Rust: Some macro patterns, const generics edge cases
- Python: Some metaclass patterns, dynamic code generation
- JavaScript: Some advanced AST manipulation

**Solutions**:
- Use `--semantic-off` for basic analysis
- Exclude problematic files if needed
- Report unsupported patterns as feature requests

### Boilerplate Detection Issues

**Overview**: Boilerplate detection identifies repetitive code patterns that are necessary but contribute to complexity scores, such as trait implementations, error handling, and validation logic.

**How Boilerplate Detection Works**:

Debtmap automatically detects common boilerplate patterns:
- **Trait implementations**: Standard trait method implementations (Debug, Display, From, etc.)
- **Error handling**: Repetitive error conversion and propagation code
- **Validation functions**: Similar validation logic across multiple functions
- **Macro-generated code**: Repetitive patterns from macro expansions
- **Builder patterns**: Setter methods and builder implementations

**Impact on Scoring**:

Detected boilerplate receives dampened complexity scores to avoid inflating technical debt for necessary repetitive code.

**Common Issues**:

**Q: Legitimate complex code being marked as boilerplate?**

A: Boilerplate detection uses pattern similarity thresholds. If unique logic is being incorrectly dampened:
```bash
# See what's being detected as boilerplate
debtmap -vv 2>&1 | grep "boilerplate"

# Check entropy analysis settings (used for boilerplate detection)
# In .debtmap.toml:
# [entropy]
# pattern_threshold = 0.8  # Increase for stricter matching
```

**Q: Boilerplate code still showing high scores?**

A: Some boilerplate patterns may not be recognized. Common cases:
```bash
# Trait implementations should be automatically detected
# If not dampened, check that code follows standard patterns

# For custom validation patterns, ensure similarity is high enough
# In .debtmap.toml:
# [entropy]
# pattern_threshold = 0.7  # Lower to catch more patterns
# enabled = true
```

**Q: How to identify what debtmap considers boilerplate?**

A: Use verbose output:
```bash
# See boilerplate detection in action
debtmap -vv 2>&1 | grep -i "boilerplate\|pattern\|entropy"

# Check specific file
debtmap path/to/file.rs -vv
```

**Boilerplate Reduction Strategies**:

```toml
# In .debtmap.toml
[entropy]
enabled = true                    # Enable pattern-based dampening
pattern_threshold = 0.7           # Similarity threshold (0.0-1.0)
weight = 0.3                      # Impact on complexity adjustment
min_tokens = 50                   # Minimum size for pattern analysis
```

**When boilerplate detection helps**:
- Codebases with many trait implementations
- Projects with extensive validation logic
- Macro-heavy code (derives, procedural macros)
- Builder pattern implementations
- Error handling boilerplate

**When to adjust thresholds**:
- **Increase `pattern_threshold`** (0.8-0.9): If unique code is being dampened
- **Decrease `pattern_threshold`** (0.5-0.6): If obvious boilerplate isn't being detected
- **Disable entropy** (`enabled = false`): If causing too many false dampening

### False Positives

Reduce false positives for validation functions and repetitive code patterns using entropy analysis:

**Enable and Configure Entropy Analysis**:
```toml
# In .debtmap.toml
[entropy]
enabled = true                    # Enable entropy-based dampening
weight = 0.3                      # Weight in complexity adjustment (0.0-1.0)
min_tokens = 50                   # Minimum tokens for entropy calculation
pattern_threshold = 0.7           # Pattern similarity threshold (0.0-1.0)
use_classification = true         # Enable advanced token classification
entropy_threshold = 0.5           # Entropy level for dampening (0.0-1.0)
branch_threshold = 0.8            # Branch similarity threshold (0.0-1.0)
max_combined_reduction = 0.5      # Max reduction percentage (0.0-1.0)
```

**When to Adjust Parameters**:
- **Increase `pattern_threshold`** (e.g., 0.8-0.9): Be more strict, reduce dampening for truly unique code
- **Decrease `entropy_threshold`** (e.g., 0.3-0.4): Apply dampening more broadly to catch more repetitive patterns
- **Increase `weight`** (e.g., 0.4-0.5): Make entropy have stronger impact on final scores
- **Increase `min_tokens`** (e.g., 100): Only apply entropy analysis to larger functions
- **Increase `branch_threshold`** (e.g., 0.9): Be more strict about branching pattern similarity

Entropy analysis can reduce false positives by up to 70% for validation functions, error handling, and other repetitive patterns.

**Other False Positive Reduction Strategies**:
```bash
# Use context-aware analysis
debtmap --context

# Adjust thresholds
debtmap --threshold-preset lenient

# Disable context-aware filtering if too aggressive
debtmap --no-context-aware
```

### Missing Detections

```bash
# Ensure semantic analysis is enabled
debtmap  # (default, semantic ON)

# Increase verbosity to see what's detected
debtmap -vv

# Check if files are being analyzed
debtmap -v 2>&1 | grep "Processing"
```

## Output Formatting Issues

### Choose Output Format

```bash
# Terminal format (default, human-readable)
debtmap

# JSON format
debtmap --format json

# Markdown format
debtmap --format markdown
```

### JSON Format Options

```bash
# Legacy format (default): {File: {...}}
debtmap --format json --output-format legacy

# Unified format: consistent structure with 'type' field
debtmap --format json --output-format unified

# Validate JSON
debtmap --format json | jq .

# Write to file
debtmap --format json --output results.json
```

### Plain Output Mode

For environments without color/emoji support:

```bash
# ASCII-only, no colors, no emoji
debtmap --plain

# Or set environment variable
export NO_EMOJI=1
debtmap
```

### Terminal Color Issues

**Problem**: Colors not rendering or showing escape codes

**Solutions**:
```bash
# Use plain mode
debtmap --plain

# Check TERM environment variable
echo $TERM

# Set appropriate TERM
export TERM=xterm-256color
```

### Emoji Issues

**Problem**: Emojis showing as boxes or ??

**Solutions**:
```bash
# Disable emojis
debtmap --plain

# Or environment variable
export NO_EMOJI=1
debtmap
```

### Markdown Rendering

Ensure viewer supports GitHub-flavored markdown:
- Tables
- Code blocks with syntax highlighting
- Task lists

### Write Output to File

```bash
# JSON to file
debtmap --format json --output results.json

# Markdown to file
debtmap --format markdown --output report.md

# Terminal format to file (preserves colors)
debtmap --output results.txt

# Plain format to file
debtmap --plain --output results.txt
```

### Summary vs Full Output

```bash
# Summary mode (compact)
debtmap --summary
debtmap -s

# Full output (default)
debtmap

# Limit number of items
debtmap --top 10       # Top 10 by priority
debtmap --tail 10      # Bottom 10 by priority
```

### Filtering Output

```bash
# Minimum priority level
debtmap --min-priority 5

# Category filters
debtmap --filter "complexity,debt"

# Combine filters
debtmap --min-priority 3 --top 20 --filter complexity
```

## Compare Command Issues

The `compare` command helps track changes in technical debt over time.

### Basic Usage

**Note**: The `compare` command defaults to JSON output format (unlike `analyze` which defaults to terminal). Use `--format terminal` or `--format markdown` if you need different output.

```bash
# Save baseline results
debtmap --format json --output before.json

# Make code changes...

# Save new results
debtmap --format json --output after.json

# Compare results (outputs JSON by default)
debtmap compare --before before.json --after after.json

# Compare with terminal output
debtmap compare --before before.json --after after.json --format terminal
```

### Targeted Comparison

Use `--plan` and `--target-location` for focused debt analysis:

```bash
# Compare based on implementation plan
debtmap compare --before before.json --after after.json --plan implementation-plan.json

# Compare specific code location
debtmap compare --before before.json --after after.json \
  --target-location src/main.rs:calculate_score:42

# Combine both for precise tracking
debtmap compare --before before.json --after after.json \
  --plan implementation-plan.json \
  --target-location src/analyzers/complexity.rs:analyze_function:128
```

**Use cases**:
- `--plan`: Track debt changes for planned refactoring tasks
- `--target-location`: Focus on specific function or code location
- Combine for granular technical debt tracking

### Incompatible Format Errors

**Problem**: "Incompatible formats" error when comparing files

**Causes**:
- Mixing legacy and unified JSON formats
- Files from different debtmap versions
- Corrupted JSON files

**Solutions**:
```bash
# Ensure both files use same output format
debtmap --format json --output-format unified --output before.json
# ... make changes ...
debtmap --format json --output-format unified --output after.json
debtmap compare --before before.json --after after.json

# Validate JSON files are well-formed
jq . before.json > /dev/null
jq . after.json > /dev/null
```

### Comparing Across Branches

```bash
# Save baseline on main branch
git checkout main
debtmap --format json --output main.json

# Switch to feature branch
git checkout feature-branch
debtmap --format json --output feature.json

# Compare branches
debtmap compare --before main.json --after feature.json
```

### Missing Files Error

**Problem**: "File not found" when running compare

**Solutions**:
- Verify file paths are correct (use absolute paths if needed)
- Ensure JSON files weren't moved or deleted
- Check current working directory with `pwd`

### Format Mismatch Issues

**Problem**: Compare shows unexpected differences or errors

**Solutions**:
```bash
# Regenerate both files with same debtmap version
debtmap --format json --output before.json
# ... make changes ...
debtmap --format json --output after.json

# Use same output format for both
debtmap --format json --output-format unified --output before.json
debtmap --format json --output-format unified --output after.json
```

## Validate Command Issues

The `validate` command checks if a codebase meets specified quality thresholds, useful for CI/CD pipelines.

### Basic Validation

```bash
# Validate codebase passes default thresholds
debtmap validate /path/to/project

# Exit code 0 if passes, non-zero if validation fails
```

### Debt Density Validation

**Flag**: `--max-debt-density <number>`

Sets the maximum acceptable technical debt per 1000 lines of code.

```bash
# Set maximum acceptable debt density (per 1000 LOC)
debtmap validate /path/to/project --max-debt-density 10.0

# Stricter threshold for critical projects
debtmap validate /path/to/project --max-debt-density 5.0

# Lenient threshold for legacy code
debtmap validate /path/to/project --max-debt-density 20.0
```

**Troubleshooting validation failures**:
```bash
# See which files exceed threshold with details
debtmap validate /path/to/project --max-debt-density 10.0 -v

# Get detailed breakdown of debt density calculations
debtmap validate /path/to/project --max-debt-density 10.0 -vv

# Analyze specific files that failed validation
debtmap /path/to/problematic/file.rs -v

# Understand debt density metric
# Debt density = (total_debt_score / total_lines_of_code) × 1000
# Example: 150 debt points across 10,000 LOC = 15.0 debt density
```

**Interpreting debt density values**:
- **< 5.0**: Excellent code quality
- **5.0 - 10.0**: Good, manageable technical debt
- **10.0 - 20.0**: Moderate debt, consider cleanup
- **> 20.0**: High debt, refactoring recommended

### CI/CD Integration

```bash
# In CI pipeline (fails build if validation fails)
debtmap validate . --max-debt-density 10.0 || exit 1

# With verbose output for debugging
debtmap validate . --max-debt-density 10.0 -v

# Save validation report
debtmap validate . --max-debt-density 10.0 --format json --output validation.json
```

**Use cases**:
- Enforce quality gates in CI/CD pipelines
- Prevent accumulation of technical debt over time
- Track debt density trends across releases
- Set different thresholds for different parts of codebase

## Validate-Improvement Command Issues

The `validate-improvement` command verifies that code changes actually reduced technical debt, useful for validating refactoring efforts.

### Basic Usage

```bash
# Validate that changes improved the codebase
debtmap validate-improvement \
  --comparison comparison.json \
  --output improvement-report.json

# Set minimum acceptable improvement threshold
debtmap validate-improvement \
  --comparison comparison.json \
  --threshold 5.0 \
  --output improvement-report.json
```

### Command Flags

```bash
# Specify comparison file from 'debtmap compare' output
debtmap validate-improvement --comparison comparison.json

# Set output file for validation results
debtmap validate-improvement \
  --comparison comparison.json \
  --output improvement-report.json

# Use previous validation for trend analysis
debtmap validate-improvement \
  --comparison comparison.json \
  --previous-validation previous-report.json

# Set minimum improvement threshold (percentage)
debtmap validate-improvement \
  --comparison comparison.json \
  --threshold 10.0  # Require 10% improvement

# Control output format (json, text, markdown)
debtmap validate-improvement \
  --comparison comparison.json \
  --format json

# Quiet mode (exit code only, no output)
debtmap validate-improvement \
  --comparison comparison.json \
  --quiet
```

### Typical Workflow

```bash
# Step 1: Save baseline before refactoring
debtmap --format json --output before.json

# Step 2: Make code changes...

# Step 3: Analyze after changes
debtmap --format json --output after.json

# Step 4: Compare results
debtmap compare --before before.json --after after.json \
  --format json --output comparison.json

# Step 5: Validate improvement
debtmap validate-improvement \
  --comparison comparison.json \
  --threshold 5.0 \
  --output validation.json

# Exit code 0 if improvement meets threshold, non-zero otherwise
```

### Common Issues

**Q: Validation fails but I fixed issues - why?**

A: Check what the validation is measuring:
```bash
# See detailed validation results (without --quiet)
debtmap validate-improvement \
  --comparison comparison.json \
  --format text

# Common reasons for failure:
# - Added new complexity elsewhere while fixing issues
# - Threshold too strict for the changes made
# - Comparison file doesn't reflect latest changes
# - File-level scores increased despite function improvements
```

**Q: How is improvement calculated?**

A: Improvement is measured as percentage reduction in total debt score:
```bash
# Formula: improvement = ((before_score - after_score) / before_score) × 100
#
# Example:
# - Before: total score = 100
# - After: total score = 80
# - Improvement: ((100 - 80) / 100) × 100 = 20%

# See detailed breakdown
debtmap validate-improvement \
  --comparison comparison.json \
  --format text -v
```

**Q: Can I track improvement over multiple refactorings?**

A: Yes, use `--previous-validation` for trend analysis:
```bash
# First validation
debtmap validate-improvement \
  --comparison refactor1-comparison.json \
  --output validation1.json

# Second validation references first
debtmap validate-improvement \
  --comparison refactor2-comparison.json \
  --previous-validation validation1.json \
  --output validation2.json

# Shows cumulative improvement trend
```

### CI/CD Integration

```bash
# In CI pipeline: enforce minimum improvement for refactoring PRs
debtmap validate-improvement \
  --comparison comparison.json \
  --threshold 5.0 \
  --quiet || exit 1

# With output for CI reporting
debtmap validate-improvement \
  --comparison comparison.json \
  --threshold 5.0 \
  --format json \
  --output improvement-report.json

# Archive validation reports for tracking
```

**Use cases**:
- Verify refactoring PRs actually reduce debt
- Enforce improvement thresholds in code review
- Track debt reduction trends over time
- Validate that tech debt fixes are effective
- Generate improvement metrics for reporting

### Troubleshooting Validation Failures

```bash
# Check the comparison file is valid
jq . comparison.json

# Verify before/after files were generated correctly
debtmap --format json --output before.json -v
# ... make changes ...
debtmap --format json --output after.json -v

# Lower threshold if being too strict
debtmap validate-improvement \
  --comparison comparison.json \
  --threshold 1.0  # Accept any improvement

# See detailed improvement breakdown
debtmap validate-improvement \
  --comparison comparison.json \
  --format markdown \
  --output improvement.md
```

## FAQ

### General Questions

**Q: Why is my analysis slow?**

A: Check several factors:
```bash
# Use all CPU cores
debtmap --jobs 0

# Try faster fallback mode
debtmap --semantic-off

# Check for large files or complex macros
debtmap -vv
```

**Q: What does 'Parse error' mean?**

A: File contains syntax debtmap cannot parse. Solutions:
- Try `--semantic-off` for fallback mode
- Use `--verbose-macro-warnings` for Rust macros
- Exclude problematic files in `.debtmap/config.toml`
- Report parse errors as potential bugs

**Q: Why do scores differ between runs?**

A: Several factors affect scores:
- Coverage file changed (use `--coverage-file`)
- Context providers enabled/disabled (`--context`)
- Code changes (intended behavior)
- Different threshold settings

**Q: How do I reduce noise in results?**

A: Use filtering options:
```bash
# Increase minimum priority
debtmap --min-priority 5

# Use threshold preset
debtmap --threshold-preset strict

# Filter categories
debtmap --filter "complexity,debt"

# Limit output
debtmap --top 20
```

### Format and Output

**Q: What's the difference between legacy and unified JSON?**

A: Two JSON output formats:
- **Legacy**: `{File: {...}}` - nested file-based structure
- **Unified**: Consistent structure with `type` field for each item

```bash
# Legacy (default)
debtmap --format json --output-format legacy

# Unified (recommended for parsing)
debtmap --format json --output-format unified
```

**Q: Can I analyze partial codebases?**

A: Yes, several approaches:
```bash
# Limit file count
debtmap --max-files 100

# Analyze specific directory
debtmap src/specific/module

# Use filters in config
# .debtmap/config.toml:
# include = ["src/**/*.rs"]
```

**Q: How is the 0-10 priority score calculated?**

A: Debtmap uses a multiplicative risk-based scoring formula to compute priority scores:

**Core Formula**:
```
Final Score = base_risk × debt_factor × complexity_factor ×
              coverage_penalty × coverage_factor
```

**Base Risk Calculation**:
```
complexity_component = (cyclomatic × 0.3 + cognitive × 0.45) / 50.0
coverage_component = (100 - coverage_percentage) / 100.0 × 0.5
base_risk = (complexity_component + coverage_component) × 5.0
```

**Coverage Penalty** (tiered based on test coverage):
- **< 20% coverage**: 3.0× penalty (critical)
- **20-40% coverage**: 2.0× penalty (high risk)
- **40-60% coverage**: 1.5× penalty (moderate risk)
- **60-80% coverage**: 1.2× penalty (low risk)
- **≥ 80% coverage**: 0.8× penalty (well tested - reduction)

**Coverage Factor** (additional reduction for well-tested code):
- **≥ 90% coverage**: 0.8 (20% score reduction)
- **70-90% coverage**: 0.9 (10% score reduction)
- **< 70% coverage**: 1.0 (no reduction)

**Role-Based Adjustments** (Evidence-Based Calculator):
- **Pure logic**: 1.2× (testable, maintainable code)
- **Entry points**: 1.1× (public API boundaries)
- **I/O wrappers**: 0.7× (thin delegation layers)

**Default Weights**:
- Coverage weight: 0.5
- Cyclomatic complexity weight: 0.3
- Cognitive complexity weight: 0.45
- Debt factor weight: 0.2

**Example**:
- Function: cyclomatic=15, cognitive=20, coverage=10%, role=entry_point
- Complexity component: (15 × 0.3 + 20 × 0.45) / 50 = 0.27
- Coverage component: (100 - 10) / 100 × 0.5 = 0.45
- Base risk: (0.27 + 0.45) × 5.0 = 3.6
- Coverage penalty: 3.0 (< 20% coverage)
- Coverage factor: 1.0 (< 70% coverage)
- Debt factor: ~1.2 (moderate debt patterns)
- Complexity factor: ~1.3 (pattern-adjusted)
- Final score: 3.6 × 1.2 × 1.3 × 3.0 × 1.0 × 1.1 (role) ≈ **18.5** (clamped to 10.0 scale)

```bash
# See score breakdown with verbosity
debtmap -v

# See detailed factor calculations including all multipliers
debtmap -vv
```

### Coverage and Testing

**Q: How does coverage affect scores?**

A: Coverage affects scores through two multiplicative factors in the risk calculation:

**1. Coverage Penalty** (tiered multiplier based on test coverage):
- **< 20% coverage**: 3.0× penalty (untested code gets highest priority)
- **20-40% coverage**: 2.0× penalty
- **40-60% coverage**: 1.5× penalty
- **60-80% coverage**: 1.2× penalty
- **≥ 80% coverage**: 0.8× reduction (well-tested code deprioritized)

**2. Coverage Factor** (additional reduction for well-tested code):
- **≥ 90% coverage**: 0.8 (20% score reduction)
- **70-90% coverage**: 0.9 (10% score reduction)
- **< 70% coverage**: 1.0 (no additional reduction)

**3. Base Risk Component** (coverage weight: 0.5):
- `coverage_component = (100 - coverage_percentage) / 100.0 × 0.5`
- Integrated into base risk calculation

**Combined Effect**:
Untested complex code (0% coverage) receives maximum penalties (3.0× coverage penalty), while well-tested code (≥90% coverage) receives both the 0.8× coverage penalty and 0.8× coverage factor, resulting in a 0.64× total reduction. This ensures untested code rises to the top of the priority list.

```bash
# Use coverage file
debtmap --coverage-file coverage.info

# See coverage impact on scoring
debtmap --coverage-file coverage.info -v

# See detailed coverage penalty and factor breakdown
debtmap --coverage-file coverage.info -vv
```

See the FAQ entry "How is the 0-10 priority score calculated?" for complete scoring formula details.

**Q: What's the difference between measured and estimated metrics?**

A: Debtmap provides both directly measured metrics and formula-based estimates:

**Measured Metrics** (from AST analysis):
- `cyclomatic_complexity`: Actual count of decision points in code
- `cognitive_complexity`: Weighted measure of code understandability
- `nesting_depth`: Maximum level of nested blocks
- `loc` (lines of code): Actual line count
- `parameters`: Number of function parameters
- `return_points`: Number of return statements

**Estimated Metrics** (formula-based):
- `est_branches`: Estimated branch count for testing effort
  - Formula: `max(nesting_depth, 1) × cyclomatic_complexity ÷ 3`
  - Not an actual count of branches in the AST
  - Represents estimated testing complexity/effort
  - Useful for understanding test coverage needs

```bash
# See all metrics including estimates
debtmap -vv

# Example output:
# cyclomatic_complexity: 15    (measured from AST)
# cognitive_complexity: 20     (measured from AST)
# nesting_depth: 4             (measured from AST)
# est_branches: 20             (estimated: max(4,1) × 15 ÷ 3 = 20)
```

**When to trust estimated metrics**:
- Comparing relative complexity between functions
- Estimating testing effort
- Understanding potential branching scenarios

**When to rely on measured metrics**:
- Precise complexity analysis
- Setting hard thresholds
- Exact cyclomatic/cognitive complexity values

### Context and Analysis

**Q: What are context providers?**

A: Additional analysis for prioritization:
- **critical_path**: Call graph analysis, entry point distance
- **dependency**: Dependency relationships and coupling
- **git_history**: Change frequency and authorship

```bash
# Enable all
debtmap --context

# Specific providers
debtmap --context --context-providers critical_path,dependency

# See context impact
debtmap --context -v
```

### Results and Comparison

**Q: Why no output?**

A: Check verbosity and filtering:
```bash
# Increase verbosity
debtmap -v

# Lower priority threshold
debtmap --min-priority 0

# Check if files were analyzed
debtmap -vv 2>&1 | grep "Processed"

# Ensure not using strict threshold
debtmap --threshold-preset lenient
```

**Q: How to compare results over time?**

A: Use the `compare` command:
```bash
# Save baseline
debtmap --format json --output before.json

# Make changes...

# Analyze again
debtmap --format json --output after.json

# Compare
debtmap compare --before before.json --after after.json
```

**Q: Why does compare fail with 'incompatible formats'?**

A: The JSON files must use the same output format:
```bash
# Use unified format for both
debtmap --format json --output-format unified --output before.json
# ... make changes ...
debtmap --format json --output-format unified --output after.json
debtmap compare --before before.json --after after.json

# Or use legacy format for both (but unified is recommended)
debtmap --format json --output-format legacy --output before.json
debtmap --format json --output-format legacy --output after.json
```

**Q: How do I compare results from different branches?**

A: Generate JSON output on each branch and compare:
```bash
# On main branch
git checkout main
debtmap --format json --output main.json

# On feature branch
git checkout feature-branch
debtmap --format json --output feature.json

# Compare (from either branch)
debtmap compare --before main.json --after feature.json
```

**Q: Can I compare legacy and unified JSON formats?**

A: No, both files must use the same format. Regenerate with matching formats:
```bash
# Convert both to unified format
debtmap --format json --output-format unified --output before.json
debtmap --format json --output-format unified --output after.json
debtmap compare --before before.json --after after.json
```

### Performance and Optimization

**Q: How many threads should I use?**

A: Depends on your machine:
```bash
# Use all cores (default, recommended)
debtmap --jobs 0

# Limit to 4 threads (if other work running)
debtmap --jobs 4

# Single threaded (debugging only)
debtmap --no-parallel
```

## When to File Bug Reports

File a bug report when:

✅ **These are bugs**:
- Parse errors on valid syntax
- Crashes or panics
- Incorrect complexity calculations
- Concurrency errors
- Incorrect error messages

❌ **These are not bugs**:
- Unsupported language constructs (file feature request)
- Disagreement with complexity scores (subjective)
- Performance on very large codebases (optimization request)
- Missing documentation (docs issue, not code bug)

### How to Report Issues

1. **Reproduce with minimal example**
2. **Include debug output**: `debtmap -vvv 2>&1 | tee error.log`
3. **Include version**: `debtmap --version`
4. **Include platform**: OS, Rust version if relevant
5. **Include configuration**: `.debtmap/config.toml` if used
6. **Expected vs actual behavior**

### Before Filing

1. Check this troubleshooting guide
2. Try `--semantic-off` fallback mode
3. Update to latest version
4. Search existing issues on GitHub

## Related Documentation

- **[Configuration Guide](./configuration.md)**: Configure debtmap behavior
- **[CLI Reference](./cli-reference.md)**: Complete CLI flag documentation
- **[Analysis Guide](./analysis-guide.md)**: Understanding analysis results
- **[Examples](./examples.md)**: Practical usage examples
- **[API Documentation](./api/index.html)**: Rust API documentation

## Troubleshooting Checklist

When debugging issues, work through this checklist:

- [ ] Run with `-vv` to see detailed output
- [ ] Try `--semantic-off` to use fallback mode
- [ ] Check file permissions and paths
- [ ] Verify configuration in `.debtmap/config.toml`
- [ ] Test with `--max-files 10` to isolate issues
- [ ] Try `--no-parallel` to rule out concurrency
- [ ] Check `debtmap --version` for updates
- [ ] Review error messages in this guide
- [ ] Search GitHub issues for similar problems
- [ ] Create minimal reproduction case
- [ ] File bug report with debug output
