# Debtmap Performance Optimization Plan

## Current Performance Analysis

### Baseline Metrics
- **Current execution time**: ~38 seconds for 282 Rust files
- **Per-file processing**: ~135ms/file
- **Claims in README**: "Parallel processing with Rayon" and "Incremental analysis caches"
- **Reality**: Limited parallelization, NO caching actually used

## Key Performance Issues Identified

### 1. **Caching Infrastructure Exists but NOT Used**
- `src/core/cache.rs` implements `AnalysisCache` and `IncrementalAnalysis`
- These modules are NEVER instantiated or called in the main analysis flow
- Every run re-analyzes all files from scratch

### 2. **Limited Parallelization**
- Only `collect_file_metrics()` uses `par_iter()` from Rayon
- Most expensive operations run sequentially:
  - Call graph construction (multiple passes)
  - Unified analysis creation
  - Risk analysis
  - Duplication detection

### 3. **Expensive Sequential Operations**
- **Call graph construction**: 3 sequential passes over all files
  - Initial graph building
  - Multi-file cross-module resolution
  - Enhanced analysis (trait dispatch, function pointers, etc.)
- **Unified analysis**: Iterates through all metrics sequentially
- **Duplication detection**: Sequential pairwise comparison

### 4. **I/O Inefficiencies**
- Files read multiple times during analysis
- No batching of file operations
- Synchronous file reading even in parallel sections

## Optimization Solutions

### Phase 1: Enable Caching (Quick Win - 50-70% speedup on re-runs)

```rust
// In main.rs analyze_project function
fn analyze_project(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
) -> Result<AnalysisResults> {
    let config = config::get_config();
    let cache_dir = path.join(".debtmap_cache");
    let mut cache = AnalysisCache::new(cache_dir)?;
    
    let files = io::walker::find_project_files_with_config(&path, languages.clone(), config)?;
    
    // Use cache for file metrics
    let file_metrics = files
        .par_iter()
        .filter_map(|path| {
            cache.get_or_compute(path, || {
                analyze_single_file(path.as_path())
                    .ok_or_else(|| anyhow::anyhow!("Failed to analyze file"))
            }).ok()
        })
        .collect();
    
    // Rest of the analysis...
}
```

### Phase 2: Maximize Parallelization (30-40% speedup)

```rust
// Parallelize call graph construction
fn build_call_graph_parallel(files: &[PathBuf]) -> CallGraph {
    // First pass: parallel analysis
    let file_graphs: Vec<_> = files
        .par_iter()
        .filter_map(|path| analyze_rust_file_for_call_graph(path).ok())
        .collect();
    
    // Merge graphs in parallel chunks
    file_graphs
        .par_chunks(10)
        .map(|chunk| merge_graphs(chunk))
        .reduce(CallGraph::new, |mut a, b| { a.merge(b); a })
}

// Parallelize duplication detection
fn detect_duplications_parallel(files: &[PathBuf], threshold: usize) -> Vec<DuplicationBlock> {
    let files_with_content = prepare_files_for_duplication_check(files);
    
    // Use rayon's parallel combinations
    files_with_content
        .par_iter()
        .enumerate()
        .flat_map(|(i, file1)| {
            files_with_content[i+1..]
                .par_iter()
                .filter_map(|file2| {
                    detect_duplication_pair(file1, file2, threshold)
                })
        })
        .collect()
}
```

### Phase 3: Incremental Analysis (60-80% speedup on small changes)

```rust
// Add incremental mode
fn analyze_project_incremental(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
) -> Result<AnalysisResults> {
    let mut incremental = IncrementalAnalysis::new();
    let cache = AnalysisCache::new(path.join(".debtmap_cache"))?;
    
    incremental.load_previous(&cache);
    
    let all_files = find_project_files(&path, languages)?;
    let files_to_analyze = incremental.get_files_to_analyze(&all_files);
    
    // Only analyze changed files
    let new_metrics = collect_file_metrics(&files_to_analyze);
    
    // Merge with cached metrics
    let all_metrics = merge_metrics(incremental.previous_state, new_metrics);
    
    // Continue with analysis...
}
```

### Phase 4: Lazy Evaluation & Streaming (20-30% memory reduction)

```rust
// Use iterators instead of collecting everything
fn analyze_functions_lazy<'a>(
    metrics: &'a [FunctionMetrics]
) -> impl Iterator<Item = DebtItem> + 'a {
    metrics
        .iter()
        .filter(|m| !m.is_test)
        .map(|m| create_debt_item(m))
}

// Stream processing for large codebases
fn process_files_streaming(files: Vec<PathBuf>) -> impl Stream<Item = FileMetrics> {
    stream::iter(files)
        .map(|path| async move {
            analyze_single_file(&path)
        })
        .buffer_unordered(num_cpus::get())
}
```

### Phase 5: Architecture Improvements

1. **Add progress indicators**: Show which file is being processed
2. **Add `--jobs` flag**: Control parallelism level
3. **Add `--incremental` flag**: Enable incremental analysis
4. **Add `--no-cache` flag**: Force fresh analysis
5. **Profile-guided optimization**: Use cargo-pgo for 10-15% additional speedup

## Implementation Priority

1. **Enable caching** (1 day) - Biggest bang for buck
2. **Parallelize call graph** (2 days) - Major bottleneck
3. **Incremental analysis** (3 days) - Best UX improvement
4. **Parallelize other operations** (2 days) - Good general speedup
5. **Architecture improvements** (1 week) - Long-term maintainability

## Expected Results

- **First run**: 30-40% faster (from parallelization)
- **Subsequent runs**: 70-90% faster (from caching + incremental)
- **Small changes**: 95%+ faster (only analyze changed files)
- **Memory usage**: 20-30% reduction (from streaming)

## Validation

After implementation:
- Benchmark suite comparing old vs new
- Test on various repo sizes (small/medium/large)
- Verify correctness (same results as sequential)
- Monitor memory usage
- Add performance regression tests