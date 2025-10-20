# Parallel Processing

Debtmap is built with Rust and Rayon for high-performance parallel analysis. This chapter covers thread configuration, performance tuning, and scaling strategies.

## Overview

Debtmap uses parallel processing to analyze multiple files simultaneously, making it **10-100x faster** than Java or Python-based competitors. The parallel architecture is built on:

- **Rayon** - Data parallelism with work-stealing thread pools
- **DashMap** - Lock-free concurrent hash maps
- **Arc** - Thread-safe reference counting for shared data

## Performance Characteristics

**Typical analysis times:**
- Small project (1k-5k LOC): <1 second
- Medium project (10k-50k LOC): 2-8 seconds
- Large project (100k-500k LOC): 10-45 seconds

**Comparison with other tools (medium-sized Rust project, ~50k LOC):**
- SonarQube: 3-4 minutes
- CodeClimate: 2-3 minutes
- Debtmap: 5-8 seconds

## Thread Configuration

### Default Behavior

By default, debtmap uses all available CPU cores:

```bash
debtmap analyze .
```

### Controlling Thread Count

Limit the number of worker threads:

```bash
# Use 4 threads
debtmap analyze . --jobs 4

# Use 1 thread (disable parallelism)
debtmap analyze . --no-parallel
```

**When to limit threads:**
- Running on shared infrastructure (CI/CD)
- Low-memory environments
- Debugging analysis issues
- Profiling performance

### Environment Variable

Set default thread count:

```bash
export RAYON_NUM_THREADS=4
debtmap analyze .
```

## Parallel Architecture

### File-Level Parallelism

Debtmap processes files in parallel using Rayon's parallel iterators:

```rust
files.par_iter()
    .map(|file| analyze_file(file))
    .collect()
```

Each file is:
1. Parsed independently
2. Analyzed for complexity
3. Scored and prioritized

### Lock-Free Concurrent Access

DashMap provides concurrent hash map access without locks:

```rust
// Multiple threads can read/write simultaneously
let coverage_index: Arc<DashMap<String, Coverage>> = ...;

// Thread-safe lookups in parallel
functions.par_iter()
    .map(|func| {
        let coverage = coverage_index.get(&func.name);
        calculate_risk(func, coverage)
    })
```

### Shared Read-Only Data

Analysis configuration and indexes are shared across threads:

```rust
let coverage_index = Arc::new(build_coverage_index());

// All threads share the same index
files.par_iter()
    .map(|file| analyze_with_coverage(file, &coverage_index))
```

## Performance Tuning

### Optimal Thread Count

**General rule:** Use physical core count, not logical cores.

```bash
# Check physical core count
lscpu | grep "Core(s) per socket"

# macOS
sysctl hw.physicalcpu
```

**Recommended settings:**

| System | Cores | Recommended --jobs |
|--------|-------|-------------------|
| Laptop | 4 | Default or 4 |
| Desktop | 8 | Default |
| Workstation | 16+ | Default |
| CI/CD | Varies | 2-4 (shared resources) |

### Memory Considerations

Each thread requires memory for:
- AST parsing (~1-5 MB per file)
- Analysis state (~500 KB per file)
- Temporary buffers

**Memory usage estimate:**
```
Total Memory ≈ (Thread Count) × (Average File Size) × 2-3
```

**Example (50 files, average 10 KB each, 8 threads):**
```
Memory ≈ 8 × 10 KB × 3 = 240 KB (negligible)
```

For very large files (>1 MB), consider reducing thread count.

### I/O Bound vs CPU Bound

**CPU-bound analysis (default):**
- Complexity calculations
- Pattern detection
- Risk scoring

Parallel processing provides 4-8x speedup.

**I/O-bound operations:**
- Reading files from disk
- Loading coverage data

Limited speedup from parallelism (1.5-2x).

**If analysis is I/O-bound:**
1. Move cache to SSD
2. Reduce thread count (less I/O contention)
3. Use `--max-files` to limit scope

## Scaling Strategies

### Small Projects (<10k LOC)

```bash
# Default settings are fine
debtmap analyze .
```

Parallel overhead may exceed benefits. Consider `--no-parallel` if analysis is <1 second.

### Medium Projects (10k-100k LOC)

```bash
# Use all cores
debtmap analyze .
```

Optimal parallel efficiency. Expect 4-8x speedup from parallelism.

### Large Projects (>100k LOC)

```bash
# Use all cores with optimized cache
export DEBTMAP_CACHE_MAX_SIZE=5368709120  # 5GB
debtmap analyze . --jobs 0  # 0 = all cores
```

Maximize cache size to avoid re-analysis.

### CI/CD Environments

```bash
# Limit threads to avoid resource contention
debtmap analyze . --jobs 2
```

CI environments often limit CPU cores per job.

## Profiling and Debugging

### Measure Analysis Time

```bash
time debtmap analyze .
```

### Disable Parallelism for Debugging

```bash
debtmap analyze . --no-parallel -vv
```

Single-threaded mode with verbose output for debugging.

### Profile Thread Usage

Use system tools to monitor thread usage:

```bash
# Linux
htop

# macOS
Activity Monitor (View > CPU Usage > Show Threads)
```

Look for:
- All cores at ~100% utilization (optimal)
- Some cores idle (I/O bound or insufficient work)
- Excessive context switching (too many threads)

## Best Practices

1. **Use default settings** - Debtmap auto-detects optimal thread count
2. **Limit threads in CI** - Use `--jobs 2` or `--jobs 4` in shared environments
3. **Profile before tuning** - Measure actual performance impact
4. **Consider I/O** - If using slow storage, reduce thread count
5. **Cache aggressively** - Large caches reduce repeated work

## Troubleshooting

### Analysis is Slow Despite Parallelism

**Possible causes:**
1. I/O bottleneck (slow disk)
2. Cache disabled or cleared
3. Excessive cache pruning
4. Memory pressure (swapping)

**Solutions:**
- Move cache to SSD
- Increase `DEBTMAP_CACHE_MAX_SIZE`
- Reduce thread count to avoid memory pressure

### High CPU Usage But No Progress

**Possible cause:** Analyzing very complex files (large ASTs)

**Solution:**
```bash
# Reduce thread count to avoid memory thrashing
debtmap analyze . --jobs 2
```

### Inconsistent Results Between Runs

**Possible cause:** Non-deterministic parallel aggregation (rare)

**Solution:**
```bash
# Use single-threaded mode
debtmap analyze . --no-parallel
```

If results differ, report as a bug.

## See Also

- [Cache Management](cache-management.md) - Cache configuration for performance
- [Configuration](configuration.md) - Project-specific settings
- [Troubleshooting](troubleshooting.md) - General troubleshooting guide
