---
number: 87
title: Extended Caching for Call Graphs and Complex Computations
category: optimization
priority: high
status: draft
dependencies: [86]
created: 2025-09-03
---

# Specification 87: Extended Caching for Call Graphs and Complex Computations

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [86] Cache Versioning and Invalidation

## Context

The current cache implementation only stores `FileMetrics`, but many expensive computations are performed repeatedly:
- Call graph construction requires parsing all files
- Dependency analysis rebuilds graphs on every run
- Trait resolution happens multiple times
- Pattern detection re-scans the same code
- Cross-file analysis isn't cached at all

These operations can take minutes on large codebases, even when the underlying files haven't changed. Extending the cache to cover these computations would dramatically improve performance for incremental analysis.

## Objective

Extend the caching system to store and retrieve expensive computational results beyond basic file metrics, including call graphs, dependency trees, and cross-file analysis results.

## Requirements

### Functional Requirements
- Cache call graph data with proper invalidation
- Store dependency analysis results
- Cache trait resolution mappings
- Store pattern detection results
- Cache cross-file relationships
- Support incremental updates for graph structures
- Provide cache keys for complex computations

### Non-Functional Requirements
- Maintain cache coherency across related data
- Support partial cache updates
- Minimize serialization overhead
- Enable lazy loading of cached data

## Acceptance Criteria

- [ ] Call graphs are cached and reused
- [ ] Dependency trees are stored persistently
- [ ] Trait resolutions are cached per module
- [ ] Pattern detection results are cached
- [ ] Cross-file relationships are preserved
- [ ] Incremental updates work correctly
- [ ] Cache invalidation cascades properly
- [ ] Performance improves by >50% on cache hits
- [ ] Memory usage remains reasonable
- [ ] Cache corruption is detected and handled

## Technical Details

### Implementation Approach
1. Create specialized cache stores for different data types
2. Implement cache key generation for complex queries
3. Add dependency tracking between cached items
4. Implement lazy deserialization for large structures
5. Create cache layers for different computation levels

### Architecture Changes
```rust
pub enum CacheableComputation {
    CallGraph(CallGraphData),
    DependencyTree(DependencyData),
    TraitResolution(TraitMap),
    PatternMatches(Vec<Pattern>),
    CrossFileRelations(RelationGraph),
}

pub struct ComputationCache {
    file_cache: AnalysisCache,
    graph_cache: GraphCache,
    pattern_cache: PatternCache,
    relation_cache: RelationCache,
}

pub struct GraphCache {
    call_graphs: HashMap<CacheKey, CallGraphData>,
    dependency_trees: HashMap<CacheKey, DependencyData>,
    invalidation_map: HashMap<PathBuf, Vec<CacheKey>>,
}

impl ComputationCache {
    pub fn get_call_graph(&mut self, files: &[PathBuf]) -> Result<CallGraphData>;
    pub fn get_dependencies(&mut self, module: &Path) -> Result<DependencyData>;
    pub fn invalidate_related(&mut self, changed_file: &Path) -> Result<()>;
}
```

### Data Structures
- `CacheKey` type for identifying computations
- Specialized cache stores for different data types
- Invalidation dependency graph
- Lazy-loaded cache entries

### APIs and Interfaces
```rust
pub trait Cacheable: Serialize + DeserializeOwned {
    fn cache_key(&self) -> CacheKey;
    fn dependencies(&self) -> Vec<CacheKey>;
    fn is_valid(&self, context: &CacheContext) -> bool;
}

impl CallGraphBuilder {
    pub fn build_with_cache(&mut self, cache: &mut GraphCache) -> Result<CallGraph>;
}

impl DependencyAnalyzer {
    pub fn analyze_with_cache(&mut self, cache: &mut ComputationCache) -> Result<Dependencies>;
}
```

## Dependencies

- **Prerequisites**: [86] Cache Versioning (for version tracking)
- **Affected Components**: 
  - `src/core/cache.rs` - Base cache implementation
  - `src/analyzers/call_graph.rs` - Call graph caching
  - `src/analyzers/dependency.rs` - Dependency caching
  - `src/analyzers/trait_resolver.rs` - Trait resolution caching
- **External Dependencies**: 
  - Consider `bincode` for efficient binary serialization
  - Consider `zstd` for compression

## Testing Strategy

- **Unit Tests**: 
  - Test cache key generation
  - Verify dependency tracking
  - Test invalidation cascading
  - Validate lazy loading
- **Integration Tests**: 
  - Test full analysis with caching
  - Verify incremental updates
  - Test cache coherency
  - Confirm memory bounds
- **Performance Tests**: 
  - Measure speedup on second run
  - Test large graph caching
  - Verify serialization overhead
- **User Acceptance**: 
  - Subsequent analyses are noticeably faster
  - Results remain accurate with caching
  - Memory usage is acceptable

## Documentation Requirements

- **Code Documentation**: 
  - Document cache key generation strategy
  - Explain invalidation rules
  - Document memory management approach
- **User Documentation**: 
  - Explain extended caching benefits
  - Document cache size implications
  - Provide performance tuning guide
- **Architecture Updates**: 
  - Add caching layers to architecture diagram
  - Document data flow with caching

## Implementation Notes

- Use content-based addressing for cache keys
- Implement reference counting for shared data
- Consider memory-mapped files for large caches
- Use copy-on-write for cache updates
- Implement cache warming strategies
- Add metrics for cache effectiveness per computation type

## Migration and Compatibility

- Gracefully handle missing cache entries
- Provide fallback to non-cached computation
- Allow selective enabling of cache layers
- Support cache export/import for CI systems
- Maintain compatibility with existing file-only cache