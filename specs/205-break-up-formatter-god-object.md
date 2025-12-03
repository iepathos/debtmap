---
number: 205
title: Break Up formatter.rs God Object
category: refactor
priority: high
status: draft
dependencies: [202, 203, 204]
created: 2025-12-02
---

# Specification 205: Break Up formatter.rs God Object

**Category**: refactor
**Priority**: high
**Status**: draft
**Dependencies**: [202, 203, 204]

## Context

`src/priority/formatter.rs` is a **God Object** with severe maintainability problems:

- **3,095 lines** in a single file
- **151 writeln!() calls** mixing I/O with logic
- **Multiple responsibilities**: scoring, formatting, dependencies, coverage, patterns
- **God functions**: `format_prioritized_items()` at 400+ lines
- **No module boundaries**: Everything in one namespace

This violates **Single Responsibility Principle** and makes the code:
- **Hard to test**: Mixed concerns prevent unit testing
- **Hard to navigate**: 3000+ lines to search through
- **Hard to change**: Changes affect unrelated code
- **Hard to review**: Too much context needed

### Current File Structure

```
src/priority/formatter.rs (3,095 lines)
├── Public API (format_prioritized_items, format_summary_items)
├── Severity classification (get_severity_label, get_severity_color)
├── Impact formatting (format_impact)
├── Complexity extraction (extract_complexity_info)
├── Dependency formatting (extract_dependency_info)
├── Coverage formatting (coverage-related helpers)
├── Pattern display (pattern-related helpers)
├── Recommendations display
├── Summary formatting
└── Helper functions (50+ utility functions)
```

### Stillwater Violation

Current code violates **Composition Over Complexity**:
- One massive file instead of composable modules
- No clear module boundaries
- Tight coupling between unrelated features

## Objective

Break up `formatter.rs` into **focused, cohesive modules** with clear responsibilities, following the **Single Responsibility Principle** and **Stillwater's Composition Over Complexity**.

## Requirements

### Functional Requirements

1. **Module Structure**
   ```
   src/priority/formatter/
   ├── mod.rs (public API only, ~50 lines)
   ├── context.rs (already exists, format context)
   ├── sections.rs (already exists, section formatting)
   ├── dependencies.rs (already exists, dependency display)
   ├── summary.rs (summary formatting, ~200 lines)
   ├── recommendations.rs (recommendations display, ~150 lines)
   └── helpers.rs (shared utilities, ~100 lines)
   ```

2. **Clear Responsibilities**
   - `mod.rs`: Public API and high-level orchestration
   - `context.rs`: Data preparation for formatting
   - `sections.rs`: Pure section formatting functions
   - `dependencies.rs`: Dependency filtering and formatting
   - `summary.rs`: Summary table generation
   - `recommendations.rs`: Recommendations list formatting
   - `helpers.rs`: Shared utilities (deprecated, will be removed in spec 203)

3. **Backward Compatibility**
   - All existing public APIs unchanged
   - No breaking changes for users
   - Migrate incrementally with deprecation warnings

4. **File Size Limits**
   - No file over 500 lines
   - Most files under 300 lines
   - Clear module boundaries

### Non-Functional Requirements

1. **Maintainability**: Each module has single, clear purpose
2. **Testability**: Pure functions in each module, easy to test
3. **Navigability**: Find code by responsibility, not by scrolling
4. **Performance**: No performance regression from refactoring

## Acceptance Criteria

- [ ] `src/priority/formatter.rs` reduced from 3,095 lines to ~50 lines
- [ ] `src/priority/formatter/mod.rs` contains public API only
- [ ] `src/priority/formatter/summary.rs` created for summary formatting
- [ ] `src/priority/formatter/recommendations.rs` created for recommendations
- [ ] `src/priority/formatter/helpers.rs` created for shared utilities
- [ ] All existing tests pass with no output changes
- [ ] No file in `formatter/` exceeds 500 lines
- [ ] All public APIs remain unchanged
- [ ] Documentation updated with new module structure

## Technical Details

### Implementation Approach

**Stage 1: Create Module Structure**

```rust
// src/priority/formatter/mod.rs

pub mod context;
pub mod sections;
pub mod dependencies;
pub mod summary;
pub mod recommendations;
mod helpers;

pub use context::create_format_context;
pub use sections::{generate_formatted_sections, apply_formatted_sections};
pub use summary::format_summary_items;
pub use recommendations::format_prioritized_items;

// Public API (unchanged from old formatter.rs)
pub fn format_priorities_terminal(
    analysis: &AnalysisResults,
    config: FormattingConfig,
) -> String {
    recommendations::format_prioritized_items(analysis, config)
}
```

**Stage 2: Extract Summary Formatting**

```rust
// src/priority/formatter/summary.rs

use super::context::FormatContext;
use super::helpers::*;
use crate::formatting::FormattingConfig;
use colored::*;
use std::fmt::Write;

/// Format summary table of top priority items
pub fn format_summary_items(
    items: &[UnifiedDebtItem],
    has_coverage_data: bool,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();

    writeln!(&mut output, "\n{}", "=== TOP PRIORITY ITEMS ===".bright_cyan().bold()).unwrap();
    writeln!(&mut output).unwrap();

    // Table header
    format_summary_header(&mut output, has_coverage_data);

    // Table rows
    for (rank, item) in items.iter().enumerate() {
        format_summary_row(&mut output, rank + 1, item, has_coverage_data);
    }

    output
}

fn format_summary_header(output: &mut String, has_coverage_data: bool) {
    if has_coverage_data {
        writeln!(
            output,
            "{:<6} {:<8} {:<10} {:<12} {:<50}",
            "Rank", "Score", "Coverage", "Severity", "Function"
        ).unwrap();
    } else {
        writeln!(
            output,
            "{:<6} {:<8} {:<12} {:<50}",
            "Rank", "Score", "Severity", "Function"
        ).unwrap();
    }
    writeln!(output, "{}", "─".repeat(80)).unwrap();
}

fn format_summary_row(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    let severity = super::get_severity_label(item.unified_score.final_score);
    let severity_color = super::get_severity_color(item.unified_score.final_score);

    if has_coverage_data {
        let coverage = if let Some(ref trans) = item.transitive_coverage {
            format!("{:.1}%", trans.direct * 100.0)
        } else {
            "N/A".to_string()
        };

        writeln!(
            output,
            "{:<6} {:<8.2} {:<10} {:<12} {}:{}:{}",
            rank,
            item.unified_score.final_score,
            coverage,
            severity.color(severity_color),
            item.location.file.display(),
            item.location.line,
            item.location.function.bright_green()
        ).unwrap();
    } else {
        writeln!(
            output,
            "{:<6} {:<8.2} {:<12} {}:{}:{}",
            rank,
            item.unified_score.final_score,
            severity.color(severity_color),
            item.location.file.display(),
            item.location.line,
            item.location.function.bright_green()
        ).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_table_has_correct_headers() {
        let items = vec![create_test_item()];
        let output = format_summary_items(&items, true, FormattingConfig::default());

        assert!(output.contains("Rank"));
        assert!(output.contains("Score"));
        assert!(output.contains("Coverage"));
        assert!(output.contains("Severity"));
    }

    #[test]
    fn summary_without_coverage_omits_column() {
        let items = vec![create_test_item()];
        let output = format_summary_items(&items, false, FormattingConfig::default());

        assert!(!output.contains("Coverage"));
    }
}
```

**Stage 3: Extract Recommendations Formatting**

```rust
// src/priority/formatter/recommendations.rs

use super::context::create_format_context;
use super::sections::{generate_formatted_sections, apply_formatted_sections};
use crate::formatting::FormattingConfig;
use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

/// Format detailed recommendations for priority items
pub fn format_prioritized_items(
    items: &[UnifiedDebtItem],
    has_coverage_data: bool,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();

    writeln!(&mut output, "\n{}", "=== TOP RECOMMENDATIONS ===".bright_cyan().bold()).unwrap();
    writeln!(&mut output).unwrap();

    for (rank, item) in items.iter().enumerate() {
        format_single_recommendation(&mut output, rank + 1, item, has_coverage_data, config);
        writeln!(&mut output).unwrap(); // Separator between items
    }

    output
}

fn format_single_recommendation(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
    config: FormattingConfig,
) {
    // Create formatting context (pure)
    let context = create_format_context(rank, item, has_coverage_data);

    // Generate formatted sections (pure)
    let sections = generate_formatted_sections(&context);

    // Apply sections to output (I/O)
    apply_formatted_sections(output, sections);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendations_include_all_sections() {
        let items = vec![create_test_item()];
        let output = format_prioritized_items(&items, true, FormattingConfig::default());

        assert!(output.contains("LOCATION:"));
        assert!(output.contains("IMPACT:"));
        assert!(output.contains("RECOMMENDED ACTION:"));
    }

    #[test]
    fn recommendations_show_pattern_when_detected() {
        let mut item = create_test_item();
        item.detected_pattern = Some(create_test_pattern());

        let output = format_prioritized_items(&vec![item], true, FormattingConfig::default());
        assert!(output.contains("PATTERN:"));
    }
}
```

**Stage 4: Extract Helpers**

```rust
// src/priority/formatter/helpers.rs

use colored::Color;

/// Get severity label for score (will be deprecated in spec 203)
#[deprecated(since = "0.8.0", note = "Use Severity::from_score() instead")]
pub fn get_severity_label(score: f64) -> &'static str {
    use crate::priority::classification::Severity;
    Severity::from_score(score).as_str()
}

/// Get severity color for score (will be deprecated in spec 203)
#[deprecated(since = "0.8.0", note = "Use Severity::from_score().color() instead")]
pub fn get_severity_color(score: f64) -> Color {
    use crate::priority::classification::Severity;
    Severity::from_score(score).color()
}

/// Format impact metrics (temporary, will be moved in spec 203)
pub fn format_impact(impact: &ImpactMetrics) -> String {
    format!(
        "Maintainability: {:.0}%, Test complexity: {:.0}%, Risk: {:.0}%",
        impact.maintainability_burden * 100.0,
        impact.test_complexity_burden * 100.0,
        impact.downstream_risk * 100.0
    )
}

/// Extract complexity info from item (temporary, will be removed in spec 203)
pub fn extract_complexity_info(item: &UnifiedDebtItem) -> (u32, u32, u32, u32, usize) {
    (
        item.cyclomatic_complexity,
        item.cognitive_complexity,
        item.estimated_branch_count,
        item.nesting_depth,
        item.function_length,
    )
}

/// Extract dependency info from item (temporary, will be removed in spec 203)
pub fn extract_dependency_info(item: &UnifiedDebtItem) -> (usize, usize) {
    (
        item.upstream_callers.len(),
        item.downstream_callees.len(),
    )
}
```

**Stage 5: Update mod.rs (Public API)**

```rust
// src/priority/formatter/mod.rs

//! Terminal formatting for priority analysis results
//!
//! This module provides formatted output for technical debt priorities,
//! including detailed recommendations and summary tables.

mod context;
mod sections;
mod dependencies;
mod summary;
mod recommendations;
mod helpers;

pub use context::create_format_context;
pub use summary::format_summary_items;
pub use recommendations::format_prioritized_items;

use crate::formatting::FormattingConfig;
use crate::priority::AnalysisResults;

/// Format complete priority analysis for terminal display
pub fn format_priorities_terminal(
    analysis: &AnalysisResults,
    config: FormattingConfig,
) -> String {
    let mut output = String::new();

    // Summary table
    output.push_str(&summary::format_summary_items(
        &analysis.items,
        analysis.has_coverage_data,
        config,
    ));

    // Detailed recommendations
    output.push_str(&recommendations::format_prioritized_items(
        &analysis.items,
        analysis.has_coverage_data,
        config,
    ));

    output
}

// Re-export helpers for backward compatibility (deprecated)
#[deprecated(since = "0.8.0", note = "Use Severity classification instead")]
pub use helpers::{get_severity_label, get_severity_color};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_unchanged() {
        let analysis = create_test_analysis();
        let output = format_priorities_terminal(&analysis, FormattingConfig::default());

        assert!(output.contains("TOP PRIORITY ITEMS"));
        assert!(output.contains("TOP RECOMMENDATIONS"));
    }
}
```

### Architecture Changes

**Before**:
```
src/priority/
├── formatter.rs (3,095 lines - GOD OBJECT)
├── formatter_markdown.rs (2,889 lines)
└── formatter_verbosity.rs (1,087 lines)
```

**After**:
```
src/priority/
├── formatter/
│   ├── mod.rs (~50 lines - public API)
│   ├── context.rs (~286 lines - format context)
│   ├── sections.rs (~410 lines - section formatting)
│   ├── dependencies.rs (~100 lines - dependency display)
│   ├── summary.rs (~200 lines - summary tables)
│   ├── recommendations.rs (~150 lines - recommendations)
│   └── helpers.rs (~100 lines - deprecated utilities)
├── formatter_markdown.rs (2,889 lines - next refactor target)
└── formatter_verbosity.rs (1,087 lines - already modular)
```

### File Size Comparison

| File | Before | After | Reduction |
|------|--------|-------|-----------|
| formatter.rs | 3,095 lines | 50 lines (mod.rs) | -98% |
| Total lines | 3,095 lines | 1,296 lines (split) | -58% |

Note: Lines are redistributed, not removed. Real reduction comes from removing deprecated helpers in spec 203.

## Dependencies

- **Prerequisites**:
  - Spec 202: Establishes classification module pattern
  - Spec 203: Defines pure formatting separation
  - Spec 204: Consolidates pattern detection
- **Affected Components**:
  - `src/priority/formatter.rs` (becomes `formatter/mod.rs`)
  - All imports of `formatter::*` (need updating)
- **Enables**:
  - Easier testing (smaller, focused modules)
  - Parallel development (different modules)
  - Spec 203 pure formatting extraction

## Testing Strategy

### Unit Tests

```rust
// Each module gets focused unit tests

// src/priority/formatter/summary.rs
#[cfg(test)]
mod tests {
    #[test]
    fn summary_table_formatting() {
        let items = vec![create_test_item()];
        let output = format_summary_items(&items, true, FormattingConfig::default());
        assert!(output.contains("Rank"));
    }
}

// src/priority/formatter/recommendations.rs
#[cfg(test)]
mod tests {
    #[test]
    fn recommendations_formatting() {
        let items = vec![create_test_item()];
        let output = format_prioritized_items(&items, true, FormattingConfig::default());
        assert!(output.contains("RECOMMENDED ACTION"));
    }
}
```

### Integration Tests

```rust
#[test]
fn formatter_module_produces_correct_output() {
    let analysis = create_test_analysis();
    let output = format_priorities_terminal(&analysis, FormattingConfig::default());

    // Should contain both summary and recommendations
    assert!(output.contains("TOP PRIORITY ITEMS"));
    assert!(output.contains("TOP RECOMMENDATIONS"));

    // Should format correctly
    assert!(output.contains("Rank"));
    assert!(output.contains("LOCATION:"));
}
```

### Regression Tests

```rust
#[test]
fn output_unchanged_after_refactor() {
    let analysis = load_test_analysis();

    // Output should be identical before/after
    let expected = include_str!("../test_data/expected_formatter_output.txt");
    let actual = format_priorities_terminal(&analysis, FormattingConfig::default());

    assert_eq!(actual, expected);
}
```

## Documentation Requirements

### Module Documentation

```rust
//! # Priority Formatter
//!
//! Formats technical debt priority analysis for terminal display.
//!
//! ## Module Structure
//!
//! - `mod.rs`: Public API and orchestration
//! - `summary.rs`: Summary table generation
//! - `recommendations.rs`: Detailed recommendations
//! - `context.rs`: Format context preparation
//! - `sections.rs`: Section formatting
//! - `dependencies.rs`: Dependency display
//! - `helpers.rs`: Shared utilities (deprecated)
//!
//! ## Usage
//!
//! ```
//! use debtmap::priority::formatter::format_priorities_terminal;
//!
//! let analysis = analyze_codebase()?;
//! let output = format_priorities_terminal(&analysis, config);
//! println!("{}", output);
//! ```
```

### Architecture Updates

Update ARCHITECTURE.md:

```markdown
## Formatter Module Structure

The `priority::formatter` module is organized by responsibility:

- **mod.rs**: Public API only
- **summary.rs**: Summary table formatting
- **recommendations.rs**: Detailed recommendations formatting
- **context.rs**: Format context preparation (pure)
- **sections.rs**: Section formatting (pure)
- **dependencies.rs**: Dependency display logic

Each module is under 500 lines with clear, single responsibility.
```

## Implementation Notes

### Key Principles

1. **Single Responsibility**: Each module does one thing
2. **Small Modules**: Keep files under 500 lines
3. **Clear Boundaries**: Minimize inter-module dependencies
4. **Pure Core**: Separate pure formatting from I/O (spec 203)

### Common Pitfalls

- **Don't** create circular dependencies between modules
- **Don't** move code without understanding its dependencies
- **Do** update all imports after moving code
- **Do** run full test suite after each move

### Migration Strategy

1. Create new module structure (empty files)
2. Move code module by module (start with summary)
3. Update imports incrementally
4. Run tests after each move
5. Deprecate old helpers (remove in spec 203)

## Migration and Compatibility

### Breaking Changes

None. This is internal reorganization with backward-compatible public API.

### Migration Path

1. **v0.8.0**: Create module structure, move code
2. **v0.8.1**: Update all internal imports
3. **v0.8.2**: Deprecate old helper functions
4. **v0.9.0**: Remove deprecated helpers (spec 203)

### Rollback Strategy

If issues are discovered:
1. Revert module structure
2. Keep code in formatter.rs temporarily
3. Fix module boundaries
4. Re-run migration

## Success Metrics

- **File Size**: formatter.rs reduced from 3,095 to 50 lines (-98%)
- **Module Count**: 1 file → 7 focused modules
- **Max File Size**: No file over 500 lines
- **Test Coverage**: Same or better coverage with focused tests
- **Maintainability**: Easier to find and modify code

## Timeline

- **Day 1**: Create module structure, move summary formatting (4h)
- **Day 2**: Move recommendations formatting (4h)
- **Day 3**: Move helpers, update imports (4h)
- **Day 4**: Update tests, fix any issues (4h)
- **Day 5**: Documentation, final testing (4h)

**Total Effort**: 20 hours (2.5 person-days)
