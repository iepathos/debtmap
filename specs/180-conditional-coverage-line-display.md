---
number: 180
title: Conditional Coverage Line Display Based on LCOV Availability
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-17
---

# Specification 180: Conditional Coverage Line Display Based on LCOV Availability

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Currently, debtmap displays a coverage line in its output for every recommendation, regardless of whether LCOV coverage data was provided via the `--lcov` flag. When no LCOV data is provided, each item shows:

```
├─ COVERAGE: no coverage data
```

This creates visual noise and confusion for users who are not using coverage analysis. The coverage line should only appear when LCOV data has been provided, making the output cleaner and more contextually relevant.

**Current behavior**:
- Without `--lcov`: Shows "├─ COVERAGE: no coverage data" for every item
- With `--lcov`: Shows "├─ COVERAGE: X.X% coverage" or "├─ COVERAGE: no coverage data" (when specific function not found)

**Desired behavior**:
- Without `--lcov`: No coverage line displayed at all
- With `--lcov`: Shows coverage percentage or "no coverage data" when function not found in LCOV

## Objective

Eliminate the coverage line from debtmap output when no LCOV coverage data has been provided, improving output clarity and reducing visual clutter for users not performing coverage analysis.

## Requirements

### Functional Requirements

1. **Conditional Display Logic**
   - When `--lcov` flag is NOT provided: Skip coverage line entirely
   - When `--lcov` flag IS provided: Display coverage line with appropriate data
   - Preserve existing "no coverage data" message when LCOV provided but specific function not found

2. **Context Propagation**
   - Track whether LCOV data was provided at the analysis level
   - Pass this context through to the formatter functions
   - Make decision based on LCOV availability, not just absence of coverage for specific item

3. **Backward Compatibility**
   - Preserve all existing coverage display behavior when `--lcov` is provided
   - Maintain verbosity level handling for detailed coverage analysis (verbosity >= 2)
   - No changes to LCOV parsing or matching logic

### Non-Functional Requirements

- **Performance**: No measurable performance impact (pure boolean check)
- **Maintainability**: Clear separation between "no LCOV provided" vs "function not found in LCOV"
- **Consistency**: Apply same logic across all output verbosity levels
- **Testing**: Existing coverage-related tests should still pass

## Acceptance Criteria

- [ ] Running `debtmap analyze .` without `--lcov` shows no coverage lines
- [ ] Running `debtmap analyze . --lcov target/coverage/lcov.info` shows coverage lines with percentages
- [ ] Functions not found in LCOV still show "no coverage data" when LCOV was provided
- [ ] All verbosity levels (0-3) respect the conditional display logic
- [ ] Existing integration tests with LCOV continue to pass
- [ ] No regression in coverage calculation or display accuracy
- [ ] Documentation updated to reflect new behavior

## Technical Details

### Implementation Approach

**Phase 1: Context Flag Addition**

Add boolean flag to track LCOV availability in `UnifiedAnalysis`:

```rust
// In src/builders/unified_analysis.rs or appropriate location
pub struct UnifiedAnalysis {
    pub items: Vec<UnifiedDebtItem>,
    pub has_coverage_data: bool,  // NEW: tracks if LCOV was provided
    // ... existing fields
}
```

Set this flag during analysis initialization:
```rust
impl UnifiedAnalysis {
    pub fn new(has_lcov: bool) -> Self {
        Self {
            items: Vec::new(),
            has_coverage_data: has_lcov,
            // ... initialize other fields
        }
    }
}
```

**Phase 2: Formatter Function Update**

Modify `format_coverage_section()` in `src/priority/formatter_verbosity.rs:834`:

```rust
fn format_coverage_section(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &ColoredFormatter,
    verbosity: u8,
    tree_pipe: &str,
    has_coverage_data: bool,  // NEW parameter
) {
    // Skip entire coverage section if no LCOV data was provided
    if !has_coverage_data {
        return;
    }

    // Rest of existing logic unchanged
    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;

        // Always show simple coverage percentage line
        writeln!(
            output,
            "├─ {}: {:.1}% coverage",
            "COVERAGE".bright_blue(),
            coverage_pct
        )
        .unwrap();

        // For verbosity >= 2, show detailed analysis with test recommendations
        if coverage_pct < 100.0 && !trans_cov.uncovered_lines.is_empty() && verbosity >= 2 {
            format_detailed_coverage_analysis(output, trans_cov, item, _formatter, tree_pipe);
        }
    } else {
        // No coverage data available for this specific function
        // But LCOV was provided, so show this explicitly
        writeln!(
            output,
            "├─ {}: no coverage data",
            "COVERAGE".bright_blue()
        )
        .unwrap();
    }
}
```

**Phase 3: Call Site Updates**

Update all call sites of `format_coverage_section()` to pass the `has_coverage_data` flag:

```rust
// In format_unified_item() or similar
format_coverage_section(
    output,
    item,
    formatter,
    verbosity,
    tree_pipe,
    analysis.has_coverage_data,  // Pass flag from analysis context
);
```

**Phase 4: Analysis Builder Update**

Update the analysis builder to propagate LCOV availability:

```rust
// In build_unified_analysis() or equivalent
pub fn build_unified_analysis(
    config: &Config,
    coverage_index: Option<&CoverageIndex>,
    // ... other params
) -> UnifiedAnalysis {
    let has_coverage_data = coverage_index.is_some();

    let mut analysis = UnifiedAnalysis::new(has_coverage_data);

    // ... rest of building logic

    analysis
}
```

### Architecture Changes

**Modified Components**:
- `UnifiedAnalysis` struct - Add `has_coverage_data: bool` field
- `format_coverage_section()` - Add conditional return at the beginning
- `format_unified_item()` - Pass `has_coverage_data` to coverage section formatter
- Analysis builders - Propagate LCOV availability flag

**No New Components**: This is a pure refactoring to conditionally display existing output.

### Data Structures

```rust
// Extension to UnifiedAnalysis
pub struct UnifiedAnalysis {
    pub items: Vec<UnifiedDebtItem>,
    pub has_coverage_data: bool,  // NEW: true if --lcov was provided
    // ... existing fields
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/builders/unified_analysis.rs` - Analysis struct and builder
  - `src/priority/formatter_verbosity.rs` - Coverage display logic
  - `src/priority/formatter.rs` - May need similar updates
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_format_coverage_section_without_lcov() {
    let mut output = String::new();
    let item = create_test_item_with_no_coverage();
    let formatter = ColoredFormatter::new();

    format_coverage_section(
        &mut output,
        &item,
        &formatter,
        1,
        "├─",
        false,  // no LCOV data provided
    );

    // Should be empty - no coverage line
    assert_eq!(output, "");
}

#[test]
fn test_format_coverage_section_with_lcov_but_no_match() {
    let mut output = String::new();
    let item = create_test_item_with_no_coverage();
    let formatter = ColoredFormatter::new();

    format_coverage_section(
        &mut output,
        &item,
        &formatter,
        1,
        "├─",
        true,  // LCOV data was provided
    );

    // Should show "no coverage data" because LCOV was provided but function not found
    assert!(output.contains("no coverage data"));
}

#[test]
fn test_format_coverage_section_with_lcov_and_coverage() {
    let mut output = String::new();
    let item = create_test_item_with_coverage(75.5);
    let formatter = ColoredFormatter::new();

    format_coverage_section(
        &mut output,
        &item,
        &formatter,
        1,
        "├─",
        true,  // LCOV data was provided
    );

    // Should show coverage percentage
    assert!(output.contains("75.5% coverage"));
}
```

### Integration Tests

Create test to verify end-to-end behavior:

```rust
#[test]
fn test_output_without_lcov_has_no_coverage_lines() {
    let config = Config {
        path: PathBuf::from("tests/fixtures/sample_project"),
        lcov_path: None,  // No LCOV provided
        verbosity: 1,
        ..Default::default()
    };

    let output = run_analysis(&config);

    // Should not contain any coverage lines
    assert!(!output.contains("COVERAGE:"));
    assert!(!output.contains("no coverage data"));
}

#[test]
fn test_output_with_lcov_shows_coverage_lines() {
    let config = Config {
        path: PathBuf::from("tests/fixtures/sample_project"),
        lcov_path: Some(PathBuf::from("tests/fixtures/sample.lcov")),
        verbosity: 1,
        ..Default::default()
    };

    let output = run_analysis(&config);

    // Should contain coverage lines
    assert!(output.contains("COVERAGE:"));
}
```

### Manual Testing

1. **Without LCOV**:
   ```bash
   debtmap analyze .
   # Verify: No "COVERAGE:" lines in output
   ```

2. **With LCOV**:
   ```bash
   debtmap analyze . --lcov target/coverage/lcov.info
   # Verify: "COVERAGE:" lines present with percentages
   ```

3. **Various Verbosity Levels**:
   ```bash
   debtmap analyze . -v 0
   debtmap analyze . -v 1
   debtmap analyze . -v 2
   debtmap analyze . -v 3
   # All should have no coverage lines without --lcov

   debtmap analyze . --lcov target/coverage/lcov.info -v 2
   # Should show detailed coverage analysis
   ```

### Regression Testing

- Run full test suite to ensure no breakage
- Verify all existing LCOV integration tests still pass
- Check coverage-related benchmarks for performance stability

## Documentation Requirements

### Code Documentation

```rust
/// Formats the coverage section of a debt item recommendation.
///
/// # Coverage Line Display Logic
///
/// - If `has_coverage_data` is false (no --lcov provided): Returns immediately, no output
/// - If `has_coverage_data` is true:
///   - If item has coverage data: Shows "X.X% coverage"
///   - If item has no coverage data: Shows "no coverage data" (function not found in LCOV)
///
/// # Parameters
///
/// - `has_coverage_data`: Whether LCOV file was provided via --lcov flag
/// - Other params: standard formatting parameters
fn format_coverage_section(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &ColoredFormatter,
    verbosity: u8,
    tree_pipe: &str,
    has_coverage_data: bool,
) {
    // ...
}
```

### User Documentation

Update README or user guide:

```markdown
## Coverage Analysis

Debtmap can incorporate code coverage data to prioritize untested code.

### Usage

```bash
# Without coverage - clean output focused on complexity
debtmap analyze .

# With coverage - includes coverage percentages in output
debtmap analyze . --lcov target/coverage/lcov.info
```

### Output Format

**Without `--lcov`**: Coverage lines are omitted entirely for cleaner output.

**With `--lcov`**: Each recommendation includes a coverage line:
- `├─ COVERAGE: 75.5% coverage` - Function has coverage data
- `├─ COVERAGE: no coverage data` - Function not found in LCOV file
```

## Implementation Notes

### Design Decisions

1. **Early Return vs Conditional Branch**: Using early return in `format_coverage_section()` makes the intent clearer and reduces nesting.

2. **Flag Location**: Storing `has_coverage_data` in `UnifiedAnalysis` ensures it's available throughout the formatting pipeline without threading through every function.

3. **Preserve "no coverage data" Message**: When LCOV is provided but a function isn't found, we still show "no coverage data" to help users understand their coverage gaps.

### Gotchas

1. **Multiple Formatters**: Ensure all formatter variants (standard, verbose, etc.) respect the flag
2. **JSON Output**: If JSON output includes coverage, ensure it also respects this flag
3. **Testing Fixtures**: Update test fixtures that expect coverage lines without LCOV

### Best Practices

- Keep the conditional check at the function entry point for clarity
- Preserve all existing behavior when LCOV is provided
- Document the distinction between "no LCOV" vs "function not in LCOV"

## Migration and Compatibility

### Breaking Changes

**None** - This is purely a display change that reduces output in a specific scenario. All functionality remains the same.

### Compatibility Considerations

- Users relying on parsing "no coverage data" in scripts may need updates if they don't use `--lcov`
- However, this is unlikely as the message is meaningless without LCOV data
- JSON output format unchanged (only affects human-readable text output)

### Migration Path

1. Deploy updated debtmap
2. Verify output is cleaner without `--lcov` flag
3. Confirm coverage analysis still works correctly with `--lcov`
4. Update any scripts that parse debtmap output (if necessary)

## Success Metrics

- [ ] Output without `--lcov` has zero "COVERAGE:" lines
- [ ] Output with `--lcov` unchanged from current behavior
- [ ] User feedback confirms improved output clarity
- [ ] No regression in coverage analysis functionality
- [ ] All tests pass, including existing coverage tests

## Example Output Comparison

### Before (without --lcov)

```
#1 SCORE: 40.5 [CRITICAL]
├─ LOCATION: ./src/cli/commands/resume.rs:568 execute_mapreduce_resume()
├─ IMPACT: -10 complexity, -8.9 risk
├─ COMPLEXITY: cyclomatic=20, cognitive=80, nesting=6
├─ COVERAGE: no coverage data
├─ WHY THIS MATTERS: Deep nesting drives cognitive complexity to 80.
├─ RECOMMENDED ACTION: Reduce nesting from 6 to 2 levels
```

### After (without --lcov)

```
#1 SCORE: 40.5 [CRITICAL]
├─ LOCATION: ./src/cli/commands/resume.rs:568 execute_mapreduce_resume()
├─ IMPACT: -10 complexity, -8.9 risk
├─ COMPLEXITY: cyclomatic=20, cognitive=80, nesting=6
├─ WHY THIS MATTERS: Deep nesting drives cognitive complexity to 80.
├─ RECOMMENDED ACTION: Reduce nesting from 6 to 2 levels
```

### With --lcov (unchanged)

```
#1 SCORE: 17.7 [CRITICAL]
├─ LOCATION: ./src/cook/workflow/resume.rs:824 ResumeExecutor::execute_remaining_steps()
├─ IMPACT: +50% function coverage, -4 complexity, -7.4 risk
├─ COMPLEXITY: cyclomatic=15, cognitive=65, nesting=4
├─ COVERAGE: 0.0% coverage
├─ WHY THIS MATTERS: Function has 0% coverage with complexity 15/65.
├─ RECOMMENDED ACTION: Add 8 tests for untested branches
```

## Related Specifications

- **Spec 179**: LCOV Generic Function Matching - Improves coverage matching accuracy
- **Spec 108**: File and Pattern Exclusion - Another output clarity improvement
- **Spec 165**: Classification Confidence - Related display enhancement
