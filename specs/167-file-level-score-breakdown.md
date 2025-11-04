---
number: 167
title: File-Level Score Breakdown with Verbosity
category: optimization
priority: medium
status: draft
dependencies: [166]
created: 2025-11-03
---

# Specification 167: File-Level Score Breakdown with Verbosity

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: [166 - Test File Detection and Context-Aware Scoring]

## Context

**Problem**: File-level debt items show categorical labels but lack numerical score calculation breakdown.

**Current behavior** with `-vv` flag:

**Function-level items** (GOOD - already implemented):
```
#1 SCORE: 25.8 [CRITICAL]
- SCORE CALCULATION:
  - Weighted Sum Model:
   - Coverage Score: 11.0 × 40% = 4.40
   - Complexity Score: 39.8 × 40% = 15.90
   - Dependency Score: 5.0 × 20% = 1.00
   - Base Score: 21.30
   - Entropy Impact: 32% dampening
   - Role Adjustment: ×1.30
   - Final Score: 25.84
```

**File-level items** (MISSING - needs implementation):
```
#1 SCORE: 86.5 [CRITICAL]
└─ ./src/cook/workflow/git_context_diff_tests.rs (354 lines, 7 functions)
└─ SCORING: File size: LOW | Functions: MODERATE | Complexity: HIGH
```

**Why this matters**:
- Categorical labels (LOW, MODERATE, HIGH) don't explain the numeric score (86.5)
- Users can't understand why files score unexpectedly high/low
- No visibility into which factors drive the score (coverage, god object, etc.)
- Makes debugging scoring anomalies impossible

**Real-world impact** (from prodigy codebase analysis):
- Test file scores 86.5 despite "LOW" size and "MODERATE" functions
- No indication that coverage_factor (3.0x) and god_object_multiplier (9.0x) are inflating score
- Users can't see that test files are being penalized for 0% coverage

## Objective

Add comprehensive score calculation breakdown for file-level debt items when `-vv` flag is used, matching the detail and clarity of existing function-level breakdowns.

## Requirements

### Functional Requirements

1. **Score Factor Display**

   Show all 6 multiplicative factors from `FileDebtMetrics::calculate_score()`:

   ```
   - FILE SCORE CALCULATION:
      - Size Factor: 1.88 (√(354/100))
      - Complexity Factor: 1.70 (avg 8.0 cyclomatic × total)
      - Coverage Factor: 3.00 (0% coverage - no data available)
      - Density Factor: 1.00 (7 functions, below threshold)
      - God Object Multiplier: 9.00 (score: 7.0, flagged as god module)
      - Function Factor: 1.00 (function scores sum: 10.2)
      - Final: 1.88 × 1.70 × 3.00 × 1.00 × 9.00 × 1.00 = 86.3
   ```

2. **Factor Explanations**

   Each factor should include:
   - **Numeric value** (e.g., 1.88)
   - **Calculation basis** (e.g., √(354/100))
   - **Context** (e.g., "below threshold" or "3x penalty for missing coverage")

3. **Highlighting Problematic Factors**

   Use color coding to indicate concerning values:
   - **Red/Warning**: coverage_factor > 2.0, god_object_multiplier > 5.0
   - **Yellow/Caution**: size_factor > 4.0, density_factor > 2.0
   - **Green/Normal**: Other values

4. **Context-Aware Messages**

   Include explanatory notes for specific scenarios:
   - Coverage factor 3.0 → "⚠️  No coverage data - assuming untested"
   - God object multiplier >5.0 → "⚠️  Flagged as god object/module"
   - File context = Test → "ℹ️  Test file - see spec 166 for scoring adjustments"

5. **Verbosity Gating**

   - `-v` (verbosity 1): Show categorical labels only (current behavior)
   - `-vv` (verbosity 2): Show full score breakdown
   - No flag: Show score and labels without breakdown

### Non-Functional Requirements

1. **Performance**: Breakdown calculation adds <1ms per file item
2. **Consistency**: Match formatting style of function-level breakdowns exactly
3. **Maintainability**: Centralize formatting logic to avoid duplication
4. **Accuracy**: Display values must match actual calculation in `file_metrics.rs:180-218`

## Acceptance Criteria

- [ ] File-level items with `-vv` show "FILE SCORE CALCULATION:" section
- [ ] All 6 factors displayed with numeric values and explanations
- [ ] Coverage factor shows "no data available" vs actual percentage
- [ ] God object multiplier shows breakdown (2.0 + score)
- [ ] Final calculation line shows multiplication formula
- [ ] Calculated final score matches displayed SCORE value (within 0.1)
- [ ] Color coding highlights coverage_factor ≥2.0 in red/yellow
- [ ] Test files (spec 166) show context note about scoring adjustments
- [ ] Verbosity level 1 continues to show categorical labels
- [ ] No breakdown shown when no verbosity flags used
- [ ] Unit tests verify correct factor extraction from FileDebtMetrics
- [ ] Integration test compares displayed values to calculated values

## Technical Details

### Implementation Approach

**Phase 1: Extract Calculation Components**

Create a helper function to decompose the score calculation:

```rust
// src/priority/file_metrics.rs

impl FileDebtMetrics {
    /// Extract individual scoring factors for display purposes
    pub fn get_score_factors(&self) -> FileScoreFactors {
        let size_factor = (self.total_lines as f64 / 100.0).sqrt();

        let avg_complexity_factor = (self.avg_complexity / 5.0).min(3.0);
        let total_complexity_factor = (self.total_complexity as f64 / 50.0).sqrt();
        let complexity_factor = avg_complexity_factor * total_complexity_factor;

        let coverage_gap = 1.0 - self.coverage_percent;
        let coverage_factor = (coverage_gap * 2.0) + 1.0;

        let density_factor = if self.function_count > 50 {
            1.0 + ((self.function_count - 50) as f64 * 0.02)
        } else {
            1.0
        };

        let god_object_multiplier = if self.god_object_indicators.is_god_object {
            2.0 + self.god_object_indicators.god_object_score
        } else {
            1.0
        };

        let function_score_sum: f64 = self.function_scores.iter().sum();
        let function_factor = (function_score_sum / 10.0).max(1.0);

        FileScoreFactors {
            size_factor,
            size_basis: self.total_lines,
            complexity_factor,
            avg_complexity: self.avg_complexity,
            total_complexity: self.total_complexity,
            coverage_factor,
            coverage_percent: self.coverage_percent,
            coverage_gap,
            density_factor,
            function_count: self.function_count,
            god_object_multiplier,
            god_object_score: self.god_object_indicators.god_object_score,
            is_god_object: self.god_object_indicators.is_god_object,
            function_factor,
            function_score_sum,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileScoreFactors {
    pub size_factor: f64,
    pub size_basis: usize,
    pub complexity_factor: f64,
    pub avg_complexity: f64,
    pub total_complexity: u32,
    pub coverage_factor: f64,
    pub coverage_percent: f64,
    pub coverage_gap: f64,
    pub density_factor: f64,
    pub function_count: usize,
    pub god_object_multiplier: f64,
    pub god_object_score: f64,
    pub is_god_object: bool,
    pub function_factor: f64,
    pub function_score_sum: f64,
}
```

**Phase 2: Format Score Breakdown**

Add formatting function to match function-level style:

```rust
// src/priority/formatter_verbosity.rs

fn format_file_score_calculation_section(
    item: &FileDebtItem,
    _formatter: &ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let factors = item.metrics.get_score_factors();

    lines.push(format!(
        "- {}",
        "FILE SCORE CALCULATION:".bright_blue()
    ));

    // Size factor
    lines.push(format!(
        "   - Size Factor: {:.2} (√({}/100))",
        factors.size_factor,
        factors.size_basis
    ));

    // Complexity factor
    lines.push(format!(
        "   - Complexity Factor: {:.2} (avg {:.1} × total factor)",
        factors.complexity_factor,
        factors.avg_complexity
    ));

    // Coverage factor with warning
    let coverage_detail = if factors.coverage_percent == 0.0 {
        " ⚠️  No coverage data - assuming untested".bright_red()
    } else if factors.coverage_factor >= 2.0 {
        format!(" ⚠️  Low coverage: {:.0}%", factors.coverage_percent * 100.0)
            .bright_yellow()
    } else {
        format!(" ({:.0}% coverage)", factors.coverage_percent * 100.0).normal()
    };

    lines.push(format!(
        "   - Coverage Factor: {:.2}{}",
        factors.coverage_factor,
        coverage_detail
    ));

    // Density factor
    let density_detail = if factors.function_count > 50 {
        format!(" ({} functions, {} over threshold)",
                factors.function_count,
                factors.function_count - 50)
    } else {
        format!(" ({} functions, below threshold)", factors.function_count)
    };

    lines.push(format!(
        "   - Density Factor: {:.2}{}",
        factors.density_factor,
        density_detail
    ));

    // God object multiplier with warning
    let god_detail = if factors.is_god_object {
        format!(
            " ⚠️  Flagged as god object (score: {:.1})",
            factors.god_object_score
        ).bright_yellow()
    } else {
        " (not flagged)".normal()
    };

    lines.push(format!(
        "   - God Object Multiplier: {:.2} (2.0 + {:.1}){}",
        factors.god_object_multiplier,
        factors.god_object_score,
        god_detail
    ));

    // Function factor
    lines.push(format!(
        "   - Function Factor: {:.2} (function scores sum: {:.1})",
        factors.function_factor,
        factors.function_score_sum
    ));

    // Final calculation
    let calculated_score = factors.size_factor
        * factors.complexity_factor
        * factors.coverage_factor
        * factors.density_factor
        * factors.god_object_multiplier
        * factors.function_factor;

    lines.push(format!(
        "   - Final: {:.2} × {:.2} × {:.2} × {:.2} × {:.2} × {:.2} = {:.1}",
        factors.size_factor,
        factors.complexity_factor,
        factors.coverage_factor,
        factors.density_factor,
        factors.god_object_multiplier,
        factors.function_factor,
        calculated_score
    ));

    // Validation check (debug mode only)
    #[cfg(debug_assertions)]
    {
        let actual_score = item.score;
        let diff = (calculated_score - actual_score).abs();
        if diff > 0.5 {
            lines.push(format!(
                "   ⚠️  Calculation mismatch: displayed={:.1}, calculated={:.1}",
                actual_score, calculated_score
            ).bright_red());
        }
    }

    lines
}
```

**Phase 3: Integration with Formatter**

Modify `format_file_priority_item_with_verbosity()`:

```rust
// src/priority/formatter.rs

fn format_file_priority_item_with_verbosity(
    output: &mut String,
    rank: usize,
    item: &priority::FileDebtItem,
    config: FormattingConfig,
    verbosity: u8,
) {
    let formatter = ColoredFormatter::new(config);

    // ... existing header and WHY THIS MATTERS ...

    // Add score breakdown for verbosity >= 2
    if verbosity >= 2 {
        let score_calc_lines = format_file_score_calculation_section(item, &formatter);
        for line in score_calc_lines {
            writeln!(output, "{}", line).unwrap();
        }
    }

    // ... rest of existing formatting ...
}
```

**Phase 4: Test File Context Integration**

Add note for test files (requires spec 166):

```rust
// After score calculation, check for test file context
if let Some(FileContext::Test { confidence, .. }) = &item.metrics.file_context {
    lines.push(format!(
        "   ℹ️  Test file detected (confidence: {:.0}%) - scoring adjustments applied",
        confidence * 100.0
    ).bright_cyan());
}
```

### Architecture Changes

1. **New struct**: `FileScoreFactors` - Holds decomposed calculation components
2. **New method**: `FileDebtMetrics::get_score_factors()` - Extracts factors
3. **New function**: `format_file_score_calculation_section()` - Formats breakdown
4. **Modified function**: `format_file_priority_item_with_verbosity()` - Adds breakdown

### Data Structures

```rust
pub struct FileScoreFactors {
    // Direct factors
    pub size_factor: f64,
    pub complexity_factor: f64,
    pub coverage_factor: f64,
    pub density_factor: f64,
    pub god_object_multiplier: f64,
    pub function_factor: f64,

    // Basis values for explanations
    pub size_basis: usize,              // total_lines
    pub avg_complexity: f64,
    pub total_complexity: u32,
    pub coverage_percent: f64,
    pub coverage_gap: f64,
    pub function_count: usize,
    pub god_object_score: f64,
    pub is_god_object: bool,
    pub function_score_sum: f64,
}
```

### Output Format Specification

**Standard output** (no flags):
```
#1 SCORE: 86.5 [CRITICAL]
└─ ./src/cook/workflow/git_context_diff_tests.rs (354 lines, 7 functions)
```

**Verbose output** (`-v`):
```
#1 SCORE: 86.5 [CRITICAL]
└─ ./src/cook/workflow/git_context_diff_tests.rs (354 lines, 7 functions)
└─ SCORING: File size: LOW | Functions: MODERATE | Complexity: HIGH
```

**Very verbose output** (`-vv`):
```
#1 SCORE: 86.5 [CRITICAL]
└─ ./src/cook/workflow/git_context_diff_tests.rs (354 lines, 7 functions)

- FILE SCORE CALCULATION:
   - Size Factor: 1.88 (√(354/100))
   - Complexity Factor: 1.70 (avg 8.0 × total factor)
   - Coverage Factor: 3.00 ⚠️  No coverage data - assuming untested
   - Density Factor: 1.00 (7 functions, below threshold)
   - God Object Multiplier: 9.00 (2.0 + 7.0) ⚠️  Flagged as god object (score: 7.0)
   - Function Factor: 1.00 (function scores sum: 10.2)
   - Final: 1.88 × 1.70 × 3.00 × 1.00 × 9.00 × 1.00 = 86.3
   ℹ️  Test file detected (confidence: 95%) - scoring adjustments applied

└─ SCORING: File size: LOW | Functions: MODERATE | Complexity: HIGH
```

## Dependencies

- **Spec 166**: Test File Detection - Provides `FileContext` for test file notes
- **Existing**: Function-level score breakdown in `formatter_verbosity.rs:389-488`

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_score_factors_extraction() {
        let metrics = FileDebtMetrics {
            total_lines: 354,
            function_count: 7,
            avg_complexity: 8.0,
            total_complexity: 56,
            coverage_percent: 0.0,
            god_object_indicators: GodObjectIndicators {
                is_god_object: true,
                god_object_score: 7.0,
                // ...
            },
            function_scores: vec![1.5, 1.5, 1.5, 1.5, 1.5, 1.5, 1.5],
            // ...
        };

        let factors = metrics.get_score_factors();

        assert!((factors.size_factor - 1.88).abs() < 0.01);
        assert_eq!(factors.coverage_factor, 3.0);
        assert_eq!(factors.god_object_multiplier, 9.0);
        assert_eq!(factors.density_factor, 1.0);
    }

    #[test]
    fn test_score_calculation_matches_display() {
        let metrics = create_test_file_metrics();
        let factors = metrics.get_score_factors();
        let actual_score = metrics.calculate_score();

        let calculated = factors.size_factor
            * factors.complexity_factor
            * factors.coverage_factor
            * factors.density_factor
            * factors.god_object_multiplier
            * factors.function_factor;

        assert!((calculated - actual_score).abs() < 0.5);
    }

    #[test]
    fn test_format_file_score_breakdown_structure() {
        let item = create_test_file_debt_item();
        let formatter = ColoredFormatter::new(FormattingConfig::default());
        let lines = format_file_score_calculation_section(&item, &formatter);

        assert!(lines.len() >= 8); // Header + 6 factors + final
        assert!(lines[0].contains("FILE SCORE CALCULATION"));
        assert!(lines.iter().any(|l| l.contains("Size Factor")));
        assert!(lines.iter().any(|l| l.contains("Coverage Factor")));
        assert!(lines.iter().any(|l| l.contains("Final:")));
    }

    #[test]
    fn test_coverage_warning_displayed() {
        let mut item = create_test_file_debt_item();
        item.metrics.coverage_percent = 0.0;

        let lines = format_file_score_calculation_section(&item, &formatter);
        let coverage_line = lines.iter()
            .find(|l| l.contains("Coverage Factor"))
            .unwrap();

        assert!(coverage_line.contains("No coverage data"));
        assert!(coverage_line.contains("⚠️"));
    }

    #[test]
    fn test_god_object_warning_displayed() {
        let mut item = create_test_file_debt_item();
        item.metrics.god_object_indicators.is_god_object = true;
        item.metrics.god_object_indicators.god_object_score = 12.5;

        let lines = format_file_score_calculation_section(&item, &formatter);
        let god_line = lines.iter()
            .find(|l| l.contains("God Object Multiplier"))
            .unwrap();

        assert!(god_line.contains("Flagged as god object"));
        assert!(god_line.contains("12.5"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_verbosity_levels_file_items() {
    let analysis = run_debtmap_on_test_project();

    // No verbosity
    let output_plain = format_output(&analysis, 0);
    assert!(!output_plain.contains("FILE SCORE CALCULATION"));
    assert!(!output_plain.contains("Size Factor"));

    // -v
    let output_v1 = format_output(&analysis, 1);
    assert!(output_v1.contains("SCORING:"));
    assert!(!output_v1.contains("FILE SCORE CALCULATION"));

    // -vv
    let output_v2 = format_output(&analysis, 2);
    assert!(output_v2.contains("FILE SCORE CALCULATION"));
    assert!(output_v2.contains("Size Factor"));
    assert!(output_v2.contains("Coverage Factor"));
    assert!(output_v2.contains("Final:"));
}

#[test]
fn test_score_accuracy_in_output() {
    let analysis = analyze_prodigy_codebase();
    let output = format_output(&analysis, 2);

    // Parse displayed score and calculated score
    let score_line = extract_line(&output, "SCORE:");
    let final_line = extract_line(&output, "Final:");

    let displayed = parse_score(score_line);
    let calculated = parse_calculated_score(final_line);

    assert!((displayed - calculated).abs() < 0.5);
}
```

### Manual Testing

Test on real codebases:
1. **Prodigy**: Verify test files show all 6 factors correctly
2. **Debtmap**: Self-analysis to check various file types
3. **Large project**: Ensure formatting consistent across many files

## Documentation Requirements

### Code Documentation

```rust
/// Extracts scoring factors from file metrics for display purposes.
///
/// This method decomposes the opaque score calculation from `calculate_score()`
/// into individual factors that can be shown to users for transparency.
///
/// # Returns
///
/// `FileScoreFactors` containing:
/// - All 6 multiplicative factors (size, complexity, coverage, density, god object, function)
/// - Basis values used to calculate each factor
/// - Contextual information for display (e.g., whether flagged as god object)
///
/// # Example
///
/// ```
/// let factors = metrics.get_score_factors();
/// println!("Coverage factor: {:.2} ({}% coverage)",
///          factors.coverage_factor,
///          factors.coverage_percent * 100.0);
/// ```
pub fn get_score_factors(&self) -> FileScoreFactors { ... }
```

### User Documentation

Update README.md:

```markdown
## Score Transparency

Debtmap provides detailed score breakdowns with the `-vv` (very verbose) flag:

### Function-Level Scores

Shows weighted sum of coverage, complexity, and dependency factors:
```
- SCORE CALCULATION:
   - Coverage Score: 11.0 × 40% = 4.40
   - Complexity Score: 39.8 × 40% = 15.90
   ...
```

### File-Level Scores

Shows all 6 multiplicative factors:
```
- FILE SCORE CALCULATION:
   - Size Factor: 1.88 (√(354/100))
   - Complexity Factor: 1.70
   - Coverage Factor: 3.00 ⚠️  No coverage data
   - God Object Multiplier: 9.00 ⚠️  Flagged
   ...
```

Warnings (⚠️) highlight problematic factors that inflate the score.
```

### Architecture Updates

Add to ARCHITECTURE.md:

```markdown
## Score Transparency

Both function-level and file-level scores provide full calculation breakdowns
with `-vv` verbosity:

- **Function scores**: Weighted sum model (coverage 40%, complexity 40%, dependencies 20%)
- **File scores**: Multiplicative factors (size × complexity × coverage × density × god_object × functions)

All factors include:
- Numeric value
- Calculation basis
- Contextual explanation
- Warning indicators for problematic values
```

## Implementation Notes

### Calculation Consistency

**Critical**: `get_score_factors()` must use identical formulas to `calculate_score()`.

Use DRY principle:
```rust
// Bad: Duplicated logic
fn calculate_score(&self) -> f64 {
    let size_factor = (self.total_lines as f64 / 100.0).sqrt();
    // ...
}

fn get_score_factors(&self) -> FileScoreFactors {
    let size_factor = (self.total_lines as f64 / 100.0).sqrt(); // DUPLICATION!
    // ...
}

// Good: Shared calculation
impl FileDebtMetrics {
    fn calculate_size_factor(&self) -> f64 {
        (self.total_lines as f64 / 100.0).sqrt()
    }

    pub fn calculate_score(&self) -> f64 {
        let size_factor = self.calculate_size_factor();
        // ...
    }

    pub fn get_score_factors(&self) -> FileScoreFactors {
        let size_factor = self.calculate_size_factor();
        // ...
    }
}
```

### Color Coding Guidelines

Use consistent color scheme:
- **Red (bright_red)**: Critical issues (coverage_factor ≥ 3.0, god_multiplier ≥ 10.0)
- **Yellow (bright_yellow)**: Warnings (coverage_factor ≥ 2.0, god_multiplier ≥ 5.0)
- **Cyan (bright_cyan)**: Informational (test file notes, entry points)
- **Normal**: Standard values

### Edge Cases

1. **Zero coverage**: Show "No coverage data" not "0% coverage" (ambiguous)
2. **Very high multipliers**: Cap display at reasonable precision (2 decimal places)
3. **NaN/Inf values**: Handle gracefully with error message
4. **Calculation mismatches**: Log warning in debug builds, show to user if >0.5 difference

## Migration and Compatibility

### Backward Compatibility

- No changes to default output (no flags)
- `-v` behavior unchanged
- Only `-vv` adds new breakdown section
- JSON output unchanged (no new fields needed)

### Rollout Plan

1. **Phase 1**: Implement `get_score_factors()` and add tests
2. **Phase 2**: Add formatting function and integrate with verbosity
3. **Phase 3**: Deploy and gather user feedback on clarity
4. **Phase 4**: Iterate on formatting based on feedback

### Breaking Changes

None - this is purely additive output enhancement.

## Success Metrics

**Adoption**:
- Track `-vv` flag usage (expect 20-30% of runs)
- Monitor GitHub issues about "scoring confusion" (expect 50% reduction)

**Quality**:
- Zero calculation mismatches >0.5 in production
- User feedback: "Scores now make sense" sentiment >80%
- False positive reports mention specific factors (coverage, god object)

**Impact**:
- Faster debugging of scoring anomalies
- Better understanding of test file scoring (with spec 166)
- More trust in debtmap recommendations

## Future Enhancements (Not in Scope)

- Interactive breakdown (click factor to see details)
- Historical factor tracking (coverage factor increased 0.5 since last run)
- Factor contribution ranking (coverage accounts for 60% of score)
- Custom factor weights via config
- Factor-based filtering (show only files with coverage_factor >2.0)
