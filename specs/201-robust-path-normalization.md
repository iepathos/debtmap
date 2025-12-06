---
number: 201
title: Robust Path Normalization for Coverage Matching
category: foundation
priority: critical
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 201: Robust Path Normalization for Coverage Matching

**Category**: foundation
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

When debtmap analyzes code with LCOV coverage data, it must match file paths from AST analysis (debtmap's view) with file paths in the LCOV file (coverage tool's view). Currently, path normalization only strips `./` prefixes, leading to match failures when:

- LCOV is generated on Windows with backslashes, analyzed on Unix
- Coverage runs in Docker containers with different mount paths
- Projects use cargo workspaces with varying relative paths
- CI/CD systems use different absolute path prefixes

This causes legitimate coverage data to show as `Cov:N/A` instead of the actual percentage, leading to false positives in technical debt reporting.

**Current Implementation (src/risk/coverage_index.rs:6-11)**:
```rust
pub fn normalize_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}
```

This is insufficient for real-world scenarios.

## Objective

Implement robust, cross-platform path normalization that eliminates false negative coverage matches due to path format differences, ensuring that coverage data is matched correctly regardless of:
- Operating system (Windows/Unix path separators)
- Absolute vs relative paths
- Workspace structure
- CI/CD environment paths

## Requirements

### Functional Requirements

**FR1**: Normalize path separators to forward slashes (`/`)
- Convert Windows backslashes (`\`) to forward slashes
- Handle mixed separators (e.g., `src\utils/helper.rs`)

**FR2**: Strip all leading `./` and `../` segments safely
- Remove single `./` prefix
- Remove multiple consecutive `./././` prefixes
- Handle parent directory references where safe

**FR3**: Extract component-based representation for suffix matching
- Convert path to vector of string components
- Filter out current directory (`.`) and root components
- Preserve relative ordering of meaningful segments

**FR4**: Support canonical path resolution (optional)
- Resolve symlinks when filesystem access available
- Gracefully degrade when paths don't exist
- Never fail due to missing files

### Non-Functional Requirements

**NFR1**: **Performance** - O(n) complexity where n is path length
- No filesystem I/O in hot path
- Component extraction should be single-pass
- Canonical resolution only when explicitly requested

**NFR2**: **Compatibility** - Work on all platforms
- Handle Windows `C:\` drive letters
- Support UNC paths (`\\server\share`)
- Work with both `PathBuf` and `&Path`

**NFR3**: **Safety** - Never panic or cause undefined behavior
- Handle invalid UTF-8 in paths gracefully
- Handle edge cases (empty paths, root paths)
- Use `to_string_lossy()` for conversion

## Acceptance Criteria

- [ ] `normalize_path_components()` extracts meaningful path segments
  - `./src/lib.rs` → `["src", "lib.rs"]`
  - `/abs/path/src/lib.rs` → `["abs", "path", "src", "lib.rs"]`
  - `C:\Users\dev\src\lib.rs` → `["Users", "dev", "src", "lib.rs"]`

- [ ] `normalize_path_separators()` converts all separators to `/`
  - `src\utils\helper.rs` → `src/utils/helper.rs`
  - `src/utils\helper.rs` → `src/utils/helper.rs` (mixed)

- [ ] `paths_match_by_suffix()` correctly identifies matches
  - `["src", "lib.rs"]` matches `["project", "src", "lib.rs"]` ✓
  - `["src", "lib.rs"]` does NOT match `["other", "lib.rs"]` ✗

- [ ] Path normalization handles edge cases
  - Empty paths return empty component list
  - Root paths (`/`, `C:\`) return empty component list
  - Trailing slashes are stripped
  - Multiple consecutive separators are normalized

- [ ] Performance benchmarks met
  - Normalize 1000 paths in <10ms on typical hardware
  - No heap allocations in comparison functions

- [ ] Cross-platform tests pass
  - Unix-style paths work correctly
  - Windows-style paths work correctly
  - Mixed paths handled gracefully

- [ ] Integration with existing matching strategies
  - All three existing strategies use new normalization
  - No regression in existing test coverage
  - New tests added for cross-platform scenarios

## Technical Details

### Implementation Approach

**Pure Function Architecture** (following Stillwater philosophy):

```rust
// ============================================================================
// PURE CORE: Path normalization logic (100% testable, no I/O)
// ============================================================================

/// Pure function: Extract meaningful path components
///
/// Filters out current directory (.) and root components,
/// preserving only the meaningful path segments.
///
/// # Examples
/// ```
/// let components = normalize_path_components(Path::new("./src/lib.rs"));
/// assert_eq!(components, vec!["src", "lib.rs"]);
/// ```
pub fn normalize_path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => None,
            Component::CurDir => None,
            Component::ParentDir => None, // Could preserve if needed
        })
        .collect()
}

/// Pure function: Normalize path separators to forward slash
///
/// Converts all backslashes to forward slashes for consistent
/// cross-platform matching.
pub fn normalize_path_separators(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

/// Pure function: Check if query path matches target by suffix
///
/// Returns true if query is a suffix of target, enabling matches
/// like src/lib.rs matching /abs/path/src/lib.rs.
pub fn paths_match_by_suffix(query: &[String], target: &[String]) -> bool {
    if query.is_empty() {
        return false;
    }
    if query.len() > target.len() {
        return false;
    }

    let target_suffix = &target[target.len() - query.len()..];
    query == target_suffix
}

/// Pure function: Find best matching path from available paths
///
/// Tries multiple strategies:
/// 1. Exact component match
/// 2. Suffix match (query is suffix of candidate)
/// 3. Reverse suffix match (candidate is suffix of query)
pub fn find_matching_path<'a>(
    query_path: &Path,
    available_paths: &'a [PathBuf],
) -> Option<(&'a PathBuf, MatchStrategy)> {
    let query_components = normalize_path_components(query_path);

    // Strategy 1: Exact match
    for candidate in available_paths {
        let candidate_components = normalize_path_components(candidate);
        if query_components == candidate_components {
            return Some((candidate, MatchStrategy::ExactComponents));
        }
    }

    // Strategy 2: Query is suffix of candidate
    for candidate in available_paths {
        let candidate_components = normalize_path_components(candidate);
        if paths_match_by_suffix(&query_components, &candidate_components) {
            return Some((candidate, MatchStrategy::QuerySuffix));
        }
    }

    // Strategy 3: Candidate is suffix of query
    for candidate in available_paths {
        let candidate_components = normalize_path_components(candidate);
        if paths_match_by_suffix(&candidate_components, &query_components) {
            return Some((candidate, MatchStrategy::CandidateSuffix));
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStrategy {
    ExactComponents,
    QuerySuffix,
    CandidateSuffix,
}
```

### Architecture Changes

**New Module**: `src/risk/path_normalization.rs`
- Pure functions for path manipulation
- No dependencies on I/O or external state
- Comprehensive inline documentation
- Extensive property-based tests

**Updated Module**: `src/risk/coverage_index.rs`
- Use new normalization functions in all three path matching strategies
- Replace existing `normalize_path()` with `normalize_path_components()`
- Add diagnostic information to match results

### Data Structures

```rust
/// Result of path normalization with diagnostic info
#[derive(Debug, Clone)]
pub struct NormalizedPath {
    /// Original path as provided
    pub original: PathBuf,
    /// Normalized component representation
    pub components: Vec<String>,
    /// Normalized string form (forward slashes)
    pub normalized_str: String,
}

impl NormalizedPath {
    pub fn from_path(path: &Path) -> Self {
        Self {
            original: path.to_path_buf(),
            components: normalize_path_components(path),
            normalized_str: normalize_path_separators(path),
        }
    }
}
```

## Dependencies

**Prerequisites**: None - this is foundational infrastructure

**Affected Components**:
- `src/risk/coverage_index.rs` - Uses new normalization functions
- `src/risk/lcov.rs` - May benefit from path normalization utilities

**External Dependencies**: None - uses only `std::path`

## Testing Strategy

### Unit Tests

**Test Pure Functions** (100% coverage target):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_components_unix() {
        assert_eq!(
            normalize_path_components(Path::new("./src/lib.rs")),
            vec!["src", "lib.rs"]
        );
        assert_eq!(
            normalize_path_components(Path::new("/home/user/project/src/lib.rs")),
            vec!["home", "user", "project", "src", "lib.rs"]
        );
    }

    #[test]
    fn test_normalize_path_components_windows() {
        // Simulate Windows paths
        let path = PathBuf::from(r"C:\Users\dev\project\src\lib.rs");
        let components = normalize_path_components(&path);
        assert!(components.contains(&"src".to_string()));
        assert!(components.contains(&"lib.rs".to_string()));
    }

    #[test]
    fn test_paths_match_by_suffix() {
        let query = vec!["src".to_string(), "lib.rs".to_string()];
        let target = vec!["project".to_string(), "src".to_string(), "lib.rs".to_string()];
        assert!(paths_match_by_suffix(&query, &target));

        let non_matching = vec!["other".to_string(), "lib.rs".to_string()];
        assert!(!paths_match_by_suffix(&query, &non_matching));
    }

    #[test]
    fn test_find_matching_path_strategies() {
        let query = PathBuf::from("src/lib.rs");
        let candidates = vec![
            PathBuf::from("/abs/path/src/lib.rs"),
            PathBuf::from("other/file.rs"),
        ];

        let result = find_matching_path(&query, &candidates);
        assert!(result.is_some());
        let (matched, strategy) = result.unwrap();
        assert_eq!(matched, &PathBuf::from("/abs/path/src/lib.rs"));
        assert_eq!(strategy, MatchStrategy::QuerySuffix);
    }
}
```

**Property-Based Tests** (using `proptest`):

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn normalize_components_never_panics(path in ".*") {
        let _ = normalize_path_components(Path::new(&path));
    }

    #[test]
    fn suffix_match_is_transitive(
        a in prop::collection::vec("[a-z]+", 1..5),
        b in prop::collection::vec("[a-z]+", 1..5),
    ) {
        // If A suffix of B and B suffix of C, then A suffix of C
        let mut c = b.clone();
        c.extend(a.clone());

        prop_assert!(paths_match_by_suffix(&a, &c));
        prop_assert!(paths_match_by_suffix(&b, &c));
    }
}
```

### Integration Tests

**Cross-Platform Scenarios** (`tests/path_normalization_integration_test.rs`):

```rust
#[test]
fn test_windows_unix_cross_platform_match() {
    // LCOV from Windows
    let lcov_path = PathBuf::from(r"C:\project\src\lib.rs");
    // Query from Unix
    let query_path = PathBuf::from("/home/dev/project/src/lib.rs");

    let lcov_components = normalize_path_components(&lcov_path);
    let query_components = normalize_path_components(&query_path);

    // Should match on suffix
    assert!(paths_match_by_suffix(
        &vec!["src".to_string(), "lib.rs".to_string()],
        &lcov_components
    ));
}

#[test]
fn test_docker_container_path_match() {
    let container_path = PathBuf::from("/app/src/lib.rs");
    let host_path = PathBuf::from("/Users/dev/project/src/lib.rs");

    let candidates = vec![container_path];
    let result = find_matching_path(&host_path, &candidates);

    assert!(result.is_some());
}

#[test]
fn test_cargo_workspace_relative_paths() {
    let workspace_root = PathBuf::from("crates/parser/src/lib.rs");
    let member_relative = PathBuf::from("src/lib.rs");

    let candidates = vec![workspace_root];
    let result = find_matching_path(&member_relative, &candidates);

    assert!(result.is_some());
}
```

### Performance Tests

**Benchmark Suite** (`benches/path_normalization_bench.rs`):

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_normalize_components(c: &mut Criterion) {
    c.bench_function("normalize_path_components", |b| {
        let path = Path::new("/long/path/to/project/src/module/submodule/file.rs");
        b.iter(|| normalize_path_components(black_box(path)))
    });
}

fn bench_suffix_matching(c: &mut Criterion) {
    c.bench_function("paths_match_by_suffix", |b| {
        let query = vec!["src".to_string(), "lib.rs".to_string()];
        let target = vec!["a".to_string(), "b".to_string(), "src".to_string(), "lib.rs".to_string()];
        b.iter(|| paths_match_by_suffix(black_box(&query), black_box(&target)))
    });
}

criterion_group!(benches, bench_normalize_components, bench_suffix_matching);
criterion_main!(benches);
```

## Documentation Requirements

### Code Documentation

- Document all public functions with rustdoc comments
- Include examples for each pure function
- Explain matching strategies with diagrams
- Document edge cases and limitations

### Architecture Updates

Update `ARCHITECTURE.md` with new path normalization layer:

```markdown
## Coverage Matching Architecture

### Path Normalization Layer

Pure functions for cross-platform path matching:
- Component extraction (OS-agnostic)
- Separator normalization
- Suffix-based matching strategies

This layer eliminates false negatives from path format differences.
```

### User Documentation

No user-facing documentation needed - this is internal infrastructure.

## Implementation Notes

### Best Practices

1. **Pure Functions First**: All logic should be pure and testable
2. **Performance**: Avoid allocations in hot paths, consider caching
3. **Robustness**: Handle all edge cases gracefully, never panic
4. **Composability**: Build complex matching from simple predicates

### Gotchas

- Windows UNC paths (`\\server\share`) need special handling
- Symlinks can't be resolved without filesystem access
- Empty paths and root paths are edge cases
- UTF-8 validation may fail on unusual filesystems

### Future Enhancements

- Optional canonical path resolution with caching
- Configurable matching strictness levels
- Support for glob patterns in path matching
- Path normalization cache for performance

## Migration and Compatibility

### Breaking Changes

None - this is additive infrastructure.

### Backward Compatibility

Existing `normalize_path()` function can be deprecated:

```rust
#[deprecated(since = "0.x.0", note = "Use normalize_path_components() instead")]
pub fn normalize_path(path: &Path) -> PathBuf {
    // Keep for compatibility, implement using new functions
    PathBuf::from(normalize_path_separators(path))
}
```

### Migration Path

1. Implement new pure functions
2. Add comprehensive tests
3. Update `coverage_index.rs` to use new functions
4. Run integration tests to verify no regressions
5. Deprecate old `normalize_path()` function
6. Remove deprecated function in next major version
