---
number: 196
title: Metric Humanization for Complexity Analysis
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 196: Metric Humanization for Complexity Analysis

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently displays raw technical metrics that are difficult for most developers to interpret:

```
├─ COMPLEXITY: cyclomatic=25 (dampened: 12, factor: 0.52), est_branches=25, cognitive=77, nesting=5, entropy=0.26
```

**Problems with current metrics display**:

1. **Opacity** - Developers don't know if entropy=0.26 is good or bad
2. **Implementation details exposed** - "dampened: 12, factor: 0.52" are internal calculations
3. **No actionable context** - What does cyclomatic=25 mean for my work?
4. **Requires expertise** - Only complexity theory experts can interpret these values
5. **Missing interpretation** - Numbers without context don't guide decisions

**User questions that metrics fail to answer**:
- "Is this function hard to understand?"
- "Is it risky to modify this code?"
- "Should I prioritize refactoring this?"
- "What specific problem does this metric indicate?"

This creates a barrier to adoption, as developers need to:
1. Research complexity metrics
2. Memorize threshold values
3. Understand dampening factors
4. Interpret combinations of metrics

## Objective

Humanize complexity metrics by:

1. **Translating numbers to interpretations** - "Very High" instead of raw scores
2. **Providing actionable context** - "Hard to modify safely" explains the risk
3. **Hiding implementation details** - Move dampening factors to expert mode
4. **Adding qualitative assessments** - Combine metrics into understandable statements
5. **Maintaining expert access** - `--metrics` flag for raw values

**Success Metric**: Non-expert developers can understand and act on complexity information without researching metric definitions.

## Requirements

### Functional Requirements

1. **Qualitative Complexity Levels**
   - Replace raw complexity numbers with interpretive levels:
     - **Very Low** (cognitive: 1-5)
     - **Low** (cognitive: 6-10)
     - **Moderate** (cognitive: 11-20)
     - **High** (cognitive: 21-40)
     - **Very High** (cognitive: 41+)
   - Show cognitive complexity as primary metric (most intuitive)
   - Include cyclomatic and nesting as supporting detail

2. **Risk and Difficulty Statements**
   - Translate metric combinations into risk assessments:
     - **Low cyclomatic + low nesting** → "Easy to modify"
     - **High cyclomatic + low nesting** → "Many decision points"
     - **Low cyclomatic + high nesting** → "Deep nesting complicates logic"
     - **High cyclomatic + high nesting** → "Hard to modify safely"
   - Use natural language that explains why complexity matters

3. **Primary Issue Identification**
   - Identify and highlight the dominant complexity driver:
     - **Nesting-driven** - "Deep nesting (depth 5)"
     - **Branching-driven** - "Many decision points (25 branches)"
     - **Mixed** - "Both nesting (5 levels) and branching (25 decisions)"
   - Direct attention to specific refactoring target

4. **Progressive Disclosure for Metrics**
   - **Default mode**: Qualitative assessment + dominant issue
   - **`--metrics` flag**: Show all raw values with explanations
   - **Detail mode**: Include metric formulas and calculations

5. **Contextual Explanations**
   - Explain what each level means in practice:
     - Very High: "Difficult for anyone to understand and modify"
     - High: "Requires significant mental effort to comprehend"
     - Moderate: "Understandable with focused attention"
     - Low: "Easy to understand at a glance"

### Non-Functional Requirements

1. **Accessibility**
   - Clear to developers without complexity theory background
   - No jargon unless explained
   - Actionable without additional research

2. **Consistency**
   - Same thresholds and interpretations across all output
   - Consistent terminology and phrasing
   - Predictable structure

3. **Accuracy**
   - Interpretations align with research (McCabe, cognitive complexity studies)
   - Thresholds match industry standards
   - Risk assessments reflect actual modification difficulty

4. **Maintainability**
   - Thresholds configurable without code changes
   - Interpretation templates extensible
   - Easy to adjust based on user feedback

## Acceptance Criteria

- [ ] Complexity displayed as qualitative levels (Very Low to Very High)
- [ ] Cognitive complexity used as primary metric
- [ ] Risk/difficulty statements explain practical implications
- [ ] Primary complexity driver identified (nesting, branching, or mixed)
- [ ] Default output hides raw numbers
- [ ] `--metrics` flag shows raw values with explanations
- [ ] Contextual explanations clarify what each level means
- [ ] Thresholds match industry standards (cyclomatic >10 = high, cognitive >15 = high)
- [ ] Nesting depth thresholds: >3 = high, >4 = very high
- [ ] Entropy interpretation removed from default (implementation detail)
- [ ] Dampening factor hidden in default mode
- [ ] Risk statements combine multiple metrics appropriately
- [ ] Output tested with non-expert developers for comprehension
- [ ] Documentation explains metric interpretation
- [ ] Backward compatibility: JSON output still includes raw values

## Technical Details

### Implementation Approach

**Phase 1: Define Interpretation Thresholds**

```rust
// src/io/humanize/complexity.rs (new module)

#[derive(Debug, Clone, Copy)]
pub enum ComplexityLevel {
    VeryLow,
    Low,
    Moderate,
    High,
    VeryHigh,
}

impl ComplexityLevel {
    pub fn from_cognitive(cognitive: u32) -> Self {
        match cognitive {
            0..=5 => ComplexityLevel::VeryLow,
            6..=10 => ComplexityLevel::Low,
            11..=20 => ComplexityLevel::Moderate,
            21..=40 => ComplexityLevel::High,
            _ => ComplexityLevel::VeryHigh,
        }
    }

    pub fn from_cyclomatic(cyclomatic: u32) -> Self {
        match cyclomatic {
            0..=5 => ComplexityLevel::VeryLow,
            6..=10 => ComplexityLevel::Low,
            11..=20 => ComplexityLevel::Moderate,
            21..=40 => ComplexityLevel::High,
            _ => ComplexityLevel::VeryHigh,
        }
    }

    pub fn from_nesting(nesting: u32) -> Self {
        match nesting {
            0..=2 => ComplexityLevel::Low,
            3 => ComplexityLevel::Moderate,
            4 => ComplexityLevel::High,
            _ => ComplexityLevel::VeryHigh,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ComplexityLevel::VeryLow => "Very Low",
            ComplexityLevel::Low => "Low",
            ComplexityLevel::Moderate => "Moderate",
            ComplexityLevel::High => "High",
            ComplexityLevel::VeryHigh => "Very High",
        }
    }

    pub fn explanation(&self) -> &'static str {
        match self {
            ComplexityLevel::VeryLow => "Trivial to understand and modify",
            ComplexityLevel::Low => "Easy to understand at a glance",
            ComplexityLevel::Moderate => "Understandable with focused attention",
            ComplexityLevel::High => "Requires significant mental effort to comprehend",
            ComplexityLevel::VeryHigh => "Difficult for anyone to understand and modify",
        }
    }
}
```

**Phase 2: Create Humanized Complexity Summary**

```rust
// src/io/humanize/complexity.rs

#[derive(Debug)]
pub struct ComplexitySummary {
    pub level: ComplexityLevel,
    pub primary_driver: ComplexityDriver,
    pub difficulty_statement: String,
}

#[derive(Debug)]
pub enum ComplexityDriver {
    Nesting { depth: u32 },
    Branching { count: u32 },
    Mixed { nesting: u32, branching: u32 },
    Acceptable, // Below concerning thresholds
}

impl ComplexitySummary {
    pub fn from_metrics(metrics: &FunctionMetrics) -> Self {
        let cognitive_level = ComplexityLevel::from_cognitive(metrics.cognitive_complexity);
        let cyclomatic_level = ComplexityLevel::from_cyclomatic(metrics.cyclomatic_complexity);
        let nesting_level = ComplexityLevel::from_nesting(metrics.max_nesting_depth);

        // Determine primary driver
        let primary_driver = identify_primary_driver(
            metrics.cyclomatic_complexity,
            metrics.max_nesting_depth,
        );

        // Create difficulty statement
        let difficulty_statement = generate_difficulty_statement(
            cyclomatic_level,
            nesting_level,
        );

        ComplexitySummary {
            level: cognitive_level,
            primary_driver,
            difficulty_statement,
        }
    }

    pub fn format_default(&self) -> String {
        format!(
            "COMPLEXITY: {} ({})\nDIFFICULTY: {}\n{}",
            self.level.as_str(),
            self.level.explanation(),
            self.difficulty_statement,
            self.format_driver_hint(),
        )
    }

    pub fn format_with_metrics(&self, metrics: &FunctionMetrics) -> String {
        format!(
            "COMPLEXITY: {} ({} cognitive, {} cyclomatic, {} nesting)\nDIFFICULTY: {}\n{}",
            self.level.as_str(),
            metrics.cognitive_complexity,
            metrics.cyclomatic_complexity,
            metrics.max_nesting_depth,
            self.difficulty_statement,
            self.format_driver_hint(),
        )
    }

    fn format_driver_hint(&self) -> String {
        match &self.primary_driver {
            ComplexityDriver::Nesting { depth } => {
                format!("└─ Primary issue: Deep nesting ({} levels)", depth)
            }
            ComplexityDriver::Branching { count } => {
                format!("└─ Primary issue: Many decision points ({} branches)", count)
            }
            ComplexityDriver::Mixed { nesting, branching } => {
                format!(
                    "└─ Primary issues: Both nesting ({} levels) and branching ({} decisions)",
                    nesting, branching
                )
            }
            ComplexityDriver::Acceptable => String::new(),
        }
    }
}

fn identify_primary_driver(cyclomatic: u32, nesting: u32) -> ComplexityDriver {
    let cyclomatic_high = cyclomatic > 10;
    let nesting_high = nesting > 3;

    match (cyclomatic_high, nesting_high) {
        (true, true) => ComplexityDriver::Mixed {
            nesting,
            branching: cyclomatic,
        },
        (true, false) => ComplexityDriver::Branching { count: cyclomatic },
        (false, true) => ComplexityDriver::Nesting { depth: nesting },
        (false, false) => ComplexityDriver::Acceptable,
    }
}

fn generate_difficulty_statement(
    cyclomatic_level: ComplexityLevel,
    nesting_level: ComplexityLevel,
) -> String {
    use ComplexityLevel::*;

    match (cyclomatic_level, nesting_level) {
        (VeryHigh, VeryHigh) => "Extremely hard to modify safely".to_string(),
        (VeryHigh, _) | (_, VeryHigh) => "Hard to modify safely".to_string(),
        (High, High) => "Moderately difficult to modify".to_string(),
        (High, _) => "Many decision points require careful handling".to_string(),
        (_, High) => "Deep nesting complicates logic flow".to_string(),
        (Moderate, Moderate) => "Requires focused attention to modify".to_string(),
        _ => "Easy to understand and modify".to_string(),
    }
}
```

**Phase 3: Integrate into Terminal Output**

```rust
// src/io/writers/terminal.rs

impl TerminalWriter {
    fn write_complexity_info(
        &mut self,
        metrics: &FunctionMetrics,
        show_metrics: bool,
    ) -> Result<()> {
        let summary = ComplexitySummary::from_metrics(metrics);

        let output = if show_metrics {
            summary.format_with_metrics(metrics)
        } else {
            summary.format_default()
        };

        writeln!(self.output, "{}", output)?;
        Ok(())
    }
}
```

### Example Output Comparison

**Before (Technical)**:
```
├─ COMPLEXITY: cyclomatic=25 (dampened: 12, factor: 0.52), est_branches=25, cognitive=77, nesting=5, entropy=0.26
├─ WHY THIS MATTERS: Deep nesting (depth 5) drives cognitive complexity to 77. Cognitive/Cyclomatic ratio of 3.1x confirms nesting is primary issue.
```

**After (Humanized - Default)**:
```
├─ COMPLEXITY: Very High (Difficult for anyone to understand and modify)
├─ DIFFICULTY: Hard to modify safely
└─ Primary issue: Deep nesting (5 levels)
```

**After (With --metrics flag)**:
```
├─ COMPLEXITY: Very High (77 cognitive, 25 cyclomatic, 5 nesting)
├─ DIFFICULTY: Hard to modify safely
└─ Primary issue: Deep nesting (5 levels)
└─ Raw metrics: cyclomatic=25, cognitive=77, nesting=5, entropy=0.26
```

### Architecture Changes

New modules:
- `src/io/humanize/` - Metric humanization logic
  - `complexity.rs` - Complexity interpretation
  - `thresholds.rs` - Configurable threshold definitions
  - `mod.rs` - Module exports

Modified files:
- `src/io/writers/terminal.rs` - Use humanized output
- `src/io/writers/markdown/core.rs` - Add humanized complexity columns
- `src/commands/analyze.rs` - Add `--metrics` flag

### Data Structures

```rust
// src/io/humanize/complexity.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityLevel {
    VeryLow,
    Low,
    Moderate,
    High,
    VeryHigh,
}

#[derive(Debug, Clone)]
pub enum ComplexityDriver {
    Nesting { depth: u32 },
    Branching { count: u32 },
    Mixed { nesting: u32, branching: u32 },
    Acceptable,
}

#[derive(Debug, Clone)]
pub struct ComplexitySummary {
    pub level: ComplexityLevel,
    pub primary_driver: ComplexityDriver,
    pub difficulty_statement: String,
}

// src/io/humanize/thresholds.rs

#[derive(Debug, Clone)]
pub struct ComplexityThresholds {
    pub cognitive: LevelThresholds,
    pub cyclomatic: LevelThresholds,
    pub nesting: LevelThresholds,
}

#[derive(Debug, Clone)]
pub struct LevelThresholds {
    pub very_low: u32,
    pub low: u32,
    pub moderate: u32,
    pub high: u32,
    // very_high is anything above 'high'
}

impl Default for ComplexityThresholds {
    fn default() -> Self {
        Self {
            cognitive: LevelThresholds {
                very_low: 5,
                low: 10,
                moderate: 20,
                high: 40,
            },
            cyclomatic: LevelThresholds {
                very_low: 5,
                low: 10,
                moderate: 20,
                high: 40,
            },
            nesting: LevelThresholds {
                very_low: 1,
                low: 2,
                moderate: 3,
                high: 4,
            },
        }
    }
}
```

### APIs and Interfaces

```rust
// src/io/humanize/complexity.rs

/// Create humanized complexity summary from function metrics
pub fn humanize_complexity(metrics: &FunctionMetrics) -> ComplexitySummary;

/// Format complexity for default display (no raw numbers)
pub fn format_complexity_default(summary: &ComplexitySummary) -> String;

/// Format complexity with raw metrics included
pub fn format_complexity_with_metrics(
    summary: &ComplexitySummary,
    metrics: &FunctionMetrics,
) -> String;

/// Identify primary complexity driver
pub fn identify_primary_driver(cyclomatic: u32, nesting: u32) -> ComplexityDriver;

/// Generate difficulty statement from metrics
pub fn generate_difficulty_statement(
    cyclomatic_level: ComplexityLevel,
    nesting_level: ComplexityLevel,
) -> String;
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/io/writers/terminal.rs` - Terminal output
  - `src/io/writers/markdown/core.rs` - Markdown output
  - `src/priority/formatter.rs` - Recommendation formatting
  - `src/commands/analyze.rs` - CLI arguments
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
// src/io/humanize/complexity.rs

#[cfg(test)]
mod tests {
    #[test]
    fn test_complexity_level_from_cognitive() {
        assert_eq!(ComplexityLevel::from_cognitive(3), ComplexityLevel::VeryLow);
        assert_eq!(ComplexityLevel::from_cognitive(8), ComplexityLevel::Low);
        assert_eq!(ComplexityLevel::from_cognitive(15), ComplexityLevel::Moderate);
        assert_eq!(ComplexityLevel::from_cognitive(30), ComplexityLevel::High);
        assert_eq!(ComplexityLevel::from_cognitive(50), ComplexityLevel::VeryHigh);
    }

    #[test]
    fn test_identify_primary_driver_nesting() {
        let driver = identify_primary_driver(5, 5);
        assert!(matches!(driver, ComplexityDriver::Nesting { depth: 5 }));
    }

    #[test]
    fn test_identify_primary_driver_branching() {
        let driver = identify_primary_driver(25, 2);
        assert!(matches!(driver, ComplexityDriver::Branching { count: 25 }));
    }

    #[test]
    fn test_identify_primary_driver_mixed() {
        let driver = identify_primary_driver(25, 5);
        assert!(matches!(driver, ComplexityDriver::Mixed { .. }));
    }

    #[test]
    fn test_difficulty_statement_very_high() {
        let stmt = generate_difficulty_statement(
            ComplexityLevel::VeryHigh,
            ComplexityLevel::VeryHigh,
        );
        assert_eq!(stmt, "Extremely hard to modify safely");
    }

    #[test]
    fn test_difficulty_statement_mixed_high() {
        let stmt = generate_difficulty_statement(
            ComplexityLevel::High,
            ComplexityLevel::High,
        );
        assert_eq!(stmt, "Moderately difficult to modify");
    }

    #[test]
    fn test_format_default_hides_numbers() {
        let summary = ComplexitySummary {
            level: ComplexityLevel::High,
            primary_driver: ComplexityDriver::Nesting { depth: 5 },
            difficulty_statement: "Hard to modify safely".to_string(),
        };

        let formatted = summary.format_default();

        assert!(formatted.contains("Very High"));
        assert!(!formatted.contains("cyclomatic="));
        assert!(!formatted.contains("cognitive="));
        assert!(formatted.contains("Deep nesting (5 levels)"));
    }
}
```

### User Comprehension Tests

```rust
#[test]
fn test_user_comprehension_levels() {
    // Test that interpretations are clear to non-experts
    let test_cases = vec![
        (3, "Very Low", "Trivial to understand"),
        (15, "Moderate", "focused attention"),
        (50, "Very High", "Difficult for anyone"),
    ];

    for (cognitive, expected_level, expected_phrase) in test_cases {
        let level = ComplexityLevel::from_cognitive(cognitive);
        assert!(level.as_str().contains(expected_level));
        assert!(level.explanation().contains(expected_phrase));
    }
}
```

### Integration Tests

```rust
// tests/humanized_output_test.rs

#[test]
fn test_terminal_output_humanized() {
    let test_file = "tests/fixtures/complex_function.rs";
    let results = analyze_file(test_file).unwrap();

    let mut buffer = Vec::new();
    let mut writer = TerminalWriter::new(&mut buffer);
    writer.write_results(&results, false).unwrap();

    let output = String::from_utf8(buffer).unwrap();

    // Should show humanized complexity
    assert!(output.contains("COMPLEXITY: Very High"));
    assert!(output.contains("DIFFICULTY:"));
    assert!(output.contains("Primary issue:"));

    // Should NOT show raw numbers in default mode
    assert!(!output.contains("cyclomatic="));
    assert!(!output.contains("cognitive="));
}

#[test]
fn test_metrics_flag_shows_raw_values() {
    let test_file = "tests/fixtures/complex_function.rs";
    let results = analyze_file(test_file).unwrap();

    let mut buffer = Vec::new();
    let mut writer = TerminalWriter::new(&mut buffer);
    writer.write_results(&results, true).unwrap(); // show_metrics = true

    let output = String::from_utf8(buffer).unwrap();

    // Should show both humanized AND raw
    assert!(output.contains("COMPLEXITY: Very High"));
    assert!(output.contains("cyclomatic="));
    assert!(output.contains("cognitive="));
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Complexity level classifications based on industry thresholds.
///
/// Thresholds derived from:
/// - McCabe (1976): Cyclomatic complexity >10 = high risk
/// - SonarQube: Cognitive complexity >15 = high
/// - Industry practice: Nesting depth >3 = problematic
///
/// # Levels
/// - **VeryLow**: Trivial to understand (cognitive 0-5)
/// - **Low**: Easy to understand (cognitive 6-10)
/// - **Moderate**: Requires focus (cognitive 11-20)
/// - **High**: Significant effort (cognitive 21-40)
/// - **VeryHigh**: Very difficult (cognitive 41+)
pub enum ComplexityLevel { ... }
```

### User Documentation

Update README.md and docs/metrics.md:

```markdown
## Understanding Complexity Metrics

Debtmap translates technical complexity metrics into understandable assessments.

### Complexity Levels

- **Very Low**: Trivial to understand and modify
- **Low**: Easy to understand at a glance
- **Moderate**: Understandable with focused attention
- **High**: Requires significant mental effort to comprehend
- **Very High**: Difficult for anyone to understand and modify

### Difficulty Statements

Debtmap combines multiple metrics to describe modification risk:

- **Easy to understand and modify**: Low complexity across all metrics
- **Many decision points require careful handling**: High branching, low nesting
- **Deep nesting complicates logic flow**: Low branching, high nesting
- **Hard to modify safely**: High complexity in multiple dimensions

### Primary Issues

Identifies the dominant complexity driver:

- **Deep nesting (N levels)**: Focus on flattening nested conditionals
- **Many decision points (N branches)**: Consider extracting decision logic
- **Both nesting and branching**: Requires comprehensive refactoring

### Viewing Raw Metrics

Use `--metrics` flag to see raw values:

```
debtmap analyze . --metrics
```

Output includes technical details:
```
├─ COMPLEXITY: Very High (77 cognitive, 25 cyclomatic, 5 nesting)
├─ DIFFICULTY: Hard to modify safely
└─ Primary issue: Deep nesting (5 levels)
└─ Raw metrics: cyclomatic=25, cognitive=77, nesting=5, entropy=0.26
```
```

## Implementation Notes

### Implementation Order

1. **Create humanization module** with complexity interpretation
2. **Define thresholds** based on research and industry standards
3. **Implement difficulty statement generation**
4. **Add primary driver identification**
5. **Integrate into terminal writer**
6. **Add `--metrics` CLI flag**
7. **Update all output formats** (terminal, markdown)
8. **Test with non-expert developers** for comprehension
9. **Update documentation**

### Threshold Calibration

Research-based thresholds:

**Cyclomatic Complexity** (McCabe, 1976):
- 1-10: Simple, low risk
- 11-20: Moderate, medium risk
- 21-50: Complex, high risk
- 50+: Untestable, very high risk

**Cognitive Complexity** (SonarSource):
- 1-10: Very understandable
- 11-15: Understandable
- 16-25: Hard to understand
- 25+: Very hard to understand

**Nesting Depth** (Industry practice):
- 0-2: Acceptable
- 3: Concerning
- 4: High complexity
- 5+: Very high complexity

### Edge Cases

1. **Zero complexity** - Handle functions with complexity=0 gracefully
2. **Extreme values** - Cap display at reasonable maximums (e.g., "100+")
3. **Conflicting signals** - When cognitive is low but cyclomatic is high
4. **Missing metrics** - Handle incomplete metric data
5. **Custom thresholds** - Future: Allow user-configured thresholds

## Migration and Compatibility

### Breaking Changes

**Terminal output format changes** - Default display no longer shows raw numbers.

**Mitigation**:
- `--metrics` flag preserves access to raw values
- JSON output unchanged (programmatic access preserved)
- Markdown output can include both humanized and raw

### Migration Path

For users expecting raw metrics:
1. Use `--metrics` flag for current behavior
2. Update scripts to parse JSON output
3. Configure environment variable (future) for default raw display

## Success Metrics

- ✅ Complexity levels clearly defined and tested
- ✅ Difficulty statements generated from metrics
- ✅ Primary driver identification working
- ✅ Default output shows humanized metrics
- ✅ `--metrics` flag shows raw values
- ✅ Thresholds match industry standards
- ✅ Non-expert developers can understand output (user testing)
- ✅ Documentation explains all levels and statements
- ✅ Backward compatibility via JSON output
- ✅ Tests cover all complexity ranges

## Follow-up Work

1. **Configurable thresholds** - Allow users to customize via config file
2. **Language-specific thresholds** - Different thresholds per language
3. **Historical context** - "This is above your project average"
4. **Visual indicators** - Color coding for severity levels
5. **Comparative statements** - "Twice as complex as typical function"
6. **Explanation on demand** - `--explain` flag for metric definitions
7. **Team calibration** - Learn team-specific thresholds from feedback

## References

- McCabe, T. J. (1976). "A Complexity Measure"
- SonarSource: Cognitive Complexity specification
- Martin, Robert C. "Clean Code" - Complexity thresholds
- Design Analysis: Debtmap Terminal Output (parent document)
- src/core/mod.rs - FunctionMetrics structure
- src/io/writers/terminal.rs - Current complexity display
