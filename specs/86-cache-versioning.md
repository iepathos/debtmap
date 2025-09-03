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

The shared cache implementation in `src/cache/shared_cache.rs` lacks comprehensive versioning, which means cached data remains valid even when the analysis algorithms change. With the shared cache serving multiple projects and potentially multiple versions of debtmap, this leads to:
- Incorrect analysis results when algorithms are updated
- No automatic invalidation when debtmap is upgraded
- Potential compatibility issues between cache format versions across different debtmap installations
- Silent failures when cache structure changes
- Developer confusion when cached results don't reflect code changes
- Conflicts when different debtmap versions access the same shared cache

Cache versioning is critical for maintaining correctness as the analysis engine evolves and ensuring users always get accurate results across all projects using the shared cache.

## Objective

Implement a robust cache versioning system that automatically invalidates stale cache entries when the analysis version changes and provides backward compatibility where possible.

## Requirements

### Functional Requirements
- Version tracking for both cache format and analysis algorithms
- Automatic cache invalidation when version mismatch detected
- Semantic versioning for cache compatibility
- Algorithm-specific versioning for granular invalidation
- Migration support for compatible cache format changes
- Version mismatch reporting in verbose mode

### Non-Functional Requirements
- Zero performance impact when versions match
- Fast version checking (< 1ms)
- Minimal storage overhead for version metadata
- Clear error messages for version conflicts

## Acceptance Criteria

- [ ] Cache includes version metadata in index
- [ ] Cache automatically invalidates on version mismatch
- [ ] Semantic versioning determines compatibility
- [ ] Per-analyzer versioning allows selective invalidation
- [ ] Cache format migrations work for minor version changes
- [ ] Version conflicts are logged appropriately
- [ ] Unit tests verify version checking logic
- [ ] Integration tests confirm invalidation behavior
- [ ] Version bumping is documented in contributor guide
- [ ] Backward compatibility is maintained where specified

## Technical Details

### Implementation Approach
1. Add version metadata to cache index structure
2. Implement version checking on cache initialization
3. Create version registry for analyzers
4. Add migration system for cache format changes
5. Implement selective invalidation based on analyzer versions

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
    // New fields for versioning:
    pub cache_version: CacheVersion,
    pub analyzer_versions: HashMap<String, Version>,
    pub debtmap_version: String,
}

#[derive(Serialize, Deserialize)]
pub struct CacheVersion {
    pub major: u32,  // Breaking changes
    pub minor: u32,  // Backward compatible changes
    pub patch: u32,  // Bug fixes
}

pub trait Versioned {
    fn version(&self) -> Version;
    fn compatible_with(&self, other: &Version) -> bool;
}

impl SharedCache {
    pub fn validate_version(&self) -> Result<ValidationResult>;
    pub fn migrate_if_needed(&self) -> Result<()>;
    pub fn invalidate_by_analyzer(&self, analyzer: &str) -> Result<()>;
    pub fn is_compatible(&self, key: &str) -> Result<bool>;
}

impl AnalysisCache {
    pub fn check_version_compatibility(&self) -> Result<bool>;
}
```

### Data Structures
- New `CacheMetadata` structure for version tracking
- `Version` type with semantic versioning support
- `AnalyzerRegistry` for tracking analyzer versions
- Migration functions for cache format updates

### APIs and Interfaces
```rust
// Version constants for each analyzer
pub const COMPLEXITY_ANALYZER_VERSION: Version = Version::new(1, 0, 0);
pub const DEBT_ANALYZER_VERSION: Version = Version::new(1, 0, 0);
pub const DUPLICATION_ANALYZER_VERSION: Version = Version::new(1, 0, 0);

// Cache initialization with version checking
impl SharedCache {
    pub fn new_versioned(repo_path: Option<&Path>) -> Result<Self> {
        let mut cache = Self::new(repo_path)?;
        cache.validate_and_migrate()?;
        Ok(cache)
    }
}

impl AnalysisCache {
    pub fn new_with_version_check(project_path: Option<&Path>) -> Result<Self> {
        let shared_cache = SharedCache::new_versioned(project_path)?;
        // Rest of initialization...
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/cache/shared_cache.rs` - Shared cache versioning
  - `src/cache/cache_location.rs` - Version-aware cache paths
  - `src/core/cache.rs` - Analysis cache version checking
  - `src/commands/analyze.rs` - Cache initialization
  - All analyzer modules that contribute to cached data
- **External Dependencies**: 
  - Consider `semver` crate for version parsing

## Testing Strategy

- **Unit Tests**: 
  - Test version comparison logic
  - Verify compatibility checking
  - Test migration functions
  - Validate invalidation logic
- **Integration Tests**: 
  - Test cache behavior across version changes
  - Verify selective invalidation
  - Test migration scenarios
  - Confirm backward compatibility
- **Performance Tests**: 
  - Measure version checking overhead
  - Test migration performance
- **User Acceptance**: 
  - Cache invalidates when upgrading debtmap
  - Analysis results are always current
  - Version conflicts are clearly communicated

## Documentation Requirements

- **Code Documentation**: 
  - Document version bumping guidelines
  - Explain compatibility rules
  - Document migration functions
- **User Documentation**: 
  - Explain cache versioning behavior
  - Document how to clear cache manually
  - Provide troubleshooting guide
- **Architecture Updates**: 
  - Add versioning strategy to architecture docs
  - Document analyzer version registry

## Implementation Notes

- Use build-time version injection from Cargo.toml
- Consider using compile-time version constants
- Implement "cache version override" for debugging
- Add `--force-cache-rebuild` CLI option
- Consider partial cache invalidation for efficiency
- Log version mismatches at INFO level

## Migration and Compatibility

- Version 1.0.0: Initial versioned cache format
- Minor versions (1.x.0): Automatic migration supported
- Major versions (x.0.0): Full cache rebuild required
- Patch versions (1.0.x): No cache changes needed
- Provide clear migration path documentation