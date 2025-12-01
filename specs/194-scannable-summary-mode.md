---
number: 194
title: Scannable Summary Mode for Terminal Output
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 194: Scannable Summary Mode for Terminal Output

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's terminal output currently displays comprehensive detail for each technical debt item, showing 15-20 lines per recommendation. This creates information overload that makes it difficult for users to:

- **Scan all recommendations quickly** - Users must scroll extensively to see all 10 items
- **Compare priorities effectively** - Key metrics (score, impact, action) are buried in detail
- **Make fast decisions** - Working memory burden is high (150-200 facts across 10 items)
- **Get quick overview** - Default output optimizes for completeness over clarity

Example current output for god object detection:
```
#1 SCORE: 370 [CRITICAL]
└─ ./src/organization/god_object_detector.rs (4363 lines, 55 functions)
└─ WHY THIS MATTERS: This module contains 55 module functions across 8 responsibilities...
└─ ACTION: Split by analysis phase: 1) Data collection 2) Pattern detection...

  - STRUCTURE: 8 responsibilities across 12 components
  - FUNCTIONS: 55 total (6 public, 49 private)
  - LARGEST COMPONENTS:
    - GodObjectDetector impl: 51 functions, 2725 lines
    [... 11+ more lines of details]
└─ IMPACT: Estimated 16-31% complexity reduction...
└─ METRICS: Methods: 55, Fields: 5, Responsibilities: 8
└─ SCORING: File size: HIGH | Functions: EXCESSIVE | Complexity: HIGH
└─ DEPENDENCIES: 55 functions may have complex interdependencies
```

This violates the UX principle: **Default to clarity, opt-in to complexity**.

## Objective

Create a scannable summary mode as the default terminal output that:

1. **Compresses each recommendation to 3-4 lines** showing only essential decision-making information
2. **Makes all 10 items visible without scrolling** on standard terminal windows
3. **Highlights key metrics** (score, impact, file, action) for fast comparison
4. **Provides progressive disclosure** with `--detail` flag for comprehensive analysis
5. **Reduces cognitive load** from 150-200 facts to 30-40 facts

**Success Metric**: Users can understand and prioritize all 10 recommendations in < 30 seconds.

## Requirements

### Functional Requirements

1. **Compressed Summary Format**
   - Each recommendation displays in exactly 3-4 lines
   - First line: Score, severity, impact estimate
   - Second line: File path, key metrics (functions, responsibilities, complexity)
   - Third line: Concrete action statement
   - Fourth line (optional): Progressive disclosure hint

2. **Essential Information Priority**
   - **Score** - Numerical priority for sorting
   - **Severity** - Visual urgency indicator (CRITICAL, HIGH, etc.)
   - **Impact** - Estimated complexity/coverage improvement percentage
   - **File path** - Where to take action
   - **Key metrics** - Most relevant counts (functions, lines, responsibilities)
   - **Action** - Single clear next step

3. **Progressive Disclosure**
   - Default mode shows compressed summary
   - `--detail` flag shows current comprehensive output
   - `--detail=<number>` shows detail for specific item only
   - Summary hints at detail availability

4. **Maintain Semantic Information**
   - Don't lose critical information in compression
   - Ensure action statements remain actionable
   - Keep impact estimates prominent
   - Preserve severity distinctions

### Non-Functional Requirements

1. **Scannability**
   - All 10 items fit in ~40 terminal lines (typical 80x24 window with scrollback)
   - Visual hierarchy clear with consistent indentation
   - Easy to compare items side-by-side

2. **Performance**
   - No performance regression from compression (should be faster)
   - Detail mode maintains current performance

3. **Backward Compatibility**
   - Existing CI/CD scripts expecting specific output should not break
   - Consider environment variable for detail mode (e.g., `DEBTMAP_DETAIL=1`)

4. **Consistency**
   - Summary format consistent across all debt types (god objects, complex functions, etc.)
   - Metrics shown follow same pattern for all items

## Acceptance Criteria

- [ ] Default terminal output shows compressed 3-4 line format per recommendation
- [ ] All 10 recommendations visible in ~40 terminal lines
- [ ] Score, severity, and impact shown on first line
- [ ] File path and key metrics shown on second line
- [ ] Action statement shown on third line
- [ ] Progressive disclosure hint shown when detail available
- [ ] `--detail` flag shows comprehensive analysis (current format)
- [ ] `--detail=N` shows detail for item #N only
- [ ] Summary format consistent across god objects, complex functions, and coverage items
- [ ] Key metrics prioritized: functions/lines for god objects, complexity for functions
- [ ] Action statements remain concrete and actionable
- [ ] Impact estimates prominently displayed
- [ ] Visual hierarchy maintained with tree symbols (├─, └─)
- [ ] Terminal output generation performance not regressed
- [ ] Environment variable `DEBTMAP_DETAIL=1` enables detail mode
- [ ] Documentation updated with summary/detail mode examples
- [ ] User can scan and understand all 10 items in < 30 seconds (user testing)

## Technical Details

### Implementation Approach

**Phase 1: Create Formatting Abstraction**

Add summary/detail formatting trait:

```rust
// src/io/writers/terminal.rs or new src/io/formatters/summary.rs
pub trait SummaryFormat {
    fn format_summary(&self) -> String;
    fn format_detail(&self) -> String;
}

pub struct DebtItemSummary<'a> {
    pub rank: usize,
    pub score: f64,
    pub severity: Severity,
    pub impact: ImpactEstimate,
    pub location: &'a Path,
    pub key_metrics: Vec<(&'static str, String)>,
    pub action: &'a str,
}

impl<'a> DebtItemSummary<'a> {
    pub fn format_summary(&self) -> String {
        format!(
            "#{rank} SCORE: {score} [{severity}] IMPACT: {impact}\n\
             └─ {location} · {metrics}\n\
             └─ ACTION: {action}\n\
             └─ Run with --detail={rank} for full analysis\n",
            rank = self.rank,
            score = self.score,
            severity = self.severity,
            impact = self.impact,
            location = self.location.display(),
            metrics = self.format_key_metrics(),
            action = self.action,
        )
    }

    fn format_key_metrics(&self) -> String {
        self.key_metrics
            .iter()
            .map(|(label, value)| format!("{}: {}", label, value))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub struct ImpactEstimate {
    pub complexity_reduction: Option<(u32, u32)>, // (min, max) percentage
    pub coverage_increase: Option<u32>, // percentage
}

impl fmt::Display for ImpactEstimate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if let Some((min, max)) = self.complexity_reduction {
            parts.push(format!("-{}-{}% complexity", min, max));
        }
        if let Some(cov) = self.coverage_increase {
            parts.push(format!("+{}% coverage", cov));
        }
        write!(f, "{}", parts.join(", "))
    }
}
```

**Phase 2: Extract Key Metrics per Debt Type**

```rust
// Different debt types highlight different metrics
fn extract_key_metrics(recommendation: &DebtRecommendation) -> Vec<(&'static str, String)> {
    match recommendation.category {
        DebtCategory::GodObject => vec![
            ("functions", recommendation.function_count.to_string()),
            ("responsibilities", recommendation.responsibility_count.to_string()),
        ],
        DebtCategory::ComplexFunction => vec![
            ("cyclomatic", recommendation.cyclomatic_complexity.to_string()),
            ("cognitive", recommendation.cognitive_complexity.to_string()),
            ("nesting", recommendation.nesting_depth.to_string()),
        ],
        DebtCategory::UncoveredComplexity => vec![
            ("coverage", format!("{}%", recommendation.coverage)),
            ("complexity", recommendation.complexity.to_string()),
            ("tests_needed", recommendation.tests_needed.to_string()),
        ],
        // ... other categories
    }
}
```

**Phase 3: Modify Terminal Writer**

```rust
// src/io/writers/terminal.rs

impl TerminalWriter {
    pub fn write_recommendations(
        &mut self,
        recommendations: &[DebtRecommendation],
        detail_mode: DetailMode,
    ) -> Result<()> {
        match detail_mode {
            DetailMode::Summary => self.write_summary_format(recommendations),
            DetailMode::DetailAll => self.write_detail_format(recommendations),
            DetailMode::DetailItem(n) => self.write_detail_for_item(recommendations, n),
        }
    }

    fn write_summary_format(&mut self, recommendations: &[DebtRecommendation]) -> Result<()> {
        for (idx, rec) in recommendations.iter().enumerate() {
            let summary = DebtItemSummary {
                rank: idx + 1,
                score: rec.score,
                severity: rec.severity,
                impact: rec.impact_estimate,
                location: &rec.file_path,
                key_metrics: extract_key_metrics(rec),
                action: &rec.action_statement,
            };

            writeln!(self.output, "{}", summary.format_summary())?;
        }
        Ok(())
    }

    fn write_detail_format(&mut self, recommendations: &[DebtRecommendation]) -> Result<()> {
        // Current comprehensive format
        for rec in recommendations {
            self.write_recommendation_detail(rec)?;
        }
        Ok(())
    }

    fn write_detail_for_item(
        &mut self,
        recommendations: &[DebtRecommendation],
        item_number: usize,
    ) -> Result<()> {
        let rec = recommendations.get(item_number - 1)
            .context("Invalid item number")?;
        self.write_recommendation_detail(rec)
    }
}

pub enum DetailMode {
    Summary,
    DetailAll,
    DetailItem(usize),
}

impl DetailMode {
    pub fn from_args(args: &AnalyzeArgs) -> Self {
        if let Some(n) = args.detail_item {
            DetailMode::DetailItem(n)
        } else if args.detail || env::var("DEBTMAP_DETAIL").is_ok() {
            DetailMode::DetailAll
        } else {
            DetailMode::Summary
        }
    }
}
```

**Phase 4: CLI Argument Parsing**

```rust
// src/commands/analyze.rs

#[derive(Debug, clap::Args)]
pub struct AnalyzeArgs {
    /// Show detailed analysis for all recommendations
    #[arg(long)]
    pub detail: bool,

    /// Show detailed analysis for specific item number
    #[arg(long, value_name = "NUMBER")]
    pub detail_item: Option<usize>,

    // ... existing args
}
```

### Example Output Comparison

**Before (Current - 20+ lines per item)**:
```
#1 SCORE: 370 [CRITICAL]
└─ ./src/organization/god_object_detector.rs (4363 lines, 55 functions)
└─ WHY THIS MATTERS: This module contains 55 module functions across 8 responsibilities. Large modules with many diverse functions are difficult to navigate, understand, and maintain.
└─ ACTION: Split by analysis phase: 1) Data collection 2) Pattern detection 3) Scoring/metrics 4) Reporting. Keep related analyses together.
  (Use --show-splits for detailed module split recommendations)

  - STRUCTURE: 8 responsibilities across 12 components
  - FUNCTIONS: 55 total (6 public, 49 private)
  - LARGEST COMPONENTS:
    - GodObjectDetector impl: 51 functions, 2725 lines
    - OrganizationDetector for OrganizationDetector: 3 functions, 42 lines
    - CallGraphProvider for CallGraphProvider: 3 functions, 25 lines

  - IMPLEMENTATION ORDER:
  -  [1] Start with lowest coupling modules (Data Access, Utilities)
  -  [2] Move 10-20 methods at a time, test after each move
  -  [3] Keep original file as facade during migration
  -  [4] Refactor incrementally: 10-20 methods at a time
└─ IMPACT: Estimated 16-31% complexity reduction (high coupling detected - splits may be challenging). Improve testability, enable parallel development
└─ METRICS: Methods: 55, Fields: 5, Responsibilities: 8
└─ SCORING: File size: HIGH | Functions: EXCESSIVE | Complexity: HIGH
└─ DEPENDENCIES: 55 functions may have complex interdependencies
```

**After (Summary - 4 lines per item)**:
```
#1 SCORE: 370 [CRITICAL] IMPACT: -16-31% complexity
  └─ god_object_detector.rs · functions: 55, responsibilities: 8
  └─ ACTION: Split by analysis phase (data → detect → score → report)
  └─ Run with --detail=1 for full analysis

#2 SCORE: 168 [CRITICAL] IMPACT: -20-35% complexity
  └─ god_object_analysis.rs · functions: 143, responsibilities: 10
  └─ ACTION: Split by data flow (input → logic → output, 6 modules max 30 functions)
  └─ Run with --detail=2 for full analysis
```

**Detail Mode (`debtmap analyze . --detail=1`)**:
```
#1 SCORE: 370 [CRITICAL]
[... current comprehensive format for item #1 only ...]
```

### Architecture Changes

Modified files:
- `src/io/writers/terminal.rs` - Add summary formatting, detail mode switching
- `src/commands/analyze.rs` - Add CLI arguments for detail mode
- `src/io/formatters/summary.rs` (new) - Pure formatting functions for summary mode
- `src/priority/recommendations.rs` - Add impact estimate extraction

New modules:
- `src/io/formatters/` - Formatting abstractions and pure functions
  - `summary.rs` - Summary format logic
  - `detail.rs` - Detail format logic (refactored from terminal.rs)

### Data Structures

```rust
// src/io/formatters/summary.rs

#[derive(Debug, Clone)]
pub struct ImpactEstimate {
    pub complexity_reduction: Option<(u32, u32)>, // (min%, max%)
    pub coverage_increase: Option<u32>, // percentage
    pub risk_reduction: Option<f64>, // absolute risk points
}

#[derive(Debug, Clone)]
pub struct DebtItemSummary<'a> {
    pub rank: usize,
    pub score: f64,
    pub severity: Severity,
    pub impact: ImpactEstimate,
    pub location: &'a Path,
    pub key_metrics: Vec<(&'static str, String)>,
    pub action: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy)]
pub enum DetailMode {
    Summary,
    DetailAll,
    DetailItem(usize),
}
```

### APIs and Interfaces

**Public API additions**:

```rust
// src/io/formatters/summary.rs

/// Extract key metrics for summary display based on debt category
pub fn extract_key_metrics(recommendation: &DebtRecommendation) -> Vec<(&'static str, String)>;

/// Format impact estimate for display
pub fn format_impact_estimate(estimate: &ImpactEstimate) -> String;

/// Create summary from recommendation
pub fn create_summary<'a>(
    rank: usize,
    recommendation: &'a DebtRecommendation,
) -> DebtItemSummary<'a>;

// src/io/writers/terminal.rs

/// Write recommendations in summary format
pub fn write_summary_format(&mut self, recommendations: &[DebtRecommendation]) -> Result<()>;

/// Write single recommendation in detail format
pub fn write_detail_for_item(&mut self, recommendations: &[DebtRecommendation], n: usize) -> Result<()>;
```

## Dependencies

- **Prerequisites**: None (independent enhancement)
- **Affected Components**:
  - `src/io/writers/terminal.rs` - Terminal output writer
  - `src/commands/analyze.rs` - CLI command handling
  - `src/priority/formatter.rs` - Recommendation formatting
  - `src/priority/recommendations.rs` - Recommendation data structures
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
// src/io/formatters/summary.rs

#[cfg(test)]
mod tests {
    #[test]
    fn test_format_summary_god_object() {
        let summary = DebtItemSummary {
            rank: 1,
            score: 370.0,
            severity: Severity::Critical,
            impact: ImpactEstimate {
                complexity_reduction: Some((16, 31)),
                coverage_increase: None,
                risk_reduction: None,
            },
            location: Path::new("src/god_object_detector.rs"),
            key_metrics: vec![
                ("functions", "55".to_string()),
                ("responsibilities", "8".to_string()),
            ],
            action: "Split by analysis phase",
        };

        let output = summary.format_summary();

        assert!(output.contains("#1 SCORE: 370 [CRITICAL]"));
        assert!(output.contains("IMPACT: -16-31% complexity"));
        assert!(output.contains("functions: 55"));
        assert!(output.contains("responsibilities: 8"));
        assert!(output.contains("ACTION: Split by analysis phase"));
        assert!(output.contains("--detail=1"));
    }

    #[test]
    fn test_extract_key_metrics_god_object() {
        let rec = create_god_object_recommendation();
        let metrics = extract_key_metrics(&rec);

        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].0, "functions");
        assert_eq!(metrics[1].0, "responsibilities");
    }

    #[test]
    fn test_extract_key_metrics_complex_function() {
        let rec = create_complex_function_recommendation();
        let metrics = extract_key_metrics(&rec);

        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[0].0, "cyclomatic");
        assert_eq!(metrics[1].0, "cognitive");
        assert_eq!(metrics[2].0, "nesting");
    }

    #[test]
    fn test_format_impact_estimate() {
        let impact = ImpactEstimate {
            complexity_reduction: Some((20, 35)),
            coverage_increase: Some(50),
            risk_reduction: None,
        };

        let formatted = format_impact_estimate(&impact);
        assert_eq!(formatted, "-20-35% complexity, +50% coverage");
    }
}
```

### Integration Tests

```rust
// tests/summary_format_integration_test.rs

#[test]
fn test_summary_format_all_items_visible() {
    let recommendations = generate_10_recommendations();

    let mut buffer = Vec::new();
    let mut writer = TerminalWriter::new(&mut buffer);
    writer.write_recommendations(&recommendations, DetailMode::Summary).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let line_count = output.lines().count();

    // 10 items × 4 lines each = 40 lines, plus header = ~50 lines max
    assert!(line_count <= 50, "Summary too long: {} lines", line_count);

    // All 10 items should be present
    for i in 1..=10 {
        assert!(output.contains(&format!("#{} SCORE:", i)));
    }
}

#[test]
fn test_detail_mode_single_item() {
    let recommendations = generate_10_recommendations();

    let mut buffer = Vec::new();
    let mut writer = TerminalWriter::new(&mut buffer);
    writer.write_recommendations(&recommendations, DetailMode::DetailItem(3)).unwrap();

    let output = String::from_utf8(buffer).unwrap();

    // Should only show item #3 in detail
    assert!(output.contains("#3 SCORE:"));
    assert!(!output.contains("#1 SCORE:"));
    assert!(!output.contains("#2 SCORE:"));

    // Should show comprehensive detail
    assert!(output.contains("WHY THIS MATTERS:"));
    assert!(output.contains("IMPLEMENTATION ORDER:"));
}

#[test]
fn test_environment_variable_enables_detail() {
    env::set_var("DEBTMAP_DETAIL", "1");

    let args = AnalyzeArgs {
        detail: false,
        detail_item: None,
        ..Default::default()
    };

    let mode = DetailMode::from_args(&args);
    assert!(matches!(mode, DetailMode::DetailAll));

    env::remove_var("DEBTMAP_DETAIL");
}
```

### User Experience Tests

```rust
#[test]
fn test_scannability_metric() {
    let recommendations = generate_10_recommendations();
    let summary = format_summary_output(&recommendations);

    // Measure cognitive load
    let fact_count = count_distinct_facts(&summary);
    assert!(fact_count <= 50, "Too many facts: {}", fact_count); // 5 facts per item × 10

    // Measure visual hierarchy
    assert!(has_consistent_indentation(&summary));
    assert!(has_clear_visual_breaks(&summary));
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Format a technical debt recommendation in summary mode.
///
/// Summary mode compresses each recommendation to 3-4 lines showing:
/// - Score, severity, and impact estimate
/// - File path and key metrics
/// - Concrete action statement
/// - Progressive disclosure hint
///
/// # Example
/// ```
/// #1 SCORE: 370 [CRITICAL] IMPACT: -16-31% complexity
///   └─ god_object_detector.rs · functions: 55, responsibilities: 8
///   └─ ACTION: Split by analysis phase (data → detect → score → report)
///   └─ Run with --detail=1 for full analysis
/// ```
pub fn format_summary(&self) -> String
```

### User Documentation

Update README.md and docs/output-formats.md:

```markdown
## Terminal Output Modes

Debtmap provides two output modes for terminal display:

### Summary Mode (Default)

Compressed 3-4 line format per recommendation for quick scanning:

```
debtmap analyze .

TOP 10 RECOMMENDATIONS
#1 SCORE: 370 [CRITICAL] IMPACT: -16-31% complexity
  └─ god_object_detector.rs · functions: 55, responsibilities: 8
  └─ ACTION: Split by analysis phase (data → detect → score → report)
  └─ Run with --detail=1 for full analysis

#2 SCORE: 168 [CRITICAL] IMPACT: -20-35% complexity
  └─ god_object_analysis.rs · functions: 143, responsibilities: 10
  └─ ACTION: Split by data flow (input → logic → output)
  └─ Run with --detail=2 for full analysis
```

### Detail Mode

Comprehensive analysis with implementation guidance:

```bash
# Show detail for all items
debtmap analyze . --detail

# Show detail for specific item only
debtmap analyze . --detail=1

# Enable detail mode via environment variable
DEBTMAP_DETAIL=1 debtmap analyze .
```

Detail mode shows:
- Extended "WHY THIS MATTERS" explanation
- Component structure breakdown
- Implementation order recommendations
- Full metric details
- Dependency analysis

### Choosing a Mode

- **Use Summary (default)** when:
  - Getting quick overview of debt landscape
  - Comparing and prioritizing multiple items
  - Scanning for quick wins

- **Use Detail** when:
  - Planning implementation for specific item
  - Understanding architectural implications
  - Reviewing refactoring strategy
```

## Implementation Notes

### Implementation Order

1. **Create formatter module** with pure formatting functions
2. **Add ImpactEstimate extraction** from existing recommendation data
3. **Implement summary format** for each debt category
4. **Add CLI arguments** for detail mode
5. **Update terminal writer** to support both modes
6. **Add environment variable** support
7. **Write tests** for all formats and modes
8. **Update documentation**

### Edge Cases

1. **Very long file paths** - Truncate middle of path, keep filename: `src/.../very_long_name.rs`
2. **Missing impact estimates** - Show "Impact: TBD" or omit impact line
3. **Zero recommendations** - Show encouraging message
4. **Invalid --detail=N** - Show error: "Invalid item number N (only 1-10 available)"
5. **Terminal width < 80** - Gracefully wrap lines

### Refactoring Opportunities

Extract pure formatting functions following functional programming principles:

```rust
// PURE: Extract summary data
fn create_summary(rank: usize, rec: &DebtRecommendation) -> DebtItemSummary { ... }

// PURE: Format summary to string
fn format_summary_string(summary: &DebtItemSummary) -> String { ... }

// IMPURE: Write to terminal (I/O wrapper)
fn write_summary(&mut self, summary: &DebtItemSummary) -> Result<()> {
    writeln!(self.output, "{}", format_summary_string(summary))
}
```

## Migration and Compatibility

### Breaking Changes

**Potential**: Default terminal output format changes significantly.

**Mitigation**:
- Provide `DEBTMAP_DETAIL=1` environment variable for old behavior
- CI/CD scripts can set environment variable or use `--detail` flag
- JSON output unchanged (for programmatic parsing)

### Migration Path

For users/scripts relying on current terminal format:

1. **Immediate**: Set `DEBTMAP_DETAIL=1` in environment
2. **Short-term**: Update scripts to parse JSON output instead
3. **Long-term**: Adopt summary mode with selective detail viewing

### Version Compatibility

- Summary mode is new default in v0.7.0+
- Detail mode preserves current behavior exactly
- Environment variable provides escape hatch

## Success Metrics

- ✅ Default output shows 3-4 lines per recommendation
- ✅ All 10 items visible in ~40-50 terminal lines
- ✅ Users can scan all items in < 30 seconds (user testing)
- ✅ `--detail` flag shows comprehensive analysis
- ✅ `--detail=N` shows detail for specific item
- ✅ Environment variable `DEBTMAP_DETAIL=1` works
- ✅ Summary format consistent across all debt types
- ✅ Impact estimates prominently displayed
- ✅ Action statements remain actionable
- ✅ Progressive disclosure hints clear
- ✅ No performance regression
- ✅ Documentation updated with examples
- ✅ Tests cover all output modes

## Follow-up Work

After implementing this specification:

1. **Color coding for severity** - Red for CRITICAL, orange for HIGH, etc.
2. **Interactive mode** - Press Enter to expand item in-place
3. **Custom summary templates** - User-configurable format strings
4. **Filter by severity** - `--severity=critical` to show only critical items
5. **Export to clipboard** - Copy specific recommendation detail
6. **Watch mode** - Auto-refresh summary on file changes
7. **Quick fix mode** - Apply automated refactorings from summary

## References

- Design Analysis: Debtmap Terminal Output (parent document)
- src/io/writers/terminal.rs:current terminal output implementation
- src/priority/formatter.rs:recommendation formatting logic
- UX Principle: Information Scent (Jakob Nielsen)
- UX Principle: Progressive Disclosure (Apple Human Interface Guidelines)
