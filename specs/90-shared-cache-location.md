---
number: 90
title: Shared Cache Location for Worktree Isolation
category: storage
priority: high
status: draft
dependencies: [86]
created: 2025-09-03
---

# Specification 90: Shared Cache Location for Worktree Isolation

**Category**: storage
**Priority**: high
**Status**: draft
**Dependencies**: [86] Cache Versioning and Invalidation

## Context

Currently, the cache is stored in `.debtmap_cache/` within the repository, which is gitignored. This creates isolation between git worktrees, meaning:
- Each worktree maintains its own cache
- No cache reuse between parallel workflows
- Redundant analysis of unchanged files across worktrees
- Increased disk usage from duplicate caches
- Slower analysis in new worktrees

In workflows that use git worktrees (like Prodigy workflows), this isolation causes significant performance overhead as each worktree must rebuild its cache from scratch, even for files that haven't changed from the base branch.

Moving to a shared cache location outside the repository would enable cache reuse across worktrees while maintaining cache correctness through proper invalidation strategies.

## Objective

Implement a shared cache system that stores cache data in a user-wide or project-wide location outside the repository, enabling efficient cache reuse across git worktrees while maintaining cache integrity and isolation where needed.

## Requirements

### Functional Requirements
- Store cache in XDG-compliant directory structure
- Support per-project cache isolation
- Enable cache sharing across worktrees of same project
- Maintain cache correctness with file content hashing
- Support fallback to local cache if needed
- Allow configuration via environment variables
- Provide cache location discovery mechanism
- Support portable cache paths for CI/CD

### Non-Functional Requirements
- No performance degradation from shared location
- Thread-safe access across multiple processes
- Atomic operations to prevent corruption
- Efficient cache key generation
- Platform-independent path handling

## Acceptance Criteria

- [ ] Cache is stored in platform-appropriate location by default
- [ ] Multiple worktrees can share the same cache
- [ ] Cache location is configurable via environment variable
- [ ] File content hashing ensures cache validity
- [ ] Concurrent access from multiple processes works correctly
- [ ] Cache can be explicitly scoped to specific branches
- [ ] Migration from local to shared cache works seamlessly
- [ ] Cache location is reported in verbose output
- [ ] Documentation explains configuration options
- [ ] Tests verify multi-process cache access

## Technical Details

### Implementation Approach
1. Implement XDG Base Directory compliance
2. Generate project-specific cache keys
3. Add file content hashing for validation
4. Implement lock-free concurrent access
5. Create migration utilities for existing caches

### Architecture Changes
```rust
pub struct CacheLocation {
    strategy: CacheStrategy,
    base_path: PathBuf,
    project_id: String,
}

pub enum CacheStrategy {
    Local,           // .debtmap_cache in repo (legacy)
    Shared,          // XDG cache dir (default)
    Custom(PathBuf), // User-specified location
}

impl CacheLocation {
    pub fn resolve() -> Result<Self> {
        // Priority order:
        // 1. DEBTMAP_CACHE_DIR environment variable
        // 2. XDG_CACHE_HOME/debtmap/<project-id>
        // 3. ~/.cache/debtmap/<project-id> (fallback)
        // 4. ./.debtmap_cache (fallback if no permissions)
    }
    
    pub fn project_id(repo_path: &Path) -> String {
        // Generate stable ID from:
        // - Remote URL (if git repo)
        // - Absolute path hash (fallback)
        // - User override (if specified)
    }
}

pub struct SharedCache {
    location: CacheLocation,
    lock_manager: LockManager,
    index: Arc<RwLock<CacheIndex>>,
}

impl SharedCache {
    pub fn ensure_thread_safe(&self) -> Result<()>;
    pub fn migrate_from_local(&self, local_path: &Path) -> Result<()>;
    pub fn scope_to_branch(&mut self, branch: &str) -> Result<()>;
}
```

### Data Structures
- `CacheLocation` for path resolution
- `ProjectIdentifier` for unique project identification
- `LockManager` for concurrent access control
- `CacheScope` for branch-specific isolation

### APIs and Interfaces
```rust
// Environment variables
pub const DEBTMAP_CACHE_DIR: &str = "DEBTMAP_CACHE_DIR";
pub const DEBTMAP_CACHE_STRATEGY: &str = "DEBTMAP_CACHE_STRATEGY";
pub const DEBTMAP_CACHE_SCOPE: &str = "DEBTMAP_CACHE_SCOPE";

// Platform-specific paths
#[cfg(target_os = "macos")]
pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("~/Library/Caches"))
        .join("debtmap")
}

#[cfg(target_os = "linux")]
pub fn default_cache_dir() -> PathBuf {
    xdg::BaseDirectories::new()
        .map(|xdg| xdg.get_cache_home())
        .unwrap_or_else(|_| PathBuf::from("~/.cache"))
        .join("debtmap")
}

#[cfg(target_os = "windows")]
pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("%LOCALAPPDATA%"))
        .join("debtmap")
}
```

## Dependencies

- **Prerequisites**: [86] Cache Versioning (for version compatibility)
- **Affected Components**: 
  - `src/core/cache.rs` - Core cache implementation
  - `src/commands/analyze.rs` - Cache initialization
  - `src/config.rs` - Configuration management
- **External Dependencies**: 
  - `dirs` or `xdg` for platform directories
  - `sha2` for project ID generation
  - `fs2` for file locking (optional)

## Testing Strategy

- **Unit Tests**: 
  - Test cache location resolution
  - Verify project ID generation
  - Test path construction
  - Validate environment variable handling
- **Integration Tests**: 
  - Test multi-process cache access
  - Verify worktree cache sharing
  - Test migration from local cache
  - Confirm branch scoping works
- **Performance Tests**: 
  - Compare shared vs local cache performance
  - Test concurrent access overhead
  - Measure lock contention
- **User Acceptance**: 
  - Worktrees share cache effectively
  - No corruption under concurrent use
  - Cache location is predictable
  - Migration is seamless

## Documentation Requirements

- **Code Documentation**: 
  - Document cache location strategy
  - Explain project ID generation
  - Document locking approach
- **User Documentation**: 
  - Add cache configuration section to README
  - Explain shared cache benefits
  - Document environment variables
  - Provide troubleshooting guide
- **Architecture Updates**: 
  - Update cache architecture diagram
  - Document multi-process considerations

## Implementation Notes

- Use content-addressable storage for cache entries
- Implement optimistic locking for better performance
- Consider using memory-mapped files for index
- Add cache statistics per worktree
- Implement cache cleanup for orphaned worktrees
- Use atomic rename for safe updates
- Consider implementing cache namespaces for experiments

### Cache Directory Structure
```
~/.cache/debtmap/
├── projects/
│   ├── <project-id-1>/
│   │   ├── index.json.zst
│   │   ├── metadata.json
│   │   └── entries/
│   │       ├── <hash-1>.cache
│   │       └── <hash-2>.cache
│   └── <project-id-2>/
│       └── ...
└── global/
    └── config.json
```

## Migration and Compatibility

- Detect existing local cache on first run
- Offer automatic migration to shared location
- Support gradual migration (copy-on-write)
- Maintain backward compatibility with local cache
- Provide `--cache-location` CLI option
- Support `DEBTMAP_CACHE_STRATEGY=local` for legacy behavior