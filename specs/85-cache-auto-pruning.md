---
number: 85
title: Automatic Cache Pruning and Size Management
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-09-03
---

# Specification 85: Automatic Cache Pruning and Size Management

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The shared cache implementation in `src/cache/shared_cache.rs` provides a foundation for cache management but lacks automatic pruning mechanisms. The cache is now stored in a shared location (`.debtmap_cache` in the home directory or configured via environment variables) with component-based organization. Without proper pruning, this shared cache can lead to:
- Excessive disk usage across all projects using debtmap
- Slower cache operations due to large index size
- Stale entries from deleted projects consuming space indefinitely
- Performance degradation when loading/saving the index
- No automatic cleanup of entries for deleted files

Automatic pruning is essential for maintaining cache efficiency and preventing unbounded growth across multiple projects using the shared cache.

## Objective

Implement automatic cache pruning with configurable size limits and age-based eviction to maintain optimal cache performance and disk usage.

## Requirements

### Functional Requirements
- Automatic pruning triggered when cache exceeds size threshold
- Age-based eviction of entries older than configurable duration
- File existence validation to remove entries for deleted files
- LRU (Least Recently Used) eviction when size limit is reached
- Configurable pruning strategies via environment variables or config
- Background pruning to avoid blocking analysis operations

### Non-Functional Requirements
- Pruning operations should not impact analysis performance
- Maintain cache hit rate above 60% after pruning
- Index file size should not exceed configurable limit (default 10MB)
- Pruning should complete within 100ms for typical cache sizes

## Acceptance Criteria

- [ ] Cache automatically prunes entries when index exceeds size limit
- [ ] Old entries are automatically removed based on configurable age
- [ ] Entries for non-existent files are automatically cleaned up
- [ ] LRU eviction is implemented for size-based pruning
- [ ] Pruning can be configured via environment variables
- [ ] Background pruning does not block cache operations
- [ ] Cache statistics include pruning metrics
- [ ] Unit tests verify all pruning strategies
- [ ] Integration tests confirm pruning under load
- [ ] Documentation explains pruning configuration

## Technical Details

### Implementation Approach
1. Add `last_accessed` field to `CacheEntry` for LRU tracking
2. Implement `AutoPruner` struct with configurable strategies
3. Add pruning triggers to cache operations
4. Use background thread for non-blocking pruning
5. Add configuration options for pruning parameters

### Architecture Changes
```rust
pub struct AutoPruner {
    max_size_bytes: usize,
    max_age_days: i64,
    max_entries: usize,
    prune_percentage: f32,  // How much to remove when limit hit
    strategy: PruneStrategy,
}

pub enum PruneStrategy {
    Lru,           // Least recently used
    Lfu,           // Least frequently used
    Fifo,          // First in, first out
    AgeBasedOnly,  // Only remove old entries
}

// Note: CacheMetadata in SharedCache already has:
// - last_accessed: SystemTime
// - access_count: u64
// These can be leveraged for LRU/LFU strategies

impl SharedCache {
    pub fn with_auto_pruner(self, pruner: AutoPruner) -> Self;
    pub fn trigger_pruning(&self) -> Result<PruneStats>;
}
```

### Data Structures
- Extended `CacheEntry` with access tracking fields
- New `AutoPruner` configuration struct
- Pruning statistics tracking

### APIs and Interfaces
```rust
impl SharedCache {
    pub fn with_auto_pruning(repo_path: Option<&Path>, pruner: AutoPruner) -> Result<Self>;
    pub fn trigger_pruning_if_needed(&self) -> Result<PruneStats>;
    pub fn prune_with_strategy(&self, strategy: PruneStrategy) -> Result<PruneStats>;
    pub fn cleanup_old_entries(&self, max_age_days: i64) -> Result<usize>;
}

impl AnalysisCache {
    pub fn trigger_shared_cache_pruning(&self) -> Result<PruneStats>;
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/cache/shared_cache.rs` - Shared cache implementation
  - `src/cache/cache_location.rs` - Cache location management
  - `src/core/cache.rs` - Analysis cache wrapper
  - `src/commands/analyze.rs` - Cache initialization
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: 
  - Test each pruning strategy independently
  - Verify size limit enforcement
  - Test age-based eviction
  - Validate file existence checking
- **Integration Tests**: 
  - Test pruning under concurrent access
  - Verify background pruning doesn't block operations
  - Test configuration from environment variables
- **Performance Tests**: 
  - Measure pruning time for various cache sizes
  - Verify cache hit rate maintenance
  - Test memory usage during pruning
- **User Acceptance**: 
  - Cache doesn't grow unbounded during extended use
  - Performance remains consistent with pruning enabled

## Documentation Requirements

- **Code Documentation**: 
  - Document all pruning strategies and their trade-offs
  - Explain configuration options with examples
- **User Documentation**: 
  - Add section on cache management to README
  - Document environment variables for configuration
  - Provide tuning recommendations
- **Architecture Updates**: 
  - Update cache architecture documentation with pruning behavior

## Implementation Notes

- Consider using `notify` crate for file system monitoring to detect deleted files
- Implement pruning in chunks to avoid locking cache for extended periods  
- Use atomic operations for access counting to avoid race conditions
- Consider implementing cache warming after aggressive pruning
- Default to conservative pruning to maintain hit rate

## Migration and Compatibility

- Breaking change: CacheEntry structure will change
- Existing cache files will need migration or will be invalidated
- Add version field to cache format for future migrations
- Provide cache migration utility or auto-migration on first run