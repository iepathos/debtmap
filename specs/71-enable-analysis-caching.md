---
number: 71
title: Enable Analysis Caching for Fast Re-runs
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-01-31
---

# Specification 71: Enable Analysis Caching for Fast Re-runs

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently has a complete caching infrastructure implemented in `src/core/cache.rs` that includes:
- `AnalysisCache` for persistent file-based caching with SHA-256 content hashing
- `IncrementalAnalysis` for tracking changed files between runs
- Cache statistics and pruning capabilities

However, this caching system is **never actually used** in the main analysis flow. Every run of debtmap analyzes all files from scratch, resulting in ~38 seconds for 282 files even when nothing has changed. The README claims "Incremental analysis caches results for lightning-fast re-runs" but this is currently false advertising.

Enabling the existing cache with minimal code changes would provide:
- 50-70% speedup on unchanged files
- 90-95% speedup when only a few files change
- Near-instant re-runs when no files have changed

## Objective

Enable the existing caching infrastructure in the main analysis flow with minimal, surgical code changes to provide dramatic performance improvements for re-runs while maintaining 100% compatibility with current functionality.

## Requirements

### Functional Requirements

- Use existing `AnalysisCache` for all file metric calculations
- Maintain exact same analysis results as non-cached runs
- Automatically invalidate cache when file content changes
- Support cache location configuration
- Provide cache statistics in verbose mode
- Allow cache clearing via command-line flag

### Non-Functional Requirements

- Zero performance overhead for cache hits (< 1ms per file)
- Minimal performance overhead for cache misses (< 5ms per file)
- Cache storage under 10MB for typical projects
- Support concurrent cache access for parallel analysis
- Graceful handling of corrupted cache data

## Acceptance Criteria

- [ ] Cache is used by default for all `debtmap analyze` commands
- [ ] Second run on unchanged codebase is 70%+ faster
- [ ] Cache correctly invalidates when files change
- [ ] `--no-cache` flag disables caching
- [ ] `--clear-cache` flag removes all cached data
- [ ] Verbose mode shows cache hit/miss statistics
- [ ] All existing tests pass without modification
- [ ] Cache files stored in `.debtmap_cache/` directory

## Technical Details

### Implementation Approach

The implementation requires minimal changes to just two functions in `main.rs`:

#### Step 1: Modify `analyze_project` to Initialize Cache
```rust
fn analyze_project(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
) -> Result<AnalysisResults> {
    let config = config::get_config();
    
    // NEW: Initialize cache (4 lines added)
    let cache_dir = path.join(".debtmap_cache");
    let cache_enabled = std::env::var("DEBTMAP_NO_CACHE").is_err();
    let mut cache = if cache_enabled {
        Some(core::cache::AnalysisCache::new(cache_dir)?)
    } else {
        None
    };
    
    let files = io::walker::find_project_files_with_config(&path, languages.clone(), config)
        .context("Failed to find project files")?;
    
    // MODIFIED: Pass cache to collect_file_metrics
    let file_metrics = if let Some(ref mut cache) = cache {
        collect_file_metrics_with_cache(&files, cache)
    } else {
        analysis_utils::collect_file_metrics(&files)
    };
    
    // NEW: Print cache statistics in verbose mode (3 lines)
    if cache_enabled && log::log_enabled!(log::Level::Debug) {
        if let Some(cache) = &cache {
            log::info!("Cache stats: {}", cache.stats());
        }
    }
    
    // Rest of function unchanged...
    let all_functions = analysis_utils::extract_all_functions(&file_metrics);
    let all_debt_items = analysis_utils::extract_all_debt_items(&file_metrics);
    let duplications = detect_duplications(&files, duplication_threshold);
    
    let complexity_report = build_complexity_report(&all_functions, complexity_threshold);
    let technical_debt = build_technical_debt_report(all_debt_items, duplications.clone());
    let dependencies = create_dependency_report(&file_metrics);
    
    Ok(AnalysisResults {
        project_path: path,
        timestamp: Utc::now(),
        complexity: complexity_report,
        technical_debt,
        dependencies,
        duplications,
    })
}
```

#### Step 2: Add Cache-Aware File Metrics Collection
```rust
fn collect_file_metrics_with_cache(
    files: &[PathBuf],
    cache: &mut core::cache::AnalysisCache,
) -> Vec<FileMetrics> {
    use rayon::prelude::*;
    use std::sync::Mutex;
    
    let cache = Arc::new(Mutex::new(cache));
    
    files
        .par_iter()
        .filter_map(|path| {
            let mut cache = cache.lock().unwrap();
            cache.get_or_compute(path, || {
                analysis_utils::analyze_single_file(path)
                    .ok_or_else(|| anyhow::anyhow!("Failed to analyze file"))
            }).ok()
        })
        .collect()
}
```

#### Step 3: Add Command-Line Flags
```rust
#[derive(Parser)]
struct AnalyzeArgs {
    // ... existing args ...
    
    /// Disable caching for this run
    #[arg(long)]
    no_cache: bool,
    
    /// Clear cache before running analysis
    #[arg(long)]
    clear_cache: bool,
}
```

#### Step 4: Handle Cache Flags in CLI
```rust
fn handle_analyze(mut config: AnalyzeConfig) -> Result<()> {
    // NEW: Handle cache flags (6 lines)
    if config.clear_cache {
        let cache_dir = config.path.join(".debtmap_cache");
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir)?;
            log::info!("Cache cleared");
        }
    }
    
    if config.no_cache {
        std::env::set_var("DEBTMAP_NO_CACHE", "1");
    }
    
    // Rest of function unchanged...
}
```

### Architecture Changes

No architectural changes required. The cache integrates seamlessly with existing code:

1. Cache is transparent to analysis logic
2. Thread-safe for parallel file processing
3. Automatic invalidation on file changes
4. No changes to data structures or APIs

### Data Structures

All data structures already exist in `src/core/cache.rs`:

```rust
// Already implemented:
pub struct AnalysisCache {
    cache_dir: PathBuf,
    index: HashMap<String, CacheEntry>,
    hits: usize,
    misses: usize,
}

pub struct CacheEntry {
    pub file_hash: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: FileMetrics,
}
```

### APIs and Interfaces

No new APIs needed. Existing cache interface is already sufficient:

```rust
impl AnalysisCache {
    // Create new cache instance
    pub fn new(cache_dir: PathBuf) -> Result<Self>
    
    // Get cached or compute new metrics
    pub fn get_or_compute<F>(&mut self, path: &Path, compute: F) -> Result<FileMetrics>
    where F: FnOnce() -> Result<FileMetrics>
    
    // Get cache statistics
    pub fn stats(&self) -> CacheStats
    
    // Clear all cached data
    pub fn clear(&mut self) -> Result<()>
}
```

## Dependencies

- **Prerequisites**: None (cache module already exists)
- **Affected Components**:
  - `src/main.rs` - Add cache initialization and usage
  - `src/analysis_utils.rs` - Optional cache-aware wrapper
- **External Dependencies**: None (all dependencies already in place)

## Testing Strategy

- **Unit Tests**:
  - Cache tests already exist in `src/core/cache.rs`
  - Add test for cache-aware metrics collection
  - Test cache flag handling
  
- **Integration Tests**:
  - Run analysis twice, verify second run is faster
  - Modify a file, verify cache invalidation
  - Test `--no-cache` flag disables caching
  - Test `--clear-cache` removes cache directory
  
- **Performance Tests**:
  - Benchmark first run vs cached run
  - Measure overhead of cache misses
  - Test cache performance with 1000+ files
  
- **Correctness Tests**:
  - Verify cached results match non-cached results
  - Test cache corruption recovery
  - Verify deterministic results

## Documentation Requirements

- **Code Documentation**:
  - Document cache behavior in analyze_project
  - Add comments explaining cache invalidation
  
- **User Documentation**:
  - Update README to reflect actual caching behavior
  - Document cache location and structure
  - Add cache troubleshooting section
  - Include benchmark results showing speedup
  
- **CLI Help**:
  - Document `--no-cache` flag
  - Document `--clear-cache` flag
  - Add cache statistics to verbose output

## Implementation Notes

### Minimal Change Philosophy
This specification prioritizes minimal code changes for maximum impact:
- Total changes: ~30 lines of code
- No breaking changes to existing functionality
- No new dependencies required
- Leverages existing, tested cache implementation

### Performance Expectations
Based on the cache implementation and file I/O patterns:
- First run: No change in performance
- Second run (no changes): 70-90% faster
- Incremental changes: 50-70% faster
- Cache overhead: < 5% on cache misses

### Cache Storage
- Location: `.debtmap_cache/` in project root
- Format: JSON index + SHA-256 content hashing
- Size: ~1-2KB per file (typically < 10MB total)
- Automatic cleanup of old entries (30 days by default)

### Error Handling
- Corrupted cache: Log warning and recompute
- Missing cache directory: Create automatically
- Permission errors: Fall back to non-cached analysis
- Disk full: Disable caching for current run

## Migration and Compatibility

This change is 100% backward compatible:
- Cache is optional and can be disabled
- No changes to output format or results
- No changes to existing command-line interface
- Cache files are gitignored by default

Future enhancements (not part of this spec):
- Network cache sharing for CI/CD
- Compression for cache entries
- Partial file analysis caching
- Dependency-aware cache invalidation