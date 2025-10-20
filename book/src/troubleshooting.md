# Troubleshooting

Common issues and solutions for using debtmap effectively.

## Quick Fixes for Common Issues

If you're experiencing problems, try these first:

1. **Analysis is slow**: Check `--cache-stats`, ensure caching is enabled, adjust threads with `--jobs`
2. **Parse errors**: Use `--semantic-off` for faster fallback mode or exclude problematic files
3. **No output**: Increase verbosity with `-v` or lower `--min-priority`
4. **Cache corruption**: Run with `--clear-cache` to rebuild
5. **Inconsistent results**: Check if coverage file changed or context providers are enabled

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
# Check cache statistics
debtmap --cache-stats

# Ensure caching is enabled (it is by default)
# If cache was disabled, remove --no-cache flag

# Use all available CPU cores
debtmap --jobs 0

# Try faster fallback mode (less accurate)
debtmap --semantic-off

# Use plain output for faster terminal rendering
debtmap --plain
```

See [Performance Tips](#performance-tips) for detailed optimization strategies.

### Cache Corruption

**Problem**: Getting "Cache error" messages or stale results

**Solutions**:
```bash
# Clear cache and rebuild
debtmap --clear-cache

# Force cache rebuild
debtmap --force-cache-rebuild

# Check cache status
debtmap --cache-stats

# Use different cache location
debtmap --cache-location /path/to/cache
```

See [Cache Troubleshooting](#cache-troubleshooting) for more details.

### File Permission Errors

**Problem**: "File system error" when accessing files

**Solutions**:
- Ensure you have read permissions for all source files
- Check that the project directory is accessible
- Verify cache directory is writable (default: `.debtmap/cache`)
- Use `--cache-location` to specify an accessible cache directory

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
- `-vvv`: Full debug output, context provider details, cache operations

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

# Show cache statistics
debtmap --cache-stats
```

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

# Step 2: Check cache stats
debtmap --cache-stats

# Step 3: Try without semantic analysis
debtmap --semantic-off -v

# Step 4: Check specific file
debtmap path/to/file.rs -vvv

# Step 5: Validate results
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

### Caching Strategies

Caching is **enabled by default** and provides the biggest performance improvement.

**Note**: The `--cache` flag (to enable caching) is deprecated and hidden. Caching is now always enabled by default; use `--no-cache` to disable it.

```bash
# Check cache effectiveness
debtmap --cache-stats

# Clear cache if corrupted
debtmap --clear-cache

# Force cache rebuild
debtmap --force-cache-rebuild

# Disable cache (not recommended)
debtmap --no-cache
```

**Cache locations**:
```bash
# Local cache (default): .debtmap/cache
debtmap

# Shared cache for multiple projects
debtmap --cache-location ~/.cache/debtmap

# Migrate existing cache to shared location
debtmap --migrate-cache

# Set via environment variable
export DEBTMAP_CACHE_DIR=~/.cache/debtmap
debtmap
```

**Cache best practices**:
1. Use shared cache for multiple similar projects
2. Monitor cache size with `--cache-stats` periodically
3. Clear cache after major refactorings
4. Use local cache for project-specific configurations

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
| Default (cached) | Fast | High |
| `--no-cache` | Slow | High |
| `--semantic-off` | Fastest | Medium |
| `--no-parallel` | Slowest | High |
| `--jobs 4` | Medium | High |

### Monitoring Performance

```bash
# Time analysis
time debtmap

# Check cache hit rate
debtmap --cache-stats

# Profile with verbosity
debtmap -vv 2>&1 | grep "processed in"
```

## Cache Troubleshooting

Detailed guidance for cache-related issues.

### Check Cache Status

```bash
# View cache statistics
debtmap --cache-stats

# Sample output:
# Cache location: .debtmap/cache
# Cache entries: 1,234
# Cache size: 45.2 MB
# Hit rate: 87.3%
```

### Clear Corrupted Cache

```bash
# Clear all cache entries
debtmap --clear-cache

# Force rebuild on next run
debtmap --force-cache-rebuild

# Manual cache deletion
rm -rf .debtmap/cache
# or for shared cache:
rm -rf ~/.cache/debtmap
```

### Cache Location Management

```bash
# Use local cache (default)
debtmap
# Cache at: .debtmap/cache

# Use shared cache
debtmap --cache-location ~/.cache/debtmap

# Set permanently via environment
export DEBTMAP_CACHE_DIR=~/.cache/debtmap
debtmap

# Migrate existing cache
debtmap --migrate-cache
```

### Cache Pruning Configuration

Debtmap automatically manages cache size and age to prevent unbounded growth:

**Environment Variables**:
```bash
# Enable/disable automatic cache pruning (default: true)
export DEBTMAP_CACHE_AUTO_PRUNE=true

# Maximum cache size in bytes (default: 1GB)
export DEBTMAP_CACHE_MAX_SIZE=1073741824

# Maximum cache entry age in days (default: 30)
export DEBTMAP_CACHE_MAX_AGE_DAYS=30

# Maximum number of cache entries (default: 10000)
export DEBTMAP_CACHE_MAX_ENTRIES=10000

# Percentage of entries to remove when pruning (default: 0.25)
export DEBTMAP_CACHE_PRUNE_PERCENTAGE=0.25

# Pruning strategy: lru, lfu, fifo, age_based (default: lru)
export DEBTMAP_CACHE_STRATEGY=lru
```

**Pruning Strategies**:
- **LRU (Least Recently Used)**: Removes oldest accessed entries (recommended for general use)
- **LFU (Least Frequently Used)**: Removes least accessed entries (best for stable workloads)
- **FIFO (First In First Out)**: Removes oldest created entries (simple, useful for testing)
- **Age-based**: Removes entries older than MAX_AGE_DAYS (useful for compliance requirements)

**Performance Implications**:
- LRU/LFU provide better cache hit rates than FIFO
- Age-based strategy ensures data freshness but may reduce hit rate
- Adjust MAX_SIZE based on available disk space and project count
- Lower PRUNE_PERCENTAGE means more frequent but smaller cleanup operations

### Cache Strategies

**Local cache** (`.debtmap/cache`):
- **Pros**: Isolated per project, automatically managed
- **Cons**: Duplicates across projects

**Shared cache** (`~/.cache/debtmap`):
- **Pros**: Shared across projects, saves disk space
- **Cons**: Requires manual management, may mix unrelated projects

### Cache Consistency

```bash
# Validate LOC consistency
debtmap --validate-loc

# Cache automatically invalidates on file changes
# Uses file hashes to detect modifications

# Force fresh analysis
debtmap --no-cache
```

### Cache Size Monitoring

```bash
# Check cache size
debtmap --cache-stats

# Clean up old entries (manual)
# No automatic cleanup - manage cache size manually
# Consider clearing cache periodically for large projects
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

**Flag**: `--attribution`

Shows attribution information for detected issues.

```bash
# Enable attribution output
debtmap --attribution

# Combine with verbosity for details
debtmap --attribution -v
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

```bash
# Disable god object detection
debtmap --no-god-object

# Useful if false positives on legitimately large modules
# Or if your architecture uses centralized classes
```

**When to use**:
- False positives on framework files
- Intentional large aggregator classes
- Reducing noise in results

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

### Tiered Prioritization Issues

**Overview**: Debtmap uses a 4-tier system to classify technical debt items by priority:

**Tier Classification**:
- **Tier 1 (Critical Architecture)**: High complexity, low coverage, high dependencies, entry points
- **Tier 2 (Complex Untested)**: Significant complexity or coverage gaps
- **Tier 3 (Testing Gaps)**: Moderate issues that need attention
- **Tier 4 (Maintenance)**: Low-priority items, routine maintenance

**Tier Weights**:
- Tier 1: 1.5× multiplier (highest priority)
- Tier 2: 1.0× multiplier (standard priority)
- Tier 3: 0.7× multiplier (reduced priority)
- Tier 4: 0.3× multiplier (lowest priority)

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

**Overview**: Debtmap aggregates function/class scores into file-level scores using a multi-factor formula.

**File Score Formula**:
```
File Score = size_factor × complexity_factor × coverage_factor ×
             density_factor × god_object_multiplier × function_scores
```

**Factors**:
- **size_factor**: Based on lines of code (larger files = higher factor)
- **complexity_factor**: Average complexity across functions
- **coverage_factor**: File-level test coverage (from LCOV file)
- **density_factor**: Penalizes files with >50 functions (indicates code smell)
- **god_object_multiplier**: 1.5× for files exceeding god object thresholds

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

### Combining Advanced Flags

```bash
# Comprehensive analysis with all features
debtmap --multi-pass --attribution --context -vv

# Minimal filtering for exploration
debtmap --min-problematic 1 --min-priority 0 --no-god-object

# Performance-focused advanced analysis
debtmap --multi-pass --jobs 8 --cache-location ~/.cache/debtmap

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
- Verify cache directory is writable
- Use `--cache-location` for accessible directory

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

### Cache Errors

**Message**: `Cache error: corrupted cache entry`

**Meaning**: Cache data is invalid or corrupted

**Solutions**:
```bash
# Clear cache
debtmap --clear-cache

# Force rebuild
debtmap --force-cache-rebuild

# Use different cache location
debtmap --cache-location /tmp/debtmap-cache
```

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
# See which files exceed threshold
debtmap validate /path/to/project --max-debt-density 10.0 -v

# Get detailed breakdown
debtmap validate /path/to/project --max-debt-density 10.0 -vv

# Analyze specific files that failed
debtmap /path/to/problematic/file.rs -v
```

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

## FAQ

### General Questions

**Q: Why is my analysis slow?**

A: Check several factors:
```bash
# Check cache status
debtmap --cache-stats

# Ensure caching is enabled (default)
# Remove --no-cache if present

# Use all CPU cores
debtmap --jobs 0

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
- Cache was cleared (`--clear-cache`)
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

A: Debtmap uses a multi-factor unified scoring formula to compute priority scores:

**Base Score Formula**:
```
Base Score = (Complexity Factor × 0.40) +
             (Coverage Factor × 0.40) +
             (Dependency Factor × 0.20)
```

**Where**:
- **Complexity Factor**: Normalized cognitive complexity (0.0-10.0 scale)
- **Coverage Factor**: `(1 - coverage_percentage) × 2 + 1` (ranges from 1.0-3.0)
- **Dependency Factor**: Based on incoming/outgoing dependencies and coupling

**Role Multipliers** adjust the base score by function role:
- **Entry points**: 1.5× (critical path, highest priority)
- **Business logic**: 1.2× (core functionality)
- **Data access**: 1.0× (important but stable)
- **Infrastructure**: 0.8× (lower priority)
- **Utilities**: 0.5× (minimal priority)
- **Test code**: 0.1× (lowest priority)

**Final Score**:
```
Final Score = Base Score × Role Multiplier
```

This ensures functions are prioritized by true impact, not just raw complexity. Entry points with high complexity and low coverage get the highest scores, while well-tested utility functions get lower scores.

**Example**:
- Function with complexity=20, coverage=0%, dependencies=5, role=entry_point
- Complexity factor: 8.0 (normalized)
- Coverage factor: 3.0 (untested)
- Dependency factor: 2.5 (moderate coupling)
- Base score: (8.0 × 0.40) + (3.0 × 0.40) + (2.5 × 0.20) = 4.9
- Final score: 4.9 × 1.5 (entry point) = **7.35**

```bash
# See score breakdown with verbosity
debtmap -v

# See detailed factor calculations
debtmap -vv
```

### Coverage and Testing

**Q: How does coverage affect scores?**

A: Coverage is integrated into the unified scoring system as a factor:
- **Coverage Factor Formula**: `(1 - coverage_percentage) × 2 + 1`
- This factor is weighted at 40% in the unified scoring formula
- **Base Score** = (Complexity × 0.40) + (Coverage × 0.40) + (Dependency × 0.20)
- **Final Score** = Base Score × Role Multiplier

This means untested code (0% coverage) gets the highest coverage factor (3.0), while fully tested code (100% coverage) gets the lowest (1.0), ensuring untested complex code rises to the top.

```bash
# Use coverage file
debtmap --coverage-file coverage.info

# See coverage impact
debtmap --coverage-file coverage.info -v
```

See the FAQ entry "How is the 0-10 priority score calculated?" for complete scoring formula details.

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

**Q: Should I use shared or local cache?**

A: Depends on your workflow:
- **Local cache** (`.debtmap/cache`): Isolated, automatic
- **Shared cache** (`~/.cache/debtmap`): Saves space across projects

```bash
# Shared cache
debtmap --cache-location ~/.cache/debtmap

# Set permanently
export DEBTMAP_CACHE_DIR=~/.cache/debtmap
```

## When to File Bug Reports

File a bug report when:

✅ **These are bugs**:
- Parse errors on valid syntax
- Crashes or panics
- Incorrect complexity calculations
- Cache corruption
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
3. Clear cache with `--clear-cache`
4. Update to latest version
5. Search existing issues on GitHub

## Related Documentation

- **[Configuration Guide](./configuration.md)**: Configure debtmap behavior
- **[CLI Reference](./cli-reference.md)**: Complete CLI flag documentation
- **[Analysis Guide](./analysis-guide.md)**: Understanding analysis results
- **[Examples](./examples.md)**: Practical usage examples
- **[API Documentation](./api/index.html)**: Rust API documentation

## Troubleshooting Checklist

When debugging issues, work through this checklist:

- [ ] Run with `-vv` to see detailed output
- [ ] Check `--cache-stats` for cache issues
- [ ] Try `--clear-cache` to rule out cache corruption
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
