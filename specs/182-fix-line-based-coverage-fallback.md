---
number: 182
title: Fix Line-Based Coverage Fallback Reliability
category: testing
priority: high
status: draft
dependencies: [181]
created: 2025-01-17
---

# Specification 182: Fix Line-Based Coverage Fallback Reliability

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: Spec 181 (Trait Method Coverage Matching)

## Context

Debtmap has a line-based coverage fallback mechanism in `CoverageIndex::find_function_by_line()` that should match functions when name-based matching fails. This fallback uses a tolerance of ±2 lines to handle minor discrepancies between AST-reported line numbers and LCOV line numbers.

However, this fallback appears to be failing for trait implementation methods despite being within tolerance range. This suggests a bug in the fallback logic or its invocation.

### Current Implementation

```rust
// src/risk/coverage_index.rs:465
pub fn get_function_coverage_with_line(
    &self,
    file: &Path,
    function_name: &str,
    line: usize,
) -> Option<f64> {
    // Try aggregated coverage first (handles generics)
    if let Some(agg) = self.get_aggregated_coverage(file, function_name) {
        return Some(agg.coverage_pct / 100.0);  // ✅ Works for exact matches
    }

    // Try line-based lookup (O(log n)) - SHOULD catch mismatches
    if let Some(coverage) = self
        .find_function_by_line(file, line, 2)   // ❌ Not being reached or failing?
        .map(|f| f.coverage_percentage / 100.0)
    {
        return Some(coverage);
    }

    // Only fall back to path matching strategies if line lookup fails
    self.find_by_path_strategies(file, function_name)
        .map(|f| f.coverage_percentage / 100.0)
}
```

### Evidence of Failure

**Case Study**: `RecursiveMatchDetector::visit_expr`

- **AST line number**: 177
- **LCOV line number**: 177 (exact match!)
- **Tolerance**: ±2 lines (175-179)
- **Expected behavior**: Line-based fallback should find it
- **Actual behavior**: Reports "no coverage data"

**Verification**:
```bash
# Name-based matching fails (spec 181 issue)
$ debtmap explain-coverage . --coverage-file target/coverage/lcov.info \
    --function "RecursiveMatchDetector::visit_expr"
✗ Coverage Not Found

# But line-based matching with just method name works!
$ debtmap explain-coverage . --coverage-file target/coverage/lcov.info \
    --function "visit_expr"
✓ Coverage Found!
  Strategy: exact_match
  Coverage: 90.2%
```

This proves:
1. LCOV data exists at the correct line (177)
2. Line-based fallback should trigger but doesn't
3. Something prevents fallback from being reached or working

### Possible Root Causes

**Hypothesis 1: Aggregated Coverage Early Return**
- `get_aggregated_coverage()` may be matching something incorrectly
- Returns early before line-based fallback runs
- Needs investigation and logging

**Hypothesis 2: File Path Mismatch**
- Line-based lookup uses file path as key
- Path normalization issues prevent index lookup
- `self.by_line.get(file)` returns None

**Hypothesis 3: Tolerance Calculation Bug**
- Range calculation in `find_function_by_line()` has off-by-one error
- Or BTreeMap range query excludes boundary
- Needs thorough testing

**Hypothesis 4: Function Not in Line Index**
- LCOV parsing may not populate `by_line` index correctly
- Especially for trait implementations or complex symbols
- Index construction needs validation

## Objective

Debug and fix the line-based coverage fallback mechanism to ensure it reliably finds coverage data when name-based matching fails, specifically for trait implementation methods and other complex function names.

## Requirements

### Functional Requirements

**FR1: Line-Based Fallback Reliability**
- Line-based fallback MUST trigger when name matching fails
- MUST successfully match functions within ±2 line tolerance
- MUST work regardless of function name format
- MUST be independent of name-based matching strategies

**FR2: Diagnostic Logging**
- Add debug logging to trace fallback execution path
- Log why line-based lookup succeeds or fails
- Track which matching strategy actually finds coverage
- Enable troubleshooting without code changes

**FR3: Index Population Validation**
- Verify all LCOV functions are indexed by line
- Detect and report indexing failures
- Validate file path normalization consistency
- Ensure trait methods are indexed properly

**FR4: Fallback Execution Order**
- Document and test exact order of fallback attempts
- Ensure line-based fallback has priority over path matching
- Prevent early returns from bypassing fallback
- Make fallback chain explicit and testable

### Non-Functional Requirements

**NFR1: Performance**
- Line-based fallback remains O(log n) using BTreeMap
- No performance regression from debugging/logging
- Maintain fast path for successful name matches
- Index size and construction time unchanged

**NFR2: Debuggability**
- Clear error messages when fallback fails
- Trace-level logging shows all attempted strategies
- explain-coverage tool reports fallback status
- Easy to reproduce and diagnose failures

**NFR3: Testability**
- Unit tests for each fallback failure mode
- Integration tests with real LCOV data
- Property tests for tolerance calculation
- Regression tests for known failures

## Acceptance Criteria

- [ ] **AC1**: `RecursiveMatchDetector::visit_expr` matches via line-based fallback (before spec 181 fix)
- [ ] **AC2**: Line-based fallback triggers and succeeds when name matching fails
- [ ] **AC3**: Diagnostic logging shows exact fallback execution path
- [ ] **AC4**: All functions in LCOV data are present in line-based index
- [ ] **AC5**: Tolerance calculation correctly includes ±2 line range
- [ ] **AC6**: File path normalization consistent between index and lookup
- [ ] **AC7**: No performance regression in coverage matching (benchmark)
- [ ] **AC8**: explain-coverage tool shows line-based fallback successes
- [ ] **AC9**: Zero instances of "no coverage data" when line-based match possible

## Technical Details

### Implementation Approach

**Phase 1: Add Diagnostic Logging**

```rust
pub fn get_function_coverage_with_line(
    &self,
    file: &Path,
    function_name: &str,
    line: usize,
) -> Option<f64> {
    tracing::debug!(
        file = %file.display(),
        function_name,
        line,
        "Attempting coverage lookup"
    );

    // Try aggregated coverage first
    if let Some(agg) = self.get_aggregated_coverage(file, function_name) {
        tracing::debug!(
            strategy = "aggregated",
            coverage = agg.coverage_pct,
            "Coverage found"
        );
        return Some(agg.coverage_pct / 100.0);
    }

    // Try line-based lookup
    tracing::debug!("Trying line-based lookup with tolerance ±2");

    match self.find_function_by_line(file, line, 2) {
        Some(f) => {
            tracing::debug!(
                strategy = "line_based",
                matched_line = f.start_line,
                coverage = f.coverage_percentage,
                "Coverage found"
            );
            return Some(f.coverage_percentage / 100.0);
        }
        None => {
            tracing::debug!(
                "Line-based lookup failed, checking if file in index"
            );

            if !self.by_line.contains_key(file) {
                tracing::warn!(
                    file = %file.display(),
                    "File not found in line-based index"
                );
            } else {
                let line_map = &self.by_line[file];
                tracing::debug!(
                    functions_in_file = line_map.len(),
                    "Line-based index has functions for this file"
                );
            }
        }
    }

    // Fallback to path strategies
    tracing::debug!("Trying path matching strategies");
    if let Some(f) = self.find_by_path_strategies(file, function_name) {
        tracing::debug!(
            strategy = "path_matching",
            coverage = f.coverage_percentage,
            "Coverage found"
        );
        return Some(f.coverage_percentage / 100.0);
    }

    tracing::warn!(
        file = %file.display(),
        function_name,
        line,
        "No coverage found after all strategies"
    );
    None
}
```

**Phase 2: Validate Index Population**

```rust
/// Verify line-based index is populated correctly
pub fn validate_line_index(&self) -> IndexValidationReport {
    let mut report = IndexValidationReport::default();

    for (file, file_map) in &self.by_file {
        let by_name_count = file_map.len();

        let by_line_count = self.by_line
            .get(file)
            .map(|m| m.len())
            .unwrap_or(0);

        if by_name_count != by_line_count {
            report.mismatches.push(IndexMismatch {
                file: file.clone(),
                by_name: by_name_count,
                by_line: by_line_count,
            });
        }

        report.total_files += 1;
        report.total_functions += by_name_count;
    }

    report
}

#[derive(Debug, Default)]
pub struct IndexValidationReport {
    pub total_files: usize,
    pub total_functions: usize,
    pub mismatches: Vec<IndexMismatch>,
}

#[derive(Debug)]
pub struct IndexMismatch {
    pub file: PathBuf,
    pub by_name: usize,
    pub by_line: usize,
}
```

**Phase 3: Fix Tolerance Calculation**

```rust
fn find_function_by_line(
    &self,
    file: &Path,
    target_line: usize,
    tolerance: usize,
) -> Option<&FunctionCoverage> {
    let line_map = self.by_line.get(file)?;

    // Define search range with tolerance
    let min_line = target_line.saturating_sub(tolerance);
    let max_line = target_line.saturating_add(tolerance);

    tracing::trace!(
        target_line,
        min_line,
        max_line,
        available_lines = ?line_map.keys().collect::<Vec<_>>(),
        "Searching line-based index"
    );

    // Use BTreeMap range query - INCLUSIVE on both ends
    line_map
        .range(min_line..=max_line)  // Ensure inclusive range
        .min_by_key(|(line, _)| line.abs_diff(target_line))
        .map(|(_, func)| func)
}
```

**Phase 4: Verify File Path Normalization**

```rust
/// Ensure consistent path normalization for indexing and lookup
fn normalize_coverage_path(path: &Path) -> PathBuf {
    // Strip leading "./" or "../" prefixes
    // Canonicalize to absolute path if possible
    // Use consistent separator (always /)

    let path_str = path.to_string_lossy();

    // Remove leading "./"
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);

    PathBuf::from(cleaned)
}

// Apply normalization consistently:
// 1. When building index from LCOV
// 2. When looking up coverage by file
// 3. When creating function identifiers
```

### Architecture Changes

**Modified Components**:
- `src/risk/coverage_index.rs` - Add logging, validation, fixes
- `src/risk/lcov.rs` - Ensure consistent path normalization
- `src/commands/explain_coverage.rs` - Show fallback strategy used

**New Components**:
- `src/risk/coverage_index_validation.rs` - Index validation utilities

### Data Structures

```rust
/// Track which strategy successfully matched coverage
#[derive(Debug, Clone)]
pub enum CoverageMatchStrategy {
    AggregatedExact,
    LineBasedFallback { matched_line: usize, distance: usize },
    PathStrategies { strategy_name: String },
    NotFound,
}

/// Extend FunctionCoverage to track match metadata
pub struct CoverageMatchResult {
    pub coverage: f64,
    pub strategy: CoverageMatchStrategy,
    pub function_name: String,
    pub start_line: usize,
}
```

### APIs and Interfaces

**Enhanced Coverage Lookup**:

```rust
// Add optional match strategy output
pub fn get_function_coverage_with_line_detailed(
    &self,
    file: &Path,
    function_name: &str,
    line: usize,
) -> Option<CoverageMatchResult> {
    // Returns coverage AND how it was found
}

// Backward compatible wrapper
pub fn get_function_coverage_with_line(
    &self,
    file: &Path,
    function_name: &str,
    line: usize,
) -> Option<f64> {
    self.get_function_coverage_with_line_detailed(file, function_name, line)
        .map(|r| r.coverage)
}
```

## Dependencies

- **Prerequisites**: Spec 181 (demonstrates the issue, but fix is independent)
- **Affected Components**:
  - `src/risk/coverage_index.rs` - Core fix location
  - `src/risk/lcov.rs` - Path normalization
  - `src/commands/explain_coverage.rs` - Diagnostic reporting
- **External Dependencies**:
  - `tracing` crate (already used) - for diagnostic logging

## Testing Strategy

### Unit Tests

**Test Tolerance Calculation**:
```rust
#[test]
fn test_line_based_fallback_exact_match() {
    let index = build_index_with_function("test.rs", "foo", 100, 0.85);

    let result = index.find_function_by_line(
        Path::new("test.rs"),
        100, // Exact line
        2    // Tolerance
    );

    assert!(result.is_some());
    assert_eq!(result.unwrap().coverage_percentage, 85.0);
}

#[test]
fn test_line_based_fallback_within_tolerance() {
    let index = build_index_with_function("test.rs", "foo", 100, 0.85);

    // Try lines 98-102 (all within ±2 tolerance)
    for line in 98..=102 {
        let result = index.find_function_by_line(
            Path::new("test.rs"),
            line,
            2
        );

        assert!(
            result.is_some(),
            "Line {} should match function at 100 with ±2 tolerance",
            line
        );
    }
}

#[test]
fn test_line_based_fallback_outside_tolerance() {
    let index = build_index_with_function("test.rs", "foo", 100, 0.85);

    let result = index.find_function_by_line(
        Path::new("test.rs"),
        97, // Just outside ±2 tolerance
        2
    );

    assert!(result.is_none());
}

#[test]
fn test_line_based_fallback_chooses_closest() {
    let mut index = CoverageIndex::empty();
    index.add_function("test.rs", "foo", 100, 0.85);
    index.add_function("test.rs", "bar", 102, 0.90);

    let result = index.find_function_by_line(
        Path::new("test.rs"),
        101, // Closer to 100 (distance 1) than 102 (distance 1) - tie breaks by order
        2
    );

    assert!(result.is_some());
    // Should pick closest (or first if tied)
}
```

**Test Index Population**:
```rust
#[test]
fn test_all_functions_in_line_index() {
    let lcov_data = parse_lcov_file("tests/fixtures/sample.lcov");
    let index = CoverageIndex::from_coverage(&lcov_data);

    let report = index.validate_line_index();

    assert_eq!(
        report.mismatches.len(),
        0,
        "All functions should be in both by_name and by_line indexes"
    );
}

#[test]
fn test_trait_methods_in_line_index() {
    let lcov_data = parse_lcov_with_trait_method();
    let index = CoverageIndex::from_coverage(&lcov_data);

    // Verify trait method is in line index
    let file = Path::new("src/test.rs");
    assert!(
        index.by_line.contains_key(file),
        "File should be in line index"
    );

    let line_map = &index.by_line[file];
    assert!(
        line_map.contains_key(&177),
        "Line 177 should have indexed function"
    );
}
```

**Test Path Normalization**:
```rust
#[test]
fn test_path_normalization_consistency() {
    let paths = vec![
        "./src/test.rs",
        "src/test.rs",
        "../debtmap/src/test.rs",
    ];

    let normalized: Vec<_> = paths.iter()
        .map(|p| normalize_coverage_path(Path::new(p)))
        .collect();

    // All should normalize to same path
    assert_eq!(normalized[0], normalized[1]);
}
```

### Integration Tests

**Test Real Trait Implementation**:
```rust
#[test]
fn test_visitor_line_based_fallback() {
    let lcov = parse_lcov_file("target/coverage/lcov.info");
    let index = CoverageIndex::from_coverage(&lcov);

    // Attempt lookup with wrong name but correct line
    let coverage = index.get_function_coverage_with_line(
        Path::new("src/complexity/recursive_detector.rs"),
        "WRONG_NAME_SHOULD_FALLBACK_TO_LINE", // Name won't match
        177, // Correct line
    );

    assert!(
        coverage.is_some(),
        "Line-based fallback should find coverage despite wrong name"
    );
    assert!(coverage.unwrap() > 0.80, "Should find 90%+ coverage");
}
```

**Test with Diagnostic Logging**:
```rust
#[test]
fn test_fallback_strategy_logging() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .try_init();

    let lcov = parse_lcov_file("target/coverage/lcov.info");
    let index = CoverageIndex::from_coverage(&lcov);

    let result = index.get_function_coverage_with_line_detailed(
        Path::new("src/complexity/recursive_detector.rs"),
        "RecursiveMatchDetector::visit_expr",
        177,
    );

    // Verify strategy used
    match result {
        Some(CoverageMatchResult { strategy: CoverageMatchStrategy::LineBasedFallback { .. }, .. }) => {
            // Success! Fallback worked
        }
        _ => panic!("Expected line-based fallback to succeed"),
    }
}
```

### Performance Tests

```rust
#[bench]
fn bench_line_based_fallback(b: &mut Bencher) {
    let index = build_large_index(10_000);

    b.iter(|| {
        index.find_function_by_line(
            Path::new("src/test.rs"),
            5000, // Middle of file
            2
        )
    });
}
```

### User Acceptance

- [ ] Run full analysis with RUST_LOG=debug and verify fallback attempts logged
- [ ] Check that all trait methods find coverage via fallback
- [ ] Validate explain-coverage shows which strategy matched
- [ ] Confirm zero performance regression in benchmarks

## Documentation Requirements

### Code Documentation

```rust
/// Find a function by line number with tolerance.
///
/// This is a fallback mechanism for when name-based matching fails.
/// It's particularly useful for:
/// - Trait implementation methods (name format varies)
/// - Generic functions (multiple monomorphizations)
/// - Functions where LCOV and AST disagree on naming
///
/// # Arguments
/// * `file` - Path to source file
/// * `target_line` - Line number to search for
/// * `tolerance` - Number of lines above/below to check (typically 2)
///
/// # Returns
/// The closest function within tolerance, or None if no match found.
///
/// # Algorithm
/// Uses BTreeMap range query for O(log n) performance. When multiple
/// functions are within tolerance, returns the closest by absolute distance.
fn find_function_by_line(
    &self,
    file: &Path,
    target_line: usize,
    tolerance: usize,
) -> Option<&FunctionCoverage>
```

### User Documentation

Add troubleshooting guide:

```markdown
## Coverage Detection Troubleshooting

### Diagnostic Logging

Enable trace logging to see coverage matching attempts:

\`\`\`bash
RUST_LOG=debtmap::risk=debug debtmap analyze . --lcov coverage.lcov
\`\`\`

Look for:
- "Attempting coverage lookup" - shows each attempt
- "Coverage found" - shows which strategy succeeded
- "Line-based lookup failed" - indicates fallback didn't work

### Common Issues

**"File not found in line-based index"**
- LCOV path doesn't match source file path
- Run with --verbose to see path normalization
- Check that LCOV file paths are relative or absolute consistently

**"No coverage found after all strategies"**
- Function genuinely has no coverage, OR
- Line number mismatch exceeds ±2 tolerance, OR
- File path normalization issue

Use explain-coverage for detailed diagnosis:
\`\`\`bash
debtmap explain-coverage . --coverage-file coverage.lcov \
  --function "MyFunction" --file src/myfile.rs --verbose
\`\`\`
```

### Architecture Updates

```markdown
## Coverage Matching Strategy (Updated)

Coverage lookup attempts strategies in order:

1. **Aggregated Coverage** (O(1))
   - Exact name match in by_file index
   - Handles generic monomorphizations

2. **Line-Based Fallback** (O(log n))
   - BTreeMap range query with ±2 tolerance
   - Finds closest function by line number
   - Critical for trait methods and name mismatches

3. **Path Matching Strategies** (O(n))
   - Fuzzy file path matching
   - Handles different path formats
   - Last resort

**Diagnostic Logging**: All strategies log attempts for troubleshooting.

See: src/risk/coverage_index.rs, Spec 182
```

## Implementation Notes

### Root Cause Investigation

Before implementing fixes, must determine WHY fallback currently fails:

1. Add trace logging to every step
2. Run on known failure case (RecursiveMatchDetector::visit_expr)
3. Examine logs to find exact failure point
4. Determine if issue is:
   - Index population (function not in by_line)
   - Path normalization (can't find file)
   - Tolerance calculation (range query misses)
   - Early return (fallback never reached)

### Logging Best Practices

- Use `tracing::debug` for normal operation
- Use `tracing::trace` for detailed internals
- Use `tracing::warn` for unexpected conditions
- Include relevant context (file, function, line) in all logs

### Testing Strategy

Test in isolation BEFORE integrating with Spec 181:
- Ensures fixes stand alone
- Avoids confusing two separate issues
- Allows independent deployment if needed

## Migration and Compatibility

### No Breaking Changes

- Public API signatures unchanged
- LCOV parsing unchanged
- Existing coverage matches unaffected
- Only adds reliability, doesn't change behavior

### Deployment

1. Add logging first (safe, informational only)
2. Validate index population (diagnostic)
3. Fix any identified bugs (tolerance, normalization, etc.)
4. Verify with integration tests
5. Deploy and monitor logs

### Rollback

- Logging can be disabled via log level
- Fixes are localized to coverage_index.rs
- Can revert individual commits if needed
