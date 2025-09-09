---
number: 102
title: Incremental Unified Analysis Caching
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-09-09
---

# Specification 102: Incremental Unified Analysis Caching

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current unified analysis caching implementation (`src/cache/unified_analysis_cache.rs`) provides significant performance improvements (47x faster) when no files have changed. However, it uses an all-or-nothing approach: if any single file changes, the entire cache is invalidated and the full unified analysis must be recomputed.

Current behavior:
- **No changes**: ~0.8 seconds (unified cache hit)
- **1 file changed**: ~15-25 seconds (full recomputation despite file-level caching)
- **Cold cache**: ~40 seconds (complete analysis)

While individual file metrics are cached incrementally, expensive operations like call graph construction, trait resolution, and unified analysis aggregation are fully recomputed even for minor changes. This significantly impacts developer productivity during iterative development.

## Objective

Implement true incremental caching for unified analysis that only recomputes the portions affected by changed files, reducing the "1 file changed" scenario from 15-25 seconds to 2-5 seconds while maintaining analysis accuracy.

## Requirements

### Functional Requirements

1. **Incremental Cache Key Generation**
   - Generate separate cache keys for different analysis components (call graph, trait resolution, aggregation)
   - Track dependencies between components to determine invalidation scope
   - Support partial cache invalidation based on file changes

2. **Differential Analysis**
   - Detect which files have changed since last analysis
   - Determine the impact scope of changes (direct and transitive)
   - Identify which analysis components need recomputation

3. **Incremental Call Graph Updates**
   - Cache call graph at module/file level rather than project level
   - Support incremental updates when individual files change
   - Maintain cross-module dependency tracking for accurate invalidation

4. **Incremental Aggregation**
   - Cache intermediate aggregation results at file and module levels
   - Support incremental score recalculation for affected components
   - Maintain aggregate statistics that can be updated incrementally

5. **Cache Coherency**
   - Ensure consistency between different cache levels
   - Validate cache integrity after incremental updates
   - Support automatic fallback to full recomputation if inconsistencies detected

### Non-Functional Requirements

1. **Performance**
   - Single file change: < 5 seconds for 250-file project
   - Multiple file changes (< 10): < 10 seconds
   - Cache overhead: < 10% additional memory/storage
   - Cache lookup time: < 100ms

2. **Reliability**
   - Zero false cache hits (never serve stale data)
   - Automatic recovery from cache corruption
   - Graceful degradation to full recomputation if needed

3. **Maintainability**
   - Clear separation between cache layers
   - Comprehensive logging for cache operations
   - Debugging tools for cache inspection

## Acceptance Criteria

- [ ] Single file changes complete analysis in < 5 seconds for 250-file project
- [ ] Call graph updates incrementally for file changes
- [ ] Trait resolution caches and updates incrementally
- [ ] Aggregation scores update only for affected files
- [ ] Cache coherency maintained across all operations
- [ ] No regression in analysis accuracy compared to full recomputation
- [ ] Memory usage increases by < 20% with incremental caching
- [ ] Disk cache size increases by < 50% with granular caching
- [ ] All existing tests pass with incremental caching enabled
- [ ] New tests validate incremental cache correctness

## Technical Details

### Implementation Approach

1. **Multi-Level Cache Architecture**
   ```rust
   pub struct IncrementalUnifiedCache {
       file_cache: HashMap<PathBuf, FileCacheEntry>,
       module_cache: HashMap<ModuleId, ModuleCacheEntry>,
       call_graph_cache: CallGraphCache,
       aggregation_cache: AggregationCache,
       dependency_graph: DependencyGraph,
   }
   ```

2. **Dependency Tracking**
   ```rust
   pub struct DependencyGraph {
       file_deps: HashMap<PathBuf, HashSet<PathBuf>>,
       module_deps: HashMap<ModuleId, HashSet<ModuleId>>,
       reverse_deps: HashMap<PathBuf, HashSet<PathBuf>>,
   }
   ```

3. **Incremental Update Strategy**
   - Compute diff of changed files
   - Traverse dependency graph to find affected components
   - Invalidate only affected cache entries
   - Recompute minimal set of components
   - Merge incremental results with cached data

### Architecture Changes

1. **Cache Module Restructuring**
   - Split `unified_analysis_cache.rs` into multiple specialized caches
   - Add `incremental_cache/` module with sub-modules for each cache type
   - Implement cache coordination layer for consistency

2. **Analysis Pipeline Modifications**
   - Add change detection phase before analysis
   - Implement incremental analysis paths for each component
   - Add merge strategies for combining cached and fresh data

### Data Structures

```rust
pub struct FileCacheEntry {
    pub metrics: FileMetrics,
    pub call_graph: FileCallGraph,
    pub dependencies: HashSet<PathBuf>,
    pub hash: String,
    pub timestamp: SystemTime,
}

pub struct ModuleCacheEntry {
    pub aggregated_metrics: ModuleMetrics,
    pub module_graph: ModuleCallGraph,
    pub files: Vec<PathBuf>,
    pub hash: String,
}

pub struct IncrementalAnalysisResult {
    pub changed_files: Vec<PathBuf>,
    pub affected_modules: Vec<ModuleId>,
    pub cached_portions: CachedData,
    pub fresh_portions: FreshData,
    pub merged_result: UnifiedAnalysis,
}
```

### APIs and Interfaces

```rust
pub trait IncrementalCache {
    fn detect_changes(&self, files: &[PathBuf]) -> ChangeSet;
    fn get_affected_scope(&self, changes: &ChangeSet) -> AffectedScope;
    fn get_cached_portions(&self, scope: &AffectedScope) -> CachedData;
    fn merge_results(&self, cached: CachedData, fresh: FreshData) -> UnifiedAnalysis;
    fn update_cache(&mut self, results: &UnifiedAnalysis, scope: &AffectedScope);
}
```

## Dependencies

- **Prerequisites**: None (builds on existing caching infrastructure)
- **Affected Components**: 
  - `src/cache/unified_analysis_cache.rs` (major refactor)
  - `src/builders/unified_analysis.rs` (add incremental paths)
  - `src/analysis/call_graph/` (add incremental updates)
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: 
  - Test incremental cache key generation
  - Validate dependency graph construction
  - Test cache invalidation logic
  - Verify merge strategies

- **Integration Tests**:
  - Test single file change scenarios
  - Test multiple file change scenarios
  - Test module boundary changes
  - Validate cache coherency

- **Performance Tests**:
  - Benchmark incremental vs full analysis
  - Measure cache memory overhead
  - Test cache lookup performance
  - Validate target performance metrics

- **Correctness Tests**:
  - Compare incremental results with full recomputation
  - Test edge cases (circular dependencies, renamed files)
  - Validate cache recovery from corruption

## Documentation Requirements

- **Code Documentation**: 
  - Document cache invalidation strategies
  - Explain dependency tracking algorithm
  - Document merge strategies for each component

- **User Documentation**:
  - Update README with incremental caching benefits
  - Add troubleshooting guide for cache issues
  - Document cache tuning parameters

- **Architecture Updates**:
  - Update ARCHITECTURE.md with multi-level cache design
  - Document data flow for incremental analysis
  - Add cache coherency guarantees

## Implementation Notes

### Phase 1: Foundation (Week 1)
- Implement dependency graph tracking
- Create multi-level cache structure
- Add change detection mechanism

### Phase 2: Call Graph (Week 2)
- Implement incremental call graph updates
- Add module-level call graph caching
- Test cross-module dependency tracking

### Phase 3: Aggregation (Week 3)
- Implement incremental score calculation
- Add file and module aggregation caching
- Implement merge strategies

### Phase 4: Integration (Week 4)
- Integrate all incremental components
- Add fallback mechanisms
- Performance optimization and testing

### Optimization Opportunities

1. **Parallel Cache Operations**
   - Parallelize cache key generation
   - Concurrent cache lookups for independent components
   - Parallel merge operations

2. **Smart Invalidation**
   - Use AST diffing to determine change impact
   - Track semantic vs syntactic changes
   - Implement conservative vs aggressive invalidation modes

3. **Cache Compression**
   - Compress cached data in memory and disk
   - Use differential compression for similar entries
   - Implement lazy decompression

### Risk Mitigation

1. **Complexity Risk**
   - Start with simple incremental updates
   - Add sophistication incrementally
   - Maintain full recomputation fallback

2. **Correctness Risk**
   - Extensive testing against full recomputation
   - Add cache validation checks
   - Implement cache consistency auditing

3. **Performance Risk**
   - Profile each incremental component
   - Set performance gates for each phase
   - Optimize hot paths identified by profiling

## Migration and Compatibility

During the prototype phase, breaking changes are allowed. The incremental cache will:
- Replace the current all-or-nothing cache
- Require cache format migration (automatic on first run)
- Maintain backward compatibility with existing CLI flags
- Provide opt-out mechanism (`--no-incremental-cache`) during transition