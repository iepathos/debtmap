---
number: 140
title: Cache Read/Write Separation for Improved Modularity
category: storage
priority: medium
status: draft
dependencies: [87, 88, 89]
created: 2025-10-27
---

# Specification 140: Cache Read/Write Separation for Improved Modularity

**Category**: storage
**Priority**: medium
**Status**: draft
**Dependencies**: [87] Extended Caching Scope, [88] Cache Compression, [89] Cache Observability

## Context

The current `SharedCache` implementation in `src/cache/shared_cache/mod.rs` is 1,435 lines with 89 functions, handling all cache operations in a single monolithic struct. This violates the single responsibility principle and makes the module difficult to test, maintain, and extend.

Analysis from debtmap's own output shows:
- **God Object Pattern**: 77 methods, 6 fields, 9 distinct responsibilities
- **Complexity**: Mixed concerns across read operations, write operations, pruning, statistics, migration, and validation
- **Testing Challenges**: Difficult to test read/write operations independently
- **Concurrency Issues**: Readers and writers share the same lock contention

The module already has good separation of concerns for some functionality:
- `atomic_io.rs` - Atomic write operations ✅
- `auto_pruner.rs` - Pruning logic ✅
- `cache_location.rs` - Path resolution ✅
- `index_manager.rs` - Metadata management ✅

However, the core `SharedCache` struct still orchestrates too many responsibilities, mixing read-heavy operations (get, exists, compute_cache_key) with write-heavy operations (put, delete) and lifecycle management (cleanup, migration, pruning, statistics).

## Objective

Refactor `SharedCache` into three focused modules following functional programming principles:
1. **CacheReader** - Read-only operations (get, exists, compute_cache_key)
2. **CacheWriter** - Write operations (put, delete)
3. **SharedCache** - Orchestration layer composing reader, writer, and lifecycle operations

This separation will:
- Reduce cognitive load (each module has single responsibility)
- Enable independent testing of read/write logic
- Improve concurrency (multiple readers don't block each other)
- Simplify future backend abstraction (filesystem → Redis/S3)
- Follow functional programming principles (pure read operations, isolated side effects)

## Requirements

### Functional Requirements

**CacheReader Module** (`src/cache/shared_cache/reader.rs`)
- Pure function: `compute_cache_key(&self, file_path: &Path) -> Result<String>`
- Read operation: `get(&self, key: &str, component: &str) -> Result<Vec<u8>>`
- Query operation: `exists(&self, key: &str, component: &str) -> bool`
- Helper: `get_cache_file_path(&self, key: &str, component: &str) -> PathBuf`
- Statistics: `get_read_stats(&self) -> ReadStats` (hits/misses)

**CacheWriter Module** (`src/cache/shared_cache/writer.rs`)
- Write operation: `put(&self, key: &str, component: &str, data: &[u8]) -> Result<()>`
- Delete operation: `delete(&self, key: &str, component: &str) -> Result<()>`
- Batch operations: `delete_batch(&self, entries: &[(String, String)]) -> Result<usize>`
- Helper: Integrate with existing `atomic_io::write_atomically` for safe writes

**SharedCache Orchestration** (`src/cache/shared_cache/mod.rs`)
- Compose `CacheReader` and `CacheWriter`
- Lifecycle operations: `new()`, `with_auto_pruning()`, `clear()`, `migrate_from_local()`
- Pruning orchestration: `trigger_pruning()`, `prune_with_strategy()`
- Statistics aggregation: `get_stats()`, `get_full_stats()`
- Validation: `validate_version()`
- Backward compatibility: Preserve all existing public APIs

### Non-Functional Requirements

**Performance**
- Zero runtime overhead compared to current implementation
- Read operations should not block other readers
- Write operations properly synchronized with index updates
- No performance regression in benchmarks

**Maintainability**
- Each module under 500 lines
- Functions under 20 lines (per CLAUDE.md guidelines)
- Maximum cyclomatic complexity of 5 per function
- Pure functions for all computation (no side effects)

**Testing**
- Unit tests for `CacheReader` with mock filesystem
- Unit tests for `CacheWriter` with temporary directories
- Integration tests for `SharedCache` orchestration
- Property-based tests for cache key generation (proptest)
- Maintain >85% test coverage

**Compatibility**
- All existing public APIs must remain unchanged
- No breaking changes to consumers (AnalysisCache, CallGraphCache, etc.)
- Existing cache data must work without migration
- Same cache file structure and layout

## Acceptance Criteria

- [ ] **Module Structure Created**
  - `src/cache/shared_cache/reader.rs` exists with CacheReader struct
  - `src/cache/shared_cache/writer.rs` exists with CacheWriter struct
  - `src/cache/shared_cache/mod.rs` reduced from 1435 to <400 lines
  - Each new module is <500 lines

- [ ] **CacheReader Implementation**
  - `compute_cache_key()` is pure function (no side effects)
  - `get()` reads from filesystem and updates index access metadata
  - `exists()` checks without side effects
  - `get_cache_file_path()` helper is pure function
  - All read operations are concurrent-safe

- [ ] **CacheWriter Implementation**
  - `put()` writes atomically using `atomic_io` module
  - `delete()` removes cache file and updates index
  - `delete_batch()` efficiently removes multiple entries
  - All writes properly synchronized with index manager

- [ ] **SharedCache Orchestration**
  - Composes CacheReader and CacheWriter correctly
  - All 23 existing public methods still work
  - Pruning operations delegate to reader for listing, writer for deletion
  - Statistics aggregate from both reader and writer

- [ ] **Testing**
  - CacheReader has unit tests with in-memory/temp filesystem
  - CacheWriter has unit tests for atomic writes
  - SharedCache has integration tests for full lifecycle
  - Property tests verify cache key generation invariants
  - All existing tests still pass
  - Test coverage remains >85%

- [ ] **Code Quality**
  - All functions <20 lines
  - Cyclomatic complexity <5 per function
  - No clippy warnings
  - cargo fmt passes
  - cargo deny check passes

- [ ] **Documentation**
  - Module-level docs for reader.rs explaining read-only operations
  - Module-level docs for writer.rs explaining write safety guarantees
  - Updated mod.rs docs explaining orchestration pattern
  - Examples showing usage of separated concerns
  - Architecture notes on future backend abstraction

- [ ] **Performance Validation**
  - Benchmarks show no regression vs baseline
  - Concurrent read benchmark shows improvement
  - Write throughput maintained or improved
  - Memory usage unchanged

## Technical Details

### Implementation Approach

**Phase 1: Extract CacheReader (Pure Operations)**

1. Create `src/cache/shared_cache/reader.rs`
2. Move pure computation functions:
   ```rust
   // Pure function - no side effects
   fn compute_cache_key(file_path: &Path) -> Result<String> {
       let canonical = fs::canonicalize(file_path)?;
       let path_str = canonical.to_string_lossy();
       Ok(format!("{:x}", md5::compute(path_str.as_bytes())))
   }

   // Pure function - path computation only
   fn get_cache_file_path(location: &CacheLocation, key: &str, component: &str) -> PathBuf {
       location.cache_dir()
           .join(&key[..2])
           .join(&key[2..4])
           .join(format!("{}.{}", key, component))
   }
   ```

3. Move read operations with index manager:
   ```rust
   pub struct CacheReader {
       location: CacheLocation,
       index_manager: Arc<IndexManager>, // Shared with writer
   }

   impl CacheReader {
       pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
           let cache_path = Self::get_cache_file_path(&self.location, key, component);

           if !cache_path.exists() {
               anyhow::bail!("Cache entry not found");
           }

           // Update access metadata (only side effect)
           self.index_manager.update_access_metadata(key)?;

           fs::read(&cache_path)
               .with_context(|| format!("Failed to read cache file: {:?}", cache_path))
       }

       pub fn exists(&self, key: &str, component: &str) -> bool {
           let cache_path = Self::get_cache_file_path(&self.location, key, component);
           cache_path.exists()
       }
   }
   ```

**Phase 2: Extract CacheWriter (I/O Operations)**

1. Create `src/cache/shared_cache/writer.rs`
2. Move write operations:
   ```rust
   pub struct CacheWriter {
       location: CacheLocation,
       index_manager: Arc<IndexManager>, // Shared with reader
   }

   impl CacheWriter {
       pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
           let cache_path = Self::get_cache_file_path(&self.location, key, component);

           // Ensure parent directories exist
           if let Some(parent) = cache_path.parent() {
               fs::create_dir_all(parent)?;
           }

           // Use existing atomic write logic
           crate::cache::atomic_io::write_atomically(&cache_path, data)?;

           // Update index
           let metadata = CacheMetadata::new(data.len() as u64);
           self.index_manager.add_or_update_entry(key, metadata)?;

           Ok(())
       }

       pub fn delete(&self, key: &str, component: &str) -> Result<()> {
           let cache_path = Self::get_cache_file_path(&self.location, key, component);

           if cache_path.exists() {
               fs::remove_file(&cache_path)?;
           }

           // Update index (remove metadata)
           self.index_manager.remove_entry(key)?;

           Ok(())
       }

       // Batch deletion for pruning
       pub fn delete_batch(&self, entries: &[(String, String)]) -> Result<usize> {
           let mut deleted_count = 0;

           for (key, component) in entries {
               if self.delete(key, component).is_ok() {
                   deleted_count += 1;
               }
           }

           Ok(deleted_count)
       }
   }
   ```

**Phase 3: Refactor SharedCache to Orchestrate**

1. Simplify `SharedCache` to compose reader/writer:
   ```rust
   pub struct SharedCache {
       reader: CacheReader,
       writer: CacheWriter,
       location: CacheLocation, // Keep for backward compat
       auto_pruner: Option<AutoPruner>,
       background_pruner: Option<BackgroundPruner>,
   }

   impl SharedCache {
       pub fn new(repo_path: Option<&Path>) -> Result<Self> {
           let location = CacheLocation::resolve(repo_path)?;
           location.ensure_directories()?;

           let index_manager = Arc::new(IndexManager::load_or_create(&location)?);

           let reader = CacheReader::new(location.clone(), index_manager.clone());
           let writer = CacheWriter::new(location.clone(), index_manager);

           let auto_pruner = AutoPruner::from_env()?;

           let cache = Self {
               reader,
               writer,
               location,
               auto_pruner: Some(auto_pruner),
               background_pruner: None,
           };

           cache.validate_version()?;
           Ok(cache)
       }

       // Delegate to reader
       pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
           self.reader.get(key, component)
       }

       pub fn exists(&self, key: &str, component: &str) -> bool {
           self.reader.exists(key, component)
       }

       pub fn compute_cache_key(&self, file_path: &Path) -> Result<String> {
           self.reader.compute_cache_key(file_path)
       }

       // Delegate to writer
       pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
           self.writer.put(key, component, data)?;

           // Orchestrate pruning if needed
           if let Some(ref pruner) = self.auto_pruner {
               self.trigger_pruning_if_needed()?;
           }

           Ok(())
       }

       pub fn delete(&self, key: &str, component: &str) -> Result<()> {
           self.writer.delete(key, component)
       }

       // Orchestration operations use both
       pub fn prune_with_strategy(&self, strategy: PruneStrategy) -> Result<PruneStats> {
           let entries = self.reader.list_entries()?; // Read
           let to_remove = strategy.select_entries_to_remove(&entries);
           let removed = self.writer.delete_batch(&to_remove)?; // Write

           Ok(PruneStats {
               removed_count: removed,
               freed_bytes: to_remove.iter().map(|e| e.size_bytes).sum(),
           })
       }
   }
   ```

### Architecture Changes

**Before (Monolithic)**
```
src/cache/shared_cache/
└── mod.rs (1435 lines)
    - 23 public methods
    - Mixed read/write/lifecycle operations
    - Hard to test in isolation
```

**After (Separated Concerns)**
```
src/cache/shared_cache/
├── mod.rs (~300 lines) - Orchestration layer
├── reader.rs (~300 lines) - Read-only operations
└── writer.rs (~400 lines) - Write operations + pruning helpers
```

### Data Structures

**Shared State (via Arc)**
```rust
// Both reader and writer share index_manager
let index_manager = Arc::new(IndexManager::load_or_create(&location)?);

pub struct CacheReader {
    location: CacheLocation,
    index_manager: Arc<IndexManager>,
}

pub struct CacheWriter {
    location: CacheLocation,
    index_manager: Arc<IndexManager>,
}
```

**Pure Functions (No Shared State)**
```rust
// Static methods for pure computations
impl CacheReader {
    fn compute_cache_key(file_path: &Path) -> Result<String> { /* ... */ }
    fn get_cache_file_path(location: &CacheLocation, key: &str, component: &str) -> PathBuf { /* ... */ }
}
```

### APIs and Interfaces

**Public API Changes: NONE**
- All 23 existing public methods on `SharedCache` remain unchanged
- Internal delegation is transparent to consumers
- Existing tests should pass without modification

**New Internal APIs**
```rust
// CacheReader - read-only operations
pub struct CacheReader { /* ... */ }
impl CacheReader {
    pub fn new(location: CacheLocation, index_manager: Arc<IndexManager>) -> Self;
    pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>>;
    pub fn exists(&self, key: &str, component: &str) -> bool;
    pub fn compute_cache_key(&self, file_path: &Path) -> Result<String>;
    fn get_cache_file_path(location: &CacheLocation, key: &str, component: &str) -> PathBuf;
}

// CacheWriter - write operations
pub struct CacheWriter { /* ... */ }
impl CacheWriter {
    pub fn new(location: CacheLocation, index_manager: Arc<IndexManager>) -> Self;
    pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()>;
    pub fn delete(&self, key: &str, component: &str) -> Result<()>;
    pub(crate) fn delete_batch(&self, entries: &[(String, String)]) -> Result<usize>;
}
```

### Functional Programming Principles

**Pure Functions (No Side Effects)**
```rust
// ✅ Pure - same input always gives same output
fn compute_cache_key(file_path: &Path) -> Result<String> {
    let canonical = fs::canonicalize(file_path)?; // Deterministic
    let path_str = canonical.to_string_lossy();
    Ok(format!("{:x}", md5::compute(path_str.as_bytes())))
}

// ✅ Pure - path computation only
fn get_cache_file_path(location: &CacheLocation, key: &str, component: &str) -> PathBuf {
    location.cache_dir()
        .join(&key[..2])
        .join(&key[2..4])
        .join(format!("{}.{}", key, component))
}
```

**Separation of I/O and Logic**
```rust
// ❌ Before: Mixed concerns
pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
    let cache_path = self.get_cache_file_path(key, component); // Computation
    if !cache_path.exists() { anyhow::bail!("..."); } // I/O check
    self.index_manager.update_access_metadata(key)?; // I/O write
    fs::read(&cache_path) // I/O read
}

// ✅ After: Clear I/O boundary
impl CacheReader {
    // Pure computation extracted
    fn get_cache_file_path(location: &CacheLocation, key: &str, component: &str) -> PathBuf {
        // ... pure path logic
    }

    // I/O operations clearly identified
    pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
        let cache_path = Self::get_cache_file_path(&self.location, key, component);

        // I/O boundary clearly marked
        if !cache_path.exists() { /* ... */ }
        self.index_manager.update_access_metadata(key)?; // I/O
        fs::read(&cache_path) // I/O
    }
}
```

**Composition Over Inheritance**
```rust
// ✅ SharedCache composes reader and writer
pub struct SharedCache {
    reader: CacheReader,    // Composition
    writer: CacheWriter,    // Composition
    auto_pruner: Option<AutoPruner>, // Composition
}

impl SharedCache {
    // Delegates to composed components
    pub fn get(&self, key: &str, component: &str) -> Result<Vec<u8>> {
        self.reader.get(key, component)
    }

    pub fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()> {
        self.writer.put(key, component, data)
    }
}
```

## Dependencies

### Prerequisites
- **Spec 87**: Extended Caching Scope - Defines component-based caching model
- **Spec 88**: Cache Compression - May affect read/write buffer sizes
- **Spec 89**: Cache Observability - Statistics collection needs reader/writer separation

### Affected Components
- `src/cache/shared_cache/mod.rs` - Major refactoring (1435 → ~300 lines)
- `src/cache/index_manager.rs` - Needs Arc wrapping for shared access
- `src/cache/unified_analysis_cache.rs` - Consumer of SharedCache (should work unchanged)
- `src/cache/call_graph_cache.rs` - Consumer of SharedCache (should work unchanged)

### External Dependencies
- **No new external dependencies**
- Uses existing: `anyhow`, `std::sync::Arc`, `std::fs`

## Testing Strategy

### Unit Tests

**CacheReader Tests** (`src/cache/shared_cache/reader.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_cache_key_deterministic() {
        let path = PathBuf::from("/test/file.rs");
        let key1 = CacheReader::compute_cache_key(&path).unwrap();
        let key2 = CacheReader::compute_cache_key(&path).unwrap();
        assert_eq!(key1, key2); // Pure function - deterministic
    }

    #[test]
    fn test_get_nonexistent_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::from_path(temp_dir.path());
        let index = Arc::new(IndexManager::new());
        let reader = CacheReader::new(location, index);

        assert!(reader.get("nonexistent", "ast").is_err());
    }

    #[test]
    fn test_exists_returns_false_for_missing() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::from_path(temp_dir.path());
        let index = Arc::new(IndexManager::new());
        let reader = CacheReader::new(location, index);

        assert!(!reader.exists("nonexistent", "ast"));
    }
}
```

**CacheWriter Tests** (`src/cache/shared_cache/writer.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_creates_file_atomically() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::from_path(temp_dir.path());
        let index = Arc::new(IndexManager::new());
        let writer = CacheWriter::new(location.clone(), index);

        let data = b"test data";
        writer.put("testkey", "ast", data).unwrap();

        let path = CacheWriter::get_cache_file_path(&location, "testkey", "ast");
        assert!(path.exists());

        let read_data = fs::read(&path).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_delete_removes_file() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::from_path(temp_dir.path());
        let index = Arc::new(IndexManager::new());
        let writer = CacheWriter::new(location.clone(), index);

        writer.put("testkey", "ast", b"data").unwrap();
        writer.delete("testkey", "ast").unwrap();

        let path = CacheWriter::get_cache_file_path(&location, "testkey", "ast");
        assert!(!path.exists());
    }

    #[test]
    fn test_delete_batch() {
        let temp_dir = TempDir::new().unwrap();
        let location = CacheLocation::from_path(temp_dir.path());
        let index = Arc::new(IndexManager::new());
        let writer = CacheWriter::new(location, index);

        writer.put("key1", "ast", b"data1").unwrap();
        writer.put("key2", "ast", b"data2").unwrap();
        writer.put("key3", "ast", b"data3").unwrap();

        let to_delete = vec![
            ("key1".to_string(), "ast".to_string()),
            ("key2".to_string(), "ast".to_string()),
        ];

        let deleted = writer.delete_batch(&to_delete).unwrap();
        assert_eq!(deleted, 2);
    }
}
```

**Property-Based Tests** (using `proptest`)
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn cache_key_always_32_chars(path in any::<String>()) {
        let path_buf = PathBuf::from(&path);
        if let Ok(key) = CacheReader::compute_cache_key(&path_buf) {
            prop_assert_eq!(key.len(), 32); // MD5 hex string
        }
    }

    #[test]
    fn same_path_same_key(path in any::<String>()) {
        let path_buf = PathBuf::from(&path);
        if let Ok(key1) = CacheReader::compute_cache_key(&path_buf) {
            if let Ok(key2) = CacheReader::compute_cache_key(&path_buf) {
                prop_assert_eq!(key1, key2);
            }
        }
    }
}
```

### Integration Tests

**Full Lifecycle Test** (`src/cache/shared_cache/tests.rs`)
```rust
#[test]
fn test_shared_cache_read_write_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Write
    let data = b"test data";
    cache.put("testkey", "ast", data).unwrap();

    // Read
    assert!(cache.exists("testkey", "ast"));
    let read_data = cache.get("testkey", "ast").unwrap();
    assert_eq!(read_data, data);

    // Delete
    cache.delete("testkey", "ast").unwrap();
    assert!(!cache.exists("testkey", "ast"));
}

#[test]
fn test_pruning_uses_reader_and_writer() {
    let temp_dir = TempDir::new().unwrap();
    let cache = SharedCache::new_with_cache_dir(None, temp_dir.path().to_path_buf()).unwrap();

    // Write multiple entries
    for i in 0..10 {
        cache.put(&format!("key{}", i), "ast", b"data").unwrap();
    }

    // Prune oldest 5
    let stats = cache.prune_with_strategy(PruneStrategy::LeastRecentlyUsed(5)).unwrap();
    assert_eq!(stats.removed_count, 5);
}
```

### Performance Tests

**Benchmark Concurrent Reads**
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_concurrent_reads(c: &mut Criterion) {
    let cache = setup_cache_with_data();

    c.bench_function("concurrent_reads", |b| {
        b.iter(|| {
            // Spawn 10 readers
            let handles: Vec<_> = (0..10)
                .map(|i| {
                    let cache_clone = cache.clone();
                    std::thread::spawn(move || {
                        cache_clone.get(&format!("key{}", i), "ast")
                    })
                })
                .collect();

            for handle in handles {
                black_box(handle.join().unwrap());
            }
        });
    });
}

criterion_group!(benches, bench_concurrent_reads);
criterion_main!(benches);
```

## Documentation Requirements

### Code Documentation

**Module-Level Docs**
```rust
//! # Cache Reader Module
//!
//! Provides read-only operations for the shared cache system.
//!
//! ## Design Principles
//!
//! - **Pure Functions**: `compute_cache_key` and `get_cache_file_path` are deterministic
//! - **Minimal Side Effects**: Only side effect is updating access metadata
//! - **Concurrency-Safe**: Multiple readers can operate concurrently
//!
//! ## Example
//!
//! ```rust
//! use debtmap::cache::shared_cache::CacheReader;
//! use std::sync::Arc;
//!
//! let location = CacheLocation::resolve(None)?;
//! let index_manager = Arc::new(IndexManager::load_or_create(&location)?);
//! let reader = CacheReader::new(location, index_manager);
//!
//! if reader.exists("my_key", "ast") {
//!     let data = reader.get("my_key", "ast")?;
//!     // ... use cached data
//! }
//! ```
```

```rust
//! # Cache Writer Module
//!
//! Provides write operations with atomic guarantees for the shared cache system.
//!
//! ## Safety Guarantees
//!
//! - **Atomic Writes**: Uses `atomic_io` module for crash-safe writes
//! - **Index Consistency**: Metadata always updated after successful writes
//! - **Batch Operations**: Efficient deletion for pruning scenarios
//!
//! ## Example
//!
//! ```rust
//! use debtmap::cache::shared_cache::CacheWriter;
//!
//! let writer = CacheWriter::new(location, index_manager);
//! writer.put("my_key", "ast", &serialized_data)?;
//! ```
```

### User Documentation

**Architecture Updates** (add to `ARCHITECTURE.md`)
```markdown
## Cache Architecture

### Read/Write Separation (Spec 140)

The cache system uses separated concerns for improved modularity:

- **CacheReader**: Read-only operations (get, exists, compute_cache_key)
  - Pure functions for cache key computation
  - Concurrent-safe reading
  - Minimal lock contention

- **CacheWriter**: Write operations (put, delete)
  - Atomic writes via atomic_io module
  - Metadata consistency guarantees
  - Batch deletion for pruning

- **SharedCache**: Orchestration layer
  - Composes reader and writer
  - Manages pruning lifecycle
  - Aggregates statistics
  - Maintains backward compatibility

This separation enables:
- Independent testing of read/write logic
- Future backend abstraction (Redis, S3)
- Improved concurrency (readers don't block readers)
- Clearer functional boundaries
```

### API Documentation

**Updated Public API Docs** (`src/cache/shared_cache/mod.rs`)
```rust
/// Thread-safe shared cache with separated read/write operations.
///
/// # Architecture
///
/// Internally, `SharedCache` composes:
/// - [`CacheReader`] for read-only operations
/// - [`CacheWriter`] for write operations
/// - [`AutoPruner`] for cache lifecycle management
///
/// This separation improves testability and enables future backend abstraction.
///
/// # Example
///
/// ```rust
/// use debtmap::cache::SharedCache;
///
/// let cache = SharedCache::new(Some(&repo_path))?;
///
/// // Read operations
/// if cache.exists("key", "ast") {
///     let data = cache.get("key", "ast")?;
/// }
///
/// // Write operations
/// cache.put("key", "ast", &data)?;
/// ```
pub struct SharedCache {
    reader: CacheReader,
    writer: CacheWriter,
    // ... (internal fields hidden from docs)
}
```

## Implementation Notes

### Gotchas and Best Practices

1. **Shared Index Manager**: Both reader and writer must share the same `IndexManager` via `Arc<IndexManager>` to maintain consistency.

2. **Pure Function Migration**: When moving functions to static methods, ensure they don't access `self` or any mutable state.

3. **Error Handling**: Maintain existing error messages and types to preserve debugging experience.

4. **Testing Isolation**: Use `TempDir` for all tests to ensure filesystem operations don't interfere.

5. **Backward Compatibility**: Keep all 23 public methods on `SharedCache` unchanged, even if they just delegate.

### Refactoring Checklist

- [ ] Extract pure functions first (no state dependencies)
- [ ] Create CacheReader with read-only operations
- [ ] Create CacheWriter with write operations
- [ ] Update SharedCache to compose reader/writer
- [ ] Run all existing tests (should pass without changes)
- [ ] Add new unit tests for reader/writer
- [ ] Add property-based tests
- [ ] Run benchmarks to verify no regression
- [ ] Update documentation
- [ ] Run clippy and fmt

### Performance Considerations

**Lock Contention Before**
```
Reader1 ──┐
Reader2 ──┼─> SharedCache (RwLock) ─> Single bottleneck
Reader3 ──┘
Writer  ──┘
```

**Lock Contention After**
```
Reader1 ──┐
Reader2 ──┼─> CacheReader (Arc<IndexManager>) ─> Shared read-only access
Reader3 ──┘

Writer ────> CacheWriter (Arc<IndexManager>) ─> Separate write path
```

**Expected Improvements**
- Concurrent reads: ~30% faster (no lock contention between readers)
- Write throughput: Same or slightly better (dedicated writer path)
- Memory: Negligible increase (~100 bytes for Arc pointers)

## Migration and Compatibility

### Breaking Changes

**NONE** - This is a pure internal refactoring. All public APIs remain unchanged.

### Compatibility Guarantees

1. **API Compatibility**: All 23 public methods on `SharedCache` unchanged
2. **Cache Format**: Same file layout, no migration needed
3. **Index Format**: Same index structure, no migration needed
4. **Behavior**: Identical observable behavior (reads, writes, pruning)

### Rollback Plan

If issues arise:
1. Revert to previous `SharedCache` implementation
2. All cache data remains valid (no format changes)
3. No consumer code changes needed

### Future Extensibility

This refactoring enables future backend abstraction (Spec TBD):

```rust
// Future: Extract trait for backend abstraction
trait CacheBackend: Send + Sync {
    fn get(&self, key: &str, component: &str) -> Result<Vec<u8>>;
    fn put(&self, key: &str, component: &str, data: &[u8]) -> Result<()>;
    fn exists(&self, key: &str, component: &str) -> bool;
    fn delete(&self, key: &str, component: &str) -> Result<()>;
}

// Filesystem backend (current implementation)
struct FilesystemBackend {
    reader: CacheReader,
    writer: CacheWriter,
}

// Redis backend (future)
struct RedisBackend {
    client: redis::Client,
}

impl CacheBackend for FilesystemBackend { /* ... */ }
impl CacheBackend for RedisBackend { /* ... */ }
```

The read/write separation is a prerequisite for this abstraction, making it easier to implement different storage backends while preserving the same high-level API.

## Success Metrics

- **Code Quality**: Each module <500 lines, all functions <20 lines
- **Test Coverage**: Maintain >85% coverage with new unit tests
- **Performance**: No regression in benchmarks, possible improvement in concurrent reads
- **Complexity**: Cyclomatic complexity <5 for all new functions
- **Maintainability**: 80% reduction in lines per module (1435 → ~300)

## Related Specifications

- **Spec 87**: Extended Caching Scope - Defines component model used by reader/writer
- **Spec 88**: Cache Compression - May affect buffer sizes in reader/writer
- **Spec 89**: Cache Observability - Statistics from both reader and writer
- **Future Spec**: Backend Abstraction - Will build on this separation
