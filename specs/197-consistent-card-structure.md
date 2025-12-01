---
number: 197
title: Consistent Card Structure Across Debt Types
category: optimization
priority: medium
status: draft
dependencies: [194]
created: 2025-11-30
---

# Specification 197: Consistent Card Structure Across Debt Types

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 194 (Scannable Summary Mode)

## Context

Debtmap currently displays different information structure for different debt types:

**God Object** (8 subsections):
- File path, why it matters, action
- Structure, functions, largest components
- Implementation order
- Impact, metrics, scoring, dependencies

**Complex Function** (5 subsections):
- Location, impact, complexity
- Why it matters
- Recommended action

This inconsistency creates cognitive burden:
- **Unpredictable structure** - Users can't anticipate what information will appear
- **Harder to scan** - Different layouts require different scanning strategies
- **Inconsistent detail levels** - Some items verbose, others terse
- **Missing information** - Some debt types lack effort estimates or risk levels

Users must re-learn the format for each debt category, slowing comprehension and decision-making.

## Objective

Create a consistent card structure that:

1. **Uses same format across all debt types** - God objects, complex functions, coverage gaps, etc.
2. **Shows predictable information** - Same fields in same order every time
3. **Enables fast comparison** - Users can compare items side-by-side
4. **Includes decision-making context** - Effort, risk, impact for all items
5. **Maintains semantic clarity** - Format adapts to show relevant details per type

**Success Metric**: Users can scan any debt item and find the same information in the same location, regardless of debt category.

## Requirements

### Functional Requirements

1. **Standard Card Format**
   ```
   #N SCORE: X [SEVERITY] IMPACT: ±Y%
     ├─ FILE/LOCATION: <path>:<line> <entity>
     ├─ ISSUE: <one-sentence problem description>
     ├─ ACTION: <concrete next step>
     ├─ EFFORT: [S/M/L] · RISK: [LOW/MED/HIGH]
     └─ Detail: --detail=N for full analysis
   ```

2. **Required Fields (All Debt Types)**
   - **Score** - Numerical priority
   - **Severity** - CRITICAL, HIGH, MEDIUM, LOW
   - **Impact** - Estimated improvement (complexity %, coverage %, risk reduction)
   - **Location** - File path, line number, entity name
   - **Issue** - Single sentence describing the problem
   - **Action** - One concrete next step
   - **Effort** - Small (S), Medium (M), Large (L)
   - **Risk** - LOW, MEDIUM, HIGH (risk of refactoring)

3. **Debt-Specific Adaptation**
   - **God Object**: Location shows file, issue mentions function count
   - **Complex Function**: Location shows file:line and function name
   - **Coverage Gap**: Issue mentions tests needed, action specifies test count

4. **Effort Estimation**
   - **Small (S)**: 1-4 hours, single PR
   - **Medium (M)**: 1-3 days, multiple small PRs
   - **Large (L)**: 1+ weeks, phased approach

5. **Risk Assessment**
   - **LOW**: Well-tested, low coupling, clear boundaries
   - **MEDIUM**: Moderate coupling, some test coverage
   - **HIGH**: High coupling, low coverage, critical path

### Non-Functional Requirements

1. **Consistency**
   - Same field order across all debt types
   - Same formatting patterns (tree symbols, spacing)
   - Same terminology and labels

2. **Scannability**
   - Fields aligned vertically for easy column scanning
   - Visual hierarchy with tree symbols
   - Consistent indentation

3. **Completeness**
   - No debt type missing required fields
   - All items provide decision-making context
   - Effort and risk always included

4. **Flexibility**
   - Format extensible for new debt types
   - Field values adapt to context
   - Detail level controlled by flags

## Acceptance Criteria

- [ ] All debt types use identical card structure
- [ ] Score, severity, and impact on first line for all types
- [ ] Location field consistently formatted as "path:line entity"
- [ ] Issue field provides one-sentence problem description
- [ ] Action field gives concrete next step
- [ ] Effort estimate (S/M/L) included for all items
- [ ] Risk level (LOW/MED/HIGH) included for all items
- [ ] Detail hint shown for all items
- [ ] God object cards follow standard structure
- [ ] Complex function cards follow standard structure
- [ ] Coverage gap cards follow standard structure
- [ ] Large file cards follow standard structure
- [ ] Anti-pattern cards follow standard structure
- [ ] Visual hierarchy consistent with tree symbols
- [ ] Field alignment consistent across items
- [ ] User testing confirms improved comprehension vs. current format
- [ ] Documentation updated with card structure examples

## Technical Details

### Implementation Approach

**Phase 1: Define Standard Card Structure**

```rust
// src/io/formatters/card.rs (new module)

#[derive(Debug, Clone)]
pub struct DebtCard {
    pub rank: usize,
    pub score: f64,
    pub severity: Severity,
    pub impact: ImpactEstimate,
    pub location: Location,
    pub issue: String,
    pub action: String,
    pub effort: EffortEstimate,
    pub risk: RiskLevel,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub file_path: PathBuf,
    pub line: Option<usize>,
    pub entity_name: Option<String>,
}

impl Location {
    pub fn format(&self) -> String {
        match (&self.line, &self.entity_name) {
            (Some(line), Some(name)) => {
                format!("{}:{} {}", self.file_path.display(), line, name)
            }
            (Some(line), None) => {
                format!("{}:{}", self.file_path.display(), line)
            }
            (None, Some(name)) => {
                format!("{} · {}", self.file_path.display(), name)
            }
            (None, None) => {
                format!("{}", self.file_path.display())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EffortEstimate {
    Small,   // S: 1-4 hours
    Medium,  // M: 1-3 days
    Large,   // L: 1+ weeks
}

impl EffortEstimate {
    pub fn as_str(&self) -> &'static str {
        match self {
            EffortEstimate::Small => "S",
            EffortEstimate::Medium => "M",
            EffortEstimate::Large => "L",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EffortEstimate::Small => "1-4 hours",
            EffortEstimate::Medium => "1-3 days",
            EffortEstimate::Large => "1+ weeks",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RiskLevel {
    Low,     // Well-tested, low coupling
    Medium,  // Moderate coupling, some tests
    High,    // High coupling, low coverage
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MED",
            RiskLevel::High => "HIGH",
        }
    }
}

impl DebtCard {
    pub fn format_summary(&self) -> String {
        format!(
            "#{rank} SCORE: {score} [{severity}] IMPACT: {impact}\n\
             ├─ LOCATION: {location}\n\
             ├─ ISSUE: {issue}\n\
             ├─ ACTION: {action}\n\
             ├─ EFFORT: {effort} · RISK: {risk}\n\
             └─ Run with --detail={rank} for full analysis\n",
            rank = self.rank,
            score = self.score,
            severity = self.severity,
            impact = self.impact,
            location = self.location.format(),
            issue = self.issue,
            action = self.action,
            effort = self.effort.as_str(),
            risk = self.risk.as_str(),
        )
    }
}
```

**Phase 2: Create Card Builders per Debt Type**

```rust
// src/io/formatters/card_builders.rs

impl DebtCard {
    pub fn from_god_object(
        rank: usize,
        god_object: &GodObjectRecommendation,
    ) -> Self {
        DebtCard {
            rank,
            score: god_object.score,
            severity: god_object.severity,
            impact: ImpactEstimate {
                complexity_reduction: Some((16, 31)),
                coverage_increase: None,
                risk_reduction: None,
            },
            location: Location {
                file_path: god_object.file_path.clone(),
                line: None,
                entity_name: None,
            },
            issue: format!(
                "{} functions across {} responsibilities in single module",
                god_object.function_count,
                god_object.responsibility_count
            ),
            action: generate_god_object_action(god_object),
            effort: estimate_god_object_effort(god_object),
            risk: assess_god_object_risk(god_object),
        }
    }

    pub fn from_complex_function(
        rank: usize,
        func: &ComplexFunctionRecommendation,
    ) -> Self {
        DebtCard {
            rank,
            score: func.score,
            severity: func.severity,
            impact: ImpactEstimate {
                complexity_reduction: Some((func.complexity_reduction_min, func.complexity_reduction_max)),
                coverage_increase: None,
                risk_reduction: Some(func.risk_reduction),
            },
            location: Location {
                file_path: func.file_path.clone(),
                line: Some(func.line_number),
                entity_name: Some(func.function_name.clone()),
            },
            issue: format!(
                "Complexity {} (cognitive: {}, cyclomatic: {})",
                func.complexity_level,
                func.cognitive_complexity,
                func.cyclomatic_complexity
            ),
            action: func.recommended_action.clone(),
            effort: estimate_function_effort(func),
            risk: assess_function_risk(func),
        }
    }

    pub fn from_coverage_gap(
        rank: usize,
        gap: &CoverageGapRecommendation,
    ) -> Self {
        DebtCard {
            rank,
            score: gap.score,
            severity: gap.severity,
            impact: ImpactEstimate {
                complexity_reduction: None,
                coverage_increase: Some(gap.coverage_increase),
                risk_reduction: None,
            },
            location: Location {
                file_path: gap.file_path.clone(),
                line: Some(gap.line_number),
                entity_name: Some(gap.function_name.clone()),
            },
            issue: format!(
                "{}% coverage with complexity {} (needs {} tests)",
                gap.current_coverage,
                gap.complexity,
                gap.tests_needed
            ),
            action: format!("Add {} tests for untested branches", gap.tests_needed),
            effort: estimate_coverage_effort(gap),
            risk: RiskLevel::Low, // Adding tests is low risk
        }
    }
}

fn estimate_god_object_effort(god_object: &GodObjectRecommendation) -> EffortEstimate {
    // Estimate based on function count and responsibilities
    match (god_object.function_count, god_object.responsibility_count) {
        (0..=20, 0..=3) => EffortEstimate::Small,
        (0..=50, 0..=5) => EffortEstimate::Medium,
        _ => EffortEstimate::Large,
    }
}

fn assess_god_object_risk(god_object: &GodObjectRecommendation) -> RiskLevel {
    // Assess based on coupling and test coverage
    if god_object.has_high_coupling {
        RiskLevel::High
    } else if god_object.test_coverage < 50.0 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

fn estimate_function_effort(func: &ComplexFunctionRecommendation) -> EffortEstimate {
    // Estimate based on complexity reduction needed
    match func.complexity_reduction_max {
        0..=20 => EffortEstimate::Small,
        21..=50 => EffortEstimate::Medium,
        _ => EffortEstimate::Large,
    }
}

fn assess_function_risk(func: &ComplexFunctionRecommendation) -> RiskLevel {
    // Assess based on coverage and coupling
    if func.test_coverage < 50.0 {
        RiskLevel::High
    } else if func.test_coverage < 75.0 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

fn estimate_coverage_effort(gap: &CoverageGapRecommendation) -> EffortEstimate {
    // Estimate based on tests needed
    match gap.tests_needed {
        0..=5 => EffortEstimate::Small,
        6..=15 => EffortEstimate::Medium,
        _ => EffortEstimate::Large,
    }
}
```

**Phase 3: Integrate into Terminal Writer**

```rust
// src/io/writers/terminal.rs

impl TerminalWriter {
    pub fn write_recommendations_with_cards(
        &mut self,
        recommendations: &[DebtRecommendation],
    ) -> Result<()> {
        for (idx, rec) in recommendations.iter().enumerate() {
            let card = match rec {
                DebtRecommendation::GodObject(go) => {
                    DebtCard::from_god_object(idx + 1, go)
                }
                DebtRecommendation::ComplexFunction(cf) => {
                    DebtCard::from_complex_function(idx + 1, cf)
                }
                DebtRecommendation::CoverageGap(cg) => {
                    DebtCard::from_coverage_gap(idx + 1, cg)
                }
                // ... other debt types
            };

            writeln!(self.output, "{}", card.format_summary())?;
        }
        Ok(())
    }
}
```

### Example Output

**God Object**:
```
#1 SCORE: 370 [CRITICAL] IMPACT: -16-31% complexity
├─ LOCATION: src/organization/god_object_detector.rs
├─ ISSUE: 55 functions across 8 responsibilities in single module
├─ ACTION: Split by analysis phase (data → detect → score → report)
├─ EFFORT: L · RISK: HIGH
└─ Run with --detail=1 for full analysis
```

**Complex Function**:
```
#5 SCORE: 30.6 [CRITICAL] IMPACT: -12% complexity, -7.8 risk
├─ LOCATION: src/organization/god_object_detector.rs:755 analyze_domains_and_recommend_splits()
├─ ISSUE: Complexity Very High (cognitive: 77, cyclomatic: 25)
├─ ACTION: Reduce nesting from 5 to 2 levels
├─ EFFORT: M · RISK: MED
└─ Run with --detail=5 for full analysis
```

**Coverage Gap**:
```
#4 SCORE: 18.4 [CRITICAL] IMPACT: +50% coverage, -6% complexity
├─ LOCATION: src/commands/explain_coverage.rs:275 output_text_format()
├─ ISSUE: 0% coverage with complexity 21 (needs 9 tests)
├─ ACTION: Add 9 tests for untested branches
├─ EFFORT: S · RISK: LOW
└─ Run with --detail=4 for full analysis
```

### Architecture Changes

New modules:
- `src/io/formatters/card.rs` - Standard card structure
- `src/io/formatters/card_builders.rs` - Card builders per debt type
- `src/io/formatters/effort.rs` - Effort estimation logic
- `src/io/formatters/risk.rs` - Risk assessment logic

Modified files:
- `src/io/writers/terminal.rs` - Use card-based formatting
- `src/priority/recommendations.rs` - Extract effort/risk data

### Data Structures

```rust
pub struct DebtCard {
    pub rank: usize,
    pub score: f64,
    pub severity: Severity,
    pub impact: ImpactEstimate,
    pub location: Location,
    pub issue: String,
    pub action: String,
    pub effort: EffortEstimate,
    pub risk: RiskLevel,
}

pub struct Location {
    pub file_path: PathBuf,
    pub line: Option<usize>,
    pub entity_name: Option<String>,
}

pub enum EffortEstimate {
    Small,   // S
    Medium,  // M
    Large,   // L
}

pub enum RiskLevel {
    Low,
    Medium,
    High,
}
```

## Dependencies

- **Prerequisites**: Spec 194 (Scannable Summary Mode)
- **Affected Components**:
  - `src/io/writers/terminal.rs`
  - `src/priority/recommendations.rs`
  - All debt recommendation types
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_card_format_consistency() {
        let god_object_card = DebtCard::from_god_object(1, &create_test_god_object());
        let function_card = DebtCard::from_complex_function(2, &create_test_function());

        let go_lines: Vec<&str> = god_object_card.format_summary().lines().collect();
        let fn_lines: Vec<&str> = function_card.format_summary().lines().collect();

        // Both should have same number of lines
        assert_eq!(go_lines.len(), fn_lines.len());

        // Both should have same field order
        assert!(go_lines[1].starts_with("├─ LOCATION:"));
        assert!(fn_lines[1].starts_with("├─ LOCATION:"));

        assert!(go_lines[2].starts_with("├─ ISSUE:"));
        assert!(fn_lines[2].starts_with("├─ ISSUE:"));

        assert!(go_lines[3].starts_with("├─ ACTION:"));
        assert!(fn_lines[3].starts_with("├─ ACTION:"));

        assert!(go_lines[4].starts_with("├─ EFFORT:"));
        assert!(fn_lines[4].starts_with("├─ EFFORT:"));
    }

    #[test]
    fn test_effort_estimation() {
        let small_go = create_god_object(20, 3);
        assert_eq!(estimate_god_object_effort(&small_go), EffortEstimate::Small);

        let large_go = create_god_object(100, 10);
        assert_eq!(estimate_god_object_effort(&large_go), EffortEstimate::Large);
    }

    #[test]
    fn test_risk_assessment() {
        let high_coupling = create_god_object_with_coupling(true);
        assert_eq!(assess_god_object_risk(&high_coupling), RiskLevel::High);

        let low_coverage = create_function_with_coverage(30.0);
        assert_eq!(assess_function_risk(&low_coverage), RiskLevel::High);
    }
}
```

## Success Metrics

- ✅ All debt types use identical card structure
- ✅ Same field order across all cards
- ✅ Effort and risk included for all items
- ✅ User testing confirms improved comprehension
- ✅ Visual hierarchy consistent
- ✅ Tests cover all debt types

## Follow-up Work

1. **Customizable card templates**
2. **Additional fields** (e.g., last modified date)
3. **Card sorting** by effort, risk, or impact
4. **Card filtering** by effort or risk level

## References

- Spec 194: Scannable Summary Mode
- Design Analysis: Debtmap Terminal Output
- src/priority/recommendations.rs
