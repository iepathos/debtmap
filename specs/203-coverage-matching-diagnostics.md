---
number: 203
title: Coverage Matching Diagnostics and Integration
category: testing
priority: high
status: draft
dependencies: [201, 202]
created: 2025-12-06
---

# Specification 203: Coverage Matching Diagnostics and Integration

**Category**: testing
**Priority**: high
**Status**: draft
**Dependencies**: Spec 201 (Path Normalization), Spec 202 (Function Name Matching)

## Context

With robust path normalization (Spec 201) and enhanced function name matching (Spec 202) in place, we need to:

1. **Integrate** these improvements into the coverage matching pipeline
2. **Eliminate** `Cov:N/A` false negatives by always returning `0%` when LCOV data is provided
3. **Diagnose** matching failures to help users understand and fix coverage issues
4. **Monitor** matching success rates to validate improvements

Currently, when coverage matching fails, functions show `Cov:N/A` with no explanation of why. Users don't know if:
- The path didn't match
- The function name didn't match
- The coverage file is incomplete
- There's a real bug in matching logic

## Objective

Complete the coverage matching improvements by:
- Integrating path normalization and function name matching
- Changing `None` → `Some(0.0)` to eliminate `Cov:N/A` when LCOV is provided
- Adding diagnostic mode to debug matching failures
- Providing validation tools for coverage configuration

## Requirements

### Functional Requirements

**FR1**: Integrate Spec 201 and Spec 202 into coverage pipeline
- Use `find_matching_path()` in all coverage lookups
- Use `find_matching_function()` after path matching
- Compose matchers in order of specificity
- Report match strategy and confidence

**FR2**: Eliminate `Cov:N/A` when coverage file provided
- Return `Some(0.0)` instead of `None` when LCOV exists
- Distinguish "no coverage file" from "0% coverage"
- Preserve `None` only when `--coverage-file` not provided
- Update TUI to show `Cov:0%` instead of `Cov:N/A`

**FR3**: Add diagnostic mode for debugging matches
- `DEBTMAP_COVERAGE_DEBUG=1` environment variable
- Log every match attempt with path/name
- Show which strategy succeeded/failed
- Report confidence levels
- Aggregate statistics at end

**FR4**: Provide coverage validation command
- `debtmap diagnose-coverage <lcov>` subcommand
- Analyze LCOV file for common issues
- Report match success rate
- List all functions with `Cov:0%` and why
- Suggest fixes for common problems

### Non-Functional Requirements

**NFR1**: **Zero Regression** - All existing tests must pass
- Maintain backward compatibility
- Don't break existing coverage workflows
- Preserve performance characteristics

**NFR2**: **Observable** - Users can debug issues
- Clear diagnostic messages
- Structured logging for automation
- JSON output option for tooling

**NFR3**: **Testable** - Validate improvements work
- Integration tests with real LCOV files
- Test fixtures from multiple coverage tools
- Regression tests for reported issues

## Acceptance Criteria

- [ ] Path and name matching integrated into `get_function_coverage_with_bounds()`
  - Calls `find_matching_path()` first
  - Then calls `find_matching_function()` on matched path
  - Reports full matching diagnostic trail

- [ ] `Cov:N/A` eliminated when LCOV provided
  - `transitive_coverage` is `Some(TransitiveCoverage { direct: 0.0, ... })` not `None`
  - TUI shows `Cov:0%` for unmatched functions
  - `None` only when `--coverage-file` not provided

- [ ] Diagnostic mode functional
  - `DEBTMAP_COVERAGE_DEBUG=1 debtmap analyze` shows detailed logs
  - Each match attempt logged with path, function, result
  - Statistics summarized at end
  - Example output:
    ```
    [COVERAGE] Attempting match for src/lib.rs::parse_file
    [COVERAGE]   Path match: ✓ Strategy=QuerySuffix
    [COVERAGE]   Function match: ✓ Confidence=High (exact)
    [COVERAGE]   Result: 85.5%

    [COVERAGE] Match Statistics:
    [COVERAGE]   Total functions: 247
    [COVERAGE]   Matched: 245 (99.2%)
    [COVERAGE]   Unmatched (0%): 2 (0.8%)
    ```

- [ ] `diagnose-coverage` subcommand works
  - `debtmap diagnose-coverage coverage.lcov` runs successfully
  - Reports LCOV file statistics (files, functions, overall coverage)
  - Validates file format and warns about issues
  - Shows sample functions and paths for verification

- [ ] Integration tests with real coverage tools
  - Fixtures from `cargo-tarpaulin` (Rust)
  - Fixtures from `llvm-cov` (Rust/LLVM)
  - Fixtures from `kcov` (alternative tool)
  - All fixtures achieve >95% match rate

- [ ] Performance maintained or improved
  - No regression in matching performance
  - Diagnostic mode adds <10% overhead
  - Caching used for repeated matches

- [ ] Documentation complete
  - User guide for coverage troubleshooting
  - Developer guide for matching architecture
  - Examples of diagnostic output

## Technical Details

### Implementation Approach

**Integration Architecture**:

```rust
// ============================================================================
// INTEGRATION: Compose path and function matching
// ============================================================================

use crate::risk::path_normalization::{find_matching_path, MatchStrategy as PathStrategy};
use crate::risk::function_name_matching::{find_matching_function, MatchConfidence};

/// Complete coverage match result with full diagnostic trail
#[derive(Debug, Clone)]
pub struct CoverageMatchResult {
    /// Coverage percentage if found
    pub coverage: Option<f64>,
    /// Path matching result
    pub path_match: PathMatchDiagnostic,
    /// Function matching result (if path matched)
    pub function_match: Option<FunctionMatchDiagnostic>,
    /// Overall diagnostic message
    pub diagnostic: String,
}

#[derive(Debug, Clone)]
pub struct PathMatchDiagnostic {
    pub matched: bool,
    pub strategy: Option<PathStrategy>,
    pub query_path: PathBuf,
    pub matched_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FunctionMatchDiagnostic {
    pub matched: bool,
    pub confidence: MatchConfidence,
    pub query_name: String,
    pub matched_name: Option<String>,
    pub matched_variant: Option<(String, String)>,
}

impl LcovData {
    /// Enhanced coverage lookup with full diagnostics
    pub fn get_function_coverage_with_bounds_diagnostic(
        &self,
        file: &Path,
        function_name: &str,
        start_line: usize,
        end_line: usize,
    ) -> CoverageMatchResult {
        // Phase 1: Path matching
        let available_paths: Vec<PathBuf> = self.functions.keys().cloned().collect();
        let path_result = find_matching_path(file, &available_paths);

        let mut result = CoverageMatchResult {
            coverage: None,
            path_match: PathMatchDiagnostic {
                matched: path_result.is_some(),
                strategy: path_result.as_ref().map(|(_, strategy)| *strategy),
                query_path: file.to_path_buf(),
                matched_path: path_result.as_ref().map(|(path, _)| (*path).clone()),
            },
            function_match: None,
            diagnostic: String::new(),
        };

        // Early return if path doesn't match
        let Some((matched_path, path_strategy)) = path_result else {
            result.diagnostic = format!(
                "Path '{}' not found in LCOV. Searched {} paths.",
                file.display(),
                available_paths.len()
            );

            // CRITICAL: Return 0% not None when LCOV file was provided
            result.coverage = Some(0.0);
            return result;
        };

        // Phase 2: Function matching
        let functions = self.functions.get(matched_path).unwrap();
        let func_result = find_matching_function(function_name, functions);

        result.function_match = Some(FunctionMatchDiagnostic {
            matched: func_result.is_some(),
            confidence: func_result.as_ref().map(|(_, conf)| *conf).unwrap_or(MatchConfidence::None),
            query_name: function_name.to_string(),
            matched_name: func_result.as_ref().map(|(f, _)| f.name.clone()),
            matched_variant: None, // TODO: capture from matching
        });

        if let Some((func, confidence)) = func_result {
            result.coverage = Some(func.coverage_percentage / 100.0);
            result.diagnostic = format!(
                "Matched via path={:?}, function={:?}",
                path_strategy, confidence
            );
        } else {
            // Function not found in matched file - return 0%
            result.coverage = Some(0.0);
            result.diagnostic = format!(
                "Path matched ({:?}) but function '{}' not found in LCOV. File has {} functions.",
                path_strategy,
                function_name,
                functions.len()
            );
        }

        result
    }

    /// Existing interface - uses diagnostic version internally
    pub fn get_function_coverage_with_bounds(
        &self,
        file: &Path,
        function_name: &str,
        start_line: usize,
        end_line: usize,
    ) -> Option<f64> {
        let result = self.get_function_coverage_with_bounds_diagnostic(
            file,
            function_name,
            start_line,
            end_line,
        );

        // Log diagnostic if debug mode enabled
        if std::env::var("DEBTMAP_COVERAGE_DEBUG").is_ok() {
            log_coverage_diagnostic(&result);
        }

        result.coverage
    }
}

/// Log coverage matching diagnostic
fn log_coverage_diagnostic(result: &CoverageMatchResult) {
    let path_status = if result.path_match.matched { "✓" } else { "✗" };
    let func_status = result.function_match.as_ref()
        .map(|f| if f.matched { "✓" } else { "✗" })
        .unwrap_or("—");

    eprintln!(
        "[COVERAGE] {}::{}  Path:{} Func:{}  Coverage: {:.1}%",
        result.path_match.query_path.display(),
        result.function_match.as_ref().map(|f| f.query_name.as_str()).unwrap_or("?"),
        path_status,
        func_status,
        result.coverage.unwrap_or(0.0) * 100.0
    );

    if !result.diagnostic.is_empty() {
        eprintln!("[COVERAGE]   {}", result.diagnostic);
    }
}
```

**Eliminate `Cov:N/A`**:

```rust
// In src/priority/scoring/construction.rs:233-237
// BEFORE:
let transitive_coverage = coverage.and_then(|lcov| {
    lcov.get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line)
        .map(|_| calculate_transitive_coverage(func_id, call_graph, lcov))
})

// AFTER:
let transitive_coverage = coverage.map(|lcov| {
    // When coverage file provided, always return Some(TransitiveCoverage)
    // Use 0.0 as direct coverage if function not found in LCOV
    let direct_coverage = lcov
        .get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line)
        .unwrap_or(0.0);

    // Calculate transitive coverage even when direct is 0.0
    // (callees might have coverage)
    calculate_transitive_coverage(func_id, call_graph, lcov)
});
// Now transitive_coverage is Some(...) when LCOV provided, None only when no --coverage-file
```

**Diagnose Command**:

```rust
// New file: src/commands/diagnose_coverage.rs

use crate::risk::lcov::parse_lcov_file;
use anyhow::Result;
use std::path::Path;

pub fn diagnose_coverage_file(lcov_path: &Path) -> Result<()> {
    println!("Analyzing coverage file: {}", lcov_path.display());
    println!();

    let lcov_data = parse_lcov_file(lcov_path)?;

    // Basic statistics
    let total_files = lcov_data.functions.len();
    let total_functions: usize = lcov_data
        .functions
        .values()
        .map(|funcs| funcs.len())
        .sum();
    let overall_coverage = lcov_data.get_overall_coverage();

    println!("📊 Coverage Statistics:");
    println!("   Files: {}", total_files);
    println!("   Functions: {}", total_functions);
    println!("   Overall Coverage: {:.1}%", overall_coverage);
    println!();

    // File samples
    println!("📁 Sample Paths (first 10):");
    for (i, path) in lcov_data.functions.keys().take(10).enumerate() {
        println!("   {}. {}", i + 1, path.display());
    }
    if total_files > 10 {
        println!("   ... and {} more", total_files - 10);
    }
    println!();

    // Function samples
    println!("🔧 Sample Functions (first 10):");
    for (i, (file, funcs)) in lcov_data.functions.iter().take(10).enumerate() {
        if let Some(func) = funcs.first() {
            println!(
                "   {}. {}::{} ({:.1}%)",
                i + 1,
                file.file_name().unwrap_or_default().to_string_lossy(),
                func.name,
                func.coverage_percentage
            );
        }
    }
    println!();

    // Coverage distribution
    println!("📈 Coverage Distribution:");
    let mut uncovered = 0;
    let mut low = 0;    // 0-50%
    let mut medium = 0; // 50-80%
    let mut high = 0;   // 80-100%

    for funcs in lcov_data.functions.values() {
        for func in funcs {
            if func.coverage_percentage == 0.0 {
                uncovered += 1;
            } else if func.coverage_percentage < 50.0 {
                low += 1;
            } else if func.coverage_percentage < 80.0 {
                medium += 1;
            } else {
                high += 1;
            }
        }
    }

    println!("   Uncovered (0%): {}", uncovered);
    println!("   Low (1-50%): {}", low);
    println!("   Medium (50-80%): {}", medium);
    println!("   High (80-100%): {}", high);
    println!();

    println!("✓ Coverage file appears valid and can be used with debtmap");

    Ok(())
}
```

### Architecture Changes

**Modified Files**:
- `src/risk/lcov.rs` - Add `get_function_coverage_with_bounds_diagnostic()`
- `src/risk/coverage_index.rs` - Integrate path and function matching
- `src/priority/scoring/construction.rs` - Change `None` to `Some(0.0)`
- `src/commands/mod.rs` - Add `diagnose_coverage` command
- `src/cli.rs` - Add `diagnose-coverage` subcommand

**New Files**:
- `src/commands/diagnose_coverage.rs` - Coverage validation tool

## Dependencies

**Prerequisites**:
- Spec 201: Robust Path Normalization
- Spec 202: Enhanced Function Name Matching

**Affected Components**:
- All coverage-dependent scoring and analysis
- TUI display of coverage metrics
- Terminal output formatting

## Testing Strategy

### Integration Tests

**Real Coverage Tool Fixtures**:

```rust
// tests/coverage_tool_integration_test.rs

#[test]
fn test_tarpaulin_generated_lcov_matches() {
    let lcov = parse_lcov_file("tests/fixtures/tarpaulin-coverage.lcov").unwrap();
    let debtmap_results = analyze_project_with_coverage("tests/fixtures/sample-project", &lcov);

    let match_rate = calculate_match_rate(&debtmap_results);
    assert!(
        match_rate > 0.95,
        "Tarpaulin LCOV should have >95% match rate, got {:.1}%",
        match_rate * 100.0
    );
}

#[test]
fn test_llvm_cov_generated_lcov_matches() {
    // Similar test for llvm-cov output
}

#[test]
fn test_no_na_when_lcov_provided() {
    let lcov = parse_lcov_file("tests/fixtures/sample-coverage.lcov").unwrap();
    let debtmap_results = analyze_project_with_coverage("tests/fixtures/sample-project", &lcov);

    for item in &debtmap_results.items {
        assert!(
            item.transitive_coverage.is_some(),
            "All items should have Some(coverage) when LCOV provided, got None for {}::{}",
            item.location.file.display(),
            item.location.function
        );
    }
}
```

**Diagnostic Mode Tests**:

```rust
#[test]
fn test_diagnostic_mode_output() {
    std::env::set_var("DEBTMAP_COVERAGE_DEBUG", "1");

    let output = capture_stderr(|| {
        let lcov = parse_lcov_file("tests/fixtures/sample.lcov").unwrap();
        analyze_with_coverage(&lcov);
    });

    assert!(output.contains("[COVERAGE]"));
    assert!(output.contains("Match Statistics"));

    std::env::remove_var("DEBTMAP_COVERAGE_DEBUG");
}
```

### Regression Tests

**Ensure No Breakage**:

```rust
#[test]
fn test_backward_compatibility_exact_matches() {
    // Existing exact matches should still work
    let lcov = create_simple_lcov();
    let coverage = lcov.get_function_coverage(
        Path::new("src/lib.rs"),
        "exact_name_function"
    );
    assert_eq!(coverage, Some(1.0));
}

#[test]
fn test_none_when_no_coverage_file() {
    // When no --coverage-file provided, should still be None
    let analysis = analyze_without_coverage("tests/fixtures/sample-project");

    for item in &analysis.items {
        assert!(
            item.transitive_coverage.is_none(),
            "Should be None when no coverage file provided"
        );
    }
}
```

## Documentation Requirements

### User Documentation

**Coverage Troubleshooting Guide** (`docs/coverage-troubleshooting.md`):

```markdown
# Coverage Troubleshooting

## Diagnostic Mode

Enable detailed coverage matching diagnostics:

```bash
DEBTMAP_COVERAGE_DEBUG=1 debtmap analyze --coverage-file coverage.lcov
```

This shows every match attempt and statistics.

## Diagnose Coverage File

Validate your LCOV file before analysis:

```bash
debtmap diagnose-coverage coverage.lcov
```

This reports file statistics and potential issues.

## Common Issues

### Cov:0% for functions with coverage

**Cause**: Path or function name mismatch
**Fix**: Run with `DEBTMAP_COVERAGE_DEBUG=1` to see why
**Solutions**:
- Use relative paths consistently
- Check function naming in LCOV vs source
```

### Developer Documentation

**Matching Architecture** (update `ARCHITECTURE.md`):

```markdown
## Coverage Matching Architecture

### Three-Phase Matching

1. **Path Normalization** (Spec 201)
   - Component-based suffix matching
   - Cross-platform path handling

2. **Function Name Matching** (Spec 202)
   - Variant generation (generics, qualifiers)
   - Closure parent attribution
   - Confidence-based selection

3. **Integration** (Spec 203)
   - Compose path + function matching
   - Report diagnostic trail
   - Return 0% not None when LCOV provided

### Diagnostic Pipeline

```
Query (file, function)
  → Path Match (3 strategies)
  → Function Match (exact/variant/fuzzy)
  → Confidence Selection
  → Diagnostic Logging
  → Result (Some(coverage) or Some(0.0))
```
```

## Implementation Notes

### Critical Requirements

1. **Must return `Some(0.0)` not `None`** when LCOV file provided
2. **Must preserve `None`** when no `--coverage-file` argument
3. **Must not regress** existing test coverage
4. **Must be backward compatible** for existing users

### Performance Considerations

- Diagnostic logging only when env var set
- Cache normalized paths and function variants
- Avoid allocations in hot path
- Use structured logging for machine parsing

### Future Enhancements

- Web-based coverage report viewer
- Coverage diff between runs
- Automatic path/name fixup suggestions
- Machine learning for match confidence

## Migration and Compatibility

### Breaking Changes

None - this is backward compatible.

### Behavior Changes

**Visible to Users**:
- `Cov:N/A` becomes `Cov:0%` in TUI when LCOV provided
- New diagnostic mode available via env var
- New `diagnose-coverage` subcommand

**Internal Changes**:
- `transitive_coverage` field changes from `None` to `Some(0.0)`
- Match success rate should improve significantly

### Migration Path

1. Implement Spec 201 (path normalization)
2. Implement Spec 202 (function matching)
3. Implement this spec (integration)
4. Test extensively with real projects
5. Monitor match success rates
6. Tune matching strategies based on data
