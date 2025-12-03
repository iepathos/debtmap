---
number: 207
title: Refactor formatter_verbosity.rs for Consistency
category: refactor
priority: medium
status: draft
dependencies: [202, 203, 204, 205, 206]
created: 2025-12-02
---

# Specification 207: Refactor formatter_verbosity.rs for Consistency

**Category**: refactor
**Priority**: medium
**Status**: draft
**Dependencies**: [202, 203, 204, 205, 206]

## Context

`src/priority/formatter_verbosity.rs` is the **third major formatter** with 1,087 lines. While smaller than formatter.rs (3,095) and formatter_markdown.rs (2,889), it still has consistency and duplication issues:

- **4+ duplicate coverage classification implementations**
- **Duplicate pattern detection logic** (recently added for spec 190 fix)
- **Mixed concerns** (verbosity logic intertwined with formatting)
- **Inconsistent with other formatters** after specs 202-206 refactoring

### Current Issues

1. **Coverage Duplication** (spec 202):
   - `classify_coverage_percentage()` at line 10
   - `format_coverage_status()` at line 27
   - `format_coverage_factor_description()` at line 39
   - Inline coverage matching in markdown formatter

2. **Pattern Detection** (spec 204):
   - `format_pattern_detection()` reconstructs `ComplexityMetrics` to detect pattern
   - Should use `item.detected_pattern` instead

3. **Inconsistent Structure**:
   - Not modularized like specs 205/206
   - Still has some helper functions that should use spec 202

### Why This Matters

After completing specs 202-206, formatter_verbosity.rs will be **the last holdout** with:
- Old coverage classification code
- Pattern re-detection instead of using stored results
- Inconsistent structure compared to refactored formatters

## Objective

Align formatter_verbosity.rs with specs 202-206 by:
1. Migrating to shared classification modules (spec 202)
2. Using stored pattern detection results (spec 204)
3. Following modular structure pattern (spec 205)
4. Maintaining backward-compatible output

## Requirements

### Functional Requirements

1. **Use Shared Coverage Classification** (spec 202)
   - Remove `classify_coverage_percentage()`
   - Remove `format_coverage_status()`
   - Remove `format_coverage_factor_description()`
   - Use `CoverageLevel::from_percentage()` from spec 202

2. **Use Stored Pattern Detection** (spec 204)
   - Remove `format_pattern_detection()` reconstruction logic
   - Use `item.detected_pattern` field directly
   - No `ComplexityMetrics` reconstruction

3. **Modular Structure** (spec 205 pattern)
   ```
   src/priority/formatter_verbosity/
   ├── mod.rs (public API, ~50 lines)
   ├── body.rs (format_item_body, ~300 lines)
   ├── coverage.rs (coverage display, ~150 lines)
   ├── complexity.rs (complexity display, ~200 lines)
   └── sections.rs (section helpers, ~200 lines)
   ```

4. **Backward Compatibility**
   - Public API unchanged
   - Output format identical
   - All tests pass

### Non-Functional Requirements

1. **Consistency**: Uses same classification as terminal and markdown formatters
2. **Maintainability**: Clear module boundaries, no duplication
3. **Testability**: Pure functions where possible
4. **Performance**: No regression

## Acceptance Criteria

- [ ] `formatter_verbosity.rs` reduced from 1,087 lines to ~50 lines (mod.rs)
- [ ] All 4+ coverage classification implementations removed (uses spec 202)
- [ ] Pattern detection reconstruction removed (uses spec 204)
- [ ] `formatter_verbosity/` module structure created
- [ ] All tests pass with identical output
- [ ] Coverage labels match terminal formatter exactly
- [ ] Pattern display uses `item.detected_pattern`
- [ ] No file in `formatter_verbosity/` exceeds 500 lines

## Technical Details

### Implementation Approach

**Stage 1: Migrate Coverage Classification**

```rust
// Before (formatter_verbosity.rs:10-37)
fn classify_coverage_percentage(percentage: f64) -> &'static str {
    match percentage {
        0.0 => "UNTESTED",
        p if p < 20.0 => "LOW",
        p if p < 50.0 => "PARTIAL",
        p if p < 80.0 => "MODERATE",
        p if p < 95.0 => "GOOD",
        _ => "EXCELLENT",
    }
}

fn format_coverage_status(percentage: f64) -> String {
    let level = classify_coverage_percentage(percentage);
    match level {
        "UNTESTED" => "[ERROR UNTESTED]".to_string(),
        "LOW" => "[WARN LOW]".to_string(),
        "PARTIAL" => "[WARN PARTIAL]".to_string(),
        "MODERATE" => "[INFO MODERATE]".to_string(),
        "GOOD" => "[OK GOOD]".to_string(),
        "EXCELLENT" => "[OK EXCELLENT]".to_string(),
        _ => unreachable!(),
    }
}

fn format_coverage_factor_description(percentage: f64) -> &'static str {
    let level = classify_coverage_percentage(percentage);
    match level {
        "UNTESTED" => "No test coverage",
        "LOW" => "Low coverage",
        "PARTIAL" => "Partial coverage",
        "MODERATE" => "Moderate coverage",
        "GOOD" => "Good coverage",
        "EXCELLENT" => "Excellent coverage",
        _ => unreachable!(),
    }
}

// After (using spec 202)
use crate::priority::classification::CoverageLevel;

fn format_coverage_status(percentage: f64) -> String {
    CoverageLevel::from_percentage(percentage).status_tag().to_string()
}

fn format_coverage_description(percentage: f64) -> &'static str {
    CoverageLevel::from_percentage(percentage).description()
}
```

**Stage 2: Fix Pattern Detection**

```rust
// Before (formatter_verbosity.rs - spec 190 fix)
fn format_pattern_detection(output: &mut String, item: &UnifiedDebtItem) {
    use crate::core::LanguageSpecificData;
    use crate::priority::complexity_patterns::{ComplexityMetrics, ComplexityPattern};

    // Reconstruct ComplexityMetrics to detect pattern
    let complexity_metrics = if let Some(LanguageSpecificData::Rust(rust_data)) = &item.language_specific {
        ComplexityMetrics {
            cyclomatic: item.cyclomatic_complexity,
            cognitive: item.cognitive_complexity,
            nesting: item.nesting_depth,
            entropy_score: item.entropy_details.as_ref().map(|e| e.entropy_score),
            state_signals: rust_data.state_machine_signals.clone(),
            coordinator_signals: rust_data.coordinator_signals.clone(),
            validation_signals: None,
        }
    } else {
        return;
    };

    let pattern = ComplexityPattern::detect(&complexity_metrics);

    match pattern {
        ComplexityPattern::Coordinator { action_count, comparison_count, .. } => {
            // ... display logic
        }
        _ => {}
    }
}

// After (using spec 204)
fn format_pattern_detection(output: &mut String, item: &UnifiedDebtItem) {
    // Simply use the stored pattern from spec 204
    if let Some(ref pattern) = item.detected_pattern {
        let metrics_str = pattern.display_metrics().join(", ");

        writeln!(
            output,
            "├─ PATTERN: {} {} ({}, confidence: {:.2})",
            pattern.icon(),
            pattern.type_name().bright_magenta().bold(),
            metrics_str.cyan(),
            pattern.confidence
        ).unwrap();
    }
}
```

**Stage 3: Create Module Structure**

```rust
// src/priority/formatter_verbosity/mod.rs

//! Verbosity-aware terminal formatting for priority items

pub mod body;
pub mod coverage;
pub mod complexity;
pub mod sections;

use crate::formatting::FormattingConfig;
use crate::priority::UnifiedDebtItem;

/// Format priority item with verbosity control
pub fn format_priority_item_with_config(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) {
    body::format_item_body(output, rank, item, verbosity, config, has_coverage_data);
}

// Re-export for backward compatibility
pub use body::format_item_body;
```

**Stage 4: Extract Coverage Module**

```rust
// src/priority/formatter_verbosity/coverage.rs

use crate::priority::classification::CoverageLevel;
use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

/// Format coverage section using shared classification (spec 202)
pub fn format_coverage_section(
    output: &mut String,
    item: &UnifiedDebtItem,
    has_coverage_data: bool,
) {
    if !has_coverage_data {
        return;
    }

    if let Some(ref trans_cov) = item.transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        let level = CoverageLevel::from_percentage(coverage_pct);

        writeln!(
            output,
            "├─ COVERAGE: {:.1}% {}",
            coverage_pct,
            level.status_tag().cyan()
        ).unwrap();
    } else if item.unified_score.coverage_factor >= 10.0 {
        writeln!(
            output,
            "├─ COVERAGE: {} {}",
            "N/A",
            "[ERROR UNTESTED]".bright_red()
        ).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_uses_consistent_labels() {
        let mut output = String::new();
        let item = create_test_item_with_coverage(85.0);

        format_coverage_section(&mut output, &item, true);

        // Must use spec 202 labels
        assert!(output.contains("[OK GOOD]"));
    }

    #[test]
    fn coverage_thresholds_match_classification() {
        // 80% should be "GOOD" not "MODERATE"
        let mut output = String::new();
        let item = create_test_item_with_coverage(80.0);

        format_coverage_section(&mut output, &item, true);

        assert!(output.contains("[OK GOOD]"));
    }
}
```

**Stage 5: Extract Complexity Module**

```rust
// src/priority/formatter_verbosity/complexity.rs

use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

/// Format complexity summary
pub fn format_complexity_summary(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) {
    if item.cyclomatic_complexity == 0 && item.cognitive_complexity == 0 {
        return;
    }

    writeln!(
        output,
        "├─ COMPLEXITY: cyclomatic={}, cognitive={}, nesting={}, branches={}",
        format!("{}", item.cyclomatic_complexity).yellow(),
        format!("{}", item.cognitive_complexity).yellow(),
        format!("{}", item.nesting_depth).yellow(),
        format!("{}", item.estimated_branch_count).yellow()
    ).unwrap();

    // Entropy adjustment (if present)
    if let Some(ref entropy) = item.entropy_details {
        writeln!(
            output,
            "│  {} Entropy-adjusted: {} → {} (dampening: {:.2})",
            "├─",
            item.cyclomatic_complexity,
            entropy.adjusted_complexity,
            entropy.dampening_factor
        ).unwrap();
    }
}

/// Format pattern detection using stored pattern (spec 204)
pub fn format_pattern_detection(output: &mut String, item: &UnifiedDebtItem) {
    if let Some(ref pattern) = item.detected_pattern {
        let metrics_str = pattern.display_metrics().join(", ");

        writeln!(
            output,
            "├─ PATTERN: {} {} ({}, confidence: {:.2})",
            pattern.icon(),
            pattern.type_name().bright_magenta().bold(),
            metrics_str.cyan(),
            pattern.confidence
        ).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_uses_stored_detection() {
        let mut item = create_test_item();
        item.detected_pattern = Some(create_coordinator_pattern());

        let mut output = String::new();
        format_pattern_detection(&mut output, &item);

        // Should use stored pattern, not re-detect
        assert!(output.contains("🎯 Coordinator"));
    }
}
```

**Stage 6: Extract Body Module**

```rust
// src/priority/formatter_verbosity/body.rs

use super::{complexity, coverage};
use crate::formatting::{ColoredFormatter, FormattingConfig};
use crate::priority::classification::Severity;
use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

/// Format complete item body with all sections
pub fn format_item_body(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    _verbosity: u8,
    config: FormattingConfig,
    has_coverage_data: bool,
) {
    let formatter = ColoredFormatter::new(config);
    let severity = Severity::from_score(item.unified_score.final_score);

    // Header
    writeln!(
        output,
        "#{} SCORE: {:.1} [{}]",
        rank,
        item.unified_score.final_score,
        severity.as_str().color(severity.color()).bold()
    ).unwrap();

    // Location
    writeln!(
        output,
        "├─ LOCATION: {}:{} {}()",
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    ).unwrap();

    // Impact
    writeln!(
        output,
        "├─ IMPACT: {}",
        format_impact(&item.expected_impact).bright_cyan()
    ).unwrap();

    // Complexity
    complexity::format_complexity_summary(output, item, &formatter);

    // Pattern (spec 204)
    complexity::format_pattern_detection(output, item);

    // Coverage (spec 202)
    coverage::format_coverage_section(output, item, has_coverage_data);

    // Recommendation
    writeln!(
        output,
        "├─ RECOMMENDED ACTION: {}",
        item.recommendation.primary_action.bright_green().bold()
    ).unwrap();

    // Rationale
    writeln!(
        output,
        "├─ WHY THIS MATTERS: {}",
        item.recommendation.rationale
    ).unwrap();
}

fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    format!(
        "Maintainability: {:.0}%, Test complexity: {:.0}%, Risk: {:.0}%",
        impact.maintainability_burden * 100.0,
        impact.test_complexity_burden * 100.0,
        impact.downstream_risk * 100.0
    )
}
```

### Architecture Changes

**Before**:
```
src/priority/
└── formatter_verbosity.rs (1,087 lines)
    ├── 4+ coverage classification implementations
    ├── Pattern detection reconstruction
    └── Mixed concerns
```

**After**:
```
src/priority/
├── formatter_verbosity/
│   ├── mod.rs (~50 lines - public API)
│   ├── body.rs (~300 lines - main formatting)
│   ├── coverage.rs (~150 lines - coverage display, uses spec 202)
│   ├── complexity.rs (~200 lines - complexity/pattern display)
│   └── sections.rs (~200 lines - section helpers)
└── classification/ (shared - spec 202)
    └── coverage.rs (SINGLE SOURCE OF TRUTH)
```

### Code Reduction

| Component | Before | After | Change |
|-----------|--------|-------|--------|
| Coverage classification | 4+ implementations (~150 lines) | Uses spec 202 | -150 lines |
| Pattern detection | Reconstruction (~50 lines) | Uses spec 204 | -30 lines |
| **Total formatter_verbosity** | **1,087 lines** | **900 lines** | **-17%** |

## Dependencies

- **Prerequisites**:
  - Spec 202: Must use CoverageLevel (remove all 4+ duplicates)
  - Spec 204: Must use item.detected_pattern (remove reconstruction)
  - Spec 205: Follow module structure pattern
- **Affected Components**:
  - `src/priority/formatter_verbosity.rs` (becomes directory)
  - Coverage display tests (must verify spec 202 consistency)
  - Pattern display tests (must verify spec 204 usage)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_labels_match_classification() {
        // Verify spec 202 consistency
        let levels = vec![
            (0.0, "[ERROR UNTESTED]"),
            (15.0, "[WARN LOW]"),
            (35.0, "[WARN PARTIAL]"),
            (65.0, "[INFO MODERATE]"),
            (85.0, "[OK GOOD]"),
            (98.0, "[OK EXCELLENT]"),
        ];

        for (pct, expected_tag) in levels {
            let mut output = String::new();
            let item = create_test_item_with_coverage(pct);

            coverage::format_coverage_section(&mut output, &item, true);

            assert!(
                output.contains(expected_tag),
                "Coverage {}% should show {}, output: {}",
                pct, expected_tag, output
            );
        }
    }

    #[test]
    fn pattern_uses_stored_not_reconstructed() {
        // Verify spec 204 usage
        let mut item = create_test_item();

        // Set detected pattern (spec 204)
        item.detected_pattern = Some(DetectedPattern {
            pattern_type: PatternType::StateMachine,
            confidence: 0.85,
            metrics: PatternMetrics {
                state_transitions: Some(4),
                match_expressions: Some(2),
                action_dispatches: Some(8),
                comparisons: None,
            },
        });

        let mut output = String::new();
        complexity::format_pattern_detection(&mut output, &item);

        assert!(output.contains("🔄 State Machine"));
        assert!(output.contains("transitions: 4"));
        assert!(output.contains("confidence: 0.85"));
    }
}
```

### Cross-Format Consistency Tests

```rust
#[test]
fn verbosity_coverage_matches_terminal() {
    let item = create_test_item_with_coverage(85.0);

    // Terminal formatter
    let terminal = formatter::format_priority_item(&item);

    // Verbosity formatter
    let mut verbosity = String::new();
    formatter_verbosity::format_priority_item_with_config(
        &mut verbosity, 1, &item, 0, config, true
    );

    // Both should show "[OK GOOD]"
    assert!(terminal.contains("[OK GOOD]"));
    assert!(verbosity.contains("[OK GOOD]"));
}

#[test]
fn verbosity_pattern_matches_terminal() {
    let mut item = create_test_item();
    item.detected_pattern = Some(create_coordinator_pattern());

    // Both should display pattern identically
    let terminal = formatter::format_priority_item(&item);

    let mut verbosity = String::new();
    formatter_verbosity::format_priority_item_with_config(
        &mut verbosity, 1, &item, 0, config, false
    );

    assert!(terminal.contains("🎯 Coordinator"));
    assert!(verbosity.contains("🎯 Coordinator"));
}
```

## Documentation Requirements

### Module Documentation

```rust
//! # Verbosity-Aware Formatter
//!
//! Formats priority items with verbosity control using shared components.
//!
//! ## Shared Components
//!
//! - **Spec 202**: Uses `CoverageLevel` for consistent coverage labels
//! - **Spec 204**: Uses `item.detected_pattern` for pattern display
//! - **Spec 205**: Follows modular structure pattern
//!
//! ## Consistency Guarantees
//!
//! Coverage labels, severity classification, and pattern detection are
//! consistent across all formatters (terminal, markdown, verbosity).
```

## Implementation Notes

### Key Principles

1. **Eliminate Duplication**: Remove all 4+ coverage implementations
2. **Use Stored Patterns**: Never reconstruct pattern detection
3. **Consistent Labels**: Must match terminal formatter exactly
4. **Modular Structure**: Follow spec 205 pattern

### Common Pitfalls

- **Don't** keep any of the 4 coverage classification functions
- **Don't** reconstruct ComplexityMetrics for pattern detection
- **Don't** create new coverage thresholds (use spec 202!)
- **Do** verify output is identical after migration

## Migration and Compatibility

### Breaking Changes

None. Pure internal refactoring.

### Migration Path

1. **v0.8.0**: Migrate to spec 202/204 (remove duplicates)
2. **v0.8.1**: Create module structure
3. **v0.8.2**: Extract and test modules
4. **v0.9.0**: Remove old formatter_verbosity.rs

## Success Metrics

- **Duplication Eliminated**: 0 coverage classification implementations (uses spec 202)
- **Pattern Efficiency**: No ComplexityMetrics reconstruction (uses spec 204)
- **Code Reduction**: 1,087 lines → 900 lines (-17%)
- **Consistency**: 100% alignment with terminal/markdown formatters

## Timeline

- **Day 1**: Migrate to spec 202 (coverage) and 204 (patterns) (4h)
- **Day 2**: Create module structure (2h)
- **Day 3**: Extract coverage, complexity, body modules (4h)
- **Day 4**: Testing and verification (4h)
- **Day 5**: Cross-format consistency testing (2h)

**Total Effort**: 16 hours (2 person-days)
