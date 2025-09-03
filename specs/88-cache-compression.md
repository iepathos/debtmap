---
number: 88
title: Cache Index Compression
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-09-03
---

# Specification 88: Cache Index Compression

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The current cache index uses pretty-printed JSON format, resulting in an 8.8MB index file for a moderately-sized project. This causes:
- Slow disk I/O when loading/saving the cache
- Excessive disk space usage
- Network transfer overhead in CI/CD environments
- Memory pressure when loading large indices
- Slower cache operations due to I/O bottleneck

Compression can reduce the index size by 80-90%, significantly improving cache performance and reducing resource usage.

## Objective

Implement transparent compression for the cache index to reduce disk usage and improve I/O performance while maintaining compatibility and ease of debugging.

## Requirements

### Functional Requirements
- Transparent compression/decompression of cache index
- Support for multiple compression algorithms
- Fallback to uncompressed format if needed
- Ability to export human-readable format for debugging
- Automatic format detection when reading
- Migration from uncompressed to compressed format

### Non-Functional Requirements
- Compression ratio of at least 5:1 for typical data
- Compression overhead < 50ms for 10MB index
- Decompression overhead < 30ms for 10MB index
- Zero data loss or corruption
- Maintainable and debuggable format

## Acceptance Criteria

- [ ] Cache index is compressed by default
- [ ] Compression reduces file size by >80%
- [ ] Loading compressed cache is faster than uncompressed
- [ ] Format auto-detection works correctly
- [ ] Debug export produces readable JSON
- [ ] Migration from old format works seamlessly
- [ ] Compression algorithm is configurable
- [ ] Unit tests verify compression/decompression
- [ ] Performance tests confirm speedup
- [ ] No data corruption under concurrent access

## Technical Details

### Implementation Approach
1. Add compression layer to cache I/O operations
2. Implement format detection via magic bytes
3. Support multiple compression algorithms
4. Add debug export functionality
5. Implement streaming compression for large files

### Architecture Changes
```rust
pub enum CacheFormat {
    Json,           // Uncompressed JSON (legacy)
    GzipJson,       // Gzip compressed JSON
    ZstdJson,       // Zstandard compressed JSON
    BincodeZstd,    // Binary format with Zstd
}

pub struct CompressedCache {
    format: CacheFormat,
    compression_level: i32,
}

impl AnalysisCache {
    pub fn save_compressed(&self) -> Result<()>;
    pub fn load_auto_detect(path: &Path) -> Result<Self>;
    pub fn export_debug(&self, path: &Path) -> Result<()>;
    pub fn get_format(&self) -> CacheFormat;
    pub fn migrate_format(&mut self, new_format: CacheFormat) -> Result<()>;
}

// Magic bytes for format detection
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];
const JSON_MAGIC: [u8; 1] = [b'['];  // JSON array start
```

### Data Structures
- `CacheFormat` enum for format selection
- Compression configuration options
- Format detection utilities
- Streaming compression buffers

### APIs and Interfaces
```rust
pub trait CacheSerializer {
    fn serialize<W: Write>(&self, writer: W, data: &CacheIndex) -> Result<()>;
    fn deserialize<R: Read>(&self, reader: R) -> Result<CacheIndex>;
    fn detect_format<R: Read>(reader: R) -> Result<CacheFormat>;
}

impl CacheConfig {
    pub fn compression_format(&self) -> CacheFormat {
        env::var("DEBTMAP_CACHE_FORMAT")
            .map(|s| CacheFormat::from_str(&s))
            .unwrap_or(CacheFormat::ZstdJson)
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/core/cache.rs` - Cache implementation
  - `src/commands/analyze.rs` - Cache configuration
- **External Dependencies**: 
  - `flate2` for gzip compression
  - `zstd` for Zstandard compression
  - `bincode` for binary serialization (optional)

## Testing Strategy

- **Unit Tests**: 
  - Test each compression format
  - Verify format detection
  - Test compression ratios
  - Validate round-trip integrity
- **Integration Tests**: 
  - Test format migration
  - Verify concurrent access safety
  - Test large file handling
  - Confirm backward compatibility
- **Performance Tests**: 
  - Measure compression ratios
  - Time compression/decompression
  - Compare I/O performance
  - Test memory usage
- **User Acceptance**: 
  - Cache operations are faster
  - Disk usage is reduced
  - No noticeable CPU overhead
  - Debug export works when needed

## Documentation Requirements

- **Code Documentation**: 
  - Document compression formats and trade-offs
  - Explain format detection mechanism
  - Document configuration options
- **User Documentation**: 
  - Add compression configuration to README
  - Document debug export usage
  - Provide format migration guide
- **Architecture Updates**: 
  - Update cache architecture with compression layer
  - Document format selection criteria

## Implementation Notes

- Default to Zstandard for best compression/speed ratio
- Use compression level 3 for balanced performance
- Implement streaming to handle large indices
- Add `--cache-format` CLI option for testing
- Consider mmap for very large cache files
- Monitor compression ratio and warn if poor

## Migration and Compatibility

- Automatically detect and read old JSON format
- Migrate to compressed format on first write
- Provide `--migrate-cache` command for bulk migration
- Support reading multiple formats simultaneously
- Keep ability to export uncompressed for debugging