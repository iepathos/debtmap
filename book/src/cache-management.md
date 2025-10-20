# Cache Management

Debtmap includes an intelligent cache system that speeds up repeated analyses by caching parsed ASTs and analysis results. This chapter covers cache configuration, strategies, and troubleshooting.

## Overview

The cache system stores analysis results to avoid re-analyzing unchanged files. When you run debtmap multiple times on the same codebase, the cache can reduce analysis time by 50-90% for unchanged files.

## Cache Location

Debtmap automatically chooses a cache location based on your platform:

**Priority order:**
1. `$DEBTMAP_CACHE_DIR` environment variable (if set)
2. `$XDG_CACHE_HOME/debtmap` (Linux/Unix)
3. `~/Library/Caches/debtmap` (macOS)
4. `%APPDATA%/debtmap/cache` (Windows)

You can override the location by setting the `DEBTMAP_CACHE_DIR` environment variable:

```bash
export DEBTMAP_CACHE_DIR=/custom/path/to/cache
```

## Configuration

Configure cache behavior using environment variables:

### Cache Size and Limits

```bash
# Maximum cache size (default: 1GB)
export DEBTMAP_CACHE_MAX_SIZE=1073741824

# Maximum number of cache entries (default: 10000)
export DEBTMAP_CACHE_MAX_ENTRIES=10000

# Maximum age for cache entries in days (default: 30)
export DEBTMAP_CACHE_MAX_AGE_DAYS=30
```

### Automatic Pruning

```bash
# Enable/disable automatic pruning (default: true)
export DEBTMAP_CACHE_AUTO_PRUNE=true

# Percentage to remove when limits hit (default: 0.25 = 25%)
export DEBTMAP_CACHE_PRUNE_PERCENTAGE=0.25
```

## Cache Strategies

Debtmap supports multiple cache pruning strategies:

### LRU (Least Recently Used) - Default

Removes cache entries that haven't been accessed recently.

```bash
export DEBTMAP_CACHE_STRATEGY=lru
```

**Best for:** Active projects with frequent analysis runs.

### LFU (Least Frequently Used)

Removes cache entries with the lowest access count.

```bash
export DEBTMAP_CACHE_STRATEGY=lfu
```

**Best for:** Projects with stable core files and frequently changing peripheral files.

### FIFO (First In, First Out)

Removes the oldest cache entries based on creation time.

```bash
export DEBTMAP_CACHE_STRATEGY=fifo
```

**Best for:** Projects where newer analysis is always more relevant.

### Age-Based

Only removes entries older than the configured maximum age.

```bash
export DEBTMAP_CACHE_STRATEGY=age_based
```

**Best for:** Projects with strict cache freshness requirements.

## Cache Commands

### View Cache Statistics

```bash
debtmap analyze . --cache-stats
```

Shows:
- Cache location
- Total size
- Number of entries
- Hit/miss ratio
- Oldest and newest entries

### Clear Cache

```bash
debtmap analyze . --clear-cache
```

Removes all cache entries before running analysis.

### Disable Cache for Single Run

```bash
debtmap analyze . --no-cache
```

Runs analysis without using or updating the cache.

### Force Cache Rebuild

```bash
debtmap analyze . --force-cache-rebuild
```

Invalidates all cache entries and rebuilds from scratch.

## Performance Optimization

### Tuning for Your Project

**Large codebase (>100k LOC):**
```bash
export DEBTMAP_CACHE_MAX_SIZE=5368709120  # 5GB
export DEBTMAP_CACHE_MAX_ENTRIES=50000
export DEBTMAP_CACHE_STRATEGY=lru
```

**Small codebase (<10k LOC):**
```bash
export DEBTMAP_CACHE_MAX_SIZE=104857600  # 100MB
export DEBTMAP_CACHE_MAX_ENTRIES=1000
export DEBTMAP_CACHE_STRATEGY=age_based
```

**CI/CD pipeline:**
```bash
# Disable cache in CI (or use shared cache location)
export DEBTMAP_CACHE_AUTO_PRUNE=false
```

## Troubleshooting

### Cache Taking Too Much Disk Space

**Solution 1:** Reduce cache size limit
```bash
export DEBTMAP_CACHE_MAX_SIZE=536870912  # 512MB
```

**Solution 2:** Clear cache manually
```bash
debtmap analyze . --clear-cache
```

**Solution 3:** Enable aggressive pruning
```bash
export DEBTMAP_CACHE_PRUNE_PERCENTAGE=0.5  # Remove 50% when limit hit
```

### Cache Not Improving Performance

**Possible causes:**
1. Files are changing frequently (cache invalidation)
2. Cache location is on slow storage (network drive, slow disk)
3. Cache is disabled or cleared on every run

**Solutions:**
- Check cache hit ratio with `--cache-stats`
- Move cache to faster storage with `DEBTMAP_CACHE_DIR`
- Ensure `--no-cache` or `--clear-cache` are not set

### Orphan Cache Entries

Debtmap automatically cleans up cache entries for deleted files during pruning. To force cleanup:

```bash
debtmap analyze . --force-cache-rebuild
```

## Best Practices

1. **Use default settings** - They work well for most projects
2. **Monitor cache size** - Check periodically with `--cache-stats`
3. **Clear cache after major refactoring** - Use `--force-cache-rebuild`
4. **Use shared cache in CI** - Set `DEBTMAP_CACHE_DIR` to shared location
5. **Disable cache for one-time analyses** - Use `--no-cache` for ad-hoc runs

## See Also

- [Parallel Processing](parallel-processing.md) - Thread configuration for performance
- [Configuration](configuration.md) - Project-specific settings
- [Troubleshooting](troubleshooting.md) - General troubleshooting guide
