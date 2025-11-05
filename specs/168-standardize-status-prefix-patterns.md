---
number: 168
title: Standardize Status Prefix Patterns Across Output
category: optimization
priority: medium
status: draft
dependencies: [167]
created: 2025-01-05
---

# Specification 168: Standardize Status Prefix Patterns Across Output

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 167 (Fix Redundant Coverage Status Indicators)

## Context

The debtmap output currently uses inconsistent tag formatting patterns across different status indicators:

```
[ERROR] UNTESTED                    - Severity + descriptive status
[WARN] PARTIAL (40.0%)              - Severity + descriptive + percentage
[CRITICAL] CRITICAL                 - Redundant (to be fixed in spec 167)
[OK] GOOD (85.0%)                   - Severity + adjective + percentage
[INFO] MODERATE (60.0%)             - Severity + adjective + percentage
[ERROR UNTESTED]                    - Severity + status (no space)
[WARN LOW COVERAGE]                 - Severity + multi-word description
```

This inconsistency makes the output harder to scan and understand. Users must mentally parse different tag formats to extract the same type of information.

## Objective

Establish and implement a consistent tag format pattern across all status indicators in debtmap output, making it easier for users to quickly understand severity and state at a glance.

## Requirements

### Functional Requirements

1. **Define Standard Tag Format**
   - Choose one consistent pattern for all status tags
   - Support severity levels: `[ERROR]`, `[WARN]`, `[INFO]`, `[OK]`, `[CRITICAL]`
   - Support descriptive states: UNTESTED, LOW, PARTIAL, MODERATE, GOOD, EXCELLENT
   - Handle percentage values consistently

2. **Apply Format Consistently**
   - Update all coverage status indicators
   - Update all gap severity indicators
   - Update header tags
   - Update any other status displays

3. **Maintain Information Density**
   - Don't lose information in standardization
   - Keep percentages where valuable
   - Preserve severity→state→detail hierarchy

### Non-Functional Requirements

1. **Scannability**
   - Users should quickly identify severity level
   - Pattern should be learnable and predictable
   - Visual structure should guide eye to important information

2. **Consistency**
   - Same information should always appear in same format
   - No exceptions or special cases
   - Works across all verbosity levels

## Acceptance Criteria

- [ ] Single tag format pattern defined and documented
- [ ] All status tags follow the pattern consistently
- [ ] Coverage status tags standardized
- [ ] Gap severity tags standardized
- [ ] Header tags standardized
- [ ] Tests verify format consistency
- [ ] User documentation explains tag format
- [ ] Style guide documents standard for future additions

## Technical Details

### Proposed Standard Format

**Option 1: Severity + State (with optional percentage)**
```
[SEVERITY STATE (percentage)]
```

Examples:
- `[ERROR UNTESTED]` - No percentage needed
- `[WARN LOW (19.2%)]` - Coverage state with percentage
- `[WARN PARTIAL (40.0%)]` - Partial coverage with percentage
- `[OK GOOD (85.0%)]` - Good coverage with percentage
- `[OK EXCELLENT (98.5%)]` - Excellent coverage with percentage
- `[CRITICAL]` - Gap severity (no state needed)

**Option 2: Separated Tags (severity and state separate)**
```
[SEVERITY] STATE (percentage)
```

Examples:
- `[ERROR] UNTESTED`
- `[WARN] LOW (19.2%)`
- `[WARN] PARTIAL (40.0%)`
- `[OK] GOOD (85.0%)`
- `[CRITICAL]` - Gap severity

**Recommendation**: **Option 1** - Single bracket with all information together
- More compact
- Clear semantic grouping
- Easier to parse visually
- Consistent bracket usage

### Tag Components Breakdown

**Severity Levels** (fixed vocabulary):
- `ERROR` - Critical issues requiring immediate attention
- `WARN` - Issues that should be addressed
- `INFO` - Informational status
- `OK` - Acceptable state
- `CRITICAL` - Highest severity (for gaps)

**Coverage States** (fixed vocabulary):
- `UNTESTED` - 0% coverage
- `LOW` - <20% coverage
- `PARTIAL` - 20-50% coverage
- `MODERATE` - 50-80% coverage
- `GOOD` - 80-95% coverage
- `EXCELLENT` - ≥95% coverage

**Gap Severities** (fixed vocabulary):
- `LOW` - 0-25% gap
- `MODERATE` - 26-50% gap
- `HIGH` - 51-75% gap
- `CRITICAL` - 76-100% gap (standalone tag)

### Architecture Changes

**Files to Modify:**

1. `src/priority/formatter_verbosity.rs`
   - `format_coverage_status()` (line 34-43)
   - `get_gap_severity_indicator()` (line 78-85)
   - `format_coverage_factor_description()` (line 46-75)
   - Any other status formatters

2. `src/priority/formatter/sections.rs`
   - Header section formatting
   - Any status tag generation

3. **New Module** (Optional): `src/formatting/status_tags.rs`
   ```rust
   /// Standard tag formatter for consistent status display
   pub struct StatusTag {
       severity: SeverityLevel,
       state: Option<String>,
       percentage: Option<f64>,
   }

   impl StatusTag {
       pub fn format(&self) -> String {
           match (self.state.as_ref(), self.percentage) {
               (Some(state), Some(pct)) => format!("[{} {} ({:.1}%)]", self.severity, state, pct),
               (Some(state), None) => format!("[{} {}]", self.severity, state),
               (None, _) => format!("[{}]", self.severity),
           }
       }
   }
   ```

### Implementation Examples

**Before:**
```rust
fn format_coverage_status(coverage_pct: f64) -> String {
    match coverage_pct {
        0.0 => "[ERROR] UNTESTED".to_string(),
        c if c < 20.0 => format!("[WARN] LOW ({:.1}%)", c),
        c if c < 50.0 => format!("[WARN] PARTIAL ({:.1}%)", c),
        c if c < 80.0 => format!("[INFO] MODERATE ({:.1}%)", c),
        c if c < 95.0 => format!("[OK] GOOD ({:.1}%)", c),
        _ => format!("[OK] EXCELLENT ({:.1}%)", coverage_pct),
    }
}
```

**After (Option 1):**
```rust
fn format_coverage_status(coverage_pct: f64) -> String {
    match coverage_pct {
        0.0 => "[ERROR UNTESTED]".to_string(),
        c if c < 20.0 => format!("[WARN LOW ({:.1}%)]", c),
        c if c < 50.0 => format!("[WARN PARTIAL ({:.1}%)]", c),
        c if c < 80.0 => format!("[INFO MODERATE ({:.1}%)]", c),
        c if c < 95.0 => format!("[OK GOOD ({:.1}%)]", c),
        _ => format!("[OK EXCELLENT ({:.1}%)]", coverage_pct),
    }
}
```

**After (Option 2 - Keep Current Style):**
No change needed if we choose to keep `[SEVERITY] STATE` pattern.

### Decision Matrix

| Criterion | Option 1 (Single Tag) | Option 2 (Separated) | Winner |
|-----------|----------------------|----------------------|--------|
| Compactness | ✓ More compact | ✗ More verbose | Option 1 |
| Clarity | ✓ Clear grouping | ✓ Clear structure | Tie |
| Parseability | ✓ Easier regex | ✗ Requires multipart parse | Option 1 |
| Existing Use | ✗ New pattern | ✓ Partially exists | Option 2 |
| Consistency | ✓ Always brackets | ✓ Brackets for severity | Tie |

**Recommendation**: **Option 2** (Keep and standardize `[SEVERITY] STATE` pattern)
- Already partially in use
- Less disruptive change
- Still achieves consistency
- Easier migration path

## Dependencies

**Prerequisites:**
- Spec 167: Fix Redundant Coverage Status Indicators (should be completed first)

**Affected Components:**
- All status formatters
- Coverage display functions
- Header formatting
- Test output expectations

**External Dependencies:**
- None

## Testing Strategy

### Unit Tests

1. **Coverage Status Format Test**
   ```rust
   #[test]
   fn test_coverage_status_format_consistency() {
       let cases = vec![
           (0.0, "[ERROR] UNTESTED"),
           (10.0, "[WARN] LOW (10.0%)"),
           (40.0, "[WARN] PARTIAL (40.0%)"),
           (70.0, "[INFO] MODERATE (70.0%)"),
           (90.0, "[OK] GOOD (90.0%)"),
           (98.0, "[OK] EXCELLENT (98.0%)"),
       ];

       for (coverage, expected) in cases {
           let formatted = format_coverage_status(coverage);
           assert_eq!(formatted, expected, "Coverage {} should format as {}", coverage, expected);

           // Verify pattern consistency
           assert!(formatted.starts_with('['));
           assert!(formatted.contains(']'));
       }
   }
   ```

2. **Pattern Consistency Test**
   ```rust
   #[test]
   fn test_all_tags_follow_pattern() {
       // Test that all tag formatters follow the same pattern
       let tags = vec![
           format_coverage_status(0.0),
           format_coverage_status(50.0),
           get_gap_severity_indicator(50.0),
           // Add all other tag generators
       ];

       for tag in tags {
           // Verify all follow [SEVERITY STATE?] or [SEVERITY] pattern
           assert!(tag.starts_with('['));
           let close_bracket = tag.find(']').expect("Tag should have closing bracket");
           // Verify no nested brackets or inconsistent formatting
       }
   }
   ```

### Integration Tests

1. **End-to-End Consistency Test**
   - Run debtmap on real codebase
   - Parse all status tags from output
   - Verify all follow same pattern
   - No mixed formatting

2. **Visual Consistency Test**
   - Generate sample output with all tag types
   - Manual review for visual consistency
   - Ensure tags are scannable and aligned

## Documentation Requirements

### Code Documentation

1. **Style Guide** (New file: `docs/formatting-style-guide.md`)
   ```markdown
   # Formatting Style Guide

   ## Status Tags

   All status tags follow the pattern: `[SEVERITY] STATE (PERCENTAGE?)`

   ### Severity Levels
   - `[ERROR]` - Critical issues
   - `[WARN]` - Warnings
   - `[INFO]` - Informational
   - `[OK]` - Acceptable
   - `[CRITICAL]` - Highest severity

   ### Coverage States
   - `UNTESTED` - 0%
   - `LOW` - <20%
   - `PARTIAL` - 20-50%
   - `MODERATE` - 50-80%
   - `GOOD` - 80-95%
   - `EXCELLENT` - ≥95%

   ### Examples
   - `[ERROR] UNTESTED` - Zero coverage
   - `[WARN] LOW (15.5%)` - Low coverage
   - `[OK] GOOD (87.2%)` - Good coverage
   ```

2. **Function Documentation**
   ```rust
   /// Formats coverage percentage as standardized status tag.
   ///
   /// Returns a tag following the pattern: `[SEVERITY] STATE (percentage)`
   /// where:
   /// - SEVERITY: ERROR, WARN, INFO, or OK
   /// - STATE: UNTESTED, LOW, PARTIAL, MODERATE, GOOD, or EXCELLENT
   /// - percentage: Shown for all states except UNTESTED
   ///
   /// # Examples
   /// ```
   /// assert_eq!(format_coverage_status(0.0), "[ERROR] UNTESTED");
   /// assert_eq!(format_coverage_status(50.0), "[WARN] PARTIAL (50.0%)");
   /// ```
   fn format_coverage_status(coverage_pct: f64) -> String
   ```

### User Documentation

1. **Output Guide** (`book/src/understanding-output.md`)
   ```markdown
   ## Understanding Status Tags

   Debtmap uses standardized status tags throughout its output:

   ### Format
   All tags follow the pattern: `[SEVERITY] STATE (percentage)`

   ### Coverage Status Tags
   - `[ERROR] UNTESTED` - No test coverage (0%)
   - `[WARN] LOW (15%)` - Low coverage (<20%)
   - `[WARN] PARTIAL (45%)` - Partial coverage (20-50%)
   - `[INFO] MODERATE (65%)` - Moderate coverage (50-80%)
   - `[OK] GOOD (87%)` - Good coverage (80-95%)
   - `[OK] EXCELLENT (98%)` - Excellent coverage (≥95%)

   The severity level indicates urgency:
   - ERROR: Immediate attention needed
   - WARN: Should be addressed
   - INFO: Informational
   - OK: Acceptable state
   ```

## Implementation Notes

### Migration Strategy

1. **Phase 1: Standardize Coverage Tags** (This spec)
   - Update `format_coverage_status()`
   - Update `get_gap_severity_indicator()`
   - Update tests

2. **Phase 2: Standardize Header Tags** (Future)
   - Update header formatting in sections.rs
   - Ensure consistency with coverage tags

3. **Phase 3: Create Style Guide** (Future)
   - Document standard for future additions
   - Create linting rules to enforce

### Color Coding

Maintain color consistency with tag formatting:
```rust
match severity {
    "ERROR" | "CRITICAL" => tag.bright_red().bold(),
    "WARN" => tag.yellow(),
    "INFO" => tag.cyan(),
    "OK" => tag.green(),
}
```

### Percentage Formatting

- Always use one decimal place: `{:.1}%`
- Include space before percentage: `STATE (45.5%)`
- Closing bracket after percentage: `STATE (45.5%)]`

## Migration and Compatibility

### Breaking Changes

- Tag format changes may break text parsers
- JSON output structure unchanged
- Regex patterns may need updates

### Migration Path

Same as Spec 167 - coordinate release together.

## Related Issues

This spec addresses:
- **Issue #2**: Inconsistent Status Prefix Patterns
- **User Impact**: Harder to scan and parse output
- **Solution**: Single consistent tag format pattern
