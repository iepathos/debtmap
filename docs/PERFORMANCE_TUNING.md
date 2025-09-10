# Performance Tuning Guide

## Overview

DebtMap provides several performance tuning options to optimize analysis speed for different codebase sizes and system configurations. The tool automatically uses parallel processing by default, but you can fine-tune the behavior to match your specific needs.

## Quick Start

For most users, the default settings provide optimal performance:

```bash
# Analyze with default parallel processing (uses all CPU cores)
debtmap analyze

# Disable parallel processing for debugging
debtmap analyze --no-parallel

# Control the number of parallel jobs
debtmap analyze --jobs 4
```

## Performance Tuning Options

### Command Line Flags

#### `--jobs N`
Controls the number of parallel jobs to run simultaneously.

- **Default**: Uses all available CPU cores
- **Recommended**: Number of CPU cores minus 1 (leaves one core for system tasks)
- **Example**: `debtmap analyze --jobs 4`

**When to adjust:**
- Reduce if system becomes unresponsive during analysis
- Increase on high-core systems for maximum speed
- Set to 1 for sequential processing with controlled resource usage

#### `--batch-size N`
Sets the number of items processed in each parallel batch.

- **Default**: 100 items per batch
- **Range**: 10-500 recommended
- **Example**: `debtmap analyze --batch-size 50`

**When to adjust:**
- Smaller batches (10-50): Better for memory-constrained systems
- Larger batches (200-500): Better for systems with ample memory
- Affects memory usage more than speed

#### `--no-parallel`
Disables parallel processing entirely.

- **Use cases**: 
  - Debugging analysis issues
  - Extremely memory-constrained environments
  - Ensuring deterministic analysis order
- **Example**: `debtmap analyze --no-parallel`

#### `--progress`
Shows real-time progress indicators during analysis.

- **Default**: Enabled for interactive terminals
- **Impact**: Minimal performance overhead
- **Example**: `debtmap analyze --progress`

### Environment Variables

#### `RAYON_NUM_THREADS`
Override the thread pool size used by the parallel runtime.

```bash
export RAYON_NUM_THREADS=4
debtmap analyze
```

#### `DEBTMAP_BATCH_SIZE`
Set default batch size for all analyses.

```bash
export DEBTMAP_BATCH_SIZE=200
debtmap analyze
```

#### `DEBTMAP_CACHE_DIR`
Specify cache directory for incremental analysis (when available).

```bash
export DEBTMAP_CACHE_DIR=/path/to/cache
debtmap analyze
```

## Performance Expectations

### By Codebase Size

| Files | Sequential Time | Parallel Time (8 cores) | Speedup |
|-------|----------------|------------------------|---------|
| 50    | ~1.2s          | ~0.3s                  | 4x      |
| 250   | ~5s            | ~0.8s                  | 6x      |
| 500   | ~10s           | ~1.5s                  | 7x      |
| 1000  | ~20s           | ~3s                    | 7x      |
| 5000  | ~100s          | ~15s                   | 7x      |

### By Hardware Configuration

#### Low-End Systems (2-4 cores, 4GB RAM)
```bash
debtmap analyze --jobs 2 --batch-size 50
```
- Uses 2 parallel jobs to leave resources for system
- Smaller batches to reduce memory pressure

#### Mid-Range Systems (4-8 cores, 8-16GB RAM)
```bash
debtmap analyze  # Use defaults
```
- Default settings are optimized for this configuration

#### High-End Systems (16+ cores, 32GB+ RAM)
```bash
debtmap analyze --batch-size 200
```
- Larger batches to maximize throughput
- Jobs automatically scale to core count

## Optimization Strategies

### For Speed

Maximize analysis speed when you have sufficient resources:

```bash
# Use all cores with large batches
debtmap analyze --batch-size 200

# Or set via environment
export DEBTMAP_BATCH_SIZE=200
debtmap analyze
```

### For Memory Efficiency

Minimize memory usage on constrained systems:

```bash
# Limit parallelism and use small batches
debtmap analyze --jobs 2 --batch-size 25

# Or disable parallel processing entirely
debtmap analyze --no-parallel
```

### For Large Codebases (1000+ files)

```bash
# Balance parallelism with memory usage
debtmap analyze --jobs 4 --batch-size 100
```

### For CI/CD Environments

```bash
# Use conservative settings for stability
debtmap analyze --jobs 2 --batch-size 50 --no-progress
```

## Troubleshooting Performance Issues

### Analysis is Slow

1. **Check CPU utilization**: If not using all cores, increase `--jobs`
2. **Check available memory**: If swapping, reduce `--batch-size` or `--jobs`
3. **Verify no other intensive processes** are running
4. **Try increasing batch size** if you have memory available

### Out of Memory Errors

1. **Reduce batch size**: `--batch-size 25`
2. **Limit parallel jobs**: `--jobs 2`
3. **Disable parallel processing**: `--no-parallel`
4. **Close other applications** to free memory

### Inconsistent Results

1. **Disable parallel processing** for debugging: `--no-parallel`
2. **Use fixed job count**: `--jobs 4` instead of auto-detection
3. **Check for race conditions** in custom analyzers

## Advanced Tuning

### Profile-Guided Optimization

1. Run with different configurations and measure:
```bash
time debtmap analyze --jobs 2
time debtmap analyze --jobs 4
time debtmap analyze --jobs 8
```

2. Find the sweet spot for your system and codebase

3. Set as defaults:
```bash
export RAYON_NUM_THREADS=4
export DEBTMAP_BATCH_SIZE=100
```

### Memory Profiling

Monitor memory usage during analysis:

```bash
# On Linux/Mac
/usr/bin/time -v debtmap analyze

# Watch memory in real-time
watch -n 1 'ps aux | grep debtmap'
```

### CPU Profiling

Identify bottlenecks:

```bash
# Generate flamegraph (requires cargo-flamegraph)
cargo flamegraph --bin debtmap -- analyze

# Use system profiler
perf record debtmap analyze
perf report
```

## Best Practices

1. **Start with defaults** - They work well for most cases
2. **Measure before optimizing** - Profile to find actual bottlenecks
3. **Adjust one parameter at a time** - Isolate the impact of changes
4. **Consider your use case**:
   - Interactive use: Optimize for speed
   - CI/CD: Optimize for stability
   - Resource-limited: Optimize for memory
5. **Document your configuration** - Save optimal settings for your team

## FAQ

### Q: Why is parallel analysis sometimes slower than sequential?

A: For very small codebases (<10 files), parallel overhead may exceed benefits. Use `--no-parallel` for tiny projects.

### Q: How do I know the optimal job count?

A: Start with CPU cores minus 1. Adjust based on system responsiveness and analysis time.

### Q: Does batch size affect accuracy?

A: No, batch size only affects performance and memory usage. Results are identical regardless of batch size.

### Q: Can I use all CPU cores?

A: Yes, but leaving one core free (using `--jobs` with cores-1) often provides better system responsiveness with minimal speed impact.

### Q: How much memory does DebtMap need?

A: Approximately 1-2MB per file analyzed. A 1000-file project needs ~1-2GB with default settings.

## Benchmarking Your Configuration

Create a benchmark script to find optimal settings:

```bash
#!/bin/bash
# benchmark.sh

echo "Testing different configurations..."

for jobs in 1 2 4 8; do
  for batch in 25 50 100 200; do
    echo "Jobs: $jobs, Batch: $batch"
    time debtmap analyze --jobs $jobs --batch-size $batch > /dev/null
  done
done
```

Run this script to identify the best configuration for your specific codebase and hardware.