---
number: 3
title: Replace SHA256 with xxHash for Duplication Detection
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-21
---

# Specification 003: Replace SHA256 with xxHash for Duplication Detection

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The duplication detection system in `src/debt/duplication.rs` uses SHA256 for hashing code chunks to identify duplicates. SHA256 is a cryptographic hash designed for security properties (collision resistance, preimage resistance) that are unnecessary for duplication detection.

### Current Implementation

```rust
use sha2::{Digest, Sha256};

fn calculate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())  // Returns 64-char hex string
}
```

### Problems

1. **Performance**: SHA256 is computationally expensive (~10-20x slower than non-cryptographic hashes)
2. **Memory**: Returns a 64-character hex String, requiring heap allocation
3. **Overkill**: Cryptographic properties provide no value for duplicate detection

### Existing Dependency

The project already has `xxhash-rust` in `Cargo.toml`:
```toml
xxhash-rust = { version = "0.8", features = ["xxh64"] }
```

xxHash is a non-cryptographic hash optimized for speed while maintaining excellent distribution properties.

## Objective

Replace SHA256 with xxHash64 in the duplication detection system to improve hashing performance by 10-20x while maintaining duplicate detection accuracy.

## Requirements

### Functional Requirements

1. **Hash Function Replacement**
   - Replace `sha2::Sha256` with `xxhash_rust::xxh64::xxh64`
   - Return `u64` instead of `String` to avoid heap allocation
   - Update `DuplicationBlock.hash` field type from `String` to `u64`

2. **Hash Key Type Change**
   - Update `HashMap<String, Vec<DuplicationLocation>>` to `HashMap<u64, Vec<DuplicationLocation>>`
   - Update any serialization/display that shows hash values

3. **Maintain Detection Accuracy**
   - xxHash64 has sufficient collision resistance for code chunk deduplication
   - With ~10K chunks per file and ~1000 files, probability of false positive is negligible

### Non-Functional Requirements

1. **Performance**: Hashing should be 10-20x faster than SHA256
2. **Memory**: Eliminate String allocation for each hash (save ~64 bytes per chunk)
3. **Compatibility**: JSON output format may change (hash displayed as number vs hex string)

## Acceptance Criteria

- [ ] `calculate_hash()` uses `xxhash_rust::xxh64::xxh64` and returns `u64`
- [ ] `DuplicationBlock.hash` field type changed to `u64`
- [ ] `HashMap` key type changed from `String` to `u64`
- [ ] All existing duplication detection tests pass
- [ ] Benchmark shows 10x+ speedup in hash computation
- [ ] No change in detected duplicates on debtmap codebase (same results, faster)

## Technical Details

### Implementation Approach

```rust
use xxhash_rust::xxh64::xxh64;

fn calculate_hash(content: &str) -> u64 {
    xxh64(content.as_bytes(), 0)  // Returns u64 directly, no allocation
}
```

### Architecture Changes

**Before:**
```
chunk (String) → SHA256 → hex String (64 chars) → HashMap<String, ...>
```

**After:**
```
chunk (String) → xxHash64 → u64 (8 bytes) → HashMap<u64, ...>
```

### Data Structures

Update `DuplicationBlock` in `src/core/mod.rs` or wherever defined:

```rust
pub struct DuplicationBlock {
    pub hash: u64,  // Changed from String
    pub lines: usize,
    pub locations: Vec<DuplicationLocation>,
}
```

### APIs and Interfaces

- `calculate_hash(&str) -> u64` (changed return type)
- JSON output will show hash as number instead of hex string
- No public API changes otherwise

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/debt/duplication.rs` - Hash function and HashMap
  - `src/core/mod.rs` (or equivalent) - `DuplicationBlock` struct
  - Any serialization of `DuplicationBlock`
- **External Dependencies**: Uses existing `xxhash-rust` crate

## Testing Strategy

- **Unit Tests**: Verify hash function returns consistent u64 for same input
- **Integration Tests**: Verify same duplicates detected before/after change
- **Performance Tests**: Benchmark hash computation (expect 10-20x improvement)
- **User Acceptance**: Manual verification that output is equivalent

## Documentation Requirements

- **Code Documentation**: Update docstrings for `calculate_hash`
- **User Documentation**: None (internal change)
- **Architecture Updates**: None

## Implementation Notes

1. **Seed Value**: Use seed `0` for xxh64 (consistent across runs)
2. **Serialization**: If hash needs to be displayed, format as hex: `format!("{:016x}", hash)`
3. **Backward Compatibility**: Cached duplication results (if any) will be invalidated

## Migration and Compatibility

- No breaking changes to public API
- JSON output format changes slightly (hash as number vs hex string)
- No migration needed - this is a pure performance improvement
