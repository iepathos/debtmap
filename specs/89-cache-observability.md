---
number: 89
title: Enhanced Cache Observability and Logging
category: optimization
priority: low
status: draft
dependencies: []
created: 2025-09-03
---

# Specification 89: Enhanced Cache Observability and Logging

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: None

## Context

The shared cache implementation in `src/cache/shared_cache.rs` provides basic metadata tracking (access count, last accessed, size) but lacks comprehensive observability. With the cache now shared across multiple projects in `.debtmap_cache`, this makes it difficult to:
- Understand cache effectiveness across different projects
- Debug cache-related issues in the shared environment
- Optimize cache configuration for multiple concurrent users
- Monitor shared cache health and growth over time
- Identify performance bottlenecks affecting all projects
- Track cache usage patterns per project and component

Better observability would help developers understand and optimize shared cache behavior, leading to improved overall performance for all projects using debtmap.

## Objective

Implement comprehensive cache observability with structured logging, metrics collection, and performance monitoring to provide clear insights into cache behavior and effectiveness.

## Requirements

### Functional Requirements
- Structured logging for all cache operations
- Real-time cache metrics collection
- Cache performance dashboard output
- Hit/miss ratio tracking per file type
- Cache operation timing metrics
- Memory usage monitoring
- Detailed debug mode for troubleshooting
- Summary statistics in normal operation

### Non-Functional Requirements
- Minimal performance overhead (< 1% CPU)
- Configurable verbosity levels
- Machine-readable metric output
- Human-friendly summary format
- Thread-safe metric collection

## Acceptance Criteria

- [ ] All cache operations are logged with context
- [ ] Metrics are collected for cache performance
- [ ] Summary shows hit rate and time saved
- [ ] Per-language statistics are available
- [ ] Operation timings are tracked
- [ ] Memory usage is monitored
- [ ] Debug mode provides detailed traces
- [ ] Metrics can be exported to JSON
- [ ] Performance overhead is negligible
- [ ] Documentation explains all metrics

## Technical Details

### Implementation Approach
1. Add structured logging with tracing
2. Implement metrics collection system
3. Create cache performance dashboard
4. Add timing instrumentation
5. Implement memory tracking

### Architecture Changes
```rust
pub struct CacheMetrics {
    pub total_hits: AtomicUsize,
    pub total_misses: AtomicUsize,
    pub hits_by_language: DashMap<Language, usize>,
    pub hits_by_project: DashMap<String, usize>,  // Track per-project usage
    pub hits_by_component: DashMap<String, usize>, // Track per-component usage
    pub operation_timings: DashMap<String, Duration>,
    pub cache_size_bytes: AtomicUsize,
    pub entries_count: AtomicUsize,
    pub time_saved_ms: AtomicU64,
    pub last_prune_time: RwLock<Option<SystemTime>>,
    pub shared_cache_total_size: AtomicUsize,  // Total shared cache size
}

pub struct CacheObserver {
    metrics: Arc<CacheMetrics>,
    log_level: LogLevel,
}

impl CacheObserver {
    pub fn record_hit(&self, file: &Path, time_saved: Duration);
    pub fn record_miss(&self, file: &Path, computation_time: Duration);
    pub fn record_operation(&self, op: &str, duration: Duration);
    pub fn generate_report(&self) -> CacheReport;
    pub fn export_metrics(&self) -> serde_json::Value;
}

#[derive(Debug, Serialize)]
pub struct CacheReport {
    pub summary: CacheSummary,
    pub performance: PerformanceMetrics,
    pub usage: UsageStatistics,
    pub recommendations: Vec<String>,
}
```

### Data Structures
- `CacheMetrics` for atomic metric collection
- `CacheReport` for formatted output
- `PerformanceMetrics` for timing data
- `UsageStatistics` for patterns analysis

### APIs and Interfaces
```rust
impl SharedCache {
    pub fn with_observer(repo_path: Option<&Path>, observer: CacheObserver) -> Result<Self>;
    pub fn get_metrics(&self) -> &CacheMetrics;
    pub fn get_component_metrics(&self, component: &str) -> ComponentMetrics;
    pub fn export_telemetry(&self, path: &Path) -> Result<()>;
}

impl AnalysisCache {
    pub fn print_summary(&self);
    pub fn get_shared_metrics(&self) -> &CacheMetrics;
}

// Logging macros
macro_rules! cache_trace {
    ($($arg:tt)*) => {
        log::trace!(target: "debtmap::cache", $($arg)*);
    }
}

// CLI options
pub struct CacheLoggingOptions {
    pub show_summary: bool,      // Default: true
    pub show_per_file: bool,     // Default: false
    pub export_metrics: bool,    // Default: false
    pub trace_operations: bool,  // Default: false
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/cache/shared_cache.rs` - Metrics collection
  - `src/cache/cache_location.rs` - Location-aware metrics
  - `src/core/cache.rs` - Analysis cache metrics
  - `src/commands/analyze.rs` - Summary output
  - `src/cli.rs` - New CLI options
- **External Dependencies**: 
  - `tracing` for structured logging
  - `dashmap` for concurrent metrics
  - `indicatif` for progress bars (optional)

## Testing Strategy

- **Unit Tests**: 
  - Test metric collection accuracy
  - Verify thread-safe operations
  - Test report generation
  - Validate export formats
- **Integration Tests**: 
  - Test metrics under concurrent load
  - Verify summary accuracy
  - Test different verbosity levels
  - Confirm export functionality
- **Performance Tests**: 
  - Measure metric collection overhead
  - Test with high-frequency operations
  - Verify memory usage
- **User Acceptance**: 
  - Clear and useful cache summaries
  - Helpful debug information
  - Actionable recommendations
  - No performance degradation

## Documentation Requirements

- **Code Documentation**: 
  - Document all metrics and their meaning
  - Explain logging levels and usage
  - Document performance considerations
- **User Documentation**: 
  - Add cache monitoring section to README
  - Explain how to interpret metrics
  - Provide optimization guidelines
  - Document CLI options
- **Architecture Updates**: 
  - Add observability layer to cache architecture
  - Document metrics collection flow

## Implementation Notes

- Use atomic operations for lock-free metrics
- Implement sampling for high-frequency operations
- Add cache warmup detection
- Track "effective cache age" metric
- Consider integration with OpenTelemetry
- Add cache efficiency score calculation

### Example Output
```
Shared Cache Performance Summary:
  Location: ~/.debtmap_cache (Shared)
  Total Operations: 1,234
  Hit Rate: 87.3% (1,077 hits, 157 misses)
  Time Saved: 4.7 minutes
  Shared Cache Size: 245 MB (12,543 total entries)
  This Project: 8.8 MB (1,077 entries)
  
  By Component:
    analysis:     89.2% hit rate (823/923)
    call_graph:   72.1% hit rate (124/172)
    dependencies: 94.3% hit rate (130/138)
  
  By Language:
    Rust:       92.1% hit rate (823/894)
    JavaScript: 78.4% hit rate (254/324)
  
  Top Cached Files:
    src/core/mod.rs (142 hits, saved 18.3s)
    src/lib.rs (98 hits, saved 12.1s)
    
  Recommendations:
    ✓ Cache is performing well
    ⚠ Shared cache approaching size limit (245/500 MB)
    ℹ 3 other projects sharing this cache
```

## Migration and Compatibility

- Gracefully handle missing metrics
- Default to minimal logging for compatibility
- Provide opt-in detailed metrics
- Support metric export for analysis tools
- Maintain backward compatibility with existing logging