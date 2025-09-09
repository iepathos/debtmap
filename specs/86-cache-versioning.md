---
number: 86
title: Cache Versioning and Invalidation
category: storage
priority: critical
status: draft
dependencies: []
created: 2025-09-03
---

# Specification 86: Cache Versioning and Invalidation

**Category**: storage
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The shared cache implementation in `src/cache/shared_cache.rs` lacks version tracking, which means cached data remains valid even when the analysis algorithms change. With the shared cache serving multiple projects and potentially multiple versions of debtmap, this leads to:
- Incorrect analysis results when algorithms are updated
- No automatic invalidation when debtmap is upgraded
- Developer confusion when cached results don't reflect code changes
- Conflicts when different debtmap versions access the same shared cache

Cache versioning is critical for maintaining correctness as the analysis engine evolves and ensuring users always get accurate results across all projects using the shared cache.

## Objective

Implement a simple cache versioning system that automatically invalidates all cache entries when the debtmap version changes, ensuring correctness over optimization.

## Requirements

### Functional Requirements
- Version tracking tied to debtmap version from Cargo.toml
- Automatic full cache invalidation when version mismatch detected
- Clear cache on any version change (patch, minor, or major)
- Version mismatch reporting in verbose mode
- File hash validation remains as primary invalidation mechanism

### Non-Functional Requirements
- Zero performance impact when versions match
- Fast version checking (< 1ms)
- Minimal storage overhead for version metadata
- Clear error messages for version mismatches
- Simple implementation without complex compatibility logic

## Acceptance Criteria

- [ ] Cache includes debtmap version in metadata
- [ ] Cache automatically clears on any version mismatch
- [ ] File hash validation continues to work as primary mechanism
- [ ] Version mismatches are logged appropriately
- [ ] Unit tests verify version checking logic
- [ ] Integration tests confirm invalidation behavior
- [ ] Cache clearing is atomic and safe
- [ ] Old cache entries are fully removed on version change

## Technical Details

### Implementation Approach
1. Add debtmap version to cache metadata
2. Implement version checking on cache initialization
3. Clear entire cache on version mismatch
4. Log version changes for debugging
5. Maintain file hash as primary cache key

### Architecture Changes
```rust
// Extend existing CacheMetadata in shared_cache.rs
#[derive(Serialize, Deserialize)]
pub struct CacheMetadata {
    pub version: String,  // Existing field
    pub created_at: SystemTime,  // Existing field
    pub last_accessed: SystemTime,  // Existing field
    pub access_count: u64,  // Existing field
    pub size_bytes: u64,  // Existing field
    // New field for versioning:
    pub debtmap_version: String,  // From env!("CARGO_PKG_VERSION")
}

impl SharedCache {
    pub fn validate_version(&self) -> Result<bool> {
        let current_version = env!("CARGO_PKG_VERSION");
        if self.metadata.debtmap_version != current_version {
            // Version mismatch - clear entire cache
            self.clear()?;
            info!("Cache cleared due to version change: {} -> {}", 
                  self.metadata.debtmap_version, current_version);
            return Ok(false);
        }
        Ok(true)
    }
    
    pub fn clear(&self) -> Result<()>;
}

impl AnalysisCache {
    pub fn new_with_version_check(project_path: Option<&Path>) -> Result<Self>;
}
```

### Data Structures
- Extended `CacheMetadata` with debtmap version field
- Simple string comparison for version checking
- No complex version compatibility logic
- Cache key includes file hash as primary identifier

### APIs and Interfaces
```rust
// Cache initialization with version checking
impl SharedCache {
    pub fn new_with_version(repo_path: Option<&Path>) -> Result<Self> {
        let mut cache = Self::new(repo_path)?;
        cache.validate_version()?;
        Ok(cache)
    }
    
    // Cache key generation includes file hash
    pub fn compute_cache_key(&self, file_path: &Path) -> Result<String> {
        let content = fs::read_to_string(file_path)?;
        let hash = sha256::digest(&content);
        Ok(format!("{}:{}", file_path.display(), hash))
    }
}

impl AnalysisCache {
    pub fn new_with_version_check(project_path: Option<&Path>) -> Result<Self> {
        let shared_cache = SharedCache::new_with_version(project_path)?;
        // Rest of initialization...
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/cache/shared_cache.rs` - Add version checking and cache clearing
  - `src/core/cache.rs` - Analysis cache version checking
  - `src/commands/analyze.rs` - Cache initialization with version
- **External Dependencies**: None (uses built-in version from Cargo.toml)

## Testing Strategy

- **Unit Tests**: 
  - Test version mismatch detection
  - Verify cache clearing on version change
  - Test file hash validation still works
  - Validate cache key generation
- **Integration Tests**: 
  - Test cache behavior across version changes
  - Verify full cache invalidation
  - Confirm file changes still trigger recalculation
  - Test atomic cache clearing
- **Performance Tests**: 
  - Measure version checking overhead (should be < 1ms)
  - Test cache clearing performance
- **User Acceptance**: 
  - Cache clears when upgrading debtmap
  - Analysis results are always current
  - Version changes are logged clearly

## Documentation Requirements

- **Code Documentation**: 
  - Document that any version change clears cache
  - Explain file hash remains primary cache key
  - Document cache clearing behavior
- **User Documentation**: 
  - Explain automatic cache clearing on upgrade
  - Document how to clear cache manually if needed
  - Note that cache is shared across projects
- **Architecture Updates**: 
  - Add simple versioning strategy to architecture docs
  - Document cache invalidation hierarchy

## Implementation Notes

- Use `env!("CARGO_PKG_VERSION")` for version tracking
- Version checking happens on cache initialization
- Add `--force-cache-rebuild` CLI option for manual clearing
- Log version changes at INFO level
- Cache clearing should be atomic to prevent corruption
- File hash validation remains the primary mechanism

## Migration and Compatibility

- Any version change triggers full cache clear
- No complex migration logic needed
- Old cache entries are completely removed
- New cache starts fresh with current version
- Simple and predictable behavior for users