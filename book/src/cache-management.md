# Cache Management

Debtmap includes a comprehensive caching system designed to significantly speed up repeated analyses, particularly beneficial for large codebases and CI/CD pipelines. The cache stores parsed ASTs, call graphs, analysis results, and file metrics to avoid redundant computation.

## Cache Location and Configuration

Debtmap uses a platform-specific, XDG-compliant cache directory structure by default. The cache location is determined by the following priority order:

1. **`DEBTMAP_CACHE_DIR`** environment variable (if set)
2. **`XDG_CACHE_HOME/debtmap`** (if XDG_CACHE_HOME is set)
3. **Platform-specific defaults:**
   - macOS: `~/Library/Caches/debtmap`
   - Linux: `~/.cache/debtmap`
   - Windows: `%LOCALAPPDATA%\debtmap`
4. **Fallback:** System temporary directory

### Cache Strategy

Debtmap supports two cache storage strategies:

- **Shared (default)**: Stores cache in XDG-compliant shared directory (maps to `CacheStrategy::Shared` in code)
- **Custom**: Stores cache in user-specified location via `DEBTMAP_CACHE_DIR` (maps to `CacheStrategy::Custom` in code)

### Project Identification

To ensure cache isolation between different projects, debtmap generates a unique project ID using:

1. **Git remote URL hash** (preferred): SHA256 hash (first 16 characters) of the git remote origin URL
2. **Absolute path hash** (fallback): SHA256 hash (first 16 characters) of the project's absolute path

This ensures that different projects never share cached data, even when analyzed from the same machine.

## Cache Directory Structure

The cache directory contains several subdirectories, each serving a specific purpose:

```
debtmap/
└── projects/
    └── <project-id>/
        ├── call_graphs/     # Call graph computation results
        ├── analysis/        # Analysis results and metrics
        ├── metadata/        # Cache indices and metadata
        ├── temp/            # Temporary files created during analysis operations
        └── file_metrics/    # File-level complexity scores
```

## Environment Variables

Debtmap provides extensive cache configuration through environment variables:

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `DEBTMAP_CACHE_DIR` | Custom cache directory path | Platform-specific | `/tmp/my-cache` |
| `DEBTMAP_CACHE_AUTO_PRUNE` | Enable automatic cache pruning | `true` | `true` or `false` |
| `DEBTMAP_CACHE_MAX_SIZE` | Maximum cache size in bytes | `1073741824` (1GB) | `524288000` (500MB) |
| `DEBTMAP_CACHE_MAX_AGE_DAYS` | Maximum age for cache entries | `30` | `7` |
| `DEBTMAP_CACHE_MAX_ENTRIES` | Maximum number of cache entries | `10000` | `5000` |
| `DEBTMAP_CACHE_PRUNE_PERCENTAGE` | Percentage to remove when pruning (0.0-1.0) | `0.25` (25%) | `0.3` (30%) |
| `DEBTMAP_CACHE_STRATEGY` | Pruning strategy (lru, lfu, fifo, age). Note: 'age_based' is accepted as an alias for 'age' | `lru` | `lfu` |
| `DEBTMAP_CACHE_SYNC_PRUNE` | Use synchronous pruning (blocks) | `false` | `true` |
| `DEBTMAP_CACHE_SCOPE` | Branch-specific cache scope | None | `feature-branch` |

### Configuration Examples

```bash
# Use a custom cache location
export DEBTMAP_CACHE_DIR=/mnt/fast-ssd/debtmap-cache

# Configure cache limits for CI environment
export DEBTMAP_CACHE_MAX_SIZE=524288000  # 500MB
export DEBTMAP_CACHE_MAX_AGE_DAYS=7      # 1 week
export DEBTMAP_CACHE_STRATEGY=lru

# Disable auto-pruning (manual control)
export DEBTMAP_CACHE_AUTO_PRUNE=false

# Branch-specific caching
# Creates isolated cache namespaces for different branches, useful when
# switching between branches with different code states
export DEBTMAP_CACHE_SCOPE="$(git branch --show-current)"
```

## Command Line Options

Debtmap provides several CLI flags for cache management:

| Option | Description |
|--------|-------------|
| `--no-cache` | Disable caching for this run (caching is enabled by default) |
| `--clear-cache` | Clear cache before running analysis |
| `--force-cache-rebuild` | Force cache rebuild (same as `--clear-cache`) |
| `--cache-stats` | Show cache statistics and location |
| `--migrate-cache` | Migrate cache from local to shared location |
| `--cache-location <LOCATION>` | Cache location strategy: `local`, `shared`, or custom path. Sets DEBTMAP_CACHE_DIR for this run. |

> **Note:** The `--cache` flag (used to enable caching) is deprecated and hidden. Caching is now enabled by default; use `--no-cache` to disable it. This flag exists only for backward compatibility with older scripts.

### CLI Examples

```bash
# Run analysis without using cache
debtmap analyze . --no-cache

# Clear cache and rebuild from scratch
debtmap analyze . --clear-cache

# View cache statistics
debtmap analyze . --cache-stats

# Use custom cache location for this run
debtmap analyze . --cache-location /tmp/temp-cache

# Or use shared strategy
debtmap analyze . --cache-location shared

# Migrate existing cache to shared location
debtmap analyze . --migrate-cache
```

## Automatic Pruning Strategies

Debtmap automatically prunes the cache when configured limits are exceeded. Four pruning strategies are available:

### LRU (Least Recently Used) - Default

Removes entries that haven't been accessed recently. Best for general-purpose usage where recently analyzed code is more likely to be analyzed again.

**When to use:** Default choice for most development workflows and CI pipelines.

```bash
export DEBTMAP_CACHE_STRATEGY=lru
```

### LFU (Least Frequently Used)

Removes entries with the lowest access count. Best when certain files are analyzed repeatedly while others are analyzed infrequently.

**When to use:** Projects with stable core modules that are analyzed frequently and peripheral code that changes rarely.

```bash
export DEBTMAP_CACHE_STRATEGY=lfu
```

### FIFO (First In First Out)

Removes the oldest entries by creation time. Simpler strategy that doesn't consider access patterns.

**When to use:** When you want predictable cache behavior or are testing cache performance.

```bash
export DEBTMAP_CACHE_STRATEGY=fifo
```

### Age-Based Only

Only removes entries older than `DEBTMAP_CACHE_MAX_AGE_DAYS`. Does not prune based on size or entry count limits - this means the cache can grow unbounded if all entries are recent.

**When to use:** When disk space is not a concern but you want to ensure cache freshness. Note that this strategy ignores size and count thresholds entirely.

```bash
export DEBTMAP_CACHE_STRATEGY=age
```

## Default Configuration

When no environment variables are set, debtmap uses the following defaults:

- **Max size:** 1GB (1,073,741,824 bytes)
- **Max age:** 30 days
- **Max entries:** 10,000 entries
- **Prune percentage:** 25% (removes 25% of entries when limit is hit)
- **Strategy:** LRU (Least Recently Used)
- **Auto-prune:** Enabled

### Pruning Triggers

Automatic pruning is triggered when:

1. **Cache size exceeds max_size_bytes** - Immediate pruning
2. **Entry count exceeds max_entries** - Immediate pruning
3. **Entries older than max_age_days exist** - Periodic pruning (checked daily)

When pruning is triggered, debtmap removes enough entries to bring the cache below the configured limits, plus an additional buffer (based on prune_percentage) to avoid frequent pruning.

## Performance Optimization

### Cache Benefits

The cache system provides significant performance improvements by storing various analysis components:

- **Call graphs** (stored in `call_graphs/`): Reuse expensive call graph computation
- **Analysis results** (stored in `analysis/`): Skip redundant metric calculations
- **File metrics** (stored in `file_metrics/`): Cache file-level complexity scores
- **Metadata** (stored in `metadata/`): Cache indices and project metadata

**Performance impact:** Cache hits can reduce analysis time by 50-90% for large codebases, depending on the number of changed files.

### Best Practices for CI Environments

```bash
# Example CI configuration for fast builds
export DEBTMAP_CACHE_DIR=/ci-cache/debtmap
export DEBTMAP_CACHE_MAX_SIZE=2147483648  # 2GB for CI
export DEBTMAP_CACHE_MAX_AGE_DAYS=14      # 2 weeks
export DEBTMAP_CACHE_STRATEGY=lru

# Run analysis with cache
debtmap analyze . --cache-stats  # Show cache hit rate
```

### Background vs Synchronous Pruning

By default, cache pruning runs in a background thread, allowing analysis to continue without waiting for cleanup. This is optimal for development and CI environments.

For testing or when you need deterministic behavior, use synchronous pruning:

```bash
export DEBTMAP_CACHE_SYNC_PRUNE=true
```

**Synchronous pruning:** Blocks during cleanup, ensuring cache is pruned before analysis continues. Used automatically in test environments.

**Background pruning (default):** Spawns a separate thread for non-blocking cleanup. Analysis proceeds immediately while cleanup happens in parallel.

## Troubleshooting Cache Issues

### Cache Taking Too Much Disk Space

**Problem:** Cache directory is consuming excessive disk space.

**Solutions:**

```bash
# Option 1: Reduce max cache size
export DEBTMAP_CACHE_MAX_SIZE=524288000  # 500MB

# Option 2: Clear cache manually
debtmap analyze . --clear-cache

# Option 3: Reduce max age
export DEBTMAP_CACHE_MAX_AGE_DAYS=7

# Option 4: Inspect current cache usage
debtmap analyze . --cache-stats
```

### Stale Cache Causing Incorrect Results

**Problem:** Cache contains outdated data, causing analysis to report incorrect results.

**Solutions:**

```bash
# Force cache rebuild
debtmap analyze . --force-cache-rebuild

# Or disable cache for this run
debtmap analyze . --no-cache
```

### Permission Errors

**Problem:** Cannot write to cache directory.

**Solutions:**

```bash
# Use a custom cache location with proper permissions
export DEBTMAP_CACHE_DIR=$HOME/.local/cache/debtmap

# Or check permissions on the default cache directory
ls -la $(debtmap analyze . --cache-stats | grep "Cache location")
```

### Inspecting Cache Statistics

Use `--cache-stats` to inspect cache health:

```bash
debtmap analyze . --cache-stats
```

This displays:
- Cache location path
- Total cache size
- Number of cached entries
- Cache hit rate (if available)
- Last pruning timestamp

### Debug Cache Issues

To debug cache-related issues, check:

1. **Cache directory exists and is writable:**
   ```bash
   ls -la ~/.cache/debtmap  # Linux
   ls -la ~/Library/Caches/debtmap  # macOS
   ```

2. **Environment variables are set correctly:**
   ```bash
   env | grep DEBTMAP_CACHE
   ```

3. **Project ID generation:**
   Cache keys are based on project ID. Verify your project has a stable git remote or path.

4. **File timestamps:**
   Cache invalidation relies on file modification times. Ensure your build system doesn't modify timestamps unexpectedly.

## Cache Migration

If you previously used a local cache strategy and want to migrate to the shared XDG-compliant location:

```bash
debtmap analyze . --migrate-cache
```

This command:
1. Identifies the old cache location
2. Creates the new shared cache directory
3. Copies cache data preserving metadata
4. Verifies the migration was successful

After migration, you can safely delete the old cache directory.
