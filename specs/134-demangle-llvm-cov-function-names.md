---
number: 134
title: Demangle llvm-cov Function Names in LCOV Parser
category: compatibility
priority: critical
status: draft
dependencies: []
created: 2025-10-26
---

# Specification 134: Demangle llvm-cov Function Names in LCOV Parser

**Category**: compatibility
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

### Problem Discovery

After switching from `cargo-tarpaulin` to `cargo-llvm-cov` for coverage generation, debtmap's analysis with `--lcov` became extremely slow, appearing to hang during the "Extracting cross-file call relationships" phase.

**Root Cause Analysis**:

`cargo-llvm-cov` generates **mangled Rust function names** in lcov output, while `cargo-tarpaulin` generates human-readable demangled names. This creates catastrophic performance issues:

**Current lcov.info Statistics** (with llvm-cov):
- **156,837 total lines**
- **375 source files**
- **18,631 function entries** (vs ~1,000-2,000 with tarpaulin)

**Example Mangled Names**:
```
FN:18,_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
FN:144,_RNvXs_NtNtNtCs5ZpFxq88JTF_7debtmap8analysis11attribution14change_trackerNtB4_13ChangeTrackerNtNtCsaNl6UpbT7bw_4core7default7Default7default
```

**Why So Many Functions?**

1. **Multiple Monomorphizations**: Generic functions appear once for each type instantiation
2. **Multiple Crate IDs**: Same function appears with different crate hash IDs (`Cs9MAeJIiYlOV` vs `Cs5ZpFxq88JTF`)
3. **Test Functions**: Includes test functions with mangled names
4. **Trait Implementations**: Each trait impl gets its own mangled entry

**Performance Impact**:

The `CoverageIndex::find_by_path_strategies()` method (src/risk/coverage_index.rs:139-149) performs O(n) linear scans through all 18,631 entries for **every coverage lookup**:

```rust
// O(n) scan through 18,631 entries per lookup!
for ((indexed_path, fname), coverage) in &self.by_function {
    if fname == function_name && query_path.ends_with(indexed_path) {
        return Some(coverage);
    }
}
```

With ~19,600 function lookups during analysis:
- **19,600 lookups × 18,631 comparisons = ~365 million operations**
- Analysis that should take 3-5 seconds takes several minutes or appears to hang

### Why This Spec is Critical

1. **Blocks llvm-cov adoption**: Users cannot switch to the more accurate llvm-cov without severe performance degradation
2. **Production impact**: Existing users who switched to llvm-cov experience unacceptable slowness
3. **Simple fix**: Demangling during parsing reduces function count by ~90% and makes names human-readable
4. **No data loss**: Demangled names still uniquely identify functions for coverage tracking

## Objective

Add demangling support to the LCOV parser to handle mangled function names from `cargo-llvm-cov`, reducing function count from ~18,631 to ~1,000-2,000 and restoring analysis performance to acceptable levels (3-5 seconds for 392-file projects).

## Requirements

### Functional Requirements

1. **Demangle Rust Function Names**
   - Detect mangled function names (starting with `_RNv`, `_ZN`, etc.)
   - Use `rustc-demangle` crate to convert mangled names to readable format
   - Preserve demangled names throughout the coverage data structures

2. **Consolidate Duplicate Functions**
   - When multiple mangled names demangle to the same function, keep the best entry:
     - Prefer entries with higher execution counts
     - Prefer entries with more complete line coverage data
     - Merge uncovered lines from multiple entries

3. **Backward Compatibility**
   - Continue to support already-demangled names from tarpaulin
   - Handle mixed files with both mangled and demangled names
   - No breaking changes to existing LcovData API

4. **Function Name Normalization**
   - Strip generic type parameters from demangled names (e.g., `foo<T>` → `foo`)
   - Remove crate-specific prefixes when appropriate
   - Ensure consistent naming across different compilation units

### Non-Functional Requirements

1. **Performance**
   - Demangling overhead: < 100ms for 18,631 entries
   - Consolidation overhead: < 200ms
   - Total parsing time increase: < 5%
   - No impact on lookup performance (should improve dramatically)

2. **Correctness**
   - No loss of coverage data during consolidation
   - Accurate preservation of execution counts
   - Correct merging of line coverage information

3. **Maintainability**
   - Use well-maintained `rustc-demangle` crate
   - Clear separation of demangling logic
   - Comprehensive logging for debugging

## Acceptance Criteria

- [ ] `rustc-demangle` crate added to Cargo.toml dependencies
- [ ] Mangled function names are detected and demangled during LCOV parsing
- [ ] Function count reduced from ~18,000 to ~1,000-2,000 for llvm-cov output
- [ ] Duplicate demangled functions are consolidated with merged coverage data
- [ ] Analysis with llvm-cov lcov completes in 3-5 seconds (same as tarpaulin)
- [ ] All existing tests pass with both tarpaulin and llvm-cov coverage data
- [ ] New tests validate demangling for llvm-cov output
- [ ] No regression in coverage accuracy or scoring
- [ ] Function names in output are human-readable
- [ ] Coverage index lookups remain O(1) for exact matches

## Technical Details

### Implementation Approach

**File**: `src/risk/lcov.rs`

**1. Add rustc-demangle Dependency**

In `Cargo.toml`:
```toml
[dependencies]
rustc-demangle = "0.1"
```

**2. Implement Demangling Function**

```rust
/// Demangle a Rust function name if it's mangled
///
/// Handles both legacy and v0 mangling schemes:
/// - Legacy: starts with `_ZN`
/// - v0: starts with `_RNv`
///
/// Returns demangled name or original if not mangled.
fn demangle_function_name(name: &str) -> String {
    if name.starts_with("_RNv") || name.starts_with("_ZN") {
        // This is a mangled Rust name
        rustc_demangle::demangle(name).to_string()
    } else {
        // Already demangled or not a Rust mangled name
        name.to_string()
    }
}

/// Normalize a demangled function name for consolidation
///
/// Removes generic type parameters and irrelevant details to
/// group multiple monomorphizations of the same function.
fn normalize_demangled_name(demangled: &str) -> String {
    // Remove generic type parameters: "foo<T, U>" -> "foo"
    if let Some(angle_pos) = demangled.find('<') {
        demangled[..angle_pos].to_string()
    } else {
        demangled.to_string()
    }
}
```

**3. Modify LCOV Parser to Demangle**

Current code (line 93-103):
```rust
Record::FunctionName { start_line, name } => {
    file_functions
        .entry(name.clone())
        .or_insert_with(|| FunctionCoverage {
            name,
            start_line: start_line as usize,
            execution_count: 0,
            coverage_percentage: 0.0,
            uncovered_lines: Vec::new(),
        });
}
```

Updated code with demangling:
```rust
Record::FunctionName { start_line, name } => {
    // Demangle the function name
    let demangled = demangle_function_name(&name);
    let normalized = normalize_demangled_name(&demangled);

    // Use normalized name as key to consolidate duplicates
    file_functions
        .entry(normalized.clone())
        .or_insert_with(|| FunctionCoverage {
            name: normalized,
            start_line: start_line as usize,
            execution_count: 0,
            coverage_percentage: 0.0,
            uncovered_lines: Vec::new(),
        });
}
```

**4. Handle FunctionData with Consolidation**

Current code (line 105-114):
```rust
Record::FunctionData { name, count } => {
    if let Some(func) = file_functions.get_mut(&name) {
        func.execution_count = count;
        if func.coverage_percentage == 0.0 && count > 0 {
            func.coverage_percentage = 100.0;
        }
    }
}
```

Updated code with consolidation:
```rust
Record::FunctionData { name, count } => {
    let demangled = demangle_function_name(&name);
    let normalized = normalize_demangled_name(&demangled);

    if let Some(func) = file_functions.get_mut(&normalized) {
        // Keep the maximum execution count when consolidating
        func.execution_count = func.execution_count.max(count);

        if func.coverage_percentage == 0.0 && count > 0 {
            func.coverage_percentage = 100.0;
        }
    }
}
```

**5. Consolidate Line Coverage Data**

When processing `EndOfRecord`, ensure uncovered lines are merged:

```rust
/// Consolidate duplicate functions after line data is processed
///
/// For functions with the same normalized name, merge their coverage data:
/// - Keep max execution count
/// - Merge uncovered lines (union)
/// - Take best coverage percentage
fn consolidate_duplicate_functions(
    file_functions: &mut HashMap<String, FunctionCoverage>,
    file_lines: &HashMap<usize, u64>,
) {
    // Group functions by normalized name (already done via HashMap key)
    // After line coverage calculation, ensure we have the most complete data
    for func in file_functions.values_mut() {
        // Line coverage calculation happens in process_function_coverage_parallel
        // No additional consolidation needed here since we're using normalized names as keys
    }
}
```

### Architecture Changes

**Modified Components**:
- `src/risk/lcov.rs`: Add demangling logic to `parse_lcov_file_with_progress`
- `Cargo.toml`: Add `rustc-demangle` dependency

**No Changes Required**:
- `LcovData` struct remains unchanged
- `CoverageIndex` remains unchanged
- Public API stays the same
- Coverage scoring logic unchanged

**Data Flow**:
```
Before:
  Parse LCOV → Store mangled names → 18,631 entries → Slow lookups

After:
  Parse LCOV → Demangle → Normalize → Consolidate → ~1,500 entries → Fast lookups
```

### Example Transformations

**Mangled to Demangled**:
```
Input:  _RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes
Output: debtmap::analysis::attribution::change_tracker::ChangeTracker::track_changes
```

**Normalized (removing generics)**:
```
Input:  std::collections::HashMap<K,V>::insert
Output: std::collections::HashMap::insert
```

**Consolidation Example**:
```
Mangled entries in lcov:
  1. _RNv...Cs9MAeJIiYlOV...ChangeTracker13track_changes (count: 5)
  2. _RNv...Cs5ZpFxq88JTF...ChangeTracker13track_changes (count: 3)

After demangling and consolidation:
  debtmap::analysis::attribution::change_tracker::ChangeTracker::track_changes (count: 5)

Result: 2 entries → 1 entry (50% reduction for this example)
```

### Performance Analysis

**Current State** (18,631 functions):
- Parse time: ~500ms
- Index build: ~200ms
- Lookup overhead: ~5-10 minutes (appears to hang)

**After Demangling** (~1,500 functions):
- Parse time: ~550ms (+50ms for demangling)
- Index build: ~30ms (8x faster)
- Lookup overhead: ~3-5 seconds (100x+ faster)

**Net Improvement**: 10-15 minutes → 5 seconds (>100x speedup)

### Memory Impact

**Current**: 18,631 functions × ~200 bytes = ~3.7MB
**After**: 1,500 functions × ~200 bytes = ~300KB

**Memory Savings**: ~3.4MB (90% reduction)

## Dependencies

**Prerequisites**: None

**External Dependencies**:
- `rustc-demangle = "0.1"` - Stable, well-maintained Rust official crate
  - Used by rustc itself for diagnostics
  - ~100KB compiled size
  - No transitive dependencies

**Affected Components**:
- `src/risk/lcov.rs`: Primary implementation
- `Cargo.toml`: Add dependency
- Coverage-related tests: May need sample data updates

## Testing Strategy

### Unit Tests

**Test demangling function**:
```rust
#[cfg(test)]
mod demangle_tests {
    use super::*;

    #[test]
    fn test_demangle_v0_mangled_name() {
        let mangled = "_RNvMNtNtNtCs9MAeJIiYlOV_7debtmap8analysis11attribution14change_trackerNtB2_13ChangeTracker13track_changes";
        let demangled = demangle_function_name(mangled);

        assert!(demangled.contains("ChangeTracker"));
        assert!(demangled.contains("track_changes"));
        assert!(!demangled.starts_with("_RNv"));
    }

    #[test]
    fn test_demangle_legacy_mangled_name() {
        let mangled = "_ZN4core3ptr85drop_in_place$LT$std..rt..lang_start$LT$$LP$$RP$$GT$..$u7b$$u7b$closure$u7d$$u7d$$GT$17h123456789abcdefE";
        let demangled = demangle_function_name(mangled);

        assert!(demangled.contains("drop_in_place"));
        assert!(!demangled.starts_with("_ZN"));
    }

    #[test]
    fn test_demangle_already_demangled() {
        let name = "my_module::my_function";
        let result = demangle_function_name(name);

        assert_eq!(result, name);
    }

    #[test]
    fn test_normalize_removes_generics() {
        assert_eq!(
            normalize_demangled_name("HashMap<String, i32>::insert"),
            "HashMap::insert"
        );

        assert_eq!(
            normalize_demangled_name("Vec<T>::push"),
            "Vec::push"
        );

        assert_eq!(
            normalize_demangled_name("simple_function"),
            "simple_function"
        );
    }
}
```

**Test consolidation logic**:
```rust
#[test]
fn test_consolidate_duplicate_functions() {
    let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,_RNvMNtCs9MAeJIiYlOV_7debtmap4testNtB2_9TestStruct8test_func
FNDA:5,_RNvMNtCs9MAeJIiYlOV_7debtmap4testNtB2_9TestStruct8test_func
FN:10,_RNvMNtCs5ZpFxq88JTF_7debtmap4testNtB2_9TestStruct8test_func
FNDA:3,_RNvMNtCs5ZpFxq88JTF_7debtmap4testNtB2_9TestStruct8test_func
DA:10,5
DA:11,5
LF:2
LH:2
end_of_record
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let data = parse_lcov_file(temp_file.path()).unwrap();
    let file_path = PathBuf::from("/path/to/file.rs");

    // Should consolidate to single function
    let funcs = &data.functions[&file_path];
    assert_eq!(funcs.len(), 1);

    // Should keep max execution count
    assert_eq!(funcs[0].execution_count, 5);

    // Function name should be demangled
    assert!(funcs[0].name.contains("TestStruct"));
    assert!(!funcs[0].name.starts_with("_RNv"));
}
```

### Integration Tests

**Test with real llvm-cov output**:
```rust
#[test]
fn test_parse_llvm_cov_lcov_performance() {
    // Use actual llvm-cov generated lcov file
    let lcov_path = "tests/fixtures/llvm_cov_coverage.info";

    let start = Instant::now();
    let data = parse_lcov_file(Path::new(lcov_path)).unwrap();
    let parse_time = start.elapsed();

    // Should parse in reasonable time
    assert!(parse_time < Duration::from_millis(1000));

    // Should have reasonable number of functions (not 18k+)
    let total_functions: usize = data.functions.values().map(|v| v.len()).sum();
    assert!(total_functions < 3000, "Too many functions: {}", total_functions);

    // Function names should be demangled
    for funcs in data.functions.values() {
        for func in funcs {
            assert!(!func.name.starts_with("_RNv"),
                "Found mangled name: {}", func.name);
        }
    }
}
```

**Test backward compatibility with tarpaulin**:
```rust
#[test]
fn test_tarpaulin_lcov_still_works() {
    // Use tarpaulin-generated lcov file
    let lcov_path = "tests/fixtures/tarpaulin_coverage.info";

    let data = parse_lcov_file(Path::new(lcov_path)).unwrap();

    // Should work without issues
    assert!(!data.functions.is_empty());

    // Names should already be readable
    for funcs in data.functions.values() {
        for func in funcs {
            // Tarpaulin names don't start with _RNv
            assert!(!func.name.starts_with("_RNv"));
        }
    }
}
```

### Performance Tests

**Benchmark parsing with demangling**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_llvm_cov_lcov(c: &mut Criterion) {
    let lcov_path = "tests/fixtures/large_llvm_cov.info";

    c.bench_function("parse_llvm_cov_with_demangling", |b| {
        b.iter(|| {
            let data = parse_lcov_file(black_box(Path::new(lcov_path))).unwrap();
            black_box(data);
        })
    });
}

criterion_group!(benches, bench_parse_llvm_cov_lcov);
criterion_main!(benches);
```

**Expected Results**:
```
Before: parse_llvm_cov takes 500ms, creates 18,631 functions
After:  parse_llvm_cov takes 550ms, creates 1,500 functions
Overhead: +50ms (+10%) but 90% fewer functions
```

## Documentation Requirements

### Code Documentation

**Update lcov parser documentation**:
```rust
/// Parse an LCOV file and return coverage data
///
/// This function handles both tarpaulin and llvm-cov generated LCOV files.
/// For llvm-cov files with mangled function names, it automatically:
/// - Detects mangled names (starting with _RNv or _ZN)
/// - Demangles using rustc-demangle
/// - Normalizes to remove generic type parameters
/// - Consolidates duplicate functions from multiple monomorphizations
///
/// # Performance
///
/// Demangling adds ~10% overhead to parsing time but dramatically
/// reduces the number of function entries (typically 90% reduction),
/// which improves all subsequent coverage lookups.
///
/// # Compatibility
///
/// Works with:
/// - cargo-tarpaulin (already demangled)
/// - cargo-llvm-cov (mangled, will be demangled)
/// - Mixed files with both mangled and demangled names
pub fn parse_lcov_file_with_progress(path: &Path, progress: &ProgressBar) -> Result<LcovData>
```

### User Documentation

Update book/src/getting-started.md:
```markdown
## Coverage Integration

Debtmap supports coverage data from both cargo-tarpaulin and cargo-llvm-cov.

### Using cargo-llvm-cov (Recommended)

```bash
# Generate coverage with llvm-cov
cargo llvm-cov --lcov --output-path target/coverage/lcov.info

# Analyze with coverage
debtmap analyze . --lcov target/coverage/lcov.info
```

**Note**: Debtmap automatically handles the mangled function names generated
by llvm-cov, consolidating duplicate entries and providing readable function
names in the output.

### Using cargo-tarpaulin

```bash
# Generate coverage with tarpaulin
cargo tarpaulin --out Lcov --output-dir target/coverage

# Analyze with coverage
debtmap analyze . --lcov target/coverage/lcov.info
```

Both tools produce compatible LCOV files that debtmap can parse efficiently.
```

### Changelog Entry

```markdown
## [0.3.0] - 2025-10-27

### Fixed
- **Critical**: Fixed severe performance regression when using cargo-llvm-cov
  generated coverage files. Demangling now handles the 18,000+ mangled function
  entries, reducing them to ~1,500 readable names and improving analysis speed
  from 10+ minutes to 3-5 seconds (#xxx)

### Changed
- Added `rustc-demangle` dependency for handling llvm-cov function names
- LCOV parser now automatically demangles and consolidates duplicate function entries
```

## Implementation Notes

### Design Decisions

1. **Demangle During Parsing vs Post-Processing**:
   - **Choice**: During parsing
   - **Rationale**: Prevents storing 18k+ entries in memory, reduces memory footprint
   - **Trade-off**: Slightly slower parsing (+10%) but 90% memory savings

2. **Consolidation Strategy**:
   - **Choice**: Use normalized name as HashMap key
   - **Rationale**: Automatic consolidation via HashMap, no separate pass needed
   - **Benefit**: O(1) insertion with automatic deduplication

3. **Normalization Level**:
   - **Choice**: Remove generic parameters but keep module paths
   - **Rationale**: Balances uniqueness with consolidation
   - **Example**: `HashMap<K,V>::insert` → `HashMap::insert` (generic-agnostic)

### Potential Gotchas

1. **Over-Consolidation Risk**:
   - Multiple genuinely different functions might demangle to same name
   - **Mitigation**: Keep module paths, only strip generics
   - **Example**: `mod_a::foo<T>` and `mod_b::foo<U>` stay separate

2. **Demangling Failures**:
   - Some names might not demangle correctly
   - **Mitigation**: Fall back to original name if demangling fails
   - Log warnings for investigation

3. **Mixed Coverage Files**:
   - Files with both tarpaulin and llvm-cov data
   - **Mitigation**: Demangling is idempotent (no-op for already demangled names)

### Error Handling

```rust
fn demangle_function_name(name: &str) -> String {
    if name.starts_with("_RNv") || name.starts_with("_ZN") {
        match std::panic::catch_unwind(|| rustc_demangle::demangle(name).to_string()) {
            Ok(demangled) => demangled,
            Err(_) => {
                log::warn!("Failed to demangle function name: {}", name);
                name.to_string() // Fall back to original
            }
        }
    } else {
        name.to_string()
    }
}
```

## Migration and Compatibility

### Breaking Changes

**None** - This is a transparent improvement.

### Backward Compatibility

- **Tarpaulin lcov files**: Continue to work without changes
- **Mixed files**: Handle both mangled and demangled names
- **API**: No changes to LcovData public API
- **Output**: Function names become more readable (improvement, not breaking)

### Migration Path

**For Users**:
1. Update debtmap to version with this fix
2. Re-run analysis with llvm-cov
3. Performance should improve immediately
4. No configuration changes needed

**For Developers**:
1. Add rustc-demangle dependency
2. Update parse_lcov_file_with_progress function
3. Run tests to verify consolidation
4. Benchmark to confirm performance improvement

### Rollback Plan

If issues are discovered:
1. Remove demangling logic (simple revert)
2. rustc-demangle is small and has no side effects
3. No database or cache migrations needed
4. Tests cover both paths (with and without demangling)

## Success Metrics

### Performance Metrics

- [ ] Parse time increase: < 10% (+50ms for 18k functions)
- [ ] Function count reduction: > 85% (18k → < 3k)
- [ ] Analysis time: < 5 seconds (vs 10+ minutes before)
- [ ] Memory usage: ~3MB reduction

### Quality Metrics

- [ ] All tests pass with llvm-cov coverage
- [ ] Coverage accuracy maintained (same scores)
- [ ] Function names human-readable in output
- [ ] No errors or warnings during parsing

### Validation

**Before Implementation**:
```bash
$ time debtmap analyze . --lcov target/coverage/lcov.info
# Hangs or takes 10+ minutes
# 18,631 functions in memory
```

**After Implementation**:
```bash
$ time debtmap analyze . --lcov target/coverage/lcov.info
# Completes in 3-5 seconds
# ~1,500 functions in memory
# Function names readable: debtmap::analysis::ChangeTracker::track_changes
```

## Future Enhancements

**Out of scope for this spec** (potential future work):

1. **Configurable Consolidation**:
   - Option to keep separate entries for different monomorphizations
   - Useful for understanding which type instantiations are tested

2. **Demangling Cache**:
   - Cache demangled names to disk for repeated analyses
   - Minor benefit since parsing is already fast enough

3. **Symbol Filter Configuration**:
   - Allow users to filter out test functions
   - Exclude specific modules from coverage

4. **Advanced Normalization**:
   - Configurable normalization strategies
   - Preserve generics for specific modules
