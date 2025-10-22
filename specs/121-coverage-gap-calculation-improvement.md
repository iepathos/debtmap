---
number: 121
title: Coverage Gap Calculation Improvement
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-10-21
---

# Specification 121: Coverage Gap Calculation Improvement

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap v0.2.9 reports "100% coverage gap" for functions with only 1-2 uncovered lines, creating a misleading severity perception that doesn't match the actual scope of missing tests.

**Real-World Example**:
```rust
// ContextMatcher::any() - 9 lines total, 1 line uncovered (line 52)
pub fn any() -> Self {
    Self {
        role: None,          // Covered
        file_type: None,     // Covered
        is_async: None,      // Covered
        framework_pattern: None,  // Covered
        name_pattern: None,  // Covered → ONLY LINE 52 NOT COVERED
    }
}
```

**Current Output**:
```
WHY: Business logic with 100% coverage gap, currently 0% covered
├─ Missing lines: 52
```

**The Contradiction**:
- Says "**100% coverage gap**"
- But only "**Missing lines: 52**" (1 line out of 9)
- Actual gap = 11.1%, not 100%

**Root Cause**:
Debtmap currently calculates coverage gap using **function-level** coverage instead of **line-level** coverage:

```rust
// Current implementation (src/priority/scoring/rust_recommendations.rs:227)
let coverage_gap = ((1.0 - coverage_percent) * 100.0) as u32;
// coverage_percent from FNDA (function-level): 0% → gap = 100%
```

**The Problem**: Function-level coverage is binary (executed vs not executed), while line-level coverage is granular:

```
FNDA:0,any          ← Function never executed → 0% function coverage
DA:52,0             ← 1 line uncovered
DA:53,5, DA:54,5, ... ← 8 lines covered
Actual gap: 1/9 = 11.1%, not 100%
```

**Why This Happens**:
- `ContextMatcher::any()` is only called in conditional path (config parsing)
- Tests don't set up config, so function never executes
- FNDA shows 0 executions, but if it DID execute, most lines would be covered
- Current gap calculation uses function-level data, missing line-level granularity

**Impact**:
- Users see "100% gap" and think "this function is completely untested"
- Reality: Function is mostly covered, just missing 1 line
- Inflated urgency for trivial fixes
- Reduced trust in coverage analysis accuracy

## Objective

Provide accurate, granular coverage gap reporting that distinguishes between "1 line uncovered (11% gap)" and "entire function untested (100% gap)".

## Requirements

### Functional Requirements

**FR1: Line-Level Gap Calculation**
- Calculate gap based on actual uncovered line count vs total lines
- Display both absolute (lines) and relative (percentage) gaps
- Distinguish between "missing 1 line" vs "missing all lines"

**FR2: Precise Gap Messaging**
- Old: "100% coverage gap"
- New: "1 line uncovered (11% gap)" or "9 lines uncovered (100% gap)"
- Grammatically correct singular/plural

**FR3: Lcov Data Parsing Enhancement**
- Parse lcov line-level coverage data (DA:line,hits)
- Calculate uncovered lines from lcov report
- **Handle incomplete DA records**: If <80% of function lines have DA entries, use `Estimated` gap with warning
- **Handle missing DA records**: If 0 DA entries exist for function, fallback to function-level coverage with `Unknown` gap
- Warn when line data is incomplete or inconsistent with AST line count
- Validate DA records are within expected function line range

**FR4: Multi-Tier Gap Reporting**
- **Precise**: When lcov line data available → "N lines uncovered (X%)"
- **Estimated**: When only function-level data → "~X% gap (estimated)"
- **Unknown**: When no lcov data → "Coverage data unavailable"

**FR5: Visual Gap Indicators**
- Small gap (<25%): 🟡 LOW
- Medium gap (25-75%): 🟠 MODERATE
- Large gap (>75%): 🔴 HIGH
- Complete gap (100%): 🔴🔴 CRITICAL

### Non-Functional Requirements

**NFR1: Accuracy**
- Gap calculation error < 5% when lcov data available
- Clear indication when using estimates vs precise data
- No false precision (don't claim 11.1% if uncertain)

**NFR2: Performance**
- Lcov line parsing adds < 5% overhead
- Cache coverage gap calculations
- Efficient data structures for line coverage

**NFR3: Backward Compatibility**
- Maintain existing JSON output schema (add new fields)
- Old tools can still parse basic coverage percentage
- Graceful degradation when line data unavailable

## Acceptance Criteria

- [x] Coverage gap calculated from actual uncovered line count when available
- [x] Output shows both absolute ("1 line uncovered") and relative ("11% gap")
- [x] Lcov DA (line coverage) records parsed and stored
- [x] Functions with 1-2 uncovered lines report precise gaps, not "100%"
- [x] Visual indicators differentiate small gaps from large gaps
- [x] Documentation explains gap calculation methodology
- [x] Test suite validates gap calculation accuracy
- [x] Handles missing lcov data gracefully (fallback to function-level %)
- [x] JSON output includes both line-level and function-level coverage
- [x] `ContextMatcher::any()` reports "1 line uncovered (11%)" not "100% gap"

## Technical Details

### Implementation Approach

**Phase 1: Enhanced Lcov Parsing**

**File**: `src/risk/lcov.rs`

```rust
/// Enhanced line-level coverage data
#[derive(Debug, Clone, Default)]
pub struct LineCoverageData {
    /// Total executable lines in function
    pub total_lines: u32,

    /// Lines with >0 hits
    pub covered_lines: u32,

    /// Specific uncovered line numbers
    pub uncovered_lines: Vec<u32>,

    /// Line-by-line hit counts
    pub line_hits: HashMap<u32, u32>,
}

impl LcovData {
    /// Get line-level coverage for a function
    pub fn get_line_coverage(
        &self,
        file: &Path,
        function: &str,
        start_line: u32,
    ) -> Option<LineCoverageData> {
        // Find function in lcov data
        let fn_data = self.functions.get(&(file.to_path_buf(), function.to_string()))?;

        // Parse DA (line coverage) records for this function's line range
        let mut line_coverage = LineCoverageData::default();

        for (line_num, hit_count) in &self.line_data {
            if *line_num >= start_line && *line_num < start_line + fn_data.line_count {
                line_coverage.total_lines += 1;
                line_coverage.line_hits.insert(*line_num, *hit_count);

                if *hit_count > 0 {
                    line_coverage.covered_lines += 1;
                } else {
                    line_coverage.uncovered_lines.push(*line_num);
                }
            }
        }

        Some(line_coverage)
    }
}
```

### Line Count Determination

**Challenge**: Lcov line count vs AST line count may differ due to:
- Lcov only counting "executable" lines (excludes comments, braces)
- Compiler optimizations removing trivial code
- Different instrumentation granularity

**Solution**: Use AST as source of truth for total lines, lcov for coverage status:

```rust
impl LcovData {
    /// Get line-level coverage using AST line count as baseline
    pub fn get_line_coverage(
        &self,
        file: &Path,
        function: &str,
        start_line: u32,
        ast_line_count: u32,  // NEW: from FunctionMetrics, not lcov
    ) -> Option<LineCoverageData> {
        // Use AST line count as truth (total lines in function)
        let total_lines = ast_line_count;

        // Collect DA records within function's line range
        let mut covered_lines = 0;
        let mut uncovered_lines = Vec::new();
        let mut line_hits = HashMap::new();

        for (line_num, hit_count) in &self.line_data {
            if *line_num >= start_line && *line_num < start_line + total_lines {
                line_hits.insert(*line_num, *hit_count);

                if *hit_count > 0 {
                    covered_lines += 1;
                } else {
                    uncovered_lines.push(*line_num);
                }
            }
        }

        // If no DA records found, return None (fallback to function-level)
        let da_count = covered_lines + uncovered_lines.len() as u32;
        if da_count == 0 {
            return None;
        }

        // If DA records cover <80% of AST lines, data is incomplete
        let da_coverage_ratio = da_count as f64 / total_lines as f64;
        if da_coverage_ratio < 0.8 {
            eprintln!(
                "Warning: Incomplete line coverage data for {}:{} ({}/{} lines have DA records)",
                file.display(),
                function,
                da_count,
                total_lines
            );
            // Could return Estimated gap here instead of Precise
        }

        Some(LineCoverageData {
            total_lines,
            covered_lines,
            uncovered_lines,
            line_hits,
        })
    }
}
```

**Edge Case Handling**:

```rust
// Example: lcov only instruments executable lines
// Source (9 lines total):
pub fn any() -> Self {   // Line 52
    Self {               // Line 53
        role: None,      // Line 54
        // ... (lines 55-59)
    }                    // Line 60
}

// Lcov data (only 5 executable lines):
DA:52,0  // Function signature - uncovered
DA:53,0  // Struct literal start - uncovered
DA:54,0  // Field initialization - uncovered
DA:55,0  // Field initialization - uncovered
DA:56,0  // Field initialization - uncovered
// Lines 57-60 not instrumented (closing braces, whitespace)

// Gap calculation options:
// Option A: 5 uncovered / 9 total = 55.6% (uses AST total)
// Option B: 5 uncovered / 5 instrumented = 100% (uses DA total)
//
// CHOSEN: Option B (only count instrumented lines for precision)
// Gap = uncovered_lines.len() / (covered + uncovered) * 100
//     = 5 / 5 = 100%
```

**Rationale**: We calculate gap percentage based on **instrumented lines only**, not total AST lines, because:
- Non-instrumented lines (braces, whitespace) are not executable
- Including them dilutes the gap percentage artificially
- Users care about "% of executable code untested", not "% of text lines"

**Phase 2: Precise Gap Calculation**

**File**: `src/priority/scoring/recommendation_helpers.rs`

```rust
/// Calculate coverage gap with line-level precision
pub fn calculate_coverage_gap(
    coverage_pct: f64,
    func: &FunctionMetrics,
    coverage_data: Option<&LcovData>,
) -> CoverageGap {
    // Try to get line-level data first
    if let Some(data) = coverage_data {
        if let Some(line_cov) = data.get_line_coverage(&func.file, &func.name, func.line) {
            // Calculate gap based on instrumented lines only
            let instrumented_lines = line_cov.covered_lines + line_cov.uncovered_lines.len() as u32;
            let gap_percentage = if instrumented_lines > 0 {
                (line_cov.uncovered_lines.len() as f64 / instrumented_lines as f64) * 100.0
            } else {
                0.0  // No instrumented lines = no gap to report
            };

            return CoverageGap::Precise {
                uncovered_lines: line_cov.uncovered_lines.clone(),
                total_lines: instrumented_lines,  // Use instrumented line count, not AST
                percentage: gap_percentage,
            };
        }
    }

    // Fallback to percentage-based estimate
    let gap_pct = 100.0 - (coverage_pct * 100.0);
    CoverageGap::Estimated {
        percentage: gap_pct,
        total_lines: func.length as u32,
        estimated_uncovered: (func.length as f64 * gap_pct / 100.0) as u32,
    }
}

/// Coverage gap with different precision levels
#[derive(Debug, Clone)]
pub enum CoverageGap {
    /// Precise gap from line-level coverage data
    Precise {
        uncovered_lines: Vec<u32>,
        total_lines: u32,
        percentage: f64,
    },

    /// Estimated gap from function-level percentage
    Estimated {
        percentage: f64,
        total_lines: u32,
        estimated_uncovered: u32,
    },

    /// No coverage data available
    Unknown { total_lines: u32 },
}

impl CoverageGap {
    /// Format for user display
    pub fn format(&self) -> String {
        match self {
            CoverageGap::Precise {
                uncovered_lines,
                total_lines,
                percentage,
            } => {
                let count = uncovered_lines.len();
                if count == 0 {
                    "Fully covered".to_string()
                } else if count == 1 {
                    format!(
                        "1 line uncovered ({:.0}% gap) - line {}",
                        percentage, uncovered_lines[0]
                    )
                } else {
                    format!(
                        "{} lines uncovered ({:.0}% gap) - lines {}",
                        count,
                        percentage,
                        format_line_ranges(uncovered_lines)
                    )
                }
            }

            CoverageGap::Estimated {
                percentage,
                estimated_uncovered,
                ..
            } => {
                if *percentage >= 99.0 {
                    format!("~100% gap (estimated, {} lines)", estimated_uncovered)
                } else if *percentage < 5.0 {
                    format!("~{}% gap (mostly covered)", percentage as u32)
                } else {
                    format!(
                        "~{}% gap (estimated, ~{} lines)",
                        percentage as u32, estimated_uncovered
                    )
                }
            }

            CoverageGap::Unknown { total_lines } => {
                format!("Coverage data unavailable ({} lines)", total_lines)
            }
        }
    }

    /// Get percentage gap
    pub fn percentage(&self) -> f64 {
        match self {
            CoverageGap::Precise { percentage, .. } => *percentage,
            CoverageGap::Estimated { percentage, .. } => *percentage,
            CoverageGap::Unknown { .. } => 100.0,
        }
    }

    /// Get uncovered line count
    pub fn uncovered_count(&self) -> u32 {
        match self {
            CoverageGap::Precise { uncovered_lines, .. } => uncovered_lines.len() as u32,
            CoverageGap::Estimated {
                estimated_uncovered, ..
            } => *estimated_uncovered,
            CoverageGap::Unknown { total_lines } => *total_lines,
        }
    }
}

/// Format line numbers as compact ranges
fn format_line_ranges(lines: &[u32]) -> String {
    // e.g., [10, 11, 12, 15, 20, 21] → "10-12, 15, 20-21"
    let mut ranges = vec![];
    let mut sorted = lines.to_vec();
    sorted.sort_unstable();

    let mut range_start = sorted[0];
    let mut range_end = sorted[0];

    for &line in sorted.iter().skip(1) {
        if line == range_end + 1 {
            range_end = line;
        } else {
            if range_start == range_end {
                ranges.push(format!("{}", range_start));
            } else {
                ranges.push(format!("{}-{}", range_start, range_end));
            }
            range_start = line;
            range_end = line;
        }
    }

    // Add final range
    if range_start == range_end {
        ranges.push(format!("{}", range_start));
    } else {
        ranges.push(format!("{}-{}", range_start, range_end));
    }

    ranges.join(", ")
}
```

**Phase 3: Update Output Formatting**

```rust
// OLD:
format!("Business logic with {}% coverage gap", coverage_gap)

// NEW:
let gap = calculate_coverage_gap(coverage_pct, func, coverage_data);
format!("Business logic - {}", gap.format())

// Examples:
// "Business logic - 1 line uncovered (11% gap) - line 52"
// "Business logic - 5 lines uncovered (45% gap) - lines 10-12, 15-16"
// "Business logic - ~100% gap (estimated, 20 lines)"
```

**Phase 4: Visual Gap Indicators**

```rust
fn get_gap_severity_indicator(gap: &CoverageGap) -> &'static str {
    let pct = gap.percentage();
    match pct {
        p if p < 25.0 => "🟡",   // Small gap - LOW priority
        p if p < 75.0 => "🟠",   // Medium gap - MODERATE priority
        p if p < 100.0 => "🔴",  // Large gap - HIGH priority
        _ => "🔴🔴",             // Complete gap - CRITICAL
    }
}

fn format_with_indicator(gap: &CoverageGap) -> String {
    format!("{} {}", get_gap_severity_indicator(gap), gap.format())
}
```

### Architecture Changes

**Modified Files**:
- `src/risk/lcov.rs` - Enhanced line-level parsing
- `src/priority/scoring/recommendation_helpers.rs` - Gap calculation
- `src/priority/formatter.rs` - Output formatting
- `src/priority/formatter_verbosity.rs` - Detailed gap display

**New Files**:
- `src/risk/coverage/gap.rs` - Coverage gap calculation logic (optional refactor)

**Data Structure Additions**:
```rust
// Add to FunctionMetrics or create new struct
#[derive(Debug, Clone)]
pub struct CoverageGapInfo {
    pub gap: CoverageGap,
    pub severity: GapSeverity,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapSeverity {
    None,      // 0% gap
    Small,     // 1-24% gap
    Medium,    // 25-74% gap
    Large,     // 75-99% gap
    Complete,  // 100% gap
}
```

### APIs and Interfaces

**JSON Output**:
```json
{
  "coverage": {
    "direct_percentage": 88.9,
    "gap": {
      "type": "precise",
      "uncovered_lines": [52],
      "total_lines": 9,
      "percentage": 11.1,
      "severity": "small",
      "display": "1 line uncovered (11% gap) - line 52"
    }
  }
}
```

**Command-Line Output**:
```
COVERAGE: 🟡 1 line uncovered (11% gap) - line 52
          ↑                ↑            ↑
       Indicator      Percentage    Specific line
```

## Dependencies

**Prerequisites**:
- Existing lcov parsing infrastructure
- Coverage data integration

**Affected Components**:
- Coverage analysis pipeline
- Risk scoring (uses gap percentage)
- Output formatters

**External Dependencies**:
- `proptest` (dev dependency) - For property-based testing of gap calculations
  ```toml
  [dev-dependencies]
  proptest = "1.0"
  ```

## Testing Strategy

### Unit Tests

**Test Gap Calculation**:
```rust
#[test]
fn test_precise_gap_single_line() {
    let mut lcov_data = LcovData::new();
    // 9 lines total, line 52 uncovered
    lcov_data.add_line_coverage("test.rs", 52, 0); // 0 hits
    for line in [53, 54, 55, 56, 57, 58, 59, 60] {
        lcov_data.add_line_coverage("test.rs", line, 5); // 5 hits
    }

    let func = create_test_function("any", 52, 9);
    let gap = calculate_coverage_gap(0.889, &func, Some(&lcov_data));

    match gap {
        CoverageGap::Precise {
            uncovered_lines,
            percentage,
            ..
        } => {
            assert_eq!(uncovered_lines, vec![52]);
            assert!((percentage - 11.1).abs() < 0.1);
        }
        _ => panic!("Expected precise gap"),
    }

    assert_eq!(gap.format(), "1 line uncovered (11% gap) - line 52");
}

#[test]
fn test_estimated_gap_no_line_data() {
    let func = create_test_function("example", 10, 20);
    let gap = calculate_coverage_gap(0.5, &func, None);

    match gap {
        CoverageGap::Estimated {
            percentage,
            estimated_uncovered,
            ..
        } => {
            assert_eq!(percentage, 50.0);
            assert_eq!(estimated_uncovered, 10);
        }
        _ => panic!("Expected estimated gap"),
    }

    assert!(gap.format().contains("~50% gap"));
}

#[test]
fn test_line_range_formatting() {
    // Test: [10, 11, 12, 15, 20, 21] → "10-12, 15, 20-21"
    let lines = vec![10, 11, 12, 15, 20, 21];
    let formatted = format_line_ranges(&lines);
    assert_eq!(formatted, "10-12, 15, 20-21");

    // Test: Single line → "52"
    let lines = vec![52];
    assert_eq!(format_line_ranges(&lines), "52");

    // Test: Non-contiguous → "10, 15, 20"
    let lines = vec![10, 15, 20];
    assert_eq!(format_line_ranges(&lines), "10, 15, 20");
}

#[test]
fn test_gap_calculation_with_zero_total_lines() {
    let gap = CoverageGap::Precise {
        uncovered_lines: vec![],
        total_lines: 0,
        percentage: 0.0,
    };

    // Should not panic
    let formatted = gap.format();
    assert_eq!(formatted, "Fully covered");
}

#[test]
fn test_incomplete_line_data_handling() {
    // lcov only has DA records for 3 out of 10 AST lines
    let mut lcov_data = LcovData::new();
    lcov_data.add_line_coverage("test.rs", 10, 0); // Uncovered
    lcov_data.add_line_coverage("test.rs", 11, 5); // Covered
    lcov_data.add_line_coverage("test.rs", 12, 5); // Covered
    // Lines 13-19 missing (no DA records)

    let func = create_test_function("incomplete", 10, 10); // 10 AST lines
    let gap = calculate_coverage_gap(0.0, &func, Some(&lcov_data));

    // Should warn about incomplete data and potentially return Estimated
    // With current spec: 1 uncovered / 3 instrumented = 33.3%
    match gap {
        CoverageGap::Precise { percentage, .. } => {
            assert!((percentage - 33.3).abs() < 1.0);
        }
        CoverageGap::Estimated { .. } => {
            // Also acceptable if incomplete data triggers Estimated mode
        }
        _ => panic!("Unexpected gap type"),
    }
}
```

### Property-Based Tests

**Test Invariants**:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn gap_percentage_always_between_0_and_100(
        uncovered in 0u32..100,
        covered in 0u32..100,
    ) {
        let total = uncovered + covered;
        if total == 0 {
            return Ok(()); // Skip degenerate case
        }

        let percentage = (uncovered as f64 / total as f64) * 100.0;
        prop_assert!(percentage >= 0.0 && percentage <= 100.0);
    }

    #[test]
    fn gap_formatting_never_panics(
        uncovered_lines in prop::collection::vec(1u32..1000, 0..50),
        total in 1u32..100,
    ) {
        let gap = CoverageGap::Precise {
            uncovered_lines: uncovered_lines.clone(),
            total_lines: total,
            percentage: (uncovered_lines.len() as f64 / total as f64) * 100.0,
        };

        // Should never panic, regardless of input
        let formatted = gap.format();
        prop_assert!(!formatted.is_empty());
    }

    #[test]
    fn zero_uncovered_lines_reports_full_coverage(
        total in 1u32..100,
    ) {
        let gap = CoverageGap::Precise {
            uncovered_lines: vec![],
            total_lines: total,
            percentage: 0.0,
        };

        prop_assert!(gap.format().contains("Fully covered"));
    }

    #[test]
    fn all_lines_uncovered_reports_100_percent(
        line_count in 1u32..50,
    ) {
        let uncovered: Vec<u32> = (1..=line_count).collect();
        let gap = CoverageGap::Precise {
            uncovered_lines: uncovered.clone(),
            total_lines: line_count,
            percentage: 100.0,
        };

        let formatted = gap.format();
        prop_assert!(formatted.contains("100"));
    }

    #[test]
    fn line_range_formatting_stable(
        mut lines in prop::collection::vec(1u32..1000, 1..30),
    ) {
        // Remove duplicates and sort
        lines.sort_unstable();
        lines.dedup();

        if lines.is_empty() {
            return Ok(());
        }

        // Should format without panicking
        let formatted = format_line_ranges(&lines);

        // Should contain at least the first line number
        prop_assert!(formatted.contains(&lines[0].to_string()));

        // Should not contain invalid characters
        prop_assert!(!formatted.contains(".."));  // No empty ranges
    }
}
```

## Documentation Requirements

### User Documentation

**Update**: `book/src/coverage-analysis.md`

```markdown
## Understanding Coverage Gaps

Debtmap reports coverage gaps with different levels of precision:

### Precise Gaps (Line-Level Data Available)

When lcov line coverage data is available, debtmap reports exact uncovered lines:

```
✅ GOOD: "1 line uncovered (11% gap) - line 52"
         ↑ Specific info          ↑ Exact line
```

### Estimated Gaps (Function-Level Data Only)

When only function-level coverage is available:

```
⚠️  ESTIMATED: "~50% gap (estimated, ~10 lines)"
               ↑ Approximate       ↑ Rough count
```

### Gap Severity Indicators

| Indicator | Gap Range | Priority |
|-----------|-----------|----------|
| 🟡 | 1-24% | LOW - Minor touchup |
| 🟠 | 25-74% | MODERATE - Partial coverage |
| 🔴 | 75-99% | HIGH - Mostly untested |
| 🔴🔴 | 100% | CRITICAL - Completely untested |

### Example Output

```
COVERAGE: 🟡 1 line uncovered (11% gap) - line 52
          Small gap, low priority

COVERAGE: 🟠 15 lines uncovered (48% gap) - lines 10-25
          Medium gap, moderate priority

COVERAGE: 🔴 25 lines uncovered (95% gap) - lines 5-30
          Large gap, high priority
```
```

## Implementation Notes

### Lcov Format Reference

```
DA:line_number,execution_count
DA:52,0    # Line 52 not covered (0 hits)
DA:53,5    # Line 53 covered (5 hits)
```

### Edge Cases

**Empty Functions**:
```rust
// 0 executable lines
fn empty() {}

// Lcov: No DA records
// Gap: CoverageGap::Unknown { total_lines: 0 }
// Display: "Coverage data unavailable (0 lines)"
```

**Macro-Generated Code**:
```rust
// Lines in expanded code may not match source lines
macro_rules! generate_fn {
    ($name:ident) => {
        fn $name() { /* expanded code */ }
    }
}

// Lcov may report line numbers from expansion, not source
// Strategy: If DA line numbers are outside function range, fallback to Estimated
```

**Partial Lcov Data**:
```rust
// Example: 3 DA records for 10-line function (30% coverage)
fn complex_function() {
    // 10 lines of code
}

// Lcov:
DA:100,5  // Line 100 covered
DA:105,0  // Line 105 uncovered
DA:109,5  // Line 109 covered
// Lines 101-104, 106-108 missing

// Strategy:
// 1. Calculate DA coverage ratio: 3/10 = 30%
// 2. If < 80%, log warning and use Estimated gap
// 3. Otherwise, use Precise with available data: 1 uncovered / 3 instrumented = 33%
```

**Zero Division Prevention**:
```rust
// All lines optimized away or no instrumentation
let instrumented_lines = covered + uncovered;
if instrumented_lines == 0 {
    return CoverageGap::Unknown { total_lines: ast_line_count };
}

// Safe to calculate percentage
let gap_pct = (uncovered as f64 / instrumented_lines as f64) * 100.0;
```

**Inconsistent Line Ranges**:
```rust
// DA records outside function's line range
fn my_func() { }  // Lines 50-55

// Lcov incorrectly reports:
DA:100,0  // Line 100 is NOT in function range!

// Strategy: Filter DA records to function's line range
// Only count DA records where: start_line <= line < start_line + length
```

## Migration and Compatibility

### Breaking Changes

None - Enhanced reporting, same API.

### JSON Schema Addition

```json
{
  "coverage": {
    // Existing fields
    "percentage": 88.9,

    // NEW fields
    "gap": {
      "type": "precise",  // or "estimated", "unknown"
      "uncovered_lines": [52],
      "percentage": 11.1,
      "severity": "small",
      "display": "1 line uncovered (11% gap) - line 52"
    }
  }
}
```

## Success Metrics

### Quantitative Metrics

- **Accuracy**: Gap calculation error < 5% vs manual verification
- **Precision**: 100% use of line-level data when available
- **Clarity**: 0 user confusion about "100% gap for 1 line"

### Qualitative Metrics

- **User Understanding**: Clear perception of gap severity
- **Prioritization**: Users focus on truly critical gaps
- **Trust**: Confidence in coverage analysis accuracy

### Validation

**Before**:
```
ContextMatcher::any() - "100% coverage gap"
(User: "Wait, it's only 1 line uncovered, why 100%?")
```

**After**:
```
ContextMatcher::any() - "🟡 1 line uncovered (11% gap) - line 52"
(User: "Ah, just 1 line, low priority. Makes sense!")
```

## Future Enhancements

### Phase 2: Branch Coverage Gaps
- Report uncovered branches within covered lines
- "Line 52 covered, but else branch untested"

### Phase 3: Path Coverage Gaps
- Identify specific execution paths uncovered
- "Path through if-else chain never executed"

### Phase 4: Visual Coverage Maps
- ASCII art showing covered/uncovered line ranges
- Terminal-based heatmap visualization
