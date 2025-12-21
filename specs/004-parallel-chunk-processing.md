---
number: 4
title: Parallel Chunk Processing for Duplication Detection
category: parallel
priority: medium
status: draft
dependencies: [3]
created: 2025-12-21
---

# Specification 004: Parallel Chunk Processing for Duplication Detection

**Category**: parallel
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 003 (xxHash) recommended first for maximum benefit

## Context

After parallelizing file I/O in the duplication detection phase, the next bottleneck is the sequential chunk processing and hash aggregation. The current implementation processes all files sequentially when extracting chunks and aggregating hash locations.

### Current Implementation

```rust
let chunk_locations = files
    .into_iter()  // Sequential iteration
    .flat_map(|(path, content)| {
        extract_chunks(&content, min_lines)
            .into_iter()
            .map(move |(start_line, chunk)| {
                let hash = calculate_hash(&chunk);  // CPU-intensive
                // ...
            })
    })
    .fold(
        HashMap::<String, Vec<DuplicationLocation>>::new(),
        |mut acc, (hash, location)| {
            acc.entry(hash).or_default().push(location);  // Sequential aggregation
            acc
        },
    );
```

### Problems

1. **Sequential Processing**: `.into_iter()` processes files one at a time
2. **Sequential Aggregation**: `.fold()` cannot be parallelized with standard HashMap
3. **CPU Bound**: Hash computation is CPU-intensive (especially with SHA256, less so with xxHash)

### Profiling Data

From recent profiling:
- Duplication detection: 10.89s (after parallel I/O optimization)
- With ~340 files and ~10K chunks per large file, there's significant parallelization opportunity

## Objective

Parallelize the chunk extraction, hashing, and aggregation phases of duplication detection using rayon and DashMap to achieve 2-4x speedup on multi-core systems.

## Requirements

### Functional Requirements

1. **Parallel Chunk Processing**
   - Use `par_iter()` for processing files
   - Extract chunks and compute hashes in parallel across files

2. **Concurrent Hash Aggregation**
   - Use `DashMap` for thread-safe concurrent insertion
   - Aggregate hash → locations mapping in parallel

3. **Preserve Correctness**
   - Same duplicates must be detected as sequential implementation
   - Order of results may differ (acceptable)

### Non-Functional Requirements

1. **Performance**: 2-4x speedup on multi-core systems (4+ cores)
2. **Thread Safety**: No data races or undefined behavior
3. **Memory**: Slight increase acceptable due to DashMap overhead
4. **Scalability**: Performance should scale with available cores

## Acceptance Criteria

- [ ] `detect_duplication` uses `par_iter()` for file processing
- [ ] Hash aggregation uses `DashMap` for concurrent access
- [ ] All existing duplication detection tests pass
- [ ] No data races (verified by running with `RUSTFLAGS="-Z sanitizer=thread"` if available)
- [ ] Benchmark shows 2x+ speedup on 4+ core system
- [ ] Same duplicates detected as before (order-independent comparison)

## Technical Details

### Implementation Approach

```rust
use dashmap::DashMap;
use rayon::prelude::*;

pub fn detect_duplication(
    files: Vec<(PathBuf, String)>,
    min_lines: usize,
    _similarity_threshold: f64,
) -> Vec<DuplicationBlock> {
    // Thread-safe concurrent map for aggregation
    let chunk_locations: DashMap<u64, Vec<DuplicationLocation>> = DashMap::new();

    // Parallel processing of files
    files.par_iter().for_each(|(path, content)| {
        for (start_line, chunk) in extract_chunks(content, min_lines) {
            let hash = calculate_hash(&chunk);
            let location = DuplicationLocation {
                file: path.clone(),
                start_line,
                end_line: start_line + min_lines - 1,
            };

            // Thread-safe insertion
            chunk_locations
                .entry(hash)
                .or_default()
                .push(location);
        }
    });

    // Convert to result (single-threaded, but small compared to hashing)
    chunk_locations
        .into_iter()
        .filter_map(|(hash, locations)| {
            (locations.len() > 1).then_some(DuplicationBlock {
                hash,
                lines: min_lines,
                locations,
            })
        })
        .collect()
}
```

### Architecture Changes

**Before:**
```
files.into_iter() → flat_map(extract + hash) → fold(HashMap) → filter → collect
     [sequential]        [sequential]          [sequential]
```

**After:**
```
files.par_iter() → for_each(extract + hash → DashMap.insert) → into_iter → filter → collect
    [parallel]              [parallel + concurrent]            [sequential - small]
```

### Data Structures

- Replace `HashMap<u64, Vec<DuplicationLocation>>` with `DashMap<u64, Vec<DuplicationLocation>>`
- DashMap provides concurrent access with minimal contention

### Alternative: Parallel Reduce

An alternative approach using `par_bridge()` and parallel reduce:

```rust
let chunk_locations: HashMap<u64, Vec<DuplicationLocation>> = files
    .par_iter()
    .flat_map(|(path, content)| {
        extract_chunks(content, min_lines)
            .into_par_iter()
            .map(move |(start_line, chunk)| {
                let hash = calculate_hash(&chunk);
                (hash, DuplicationLocation { /* ... */ })
            })
    })
    .fold(
        HashMap::new,
        |mut acc, (hash, loc)| {
            acc.entry(hash).or_default().push(loc);
            acc
        },
    )
    .reduce(
        HashMap::new,
        |mut a, b| {
            for (hash, locs) in b {
                a.entry(hash).or_default().extend(locs);
            }
            a
        },
    );
```

The DashMap approach is simpler and has less overhead for this use case.

### APIs and Interfaces

No public API changes - this is an internal implementation optimization.

## Dependencies

- **Prerequisites**: Spec 003 (xxHash) recommended first - parallel hashing benefits more from fast hash
- **Affected Components**:
  - `src/debt/duplication.rs` - Main implementation
- **External Dependencies**: Uses existing `dashmap` and `rayon` crates

## Testing Strategy

- **Unit Tests**: Verify same duplicates detected (order-independent)
- **Integration Tests**: Full analysis produces same results
- **Performance Tests**: Benchmark with various file counts and sizes
- **Stress Tests**: Run with many threads to verify no races
- **User Acceptance**: Manual verification on real codebase

## Documentation Requirements

- **Code Documentation**: Update docstrings to mention parallel processing
- **User Documentation**: None (internal change)
- **Architecture Updates**: None

## Implementation Notes

1. **DashMap vs Mutex<HashMap>**: DashMap provides better concurrent performance through sharding
2. **Chunk Extraction**: Could also be parallelized within each file, but files are the natural parallelization boundary
3. **Memory Overhead**: DashMap has slightly higher memory overhead than HashMap
4. **Ordering**: Results may come in different order - use sort if deterministic order needed

## Migration and Compatibility

- No breaking changes
- Results should be identical (may differ in order)
- No migration needed
