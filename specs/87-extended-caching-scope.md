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

The shared cache implementation in `src/cache/shared_cache.rs` provides a component-based caching infrastructure that currently stores `FileMetrics` through the `AnalysisCache` wrapper. However, many expensive computations are performed repeatedly:
- Call graph construction requires parsing all files
- Dependency analysis rebuilds graphs on every run
- Trait resolution happens multiple times
- Pattern detection re-scans the same code
- Cross-file analysis isn't cached at all

The shared cache's component-based design makes it ideal for storing these different types of computational results in separate namespaces. These operations can take minutes on large codebases, even when the underlying files haven't changed. Extending the cache to cover these computations would dramatically improve performance for incremental analysis across all projects using the shared cache.

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

// Leverage SharedCache's component-based design
pub struct ComputationCache {
    shared_cache: SharedCache,
    // Components stored in separate namespaces:
    // - "analysis" for FileMetrics
    // - "call_graph" for call graph data
    // - "dependencies" for dependency trees
    // - "traits" for trait resolutions
    // - "patterns" for pattern matches
}

impl ComputationCache {
    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let shared_cache = SharedCache::new(repo_path)?;
        Ok(Self { shared_cache })
    }
    
    pub fn get_call_graph(&self, files: &[PathBuf]) -> Result<CallGraphData> {
        let key = self.compute_cache_key(files);
        self.shared_cache.get(&key, "call_graph")
    }
    
    pub fn get_dependencies(&self, module: &Path) -> Result<DependencyData> {
        let key = self.compute_cache_key(&[module.to_path_buf()]);
        self.shared_cache.get(&key, "dependencies")
    }
    
    pub fn invalidate_related(&self, changed_file: &Path) -> Result<()> {
        // Use SharedCache's component isolation for targeted invalidation
        self.shared_cache.invalidate_component_entries(changed_file)
    }
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
    pub fn build_with_cache(&mut self, cache: &ComputationCache) -> Result<CallGraph> {
        // Try to get from shared cache's "call_graph" component
        if let Ok(cached) = cache.get_call_graph(&self.files) {
            return Ok(cached);
        }
        // Build and store in cache
        let graph = self.build()?;
        cache.shared_cache.put(&key, "call_graph", &graph)?;
        Ok(graph)
    }
}

impl DependencyAnalyzer {
    pub fn analyze_with_cache(&mut self, cache: &ComputationCache) -> Result<Dependencies> {
        // Similar pattern using "dependencies" component
    }
}
```

## Dependencies

- **Prerequisites**: [86] Cache Versioning (for version tracking)
- **Affected Components**: 
  - `src/cache/shared_cache.rs` - Component-based caching backend
  - `src/core/cache.rs` - Extended to support computation caching
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