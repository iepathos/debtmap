---
number: 179
title: LCOV Coverage Matching for Generic/Monomorphized Rust Functions
category: compatibility
priority: high
status: draft
dependencies: [166]
created: 2025-11-17
---

# Specification 179: LCOV Coverage Matching for Generic/Monomorphized Rust Functions

**Category**: compatibility
**Priority**: high
**Status**: draft
**Dependencies**: Spec 166 (Rust impl method LCOV matching)

## Context

After fixing spec 166 (impl method matching), we discovered that generic Rust functions that are monomorphized by the compiler create multiple entries in LCOV files - one for each concrete type instantiation. For example:

```rust
impl<E: Executor> SetupPhaseExecutor {
    fn execute(&self, executor: &E) -> Result<()> { ... }
}
```

Gets compiled into multiple monomorphized versions in LCOV:
- `SetupPhaseExecutor::execute::<WorkflowExecutor>`
- `SetupPhaseExecutor::execute::<MockExecutor>`
- `SetupPhaseExecutor::execute::<SlowExecutor>`

Currently, debtmap queries for `SetupPhaseExecutor::execute` (from AST analysis) but fails to match any of the monomorphized LCOV entries, resulting in "no coverage data" instead of showing the actual coverage.

**Affected functions in Prodigy top 10**:
- #3: `SetupPhaseExecutor::execute()` - no coverage data (has 4 monomorphized versions)
- #4: `FileHandler::execute()` - no coverage data (likely monomorphized)
- #8: `RetryExecutor::execute_with_retry()` - no coverage data (likely monomorphized)

## Objective

Enable debtmap to correctly match and aggregate coverage data from monomorphized generic functions in LCOV files, treating all monomorphizations as a single logical function for coverage reporting.

## Requirements

### Functional Requirements

1. **Monomorphization Detection**
   - Detect when an LCOV function entry contains generic type parameters (`::<Type>` suffix)
   - Identify the base function name without type parameters
   - Group all monomorphizations of the same function together

2. **Aggregated Coverage Calculation**
   - When querying for a generic function, find ALL monomorphized versions
   - Aggregate coverage across all monomorphizations using **intersection strategy**:
     - A line is marked as **covered only if ALL monomorphizations cover it**
     - This conservative approach ensures we don't claim coverage that doesn't exist in all code paths
     - Coverage percentage is averaged across all versions
   - Return aggregated coverage as if it were a single function
   - Build index at parse time for O(1) lookups (not O(n) scans)

3. **Normalization Enhancement**
   - Extend `normalize_demangled_name()` to handle trailing generic parameters (`::<Type>`)
   - Strip generic type parameters from both the type and the function itself
   - Preserve base function path for matching

4. **Matching Strategy**
   - When exact match fails, try base name match (without generics)
   - Match query `Type::method` against any of:
     - `Type::method` (exact, non-generic)
     - `Type::method::<GenericParam>` (generic instantiation)
     - `crate::path::Type::method::<GenericParam>` (full path with generic)

### Non-Functional Requirements

- **Performance**: Matching should remain O(1) for non-generic functions
- **Backward Compatibility**: Existing non-generic function matching must not regress
- **Memory**: Minimal memory overhead for generic function grouping
- **Accuracy**: Coverage aggregation must be mathematically sound

## Acceptance Criteria

- [ ] Generic function `SetupPhaseExecutor::execute` shows aggregated coverage data (not "no coverage data")
- [ ] Coverage percentage accurately reflects all monomorphized versions (average)
- [ ] Uncovered lines list includes intersection of uncovered lines across all versions (conservative)
- [ ] Non-generic functions continue to match exactly as before
- [ ] Performance overhead for generic matching is < 10% vs current implementation
- [ ] All existing LCOV matching tests continue to pass
- [ ] New tests cover multiple monomorphization scenarios
- [ ] Prodigy top 10 now shows coverage for items #3, #4, #8

## Technical Details

### Implementation Approach

**Functional Programming Principles**

This implementation follows strict functional programming patterns:

1. **Pure Functions**: All core logic is in pure, side-effect-free functions
   - `strip_trailing_generics()` - string transformation with no side effects
   - `is_monomorphization_of()` - boolean predicate, deterministic
   - `merge_coverage()` - data transformation, no mutations

2. **Immutable Data Structures**: No mutations of input data
   - Functions return new data rather than modifying arguments
   - Use `Cow<'_, str>` to avoid unnecessary allocations

3. **Separation of Concerns**: I/O at the edges, pure logic in the core
   - Index building in `CoverageIndex::build_from_lcov()` (I/O boundary)
   - All matching/aggregation logic in pure functions
   - Easy to test, reason about, and compose

4. **Function Composition**: Build complex behavior from simple functions
   - `normalize_demangled_name()` calls `strip_trailing_generics()`
   - `get_aggregated_coverage()` orchestrates pure functions
   - Each function has single responsibility

**Phase 1: Normalization Enhancement**

Modify `normalize_demangled_name()` in `src/risk/lcov.rs`:

```rust
fn normalize_demangled_name(demangled: &str) -> NormalizedFunctionName {
    // Existing impl bracket handling...

    // NEW: Strip trailing generic parameters from functions
    // Example: "Type::method::<Generic>" -> "Type::method"
    let without_function_generics = strip_trailing_generics(without_impl_brackets);

    // Existing generic parameter removal...
    // Extract method name...
}

/// Pure function: Strip trailing generic parameters from function names.
/// Handles nested generics like `method::<Vec<HashMap<K, V>>>`.
/// Returns `Cow` to avoid allocation when no stripping is needed.
fn strip_trailing_generics(s: &str) -> Cow<'_, str> {
    if let Some(pos) = s.rfind("::<") {
        // Count angle brackets to find matching close (handles nested generics)
        let mut depth = 0;
        let mut end_pos = None;

        for (i, ch) in s[pos + 3..].char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    if depth == 0 {
                        end_pos = Some(pos + 3 + i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }

        if let Some(end) = end_pos {
            let after = &s[end + 1..];
            // If nothing after the >, this is a trailing generic
            if after.is_empty() {
                return Cow::Owned(s[..pos].to_string());
            }
        }
    }
    Cow::Borrowed(s)
}
```

**Phase 2: Index Building for O(1) Lookups**

Add base function index in `src/risk/coverage_index.rs`:

```rust
pub struct CoverageIndex {
    by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>>,
    // NEW: Index from base function name to all monomorphized versions
    base_function_index: HashMap<(PathBuf, String), Vec<String>>,
}

impl CoverageIndex {
    /// Build index at parse time, grouping monomorphized functions
    fn build_from_lcov(lcov_data: &LcovData) -> Self {
        let mut by_file = HashMap::new();
        let mut base_function_index = HashMap::new();

        for (file, functions) in lcov_data {
            let mut file_functions = HashMap::new();

            for (name, coverage) in functions {
                file_functions.insert(name.clone(), coverage.clone());

                // Extract base name and update index
                let base_name = strip_trailing_generics(name).into_owned();
                if base_name != *name {
                    // This is a monomorphized function
                    base_function_index
                        .entry((file.clone(), base_name))
                        .or_insert_with(Vec::new)
                        .push(name.clone());
                }
            }

            by_file.insert(file.clone(), file_functions);
        }

        CoverageIndex {
            by_file,
            base_function_index,
        }
    }
}
```

**Phase 3: Coverage Aggregation**

Add O(1) aggregation logic using the index:

```rust
impl CoverageIndex {
    /// Find all monomorphizations of a function and aggregate coverage.
    /// Uses pre-built index for O(1) lookup.
    fn get_aggregated_coverage(
        &self,
        file: &Path,
        function_name: &str,
    ) -> Option<AggregateCoverage> {
        // Try exact match first (O(1))
        if let Some(file_functions) = self.by_file.get(file) {
            if let Some(exact) = file_functions.get(function_name) {
                return Some(AggregateCoverage::single(exact));
            }
        }

        // Try monomorphized versions using index (O(1))
        if let Some(versions) = self.base_function_index.get(&(file.to_path_buf(), function_name.to_string())) {
            let coverages: Vec<&FunctionCoverage> = versions
                .iter()
                .filter_map(|name| {
                    self.by_file
                        .get(file)
                        .and_then(|funcs| funcs.get(name))
                })
                .collect();

            if !coverages.is_empty() {
                return Some(merge_coverage(coverages));
            }
        }

        None
    }
}

/// Pure function: Check if candidate is a monomorphization of base function.
/// Handles nested generics and path boundaries correctly.
fn is_monomorphization_of(candidate: &str, base: &str) -> bool {
    // Exact match
    if candidate == base {
        return true;
    }

    // Check for generic suffix: base::<...>
    if candidate.starts_with(base) {
        let suffix = &candidate[base.len()..];
        if suffix.starts_with("::<") {
            return true;
        }
    }

    // Check for full path match: path::base or path::base::<...>
    if let Some(pos) = candidate.rfind("::") {
        let method_part = &candidate[pos + 2..];
        if method_part == base {
            return true;
        }
        if method_part.starts_with(base) && method_part[base.len()..].starts_with("::<") {
            return true;
        }
    }

    false
}

struct AggregateCoverage {
    coverage_pct: f64,
    uncovered_lines: Vec<usize>,
    version_count: usize,
}

/// Pure function: Merge coverage data from multiple monomorphizations.
/// Uses intersection strategy: a line is uncovered only if ALL versions leave it uncovered.
/// This conservative approach ensures we don't claim coverage that doesn't exist in all paths.
fn merge_coverage(coverages: Vec<&FunctionCoverage>) -> AggregateCoverage {
    if coverages.is_empty() {
        return AggregateCoverage {
            coverage_pct: 0.0,
            uncovered_lines: vec![],
            version_count: 0,
        };
    }

    if coverages.len() == 1 {
        return AggregateCoverage {
            coverage_pct: coverages[0].coverage_percentage,
            uncovered_lines: coverages[0].uncovered_lines.clone(),
            version_count: 1,
        };
    }

    // Intersection strategy: line is uncovered only if ALL versions leave it uncovered
    let mut uncovered_in_all: HashSet<usize> = coverages[0].uncovered_lines.iter().copied().collect();

    for coverage in &coverages[1..] {
        let uncovered_set: HashSet<usize> = coverage.uncovered_lines.iter().copied().collect();
        uncovered_in_all = uncovered_in_all.intersection(&uncovered_set).copied().collect();
    }

    // Average coverage percentage across all versions
    let avg_coverage: f64 = coverages
        .iter()
        .map(|c| c.coverage_percentage)
        .sum::<f64>() / coverages.len() as f64;

    AggregateCoverage {
        coverage_pct: avg_coverage,
        uncovered_lines: uncovered_in_all.into_iter().collect(),
        version_count: coverages.len(),
    }
}
```

**Phase 4: Integration**

Update `get_function_coverage_with_bounds()` to use aggregation:

```rust
pub fn get_function_coverage_with_bounds(
    &self,
    file: &Path,
    function_name: &str,
    start_line: usize,
    _end_line: usize,
) -> Option<f64> {
    // Try aggregated coverage first (handles generics)
    if let Some(agg) = self.get_aggregated_coverage(file, function_name) {
        return Some(agg.coverage_pct / 100.0);
    }

    // Fall back to line-based lookup
    self.get_function_coverage_with_line(file, function_name, start_line)
}
```

### Architecture Changes

**New Components** (All Pure Functions):
- `strip_trailing_generics()` - Pure normalization helper (returns `Cow<'_, str>`)
- `is_monomorphization_of()` - Pure matching logic with no side effects
- `merge_coverage()` - Pure aggregation function using intersection strategy
- `AggregateCoverage` struct to represent merged coverage data

**Modified Components**:
- `CoverageIndex` - Add `base_function_index` field for O(1) lookups
- `CoverageIndex::build_from_lcov()` - Build index at parse time
- `CoverageIndex::get_aggregated_coverage()` - Use index for O(1) lookup
- `normalize_demangled_name()` - Call `strip_trailing_generics()`

### Data Structures

```rust
/// Aggregated coverage from multiple monomorphized versions
#[derive(Debug, Clone)]
pub struct AggregateCoverage {
    /// Aggregate coverage percentage (averaged across all versions)
    pub coverage_pct: f64,
    /// Intersection of uncovered lines across all versions
    pub uncovered_lines: Vec<usize>,
    /// Number of monomorphized versions found
    pub version_count: usize,
}

/// Extension to NormalizedFunctionName
#[derive(Debug, Clone)]
pub struct NormalizedFunctionName {
    pub full_path: String,
    pub method_name: String,
    pub original: String,
    pub base_name: String,  // NEW: name without generics for grouping
    pub is_generic: bool,   // NEW: whether this is a monomorphized instance
}

/// Enhanced CoverageIndex with generic function support
pub struct CoverageIndex {
    /// Existing: file -> function_name -> coverage
    by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>>,
    /// NEW: (file, base_name) -> [monomorphized_names] for O(1) lookup
    base_function_index: HashMap<(PathBuf, String), Vec<String>>,
}
```

### Pure Function Signatures

All core logic is implemented as pure functions:

```rust
/// Pure: Strip generic parameters from function names
/// No side effects, deterministic, easy to test
fn strip_trailing_generics(s: &str) -> Cow<'_, str>;

/// Pure: Check if candidate is monomorphization of base
/// No side effects, boolean predicate
fn is_monomorphization_of(candidate: &str, base: &str) -> bool;

/// Pure: Merge coverage from multiple monomorphizations
/// No mutations, returns new AggregateCoverage
fn merge_coverage(coverages: Vec<&FunctionCoverage>) -> AggregateCoverage;

/// I/O Boundary: Build index from LCOV data
/// Only place where data structures are constructed
impl CoverageIndex {
    fn build_from_lcov(lcov_data: &LcovData) -> Self;
    fn get_aggregated_coverage(&self, file: &Path, function_name: &str) -> Option<AggregateCoverage>;
}
```

## Dependencies

- **Prerequisites**: Spec 166 (impl method matching must work)
- **Affected Components**:
  - `src/risk/lcov.rs` - normalization
  - `src/risk/coverage_index.rs` - matching and aggregation
  - `src/priority/formatter_verbosity.rs` - may need to show "N versions aggregated"
- **External Dependencies**: None (uses existing rustc-demangle)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_strip_trailing_generics_simple() {
    assert_eq!(
        strip_trailing_generics("Type::method::<WorkflowExecutor>"),
        "Type::method"
    );
    assert_eq!(
        strip_trailing_generics("crate::Type::method::<T>"),
        "crate::Type::method"
    );
    assert_eq!(
        strip_trailing_generics("Type::method"),  // No generics
        "Type::method"
    );
}

#[test]
fn test_strip_trailing_generics_nested() {
    // Nested generics
    assert_eq!(
        strip_trailing_generics("method::<Vec<HashMap<K, V>>>"),
        "method"
    );
    // Multiple type parameters
    assert_eq!(
        strip_trailing_generics("method::<T, U, V>"),
        "method"
    );
    // Complex nested case
    assert_eq!(
        strip_trailing_generics("Type::method::<Result<Vec<T>, Error>>"),
        "Type::method"
    );
}

#[test]
fn test_is_monomorphization_of_exact() {
    assert!(is_monomorphization_of(
        "SetupPhaseExecutor::execute::<WorkflowExecutor>",
        "SetupPhaseExecutor::execute"
    ));
    assert!(is_monomorphization_of(
        "SetupPhaseExecutor::execute",
        "SetupPhaseExecutor::execute"
    ));
}

#[test]
fn test_is_monomorphization_of_with_path() {
    assert!(is_monomorphization_of(
        "crate::SetupPhaseExecutor::execute::<MockExecutor>",
        "SetupPhaseExecutor::execute"
    ));
    assert!(is_monomorphization_of(
        "prodigy::workflow::SetupPhaseExecutor::execute::<T>",
        "SetupPhaseExecutor::execute"
    ));
}

#[test]
fn test_is_monomorphization_of_negative() {
    assert!(!is_monomorphization_of(
        "Other::execute::<T>",
        "SetupPhaseExecutor::execute"
    ));
    // Partial match should fail
    assert!(!is_monomorphization_of(
        "foo::execute",
        "execute"
    ));
}

#[test]
fn test_is_monomorphization_of_nested_generics() {
    assert!(is_monomorphization_of(
        "Type::method::<Vec<T>>",
        "Type::method"
    ));
    assert!(is_monomorphization_of(
        "Type::method::<HashMap<K, V>>",
        "Type::method"
    ));
}

#[test]
fn test_is_monomorphization_of_trait_methods() {
    // Trait method monomorphization
    assert!(is_monomorphization_of(
        "<Type as Trait>::method::<T>",
        "method"
    ));
}

#[test]
fn test_merge_coverage_intersection() {
    let cov1 = FunctionCoverage {
        uncovered_lines: vec![10, 20, 30],
        coverage_percentage: 70.0,
        ..Default::default()
    };
    let cov2 = FunctionCoverage {
        uncovered_lines: vec![20, 40],
        coverage_percentage: 80.0,
        ..Default::default()
    };

    let agg = merge_coverage(vec![&cov1, &cov2]);
    assert_eq!(agg.version_count, 2);
    assert_eq!(agg.coverage_pct, 75.0);  // Average: (70 + 80) / 2
    // Intersection: only line 20 is uncovered in BOTH versions
    assert_eq!(agg.uncovered_lines.len(), 1);
    assert!(agg.uncovered_lines.contains(&20));
    assert!(!agg.uncovered_lines.contains(&10));  // Covered in cov2
    assert!(!agg.uncovered_lines.contains(&40));  // Covered in cov1
}

#[test]
fn test_merge_coverage_all_covered_in_some() {
    // If ANY version covers a line, it's considered covered (intersection)
    let cov1 = FunctionCoverage {
        uncovered_lines: vec![10, 20],
        coverage_percentage: 50.0,
        ..Default::default()
    };
    let cov2 = FunctionCoverage {
        uncovered_lines: vec![30, 40],
        coverage_percentage: 50.0,
        ..Default::default()
    };

    let agg = merge_coverage(vec![&cov1, &cov2]);
    // No lines uncovered in BOTH versions
    assert_eq!(agg.uncovered_lines.len(), 0);
}

#[test]
fn test_merge_coverage_single() {
    let cov = FunctionCoverage {
        uncovered_lines: vec![10, 20],
        coverage_percentage: 75.0,
        ..Default::default()
    };

    let agg = merge_coverage(vec![&cov]);
    assert_eq!(agg.version_count, 1);
    assert_eq!(agg.coverage_pct, 75.0);
    assert_eq!(agg.uncovered_lines, vec![10, 20]);
}
```

### Integration Tests

Create test LCOV file with monomorphized functions:
```
SF:src/test.rs
FN:10,_R...Type_execute_WorkflowExecutor
FNDA:5,_R...Type_execute_WorkflowExecutor
FN:10,_R...Type_execute_MockExecutor
FNDA:0,_R...Type_execute_MockExecutor
DA:10,5
DA:20,0
```

Test that querying for `Type::execute` returns aggregated coverage.

### Performance Tests

- Benchmark generic matching vs non-generic matching
- Verify O(1) lookup for exact matches still works
- Test with 100+ monomorphizations of same function
- Measure memory overhead of aggregation

### User Acceptance

- Run on real Prodigy codebase
- Verify items #3, #4, #8 now show coverage data
- Confirm coverage percentages are reasonable
- Check that "no coverage data" cases are reduced

## Documentation Requirements

### Code Documentation

```rust
/// Aggregates coverage data from multiple monomorphized versions of a generic function.
///
/// # Strategy
///
/// Uses intersection strategy for uncovered lines: a line is considered covered if ANY
/// monomorphization covers it (i.e., a line is uncovered only if ALL versions leave it
/// uncovered). This conservative approach ensures we don't miss coverage gaps that exist
/// in all type instantiations. Coverage percentage is averaged across all versions.
///
/// # Example
///
/// For a generic function with two monomorphizations:
/// - `execute::<WorkflowExecutor>` - 70% coverage, uncovered: [10, 20, 30]
/// - `execute::<MockExecutor>` - 80% coverage, uncovered: [20, 40]
///
/// Result: 75% coverage (average), uncovered: [20] (intersection - only line uncovered in BOTH)
///
/// # Performance
///
/// This function is pure with O(m*n) time complexity where m is number of monomorphizations
/// and n is average number of uncovered lines. Pre-computed index ensures O(1) lookup.
```

### User Documentation

Add to debtmap README:
```markdown
## Generic Function Coverage

Debtmap automatically aggregates coverage from monomorphized generic functions.
When a generic function like `execute<T>()` is compiled with multiple concrete
types, coverage data is merged using a conservative intersection strategy:

- **Coverage %**: Averaged across all type instantiations
- **Uncovered lines**: Only lines uncovered in ALL versions are reported as uncovered
- **Rationale**: If any type instantiation covers a line, that code path is tested
- All monomorphizations are treated as a single logical function

Example: If `execute::<TypeA>` covers line 10 but `execute::<TypeB>` doesn't,
line 10 is considered covered because at least one execution path tests it.
```

### Architecture Updates

Update ARCHITECTURE.md:
```markdown
## Coverage Aggregation for Generic Functions

Generic Rust functions create multiple LCOV entries when monomorphized. The
coverage index aggregates these using a functional, index-based approach:

1. **Parse time**: Build `base_function_index` mapping base names to all monomorphizations
2. **Normalization**: `strip_trailing_generics()` (pure function) removes `::<Type>` suffixes
3. **Lookup**: O(1) index lookup finds all monomorphizations of base function
4. **Aggregation**: `merge_coverage()` (pure function) uses intersection strategy:
   - Line is covered if ANY monomorphization covers it
   - Coverage % is averaged across all versions
5. **All logic in pure, testable functions** - I/O only at CoverageIndex boundary
```

## Implementation Notes

### Gotchas

1. **Closure Monomorphizations**: Closures also create monomorphized names - be careful not to over-match
2. **Partial Matching**: Fixed - use `starts_with()` + boundary checks, not `ends_with()`
3. **Nested Generics**: Fixed - bracket counting algorithm handles `Vec<HashMap<K, V>>`
4. **Empty Generics**: Handle `Type::method::<>` edge case (malformed LCOV data)
5. **Performance**: O(1) lookups via pre-built index, no need for runtime caching

### Best Practices

- **All logic in pure functions** - `strip_trailing_generics()`, `is_monomorphization_of()`, `merge_coverage()`
- **Build index at parse time** - O(1) lookups, no repeated scanning
- **Use `Cow<'_, str>`** - Avoid allocations when no stripping needed
- **Intersection strategy is conservative** - Only report gaps that exist in ALL code paths
- **Log aggregations in verbose mode** - Show "N versions aggregated" for debugging

## Migration and Compatibility

### Breaking Changes

None - this is purely additive functionality.

### Compatibility Considerations

- Existing exact-match lookups work as before
- Non-generic functions have zero performance impact
- LCOV file format is unchanged
- Debtmap CLI interface is unchanged

### Migration Path

1. Deploy updated debtmap with generic matching
2. Verify improved coverage reporting on Prodigy
3. Monitor for any false positive matches
4. Tune matching heuristics if needed

## Success Metrics

- [ ] Prodigy top 10: Items showing "no coverage data" reduced from 4 to 0
- [ ] Coverage matching success rate increases from 60% to >90%
- [ ] No regression in non-generic function matching performance
- [ ] Zero false positive matches in integration tests

## Resolved Design Decisions

### 1. Show version count in output?
**Decision**: Yes, but only in verbose mode (`-v` or `--verbose` flag).
- Normal mode: `75% coverage`
- Verbose mode: `75% coverage (aggregated from 3 monomorphizations)`

### 2. Aggregation strategy?
**Decision**: Intersection strategy for uncovered lines.
- A line is considered **covered if ANY monomorphization covers it**
- A line is reported as **uncovered only if ALL monomorphizations leave it uncovered**
- Rationale: Conservative approach - if any type instantiation tests a line, that code path is exercised
- Coverage percentage: Average across all versions

### 3. Handle conflicting line coverage?
**Decision**: Intersection strategy resolves this naturally.
- Line 10 covered by `execute::<TypeA>` but not `execute::<TypeB>`: marked as **covered**
- Line 20 uncovered by both: marked as **uncovered**
- This aligns with practical testing: if any type instantiation exercises code, it's tested

### 4. Expose individual monomorphization coverage?
**Decision**: Yes, via verbose flag with new option `--show-monomorphizations`.
- Shows breakdown: "execute::<WorkflowExecutor>: 70%, execute::<MockExecutor>: 80%"
- Helps debug coverage discrepancies between type instantiations
- Only shown when explicitly requested to avoid output clutter

## Related Specifications

- **Spec 166**: Rust impl method LCOV matching (prerequisite)
- **Spec 151**: Pattern analysis framework (may benefit from better coverage data)
