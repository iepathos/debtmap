---
number: 206
title: Refactor formatter_markdown.rs God Object
category: refactor
priority: medium
status: draft
dependencies: [202, 203, 204, 205]
created: 2025-12-02
---

# Specification 206: Refactor formatter_markdown.rs God Object

**Category**: refactor
**Priority**: medium
**Status**: draft
**Dependencies**: [202, 203, 204, 205]

## Context

`src/priority/formatter_markdown.rs` is the **second God Object** in the formatter system with similar problems to formatter.rs:

- **2,889 lines** in a single file
- **Duplicate severity classification** with different thresholds (fixed by spec 202)
- **Mixed I/O and logic** violating Pure Core principle
- **No modular structure** making navigation difficult
- **Duplicate functionality** with terminal formatter

### Current Structure

```
src/priority/formatter_markdown.rs (2,889 lines)
├── Public API (format_report_markdown)
├── Severity classification (duplicate of formatter.rs, different thresholds)
├── Summary tables (duplicate logic, different format)
├── Recommendations (duplicate logic, markdown syntax)
├── Coverage statistics (duplicate)
├── Pattern display (duplicate)
└── Helper functions (100+ functions)
```

### Problems

1. **Inconsistent Severity**: Uses 9.0/7.0/5.0/3.0 thresholds (terminal uses 8.0/6.0/4.0)
2. **Code Duplication**: ~70% logic duplication with formatter.rs (different output syntax)
3. **Mixed Concerns**: Business logic intertwined with markdown generation
4. **Hard to Test**: Tests require parsing markdown output
5. **Hard to Maintain**: Changes require updates in multiple locations

### Stillwater Violation

Violates **Composition Over Complexity** and **Single Source of Truth**:
- Duplicate classification logic (should use spec 202 modules)
- Duplicate pattern detection (should use spec 204 consolidated system)
- Mixed pure/impure code (should follow spec 203 separation)

## Objective

Refactor formatter_markdown.rs to use **shared classification modules**, **pure formatting functions**, and **modular structure** following specs 202-205, eliminating duplication while maintaining markdown-specific output formatting.

## Requirements

### Functional Requirements

1. **Use Shared Classification** (spec 202)
   - Replace duplicate `get_severity_label()` with `Severity::from_score()`
   - Replace duplicate coverage logic with `CoverageLevel::from_percentage()`
   - Consistent thresholds across all formats

2. **Use Shared Pattern Detection** (spec 204)
   - Use `item.detected_pattern` instead of re-detecting
   - No duplicate pattern detection logic
   - Consistent pattern info across formats

3. **Separate Pure/Impure** (spec 203)
   - Pure `format_markdown_X()` functions return structured data
   - `write_markdown_X()` functions handle markdown syntax
   - Testable without string parsing

4. **Module Structure** (spec 205 pattern)
   ```
   src/priority/formatter_markdown/
   ├── mod.rs (public API, ~50 lines)
   ├── summary.rs (summary tables, ~300 lines)
   ├── recommendations.rs (detailed items, ~400 lines)
   ├── tables.rs (markdown table formatting, ~200 lines)
   ├── statistics.rs (coverage/complexity stats, ~200 lines)
   └── writer.rs (markdown syntax generation, ~150 lines)
   ```

5. **Backward Compatibility**
   - Public API unchanged
   - Markdown output identical (verified by tests)
   - Incremental migration

### Non-Functional Requirements

1. **Maintainability**: Each module under 500 lines with single responsibility
2. **Consistency**: Same classifications as terminal formatter
3. **Testability**: Pure functions testable without markdown parsing
4. **Performance**: No performance regression

## Acceptance Criteria

- [ ] `formatter_markdown.rs` reduced from 2,889 lines to ~50 lines (mod.rs)
- [ ] Duplicate `get_severity_label()` removed (uses spec 202)
- [ ] Duplicate coverage classification removed (uses spec 202)
- [ ] Duplicate pattern detection removed (uses spec 204)
- [ ] `formatter_markdown/` module structure created
- [ ] All tests pass with identical markdown output
- [ ] Severity thresholds match terminal formatter (8.0/6.0/4.0)
- [ ] No file in `formatter_markdown/` exceeds 500 lines
- [ ] 100% backward compatibility verified

## Technical Details

### Implementation Approach

**Stage 1: Migrate to Shared Classification**

```rust
// Before (formatter_markdown.rs:1069)
fn get_severity_label(score: f64) -> &'static str {
    if score >= 9.0 {
        "CRITICAL"
    } else if score >= 7.0 {
        "HIGH"
    } else if score >= 5.0 {
        "MEDIUM"
    } else if score >= 3.0 {
        "LOW"
    } else {
        "MINIMAL"
    }
}

// After
use crate::priority::classification::Severity;

fn get_severity_label(score: f64) -> &'static str {
    Severity::from_score(score).as_str()
}
// Or directly: Severity::from_score(item.unified_score.final_score).as_str()
```

**Stage 2: Create Module Structure**

```rust
// src/priority/formatter_markdown/mod.rs

//! Markdown formatting for priority analysis results

pub mod summary;
pub mod recommendations;
pub mod tables;
pub mod statistics;
mod writer;

use crate::priority::AnalysisResults;

/// Generate markdown report for priority analysis
pub fn format_report_markdown(analysis: &AnalysisResults) -> String {
    let mut output = String::new();

    // Title and metadata
    output.push_str("# Technical Debt Priority Report\n\n");

    // Statistics summary
    output.push_str(&statistics::format_statistics(analysis));

    // Summary table
    output.push_str(&summary::format_summary_table(&analysis.items, analysis.has_coverage_data));

    // Detailed recommendations
    output.push_str(&recommendations::format_recommendations(&analysis.items, analysis.has_coverage_data));

    output
}
```

**Stage 3: Extract Summary Tables**

```rust
// src/priority/formatter_markdown/summary.rs

use crate::priority::classification::{Severity, CoverageLevel};
use crate::priority::UnifiedDebtItem;

/// Format markdown summary table of top priority items
pub fn format_summary_table(items: &[UnifiedDebtItem], has_coverage_data: bool) -> String {
    let mut output = String::from("## Top Priority Items\n\n");

    // Table header
    if has_coverage_data {
        output.push_str("| Rank | Score | Coverage | Severity | Location | Function |\n");
        output.push_str("|------|-------|----------|----------|----------|----------|\n");
    } else {
        output.push_str("| Rank | Score | Severity | Location | Function |\n");
        output.push_str("|------|-------|----------|----------|----------|\n");
    }

    // Table rows
    for (rank, item) in items.iter().enumerate() {
        format_summary_row(&mut output, rank + 1, item, has_coverage_data);
    }

    output.push_str("\n");
    output
}

fn format_summary_row(output: &mut String, rank: usize, item: &UnifiedDebtItem, has_coverage_data: bool) {
    let severity = Severity::from_score(item.unified_score.final_score);

    if has_coverage_data {
        let coverage = if let Some(ref trans) = item.transitive_coverage {
            let pct = trans.direct * 100.0;
            format!("{:.1}%", pct)
        } else {
            "N/A".to_string()
        };

        output.push_str(&format!(
            "| {} | {:.2} | {} | {} | {}:{} | `{}` |\n",
            rank,
            item.unified_score.final_score,
            coverage,
            severity.as_str(),
            item.location.file.display(),
            item.location.line,
            item.location.function
        ));
    } else {
        output.push_str(&format!(
            "| {} | {:.2} | {} | {}:{} | `{}` |\n",
            rank,
            item.unified_score.final_score,
            severity.as_str(),
            item.location.file.display(),
            item.location.line,
            item.location.function
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_table_has_markdown_headers() {
        let items = vec![create_test_item()];
        let output = format_summary_table(&items, true);

        assert!(output.contains("| Rank | Score | Coverage"));
        assert!(output.contains("|------|-------|----------"));
    }

    #[test]
    fn summary_uses_consistent_severity() {
        let item = create_test_item_with_score(8.5);
        let output = format_summary_table(&vec![item], false);

        // Should use 8.0 threshold, not 9.0
        assert!(output.contains("CRITICAL"));
    }
}
```

**Stage 4: Extract Recommendations**

```rust
// src/priority/formatter_markdown/recommendations.rs

use crate::priority::classification::Severity;
use crate::priority::UnifiedDebtItem;

/// Format detailed markdown recommendations
pub fn format_recommendations(items: &[UnifiedDebtItem], has_coverage_data: bool) -> String {
    let mut output = String::from("## Detailed Recommendations\n\n");

    for (rank, item) in items.iter().enumerate() {
        format_single_recommendation(&mut output, rank + 1, item, has_coverage_data);
        output.push_str("\n---\n\n");
    }

    output
}

fn format_single_recommendation(output: &mut String, rank: usize, item: &UnifiedDebtItem, has_coverage_data: bool) {
    let severity = Severity::from_score(item.unified_score.final_score);

    // Header
    output.push_str(&format!("### #{} - {} [{}]\n\n", rank, item.location.function, severity.as_str()));

    // Location
    output.push_str(&format!("**Location**: `{}:{}`\n\n", item.location.file.display(), item.location.line));

    // Score
    output.push_str(&format!("**Score**: {:.2}\n\n", item.unified_score.final_score));

    // Coverage (if available)
    if has_coverage_data {
        if let Some(ref trans) = item.transitive_coverage {
            let pct = trans.direct * 100.0;
            output.push_str(&format!("**Coverage**: {:.1}%\n\n", pct));
        }
    }

    // Pattern (if detected)
    if let Some(ref pattern) = item.detected_pattern {
        let metrics_str = pattern.display_metrics().join(", ");
        output.push_str(&format!(
            "**Pattern**: {} {} ({}, confidence: {:.2})\n\n",
            pattern.icon(),
            pattern.type_name(),
            metrics_str,
            pattern.confidence
        ));
    }

    // Complexity
    if item.cyclomatic_complexity > 0 {
        output.push_str(&format!(
            "**Complexity**: cyclomatic={}, cognitive={}, nesting={}\n\n",
            item.cyclomatic_complexity,
            item.cognitive_complexity,
            item.nesting_depth
        ));
    }

    // Impact
    output.push_str(&format!("**Impact**: {}\n\n", format_impact(&item.expected_impact)));

    // Recommendation
    output.push_str(&format!("**Recommended Action**: {}\n\n", item.recommendation.primary_action));

    // Rationale
    output.push_str(&format!("**Rationale**: {}\n\n", item.recommendation.rationale));
}

fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    format!(
        "Maintainability burden: {:.0}%, Test complexity: {:.0}%, Risk: {:.0}%",
        impact.maintainability_burden * 100.0,
        impact.test_complexity_burden * 100.0,
        impact.downstream_risk * 100.0
    )
}
```

**Stage 5: Extract Statistics**

```rust
// src/priority/formatter_markdown/statistics.rs

use crate::priority::AnalysisResults;
use crate::priority::classification::{Severity, CoverageLevel};

/// Format markdown statistics summary
pub fn format_statistics(analysis: &AnalysisResults) -> String {
    let mut output = String::from("## Analysis Statistics\n\n");

    // Severity breakdown
    let severity_counts = count_by_severity(&analysis.items);
    output.push_str("### Priority Distribution\n\n");
    output.push_str(&format!("- **CRITICAL**: {} items\n", severity_counts.critical));
    output.push_str(&format!("- **HIGH**: {} items\n", severity_counts.high));
    output.push_str(&format!("- **MEDIUM**: {} items\n", severity_counts.medium));
    output.push_str(&format!("- **LOW**: {} items\n\n", severity_counts.low));

    // Coverage statistics (if available)
    if analysis.has_coverage_data {
        let coverage_stats = calculate_coverage_stats(&analysis.items);
        output.push_str("### Coverage Statistics\n\n");
        output.push_str(&format!("- **Average Coverage**: {:.1}%\n", coverage_stats.average));
        output.push_str(&format!("- **Untested Functions**: {}\n", coverage_stats.untested));
        output.push_str(&format!("- **Low Coverage (<20%)**: {}\n\n", coverage_stats.low));
    }

    output
}

struct SeverityCounts {
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
}

fn count_by_severity(items: &[crate::priority::UnifiedDebtItem]) -> SeverityCounts {
    let mut counts = SeverityCounts {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
    };

    for item in items {
        match Severity::from_score(item.unified_score.final_score) {
            Severity::Critical => counts.critical += 1,
            Severity::High => counts.high += 1,
            Severity::Medium => counts.medium += 1,
            Severity::Low => counts.low += 1,
        }
    }

    counts
}

struct CoverageStats {
    average: f64,
    untested: usize,
    low: usize,
}

fn calculate_coverage_stats(items: &[crate::priority::UnifiedDebtItem]) -> CoverageStats {
    let mut total = 0.0;
    let mut count = 0;
    let mut untested = 0;
    let mut low = 0;

    for item in items {
        if let Some(ref trans) = item.transitive_coverage {
            let pct = trans.direct * 100.0;
            total += pct;
            count += 1;

            if pct == 0.0 {
                untested += 1;
            } else if pct < 20.0 {
                low += 1;
            }
        }
    }

    CoverageStats {
        average: if count > 0 { total / count as f64 } else { 0.0 },
        untested,
        low,
    }
}
```

### Architecture Changes

**Before**:
```
src/priority/
└── formatter_markdown.rs (2,889 lines - GOD OBJECT)
    ├── Duplicate severity classification (wrong thresholds)
    ├── Duplicate coverage classification
    ├── Mixed I/O and logic
    └── No module boundaries
```

**After**:
```
src/priority/
├── formatter_markdown/
│   ├── mod.rs (~50 lines - public API)
│   ├── summary.rs (~300 lines - summary tables)
│   ├── recommendations.rs (~400 lines - detailed items)
│   ├── tables.rs (~200 lines - table utilities)
│   ├── statistics.rs (~200 lines - stats summary)
│   └── writer.rs (~150 lines - markdown syntax)
└── classification/ (shared - spec 202)
    ├── severity.rs (single source of truth)
    └── coverage.rs (single source of truth)
```

### Code Reduction

| Component | Before | After | Change |
|-----------|--------|-------|--------|
| Severity classification | Duplicate (~50 lines) | Uses spec 202 | -50 lines |
| Coverage classification | Duplicate (~100 lines) | Uses spec 202 | -100 lines |
| Pattern detection | Duplicate (~80 lines) | Uses spec 204 | -80 lines |
| **Total formatter_markdown** | **2,889 lines** | **1,300 lines** | **-55%** |

## Dependencies

- **Prerequisites**:
  - Spec 202: Severity/Coverage classification (must use, not duplicate)
  - Spec 203: Pure/impure separation pattern
  - Spec 204: Pattern detection (must use, not duplicate)
  - Spec 205: Module structure pattern
- **Affected Components**:
  - `src/priority/formatter_markdown.rs` (becomes directory)
  - All markdown output tests (must pass unchanged)

## Testing Strategy

### Unit Tests

```rust
// Test pure formatting without markdown parsing
#[cfg(test)]
mod tests {
    #[test]
    fn summary_table_uses_consistent_severity() {
        let item = create_test_item_with_score(8.5);
        let output = format_summary_table(&vec![item], false);

        // Must use 8.0 threshold (spec 202), not old 9.0 threshold
        assert!(output.contains("CRITICAL"));
    }

    #[test]
    fn pattern_display_uses_detected_pattern() {
        let mut item = create_test_item();
        item.detected_pattern = Some(create_coordinator_pattern());

        let output = format_recommendations(&vec![item], false);

        // Must use item.detected_pattern (spec 204), not re-detect
        assert!(output.contains("🎯 Coordinator"));
    }
}
```

### Integration Tests

```rust
#[test]
fn markdown_output_unchanged() {
    let analysis = load_test_analysis();

    // Expected markdown (captured before refactor)
    let expected = include_str!("../test_data/expected_markdown.md");

    // Generate markdown using refactored code
    let actual = format_report_markdown(&analysis);

    assert_eq!(actual, expected);
}
```

### Cross-Format Consistency Tests

```rust
#[test]
fn markdown_severity_matches_terminal() {
    let item = create_test_item_with_score(8.5);

    // Both should classify identically
    let terminal_severity = Severity::from_score(item.unified_score.final_score);
    let markdown = format_summary_table(&vec![item.clone()], false);

    assert_eq!(terminal_severity, Severity::Critical);
    assert!(markdown.contains("CRITICAL"));
}
```

## Documentation Requirements

### Module Documentation

```rust
//! # Markdown Formatter
//!
//! Formats technical debt priority analysis as markdown reports.
//!
//! ## Shared Components
//!
//! Uses shared classification modules (spec 202):
//! - `Severity::from_score()` for consistent severity levels
//! - `CoverageLevel::from_percentage()` for consistent coverage labels
//!
//! Uses shared pattern detection (spec 204):
//! - Reads `item.detected_pattern` (no re-detection)
//!
//! ## Module Structure
//!
//! - `summary.rs`: Summary tables
//! - `recommendations.rs`: Detailed items
//! - `statistics.rs`: Statistics summary
//! - `tables.rs`: Table utilities
//! - `writer.rs`: Markdown syntax generation
```

## Implementation Notes

### Key Principles

1. **Don't Duplicate**: Use spec 202 and 204 modules, never duplicate
2. **Consistent Thresholds**: Must match terminal formatter exactly
3. **Modular Structure**: Follow spec 205 pattern
4. **Backward Compatible**: Markdown output must be identical

### Common Pitfalls

- **Don't** copy severity/coverage logic from old code (use spec 202!)
- **Don't** re-detect patterns (use `item.detected_pattern` from spec 204)
- **Don't** mix markdown syntax with business logic
- **Do** verify output is identical after migration

## Migration and Compatibility

### Breaking Changes

None. Pure internal refactoring.

### Migration Path

1. **v0.8.0**: Migrate to spec 202/204 modules
2. **v0.8.1**: Create module structure
3. **v0.8.2**: Extract and test each module
4. **v0.9.0**: Remove old formatter_markdown.rs

## Success Metrics

- **Duplication Eliminated**: 0 duplicate classification implementations
- **Code Reduction**: 2,889 lines → 1,300 lines (-55%)
- **Consistency**: 100% severity/coverage alignment with terminal
- **Maintainability**: All modules under 500 lines

## Timeline

- **Day 1**: Migrate to spec 202/204 modules (4h)
- **Day 2**: Create module structure, extract summary (4h)
- **Day 3**: Extract recommendations (4h)
- **Day 4**: Extract statistics (4h)
- **Day 5**: Testing and verification (4h)

**Total Effort**: 20 hours (2.5 person-days)
