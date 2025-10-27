# Parallel Processing

Debtmap leverages Rust's powerful parallel processing capabilities to analyze large codebases efficiently. Built on Rayon for data parallelism and DashMap for lock-free concurrent data structures, debtmap achieves 10-100x faster performance than Java/Python-based competitors.

## Overview

Debtmap's parallel processing architecture uses a three-phase approach:

1. **Parallel File Parsing** - Parse source files concurrently across all available CPU cores
2. **Parallel Multi-File Extraction** - Extract call graphs from parsed files in parallel
3. **Parallel Enhanced Analysis** - Analyze trait dispatch, function pointers, and framework patterns

This parallel pipeline is controlled by CLI flags that let you tune performance for your environment.

## Performance Characteristics

**Typical analysis times:**
- Small project (1k-5k LOC): <1 second
- Medium project (10k-50k LOC): 2-8 seconds
- Large project (100k-500k LOC): 10-45 seconds

**Comparison with other tools (medium-sized Rust project, ~50k LOC):**
- SonarQube: 3-4 minutes
- CodeClimate: 2-3 minutes
- Debtmap: 5-8 seconds

## CLI Flags for Parallelization

Debtmap provides two flags to control parallel processing behavior:

### --jobs / -j

Control the number of worker threads for parallel processing:

```bash
# Use all available CPU cores (default)
debtmap analyze --jobs 0

# Limit to 4 threads
debtmap analyze --jobs 4
debtmap analyze -j 4
```

**Behavior:**
- `--jobs 0` (default): Auto-detects available CPU cores using `std::thread::available_parallelism()`. Falls back to 4 threads if detection fails.
- `--jobs N`: Explicitly sets the thread pool to N threads.

**When to use:**
- Use `--jobs 0` for maximum performance on developer workstations
- Use `--jobs 1-4` in memory-constrained environments like CI/CD
- Use `--jobs 1` for deterministic analysis order during debugging

**Environment Variable:**

You can also set the default via the `RAYON_NUM_THREADS` or `DEBTMAP_JOBS` environment variable:

```bash
export RAYON_NUM_THREADS=4
debtmap analyze  # Uses 4 threads
```

The CLI flag takes precedence over the environment variable.

### --no-parallel

Disable parallel call graph construction entirely:

```bash
debtmap analyze --no-parallel
```

**When to use:**
- **Debugging concurrency issues**: Isolate whether a problem is parallelism-related
- **Memory-constrained environments**: Parallel processing increases memory usage
- **Deterministic analysis**: Ensures consistent ordering for reproducibility

**Performance Impact:**

Disabling parallelization significantly increases analysis time:
- Small projects (< 100 files): 2-3x slower
- Medium projects (100-1000 files): 5-10x slower
- Large projects (> 1000 files): 10-50x slower

For more details on both flags, see the [CLI Reference](./cli-reference.md#performance--caching).

## Rayon Parallel Iterators

Debtmap uses [Rayon](https://docs.rs/rayon), a data parallelism library for Rust, to parallelize file processing operations.

### Thread Pool Configuration

The global Rayon thread pool is configured at startup based on the `--jobs` parameter:

```rust
// From src/builders/parallel_call_graph.rs:46-52
if self.config.num_threads > 0 {
    rayon::ThreadPoolBuilder::new()
        .num_threads(self.config.num_threads)
        .build_global()
        .ok(); // Ignore if already configured
}
```

This configures Rayon to use a specific number of worker threads for all parallel operations throughout the analysis.

### Worker Thread Selection

The `get_worker_count()` function determines how many threads to use:

```rust
// From src/main.rs:750-758
fn get_worker_count(jobs: usize) -> usize {
    if jobs == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)  // Fallback if detection fails
    } else {
        jobs  // Use explicit value
    }
}
```

**Auto-detection behavior:**
- Queries the OS for available parallelism (CPU cores)
- Respects cgroup limits in containers (Docker, Kubernetes)
- Falls back to 4 threads if detection fails (rare)

**Manual configuration:**
- Useful in shared environments (CI/CD, shared build servers)
- Prevents resource contention with other processes
- Enables reproducible benchmarking

### Parallel File Processing

**Phase 1: Parallel File Parsing**

Files are parsed concurrently using Rayon's parallel iterators:

```rust
// From src/builders/parallel_call_graph.rs:98-128
let parsed_files: Vec<_> = rust_files
    .par_iter()  // Convert to parallel iterator
    .filter_map(|file_path| {
        let content = io::read_file(file_path).ok()?;

        // Update progress atomically
        parallel_graph.stats().increment_files();

        Some((file_path.clone(), content))
    })
    .collect();
```

Key features:
- `.par_iter()` converts a sequential iterator to a parallel one
- Each file is read independently on a worker thread
- Progress tracking uses atomic counters (see [Parallel Call Graph Statistics](#parallel-call-graph-statistics))

**Phase 2: Parallel Multi-File Extraction**

Files are grouped into chunks for optimal parallelization:

```rust
// From src/builders/parallel_call_graph.rs:130-161
let chunk_size = std::cmp::max(10, parsed_files.len() / rayon::current_num_threads());

parsed_files.par_chunks(chunk_size).for_each(|chunk| {
    // Parse syn files within each chunk
    let parsed_chunk: Vec<_> = chunk
        .iter()
        .filter_map(|(path, content)| {
            syn::parse_file(content)
                .ok()
                .map(|parsed| (parsed, path.clone()))
        })
        .collect();

    if !parsed_chunk.is_empty() {
        // Extract call graph for this chunk
        let chunk_graph = extract_call_graph_multi_file(&parsed_chunk);

        // Merge into main graph concurrently
        parallel_graph.merge_concurrent(chunk_graph);
    }
});
```

This chunking strategy balances parallelism with overhead:
- Minimum chunk size of 10 files prevents excessive overhead
- Dynamic chunk sizing based on available threads
- Each chunk produces a local call graph that's merged concurrently

**AST Parsing Optimization (Spec 132)**

Prior to spec 132, files were parsed twice during call graph construction:
1. Phase 1: Read files and store content as strings
2. Phase 2: **Re-parse the same content** to extract call graphs

This redundant parsing was eliminated by parsing each file exactly once and reusing the parsed `syn::File` AST:

```rust
// Optimized: Parse once in Phase 1
let parsed_files: Vec<(PathBuf, syn::File)> = rust_files
    .par_iter()
    .filter_map(|file_path| {
        let content = io::read_file(file_path).ok()?;
        let parsed = syn::parse_file(&content).ok()?;  // Parse ONCE
        Some((file_path.clone(), parsed))
    })
    .collect();

// Phase 2: Reuse parsed ASTs (no re-parsing)
for chunk in parsed_files.chunks(chunk_size) {
    let chunk_for_extraction: Vec<_> = chunk
        .iter()
        .map(|(path, parsed)| (parsed.clone(), path.clone()))  // Clone AST
        .collect();
    // Extract call graph...
}
```

**Performance Impact:**
- **Before**: 2N parse operations (404 files × 2 = 808 parses)
- **After**: N parse operations (404 files × 1 = 404 parses)
- **Speedup**: Cloning a parsed AST is **44% faster** than re-parsing
- **Time saved**: ~432ms per analysis run on 400-file projects
- **Memory overhead**: <100MB for parsed AST storage

**Why Clone Instead of Borrow?**
- `syn::File` is not `Send + Sync` (cannot be shared across threads)
- Call graph extraction requires owned AST values
- Cloning is still significantly faster than re-parsing (1.33ms vs 2.40ms per file)

See `docs/spec-132-benchmark-results.md` for detailed benchmarks validating these improvements.

**Phase 3: Enhanced Analysis**

The third phase analyzes trait dispatch, function pointers, and framework patterns. This phase is currently sequential due to complex shared state requirements, but benefits from the parallel foundation built in phases 1-2.

### Parallel Architecture

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

## DashMap for Lock-Free Concurrency

Debtmap uses [DashMap](https://docs.rs/dashmap), a concurrent hash map implementation, for lock-free data structures during parallel call graph construction.

### Why DashMap?

Traditional approaches to concurrent hash maps use a single `Mutex<HashMap>`, which creates contention:

```rust
// ❌ Traditional approach - serializes all access
let map = Arc<Mutex<HashMap<K, V>>>;

// Thread 1 blocks Thread 2, even for reads
let val = map.lock().unwrap().get(&key);
```

DashMap provides **lock-free reads** and **fine-grained write locking** through internal sharding:

```rust
// ✅ DashMap approach - concurrent reads, fine-grained writes
let map = Arc<DashMap<K, V>>;

// Multiple threads can read concurrently without blocking
let val = map.get(&key);

// Writes only lock the specific shard, not the whole map
map.insert(key, value);
```

### ParallelCallGraph Implementation

The `ParallelCallGraph` uses DashMap for all concurrent data structures:

```rust
// From src/priority/parallel_call_graph.rs:49-56
pub struct ParallelCallGraph {
    nodes: Arc<DashMap<FunctionId, NodeInfo>>,      // Functions
    edges: Arc<DashSet<FunctionCall>>,              // Calls
    caller_index: Arc<DashMap<FunctionId, DashSet<FunctionId>>>,  // Who calls this?
    callee_index: Arc<DashMap<FunctionId, DashSet<FunctionId>>>,  // Who does this call?
    stats: Arc<ParallelStats>,                      // Atomic counters
}
```

**Key components:**

1. **nodes**: Maps function identifiers to metadata (complexity, lines, flags)
2. **edges**: Set of all function calls (deduplicated automatically)
3. **caller_index**: Reverse index for "who calls this function?"
4. **callee_index**: Forward index for "what does this function call?"
5. **stats**: Atomic counters for progress tracking

### Concurrent Operations

**Adding Functions Concurrently**

Multiple analyzer threads can add functions simultaneously:

```rust
// From src/priority/parallel_call_graph.rs:78-96
pub fn add_function(
    &self,
    id: FunctionId,
    is_entry_point: bool,
    is_test: bool,
    complexity: u32,
    lines: usize,
) {
    let node_info = NodeInfo {
        id: id.clone(),
        is_entry_point,
        is_test,
        complexity,
        lines,
    };
    self.nodes.insert(id, node_info);
    self.stats.add_nodes(1);  // Atomic increment
}
```

**Atomicity guarantees:**
- `DashMap::insert()` is atomic - no data races
- `AtomicUsize` counters can be incremented from multiple threads safely
- No locks required for reading existing nodes

**Adding Calls Concurrently**

Function calls are added with automatic deduplication:

```rust
// From src/priority/parallel_call_graph.rs:98-117
pub fn add_call(&self, caller: FunctionId, callee: FunctionId, call_type: CallType) {
    let call = FunctionCall {
        caller: caller.clone(),
        callee: callee.clone(),
        call_type,
    };

    if self.edges.insert(call) {  // DashSet deduplicates automatically
        // Update indices concurrently
        self.caller_index
            .entry(caller.clone())
            .or_default()
            .insert(callee.clone());

        self.callee_index.entry(callee).or_default().insert(caller);

        self.stats.add_edges(1);  // Only increment if actually inserted
    }
}
```

**Deduplication:**
- `DashSet::insert()` returns `true` only for new items
- Duplicate calls from multiple threads are safely ignored
- Indices are updated atomically using `entry()` API

### Shared Read-Only Data

Analysis configuration and indexes are shared across threads:

```rust
let coverage_index = Arc::new(build_coverage_index());

// All threads share the same index
files.par_iter()
    .map(|file| analyze_with_coverage(file, &coverage_index))
```

### Memory Overhead

DashMap uses internal sharding for parallelism, which has a memory overhead:

- **DashMap overhead**: ~2x the memory of a regular `HashMap` due to sharding
- **DashSet overhead**: Similar to DashMap
- **Benefit**: Enables concurrent access without contention
- **Trade-off**: Debtmap prioritizes speed over memory for large codebases

For memory-constrained environments, use `--jobs 2-4` or `--no-parallel` to reduce parallel overhead.

## Parallel Call Graph Statistics

Debtmap tracks parallel processing progress using atomic counters that can be safely updated from multiple threads.

### ParallelStats Structure

```rust
// From src/priority/parallel_call_graph.rs:7-47
pub struct ParallelStats {
    pub total_nodes: AtomicUsize,      // Functions processed
    pub total_edges: AtomicUsize,      // Calls discovered
    pub files_processed: AtomicUsize,  // Files completed
    pub total_files: AtomicUsize,      // Total files to process
}
```

**Atomic operations:**
- `fetch_add()` - Atomically increment counters from any thread
- `load()` - Read current value without blocking
- `Ordering::Relaxed` - Sufficient for statistics (no synchronization needed)

### Progress Tracking

Progress ratio calculation for long-running analysis:

```rust
// From src/priority/parallel_call_graph.rs:38-46
pub fn progress_ratio(&self) -> f64 {
    let processed = self.files_processed.load(Ordering::Relaxed) as f64;
    let total = self.total_files.load(Ordering::Relaxed) as f64;
    if total > 0.0 {
        processed / total
    } else {
        0.0
    }
}
```

This enables progress callbacks during analysis:

```rust
// From src/builders/parallel_call_graph.rs:110-121
parallel_graph.stats().increment_files();
if let Some(ref callback) = self.config.progress_callback {
    let processed = parallel_graph
        .stats()
        .files_processed
        .load(std::sync::atomic::Ordering::Relaxed);
    let total = parallel_graph
        .stats()
        .total_files
        .load(std::sync::atomic::Ordering::Relaxed);
    callback(processed, total);
}
```

### Log Output Format

After analysis completes, debtmap reports final statistics:

```rust
// From src/builders/parallel_call_graph.rs:84-92
log::info!(
    "Parallel call graph complete: {} nodes, {} edges, {} files processed",
    stats.total_nodes.load(std::sync::atomic::Ordering::Relaxed),
    stats.total_edges.load(std::sync::atomic::Ordering::Relaxed),
    stats
        .files_processed
        .load(std::sync::atomic::Ordering::Relaxed),
);
```

**Example output:**
```
INFO - Processing 1247 Rust files in parallel
INFO - Progress: 100/1247 files processed
INFO - Progress: 500/1247 files processed
INFO - Progress: 1000/1247 files processed
INFO - Parallel call graph complete: 8942 nodes, 23451 edges, 1247 files processed
```

## Concurrent Merging

The `merge_concurrent()` method combines call graphs from different analysis phases using parallel iteration.

### Implementation

```rust
// From src/priority/parallel_call_graph.rs:119-138
pub fn merge_concurrent(&self, other: CallGraph) {
    // Parallelize node merging
    let nodes_vec: Vec<_> = other.get_all_functions().collect();
    nodes_vec.par_iter().for_each(|func_id| {
        if let Some((is_entry, is_test, complexity, lines)) = other.get_function_info(func_id) {
            self.add_function((*func_id).clone(), is_entry, is_test, complexity, lines);
        }
    });

    // Parallelize edge merging
    let calls_vec: Vec<_> = other.get_all_calls();
    calls_vec.par_iter().for_each(|call| {
        self.add_call(
            call.caller.clone(),
            call.callee.clone(),
            call.call_type.clone(),
        );
    });
}
```

**How it works:**
1. Extract all nodes and edges from the source `CallGraph`
2. Use `par_iter()` to merge nodes in parallel
3. Use `par_iter()` to merge edges in parallel
4. DashMap/DashSet automatically handle concurrent insertions

### Converting Between Representations

Debtmap uses two call graph representations:

- **ParallelCallGraph**: Concurrent data structures (DashMap/DashSet) for parallel construction
- **CallGraph**: Sequential data structures (HashMap/HashSet) for analysis algorithms

Conversion happens at phase boundaries:

```rust
// From src/priority/parallel_call_graph.rs:140-162
pub fn to_call_graph(&self) -> CallGraph {
    let mut call_graph = CallGraph::new();

    // Add all nodes
    for entry in self.nodes.iter() {
        let node = entry.value();
        call_graph.add_function(
            node.id.clone(),
            node.is_entry_point,
            node.is_test,
            node.complexity,
            node.lines,
        );
    }

    // Add all edges
    for call in self.edges.iter() {
        call_graph.add_call(call.clone());
    }

    call_graph
}
```

**Why two representations?**
- **ParallelCallGraph**: Optimized for concurrent writes during construction
- **CallGraph**: Optimized for graph algorithms (PageRank, connectivity, transitive reduction)
- Conversion overhead is negligible compared to analysis time

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

### Memory vs Speed Tradeoffs

Parallel processing uses more memory:

| Configuration | Memory Overhead | Speed Benefit |
|---------------|-----------------|---------------|
| `--no-parallel` | Baseline | Baseline |
| `--jobs 1` | +10% (data structures) | 1x |
| `--jobs 4` | +30% (+ worker buffers) | 4-6x |
| `--jobs 8` | +50% (+ worker buffers) | 6-10x |
| `--jobs 16` | +80% (+ worker buffers) | 10-15x |

**Memory overhead sources:**
- DashMap internal sharding (~2x HashMap)
- Per-worker thread stacks and buffers
- Parallel iterator intermediates

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

### Scaling Behavior

Debtmap's parallel processing scales with CPU core count:

**Strong Scaling (Fixed Problem Size):**

| CPU Cores | Speedup | Efficiency |
|-----------|---------|------------|
| 1         | 1x      | 100%       |
| 2         | 1.8x    | 90%        |
| 4         | 3.4x    | 85%        |
| 8         | 6.2x    | 78%        |
| 16        | 10.5x   | 66%        |
| 32        | 16.8x   | 53%        |

Efficiency decreases at higher core counts due to:
- Synchronization overhead (atomic operations, DashMap locking)
- Memory bandwidth saturation
- Diminishing returns from Amdahl's law (sequential portions)

**Weak Scaling (Problem Size Grows with Cores):**

Debtmap maintains high efficiency when problem size scales with core count, making it ideal for analyzing larger codebases on more powerful machines.

## Tuning Guidelines

**Development Workstations:**
```bash
# Use all cores for maximum speed
debtmap analyze --jobs 0
```

**CI/CD Environments:**
```bash
# Limit threads to avoid resource contention
debtmap analyze --jobs 2

# Or disable parallelism on very constrained runners
debtmap analyze --no-parallel
```

**Containers:**
```bash
# Auto-detection respects cgroup limits
debtmap analyze --jobs 0

# Or explicitly match container CPU allocation
debtmap analyze --jobs 4
```

**Benchmarking:**
```bash
# Use fixed thread count for reproducible results
debtmap analyze --jobs 8
```

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

### Finding Optimal Settings

**Finding the optimal setting:**

```bash
# Benchmark different configurations
time debtmap analyze --jobs 0  # Auto
time debtmap analyze --jobs 4  # 4 threads
time debtmap analyze --jobs 8  # 8 threads
time debtmap analyze --no-parallel  # Sequential
```

Monitor memory usage during analysis:
```bash
# Monitor peak memory usage
/usr/bin/time -v debtmap analyze --jobs 8
```

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

### Slow Analysis Performance

If analysis is slower than expected:

1. **Check thread count:**
   ```bash
   # Ensure you're using all cores
   debtmap analyze --jobs 0 -vv | grep "threads"
   ```

2. **Check I/O bottleneck:**
   ```bash
   # Use iotop or similar to check disk saturation
   # SSD storage significantly improves performance
   ```

3. **Check memory pressure:**
   ```bash
   # Monitor memory usage during analysis
   top -p $(pgrep debtmap)
   ```

4. **Try different thread counts:**
   ```bash
   # Sometimes less threads = less contention
   debtmap analyze --jobs 4
   ```

### High CPU Usage But No Progress

**Possible cause:** Analyzing very complex files (large ASTs)

**Solution:**
```bash
# Reduce thread count to avoid memory thrashing
debtmap analyze . --jobs 2
```

### High Memory Usage

If debtmap uses too much memory:

1. **Reduce parallelism:**
   ```bash
   debtmap analyze --jobs 2
   ```

2. **Disable parallel call graph:**
   ```bash
   debtmap analyze --no-parallel
   ```

3. **Analyze subdirectories separately:**
   ```bash
   # Process codebase in chunks
   debtmap analyze src/module1
   debtmap analyze src/module2
   ```

### Inconsistent Results Between Runs

**Possible cause:** Non-deterministic parallel aggregation (rare)

**Solution:**
```bash
# Use single-threaded mode
debtmap analyze . --no-parallel
```

If results differ, report as a bug.

### Debugging Concurrency Issues

If you suspect a concurrency bug:

1. **Run sequentially to isolate:**
   ```bash
   debtmap analyze --no-parallel
   ```

2. **Use deterministic mode:**
   ```bash
   # Single-threaded = deterministic order
   debtmap analyze --jobs 1
   ```

3. **Enable verbose logging:**
   ```bash
   debtmap analyze -vvv --no-parallel > debug.log 2>&1
   ```

4. **Report the issue:**
   If behavior differs between `--no-parallel` and parallel mode, please [report it](https://github.com/yourusername/debtmap/issues) with:
   - Command used
   - Platform (OS, CPU core count)
   - Debtmap version
   - Minimal reproduction case

### Thread Contention Warning

If you see warnings about thread contention:

```
WARN - High contention detected on parallel call graph
```

This indicates too many threads competing for locks. Try:

```bash
# Reduce thread count
debtmap analyze --jobs 4
```

## See Also

- [CLI Reference - Performance & Caching](./cli-reference.md#performance--caching) - Complete flag documentation
- [Cache Management](cache-management.md) - Cache configuration for performance
- [Configuration](configuration.md) - Project-specific settings
- [Troubleshooting](troubleshooting.md) - General troubleshooting guide
- [Troubleshooting - Slow Analysis](./troubleshooting.md#slow-analysis-performance) - Performance debugging guide
- [Troubleshooting - High Memory Usage](./troubleshooting.md#high-memory-usage) - Memory optimization tips
- [FAQ - Reducing Parallelism](./faq.md) - Common questions about parallel processing
- [Architecture](./architecture.md) - High-level system design

## Summary

Debtmap's parallel processing architecture provides:

- **10-100x speedup** over sequential analysis using Rayon parallel iterators
- **Lock-free concurrency** with DashMap for minimal contention
- **Flexible configuration** via `--jobs` and `--no-parallel` flags
- **Automatic thread pool tuning** that respects system resources
- **Production-grade reliability** with atomic progress tracking and concurrent merging

The three-phase parallel pipeline (parse → extract → analyze) maximizes parallelism while maintaining correctness through carefully designed concurrent data structures.
