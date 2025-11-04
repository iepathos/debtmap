---
number: 166
title: Fix LCOV Function Name Matching for Rust Methods
category: compatibility
priority: critical
status: draft
dependencies: []
created: 2025-01-04
---

# Specification 166: Fix LCOV Function Name Matching for Rust Methods

**Category**: compatibility
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

**Problem**: Debtmap incorrectly marks well-tested Rust functions as `[ERROR UNTESTED]` despite having coverage data in LCOV files. This creates false positives that undermine user trust and lead to wasted effort investigating non-existent testing gaps.

**Root Cause Identified**: Function name mismatch between:
- **Source extraction**: Debtmap extracts simple method names like `"create_auto_commit"` from Rust AST
- **LCOV demangling**: After demangling, LCOV contains fully-qualified names like `"prodigy::cook::commit_tracker::CommitTracker::create_auto_commit"`

**Evidence from Investigation**:
```
LCOV Entry (mangled):
FN:413,_RNvMNtNtCsaYlvcVeJQlC_7prodigy4cook14commit_trackerNtB2_13CommitTracker18create_auto_commit
FNDA:3,_RNvMNtNtCsaYlvcVeJQlC_7prodigy4cook14commit_trackerNtB2_13CommitTracker18create_auto_commit
DA:413,3  ← Line 413 has 3 executions

After demangling → "<prodigy::cook::commit_tracker::CommitTracker>::create_auto_commit"
After normalization → "prodigy::cook::commit_tracker::CommitTracker::create_auto_commit"

Debtmap FunctionId:
- name: "create_auto_commit"
- file: "src/cook/commit_tracker.rs"
- line: 413

Matching attempt: "create_auto_commit" vs "prodigy::cook::commit_tracker::CommitTracker::create_auto_commit"
Result: NO MATCH → Coverage returns 0.0 → Marked as UNTESTED
```

**Impact**:
- Prodigy codebase: 10/10 top debt items falsely flagged as untested
- User trust degraded due to >50% false positive rate on `[ERROR UNTESTED]` flag
- Teams waste time investigating well-tested code

**Current Implementation**:
- Demangling exists: `src/risk/lcov.rs:62-72` (`demangle_function_name`)
- Normalization exists: `src/risk/lcov.rs:82-110` (`normalize_demangled_name`)
- Fuzzy matching exists: suffix matching in `src/risk/lcov.rs:545-550`
- **But**: Normalization doesn't extract method names correctly for matching

## Objective

Fix LCOV function name matching to correctly identify coverage for Rust methods by enhancing the normalization and matching strategies to handle fully-qualified demangled names.

## Requirements

### Functional Requirements

1. **Enhanced Name Normalization**
   - Extract final method name from fully-qualified paths
   - Handle Rust-specific patterns: `Struct::method`, `<Crate::Module::Struct>::method`
   - Preserve both full path and method name for matching strategies
   - Support trait implementations: `<Struct as Trait>::method`

2. **Improved Matching Strategies**
   - **Strategy 1**: Exact match (current - keep as-is)
   - **Strategy 2**: Method name match (NEW - match just the final segment)
   - **Strategy 3**: Line-based match (current - keep as-is)
   - **Strategy 4**: Suffix match (current - enhance)
   - Apply strategies in order of specificity (exact → line → method → suffix)

3. **Validation and Debugging**
   - Add debug logging for matching attempts
   - Track which strategy succeeded for metrics
   - Warn when multiple strategies match different functions
   - Provide `--explain-coverage <function>` CLI flag for debugging

### Non-Functional Requirements

1. **Performance**
   - Name normalization must remain O(n) per function
   - No degradation to existing LCOV parsing performance
   - Maintain parallel processing capabilities

2. **Backward Compatibility**
   - Must not break existing coverage detection
   - Support non-Rust languages (Python, JS, TS)
   - Preserve existing LCOV file format support

3. **Accuracy**
   - Reduce false positive rate from >50% to <10%
   - Prefer false negatives (missed coverage) over false positives (fake coverage)
   - Use line numbers as tie-breaker for ambiguous matches

## Acceptance Criteria

- [ ] Functions with LCOV coverage data no longer show `[ERROR UNTESTED]` when line numbers match
- [ ] Method name extraction correctly handles patterns:
  - `Module::Struct::method` → `"method"`
  - `<Crate[hash]::Module::Struct>::method` → `"method"`
  - `<Struct as Trait>::method` → `"method"`
  - `impl_name::method` → `"method"`
- [ ] Line-based matching serves as primary disambiguation strategy
- [ ] Suffix matching works for both directions (LCOV suffix matches FunctionId, FunctionId suffix matches LCOV)
- [ ] No performance regression in LCOV parsing (benchmark: <5% slowdown)
- [ ] Debug logging shows matching strategy used for each function
- [ ] CLI flag `--explain-coverage src/file.rs:123:function_name` shows:
  - Function name from source
  - Matching LCOV entries (if any)
  - Which strategy succeeded/failed
  - Coverage percentage and execution count
- [ ] Integration test with real mangled LCOV data from cargo-tarpaulin
- [ ] Prodigy codebase analysis shows <10% false positive rate
- [ ] Documentation updated with LCOV matching behavior and troubleshooting

## Technical Details

### Implementation Approach

**Phase 1: Enhanced Normalization** (Priority: Critical)

Add method name extraction to `normalize_demangled_name()`:

```rust
/// Result of demangling and normalizing a function name
struct NormalizedFunctionName {
    /// Full normalized path: "module::Struct::method"
    full_path: String,

    /// Just the method name: "method"
    method_name: String,

    /// Original demangled name (for debugging)
    original: String,
}

fn normalize_demangled_name(demangled: &str) -> NormalizedFunctionName {
    // Existing normalization for full_path
    let full_path = remove_generics_and_hashes(demangled);

    // NEW: Extract method name (final segment after last ::)
    let method_name = full_path
        .rsplit("::")
        .next()
        .unwrap_or(&full_path)
        .to_string();

    NormalizedFunctionName {
        full_path,
        method_name,
        original: demangled.to_string(),
    }
}
```

**Phase 2: Enhanced Matching** (Priority: Critical)

Update `find_function_in_array()` to use multiple strategies:

```rust
fn find_function_in_array<'a>(
    funcs: &'a [FunctionCoverage],
    target_name: &str,
    target_line: Option<usize>,
) -> Option<&'a FunctionCoverage> {
    // Strategy 1: Exact name match
    if let Some(func) = funcs.iter().find(|f| f.normalized.full_path == target_name) {
        return Some(func);
    }

    // Strategy 2: Line-based match (most reliable for disambiguating methods)
    if let Some(line) = target_line {
        if let Some(func) = funcs.iter().find(|f| f.start_line == line) {
            return Some(func);
        }
    }

    // Strategy 3: Method name match (NEW)
    if let Some(func) = funcs.iter().find(|f| f.normalized.method_name == target_name) {
        return Some(func);
    }

    // Strategy 4: Suffix match (enhanced with method name)
    funcs.iter().find(|f| {
        f.normalized.full_path.ends_with(target_name)
        || target_name.ends_with(&f.normalized.method_name)
        || f.normalized.method_name.ends_with(target_name)
    })
}
```

**Phase 3: Debug Support** (Priority: High)

Add CLI command for coverage debugging:

```rust
// New CLI subcommand
pub struct ExplainCoverageArgs {
    /// File path
    pub file: PathBuf,

    /// Function name
    pub function: String,

    /// Line number (optional)
    pub line: Option<usize>,

    /// LCOV file path
    pub coverage_path: PathBuf,
}

pub fn explain_coverage(args: ExplainCoverageArgs) -> Result<()> {
    let lcov = parse_lcov_file(&args.coverage_path)?;

    println!("Searching for coverage of: {}::{}::{}",
        args.file.display(), args.line.unwrap_or(0), args.function);

    // Try all matching strategies and report results
    println!("\n Strategy 1 (Exact Match):");
    // ... show results

    println!("\n Strategy 2 (Line Match):");
    // ... show results

    println!("\n Strategy 3 (Method Name Match):");
    // ... show results

    println!("\n Strategy 4 (Suffix Match):");
    // ... show results

    Ok(())
}
```

### Architecture Changes

**Modified Files**:
1. `src/risk/lcov.rs`:
   - Update `normalize_demangled_name()` to return struct with multiple name variants
   - Modify `FunctionCoverage` to store `NormalizedFunctionName`
   - Enhance `find_function_in_array()` with new matching strategies
   - Add debug logging to matching attempts

2. `src/cli.rs`:
   - Add `explain-coverage` subcommand
   - Wire up new debug functionality

3. `src/priority/coverage_propagation.rs`:
   - Update to use enhanced LCOV matching
   - Add logging for coverage lookups

**New Files**:
1. `tests/lcov_rust_method_matching_test.rs`:
   - Integration test with real mangled LCOV data
   - Test cases for all Rust method patterns
   - Regression test for Prodigy false positives

### Data Structures

```rust
// In src/risk/lcov.rs

/// Normalized function name with multiple matching variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFunctionName {
    /// Full normalized path: "module::Struct::method"
    pub full_path: String,

    /// Just the method name: "method"
    pub method_name: String,

    /// Original demangled name (for debugging)
    pub original: String,
}

/// Enhanced FunctionCoverage with normalized names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCoverage {
    /// Original name from LCOV (may be mangled)
    pub name: String,

    /// Normalized name variants for matching
    #[serde(skip)]
    pub normalized: NormalizedFunctionName,

    pub start_line: usize,
    pub execution_count: u64,
    pub coverage_percentage: f64,
    pub uncovered_lines: Vec<usize>,
}

/// Matching strategy result for debugging
#[derive(Debug, Clone)]
pub enum MatchStrategy {
    ExactMatch,
    LineMatch,
    MethodNameMatch,
    SuffixMatch,
    NoMatch,
}
```

### APIs and Interfaces

**Public API Changes**:

```rust
// New public function for debugging
impl LcovData {
    /// Explain coverage lookup for a function (debug helper)
    pub fn explain_coverage(
        &self,
        file: &Path,
        function_name: &str,
        line: Option<usize>,
    ) -> CoverageExplanation {
        // Returns detailed info about matching attempts
    }
}

/// Coverage explanation for debugging
pub struct CoverageExplanation {
    pub target: String,
    pub strategies_tried: Vec<(MatchStrategy, Option<FunctionCoverage>)>,
    pub final_result: Option<f64>,
}
```

## Dependencies

**Prerequisites**: None (bug fix to existing functionality)

**Affected Components**:
- `src/risk/lcov.rs` - LCOV parsing and matching
- `src/priority/coverage_propagation.rs` - Coverage lookup
- `src/cli.rs` - CLI interface
- All code paths that use `LcovData::get_function_coverage()`

**External Dependencies**:
- `rustc-demangle` (already in use)
- No new external dependencies required

## Testing Strategy

### Unit Tests

1. **Normalization Tests** (`src/risk/lcov.rs`)
   ```rust
   #[test]
   fn test_normalize_rust_method() {
       let input = "prodigy::cook::CommitTracker::create_auto_commit";
       let result = normalize_demangled_name(input);
       assert_eq!(result.method_name, "create_auto_commit");
       assert_eq!(result.full_path, "prodigy::cook::CommitTracker::create_auto_commit");
   }

   #[test]
   fn test_normalize_trait_impl() {
       let input = "<Foo as Bar>::method";
       let result = normalize_demangled_name(input);
       assert_eq!(result.method_name, "method");
   }

   #[test]
   fn test_normalize_with_generics() {
       let input = "HashMap<K,V>::insert";
       let result = normalize_demangled_name(input);
       assert_eq!(result.method_name, "insert");
       assert_eq!(result.full_path, "HashMap::insert");
   }
   ```

2. **Matching Strategy Tests**
   ```rust
   #[test]
   fn test_method_name_match() {
       let funcs = vec![
           FunctionCoverage {
               name: "mangled_name".into(),
               normalized: NormalizedFunctionName {
                   full_path: "mod::Struct::method".into(),
                   method_name: "method".into(),
                   original: "...".into(),
               },
               start_line: 100,
               execution_count: 5,
               coverage_percentage: 80.0,
               uncovered_lines: vec![],
           }
       ];

       let result = find_function_in_array(&funcs, "method", None);
       assert!(result.is_some());
   }

   #[test]
   fn test_line_match_disambiguates() {
       // Two functions with same method name, different lines
       let funcs = vec![
           create_func("Struct1::method", 100),
           create_func("Struct2::method", 200),
       ];

       let result = find_function_in_array(&funcs, "method", Some(200));
       assert_eq!(result.unwrap().start_line, 200);
   }
   ```

3. **Demangling Integration Tests**
   ```rust
   #[test]
   fn test_demangle_real_rust_symbols() {
       let mangled = "_RNvMNtNtCsaYlvcVeJQlC_7prodigy4cook14commit_trackerNtB2_13CommitTracker18create_auto_commit";
       let demangled = demangle_function_name(mangled);
       let normalized = normalize_demangled_name(&demangled);

       assert_eq!(normalized.method_name, "create_auto_commit");
       assert!(normalized.full_path.contains("CommitTracker"));
   }
   ```

### Integration Tests

1. **Real LCOV File Test** (`tests/lcov_rust_method_matching_test.rs`)
   ```rust
   #[test]
   fn test_prodigy_create_auto_commit_coverage() {
       // Use actual LCOV snippet from Prodigy
       let lcov_data = r#"
       SF:/Users/glen/memento-mori/prodigy/src/cook/commit_tracker.rs
       FN:413,_RNvMNtNtCsaYlvcVeJQlC_7prodigy4cook14commit_trackerNtB2_13CommitTracker18create_auto_commit
       FNDA:3,_RNvMNtNtCsaYlvcVeJQlC_7prodigy4cook14commit_trackerNtB2_13CommitTracker18create_auto_commit
       DA:413,3
       end_of_record
       "#;

       let lcov = parse_lcov_string(lcov_data).unwrap();

       // Simulate debtmap lookup
       let coverage = lcov.get_function_coverage_with_line(
           Path::new("src/cook/commit_tracker.rs"),
           "create_auto_commit",
           413
       );

       assert!(coverage.is_some());
       assert!(coverage.unwrap() > 0.0);
   }
   ```

2. **Prodigy Regression Test**
   ```rust
   #[test]
   fn test_prodigy_top_10_false_positives() {
       // Test all 10 functions from bug report
       let test_cases = vec![
           ("src/cook/workflow/executor/commands.rs", "execute_command_by_type", 406, 25),
           ("src/cook/commit_tracker.rs", "create_auto_commit", 413, 3),
           // ... remaining 8 functions
       ];

       let lcov = load_prodigy_lcov(); // Use fixture

       for (file, func, line, expected_count) in test_cases {
           let coverage = lcov.get_function_coverage_with_line(
               Path::new(file), func, line
           );
           assert!(coverage.is_some(),
               "Function {}:{}:{} should have coverage", file, line, func);
       }
   }
   ```

### Performance Tests

```rust
#[bench]
fn bench_normalize_demangled_name(b: &mut Bencher) {
    let names = generate_test_names(1000);
    b.iter(|| {
        for name in &names {
            normalize_demangled_name(name);
        }
    });
}

#[bench]
fn bench_function_matching(b: &mut Bencher) {
    let funcs = generate_test_functions(1000);
    b.iter(|| {
        find_function_in_array(&funcs, "test_method", Some(500));
    });
}
```

### User Acceptance

1. **CLI Debug Tool**
   ```bash
   # Test explain-coverage command
   debtmap explain-coverage \
     --file src/cook/commit_tracker.rs \
     --function create_auto_commit \
     --line 413 \
     --coverage-path target/coverage/lcov.info

   # Expected output:
   # Searching for: src/cook/commit_tracker.rs:413:create_auto_commit
   #
   # Strategy 1 (Exact): No match
   # Strategy 2 (Line): MATCH (prodigy::cook::CommitTracker::create_auto_commit)
   #   - Execution count: 3
   #   - Coverage: 100%
   #
   # Final result: 100% coverage (matched via line strategy)
   ```

2. **End-to-End Test**
   ```bash
   # Run debtmap on Prodigy codebase
   cd ../prodigy
   debtmap analyze src/ --coverage-path target/coverage/lcov.info

   # Verify: Top 10 items should NOT show [ERROR UNTESTED] for tested functions
   # False positive rate should be <10%
   ```

## Documentation Requirements

### Code Documentation

1. **Inline Documentation**
   - Document `NormalizedFunctionName` struct and its purpose
   - Explain each matching strategy in `find_function_in_array()`
   - Add examples to `normalize_demangled_name()` showing different input patterns
   - Document why line-based matching is preferred over method name matching

2. **Module Documentation**
   - Update `src/risk/lcov.rs` module docs to explain matching behavior
   - Document the order of matching strategies and rationale
   - Provide troubleshooting guidance for coverage matching failures

### User Documentation

1. **Coverage Integration Guide** (`book/src/coverage-integration.md`)
   ```markdown
   ## LCOV Function Name Matching

   Debtmap uses multiple strategies to match function names between your
   source code and LCOV coverage data:

   1. **Exact Match**: Function name matches exactly
   2. **Line Match**: Function at the same line number (most reliable)
   3. **Method Name Match**: Matches just the method name (ignores module path)
   4. **Suffix Match**: One name is a suffix of the other

   ### Troubleshooting Coverage Detection

   If functions show [ERROR UNTESTED] despite having coverage:

   1. Verify LCOV file contains the function:
      ```bash
      grep -i "function_name" target/coverage/lcov.info
      ```

   2. Use the explain-coverage command:
      ```bash
      debtmap explain-coverage --file src/file.rs \
        --function function_name --line 123 \
        --coverage-path target/coverage/lcov.info
      ```

   3. Check for path mismatches between LCOV and source files
   ```

2. **CLI Reference** (`book/src/cli-reference.md`)
   ```markdown
   ## explain-coverage

   Debug coverage detection for a specific function.

   **Usage**:
   ```bash
   debtmap explain-coverage [OPTIONS] --file <FILE> --function <NAME>
   ```

   **Options**:
   - `--file <FILE>`: Source file path
   - `--function <NAME>`: Function name
   - `--line <LINE>`: Line number (optional, improves matching)
   - `--coverage-path <PATH>`: Path to LCOV file

   **Example**:
   ```bash
   debtmap explain-coverage \
     --file src/lib.rs \
     --function calculate_total \
     --line 42 \
     --coverage-path coverage.lcov
   ```
   ```

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## Coverage Integration

### LCOV Function Name Matching

Debtmap handles Rust's name mangling by:

1. **Demangling**: Using `rustc-demangle` to convert mangled symbols
2. **Normalization**: Extracting both full paths and method names
3. **Multi-strategy Matching**: Trying exact, line, method, and suffix matches

This ensures coverage detection works even when:
- Function names are fully-qualified (`Struct::method`)
- Trait implementations are involved (`<S as T>::method`)
- Generics and monomorphization create multiple symbol variants

See `src/risk/lcov.rs` for implementation details.
```

## Implementation Notes

### Critical Edge Cases

1. **Multiple functions with same name at different lines**
   - **Solution**: Line-based matching takes priority
   - **Example**: `new()` constructors for multiple structs in same file

2. **Trait methods with same name as inherent methods**
   - **Solution**: Use line number as tie-breaker
   - **Example**: Both `impl Foo` and `impl Display for Foo` have `format()`

3. **Generic monomorphization creating multiple LCOV entries**
   - **Solution**: Aggregate coverage across all monomorphized versions
   - **Example**: `Vec<i32>::push` and `Vec<String>::push` both map to `Vec::push`

4. **Closures and async blocks**
   - **Solution**: Match by line number only (names are compiler-generated)
   - **Example**: `{{closure}}` at line 123

### Performance Considerations

1. **Normalization Caching**:
   - Results are cached in `FunctionCoverage.normalized` field
   - Computed once during LCOV parsing, not on every lookup

2. **Parallel Processing**:
   - Maintain existing `rayon` parallelization
   - Normalization happens per-function in parallel

3. **Memory Usage**:
   - `NormalizedFunctionName` adds ~100 bytes per function
   - For 10,000 functions: ~1MB additional memory
   - Acceptable trade-off for correctness

### Gotchas and Best Practices

1. **Don't match on demangled names containing `{{`**
   - These are compiler-generated (closures, async blocks)
   - Use line-based matching only

2. **Preserve original name for debugging**
   - Store in `NormalizedFunctionName.original`
   - Essential for troubleshooting matching failures

3. **Log matching strategy used**
   - Helps identify patterns in matching failures
   - Useful for future optimizations

4. **Prefer false negatives over false positives**
   - Better to show "needs testing" for tested code
   - Than to show "tested" for untested code

## Migration and Compatibility

### Breaking Changes

**None** - This is a bug fix that improves existing functionality without changing APIs.

### Migration Requirements

**None** - Changes are internal to LCOV parsing and matching logic.

### Compatibility Considerations

1. **LCOV File Formats**
   - Must continue supporting both mangled and demangled function names
   - Must work with LCOV from cargo-tarpaulin, cargo-llvm-cov, grcov

2. **Language Support**
   - Rust: Enhanced matching (primary benefit)
   - Python: Existing matching continues to work
   - JavaScript/TypeScript: Existing matching continues to work
   - No regression in non-Rust language support

3. **Serialization**
   - `NormalizedFunctionName` uses `#[serde(skip)]`
   - No impact on JSON output format
   - Coverage data remains same structure

### Rollback Plan

If issues are discovered after deployment:
1. Feature can be disabled via config flag: `enhanced_lcov_matching: false`
2. Falls back to original suffix-only matching
3. No data migration required (normalization happens at runtime)

## Success Metrics

### Quantitative Metrics

1. **False Positive Rate**: <10% (down from >50%)
2. **Coverage Detection Accuracy**: >95% for functions with LCOV data
3. **Performance**: <5% slowdown in LCOV parsing
4. **Prodigy Regression Test**: All 10 flagged functions show correct coverage

### Qualitative Metrics

1. **User Feedback**: Reduced reports of false `[ERROR UNTESTED]` flags
2. **Debug Tool Usage**: Positive feedback on `explain-coverage` utility
3. **Documentation Clarity**: Users can self-diagnose coverage issues

### Validation Criteria

- [ ] Prodigy analysis shows <10% false positive rate
- [ ] No performance regression in benchmark suite
- [ ] All integration tests pass with real LCOV data
- [ ] Documentation reviewed and approved
- [ ] CLI debug tool works as expected

## Implementation Phases

### Phase 1: Core Fix (Week 1) - CRITICAL

1. Implement `NormalizedFunctionName` struct
2. Update `normalize_demangled_name()` to extract method names
3. Enhance `find_function_in_array()` with new strategies
4. Add unit tests for normalization and matching
5. Integration test with Prodigy LCOV data

**Success Criteria**: Prodigy top 10 functions no longer show false untested flags

### Phase 2: Debug Tools (Week 1) - HIGH

1. Implement `explain-coverage` CLI command
2. Add logging to matching strategies
3. Create troubleshooting documentation
4. User acceptance testing

**Success Criteria**: Users can self-diagnose coverage matching issues

### Phase 3: Refinement (Week 2) - MEDIUM

1. Performance benchmarking and optimization
2. Edge case handling (closures, generics)
3. Additional integration tests
4. Documentation polish

**Success Criteria**: All acceptance criteria met, documentation complete

## Related Issues

- Bug report: `debtmap-test-detection-issue.md`
- LCOV path matching bugs: `tests/lcov_coverage_matching_bug_test.rs`, `tests/lcov_path_mismatch_test.rs`
- Coverage propagation: `src/priority/coverage_propagation.rs` (Spec 120)

## Future Enhancements

1. **Coverage Tool Integration**: Support cargo-llvm-cov JSON format (more structured than LCOV)
2. **Call Graph Enhancement**: Use demangled names for better call graph construction
3. **Symbol Deduplication**: Aggregate coverage across generic monomorphizations
4. **IDE Integration**: LSP server showing coverage inline in editor
