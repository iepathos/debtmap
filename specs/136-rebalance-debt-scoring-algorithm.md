---
number: 136
title: Rebalance Debt Scoring Algorithm
category: optimization
priority: high
status: draft
dependencies: [134, 135]
created: 2025-10-27
---

# Specification 136: Rebalance Debt Scoring Algorithm

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 134 (metric consistency), Spec 135 (context-aware thresholds)

## Context

The current debt scoring algorithm heavily over-weights file size, causing low-quality prioritization:

**Current Ranking (Ripgrep Analysis)**:
- #1: Score 474 (CRITICAL) - Just large file (7775 lines, no complexity info)
- #2: Score 174 (CRITICAL) - Large file with contradictory metrics
- #3: Score 15.8 (CRITICAL) - **Actually critical**: Complexity 42, 38% coverage, real code quality issue

**The Problem**:
- Pure file size issues score 30x higher than actual code quality problems
- Complexity 42 with 62% coverage gap ranks below "file is big"
- Users see size warnings first, may ignore real technical debt
- Scoring doesn't reflect actual risk or maintenance cost

## Objective

Rebalance the debt scoring algorithm to prioritize actual code quality issues (complexity + coverage gaps + structural problems) over pure file size concerns, while keeping context-appropriate size issues visible.

## Requirements

### Functional Requirements

1. **Multi-Dimensional Scoring**
   - Complexity: Weight high for functions with cyclomatic complexity >10
   - Coverage gaps: Weight higher for complexity + low coverage combination
   - Structural issues: God objects, tight coupling, high fan-in/fan-out
   - File size: Reduce weight, make context-aware (see Spec 135)
   - Code smells: Dead code, duplicated code, long parameter lists

2. **Contextual Weighting**
   - Business logic with complexity >15: CRITICAL weight
   - High complexity + low coverage: Multiply weights
   - Pure file size (no other issues): LOW-MEDIUM weight
   - Generated code issues: Minimal or zero weight
   - Test code: Different weighting (coverage less important)

3. **Risk-Based Prioritization**
   - **High Risk**: Complex + untested + critical path
   - **Medium Risk**: Complex but tested, or simple but untested critical path
   - **Low Risk**: Simple + tested, or isolated code
   - **Minimal Risk**: Generated code, config files, isolated utilities

4. **Severity Levels**
   - CRITICAL: Immediate action required (complexity >20 + coverage <50%)
   - HIGH: Should fix soon (complexity >15 + coverage <70%)
   - MEDIUM: Important but not urgent (complexity >10 or coverage <80%)
   - LOW: Nice to improve (minor issues, size only)

### Non-Functional Requirements

1. **Transparency**: Users should understand why items are scored the way they are
2. **Configurability**: Weights should be adjustable via configuration
3. **Stability**: Small code changes shouldn't cause large score swings
4. **Performance**: Scoring should not significantly slow analysis

## Acceptance Criteria

- [ ] Complexity + coverage issues score higher than pure file size issues
- [ ] Issue #3 (complexity 42, 38% coverage) scores higher than issue #1 (just large file)
- [ ] Scoring formula is documented and explainable
- [ ] Each score component is shown in output with its contribution
- [ ] Context-aware file size uses thresholds from Spec 135
- [ ] Generated code gets minimal scores or is filtered
- [ ] Business logic complexity issues are prioritized over test file size
- [ ] Score includes rationale explaining weight factors
- [ ] Configuration allows tuning weights for different project types
- [ ] Integration test shows sensible prioritization on ripgrep corpus
- [ ] Documentation explains scoring algorithm and how to interpret scores
- [ ] Backward compatibility: old scores can be approximated with config

## Technical Details

### Implementation Approach

1. **Scoring Components**
   ```rust
   #[derive(Debug, Clone)]
   pub struct DebtScore {
       total: f64,
       components: ScoreComponents,
       severity: Severity,
       rationale: ScoringRationale,
   }

   #[derive(Debug, Clone)]
   pub struct ScoreComponents {
       complexity_score: f64,      // Weight: 0-100
       coverage_score: f64,        // Weight: 0-80
       structural_score: f64,      // Weight: 0-60
       size_score: f64,            // Weight: 0-30 (reduced from current)
       smell_score: f64,           // Weight: 0-40
   }

   impl DebtScore {
       pub fn calculate(issue: &DebtIssue, weights: &ScoreWeights) -> Self {
           let components = ScoreComponents {
               complexity_score: score_complexity(issue, weights),
               coverage_score: score_coverage_gap(issue, weights),
               structural_score: score_structural_issues(issue, weights),
               size_score: score_file_size(issue, weights),
               smell_score: score_code_smells(issue, weights),
           };

           let total = components.weighted_total(weights);
           let severity = determine_severity(&components, issue);
           let rationale = ScoringRationale::explain(&components, weights);

           DebtScore { total, components, severity, rationale }
       }
   }
   ```

2. **Component Scoring Functions**
   ```rust
   fn score_complexity(issue: &DebtIssue, weights: &ScoreWeights) -> f64 {
       match &issue.kind {
           IssueKind::ComplexFunction { complexity, .. } => {
               let base = match complexity.cyclomatic {
                   c if c > 30 => 100.0,
                   c if c > 20 => 80.0,
                   c if c > 15 => 60.0,
                   c if c > 10 => 40.0,
                   c if c > 5 => 20.0,
                   _ => 0.0,
               };

               // Multiply by cognitive complexity factor
               let cognitive_factor = complexity.cognitive as f64 / complexity.cyclomatic.max(1) as f64;
               base * (1.0 + cognitive_factor * 0.3)
           }
           _ => 0.0,
       }
   }

   fn score_coverage_gap(issue: &DebtIssue, weights: &ScoreWeights) -> f64 {
       match &issue.coverage {
           Some(cov) if cov.percentage < 80.0 => {
               let gap = 80.0 - cov.percentage;
               let base_score = gap * 0.8; // 0-80 range

               // Multiply if combined with high complexity
               let complexity_multiplier = if let IssueKind::ComplexFunction { complexity, .. } = &issue.kind {
                   if complexity.cyclomatic > 15 {
                       2.0  // Double the coverage score for complex + untested
                   } else if complexity.cyclomatic > 10 {
                       1.5
                   } else {
                       1.0
                   }
               } else {
                   1.0
               };

               base_score * complexity_multiplier
           }
           _ => 0.0,
       }
   }

   fn score_file_size(issue: &DebtIssue, weights: &ScoreWeights) -> f64 {
       match &issue.kind {
           IssueKind::LargeFile { lines, file_type, threshold, .. } => {
               // Use context-aware threshold from Spec 135
               let excess_ratio = (*lines as f64 / threshold.base_threshold as f64) - 1.0;

               if excess_ratio <= 0.0 {
                   return 0.0;
               }

               // Base score capped at 30 (reduced from previous higher values)
               let base = (excess_ratio * 20.0).min(30.0);

               // Reduce for generated/config files
               let file_type_factor = match file_type {
                   FileType::BusinessLogic => 1.0,
                   FileType::TestCode => 0.7,
                   FileType::DeclarativeConfig => 0.3,
                   FileType::GeneratedCode => 0.1,
                   _ => 0.5,
               };

               base * file_type_factor
           }
           _ => 0.0,
       }
   }

   fn score_structural_issues(issue: &DebtIssue, weights: &ScoreWeights) -> f64 {
       match &issue.kind {
           IssueKind::GodObject { metrics, .. } => {
               let responsibility_score = (metrics.responsibilities.len() as f64 - 1.0) * 10.0;
               let function_score = (metrics.total_functions as f64 / 50.0) * 15.0;
               let coupling_score = metrics.coupling_score.unwrap_or(0.0) * 20.0;

               (responsibility_score + function_score + coupling_score).min(60.0)
           }
           IssueKind::TightCoupling { fan_out, .. } => {
               (*fan_out as f64 / 2.0).min(40.0)
           }
           _ => 0.0,
       }
   }
   ```

3. **Severity Determination**
   ```rust
   fn determine_severity(components: &ScoreComponents, issue: &DebtIssue) -> Severity {
       // CRITICAL: High complexity + low coverage + significant size
       if components.complexity_score > 60.0 && components.coverage_score > 40.0 {
           return Severity::Critical;
       }

       // HIGH: Moderate complexity + coverage gap OR severe structural issue
       if (components.complexity_score > 40.0 && components.coverage_score > 20.0)
           || components.structural_score > 50.0 {
           return Severity::High;
       }

       // MEDIUM: Single moderate issue
       if components.complexity_score > 30.0
           || components.coverage_score > 30.0
           || components.structural_score > 30.0 {
           return Severity::Medium;
       }

       // LOW: Only minor issues or pure size concerns
       Severity::Low
   }
   ```

4. **Scoring Rationale**
   ```rust
   #[derive(Debug, Clone)]
   pub struct ScoringRationale {
       primary_factors: Vec<String>,
       multipliers: Vec<String>,
       context_adjustments: Vec<String>,
   }

   impl ScoringRationale {
       pub fn explain(components: &ScoreComponents, weights: &ScoreWeights) -> Self {
           let mut primary = Vec::new();
           let mut multipliers = Vec::new();
           let mut adjustments = Vec::new();

           if components.complexity_score > 40.0 {
               primary.push(format!("High cyclomatic complexity (+{:.1})", components.complexity_score));
           }

           if components.coverage_score > 30.0 {
               primary.push(format!("Significant coverage gap (+{:.1})", components.coverage_score));
           }

           if components.complexity_score > 40.0 && components.coverage_score > 20.0 {
               multipliers.push("Complex + untested: 2x coverage weight".to_string());
           }

           if components.size_score < 10.0 && components.size_score > 0.0 {
               adjustments.push(format!("File size context-adjusted (-{}%)",
                   ((30.0 - components.size_score) / 30.0 * 100.0) as i32));
           }

           ScoringRationale {
               primary_factors: primary,
               multipliers,
               context_adjustments: adjustments,
           }
       }
   }
   ```

### Scoring Weights Configuration

```rust
#[derive(Debug, Clone)]
pub struct ScoreWeights {
    complexity_weight: f64,       // Default: 1.0
    coverage_weight: f64,         // Default: 1.0
    structural_weight: f64,       // Default: 0.8
    size_weight: f64,            // Default: 0.3 (reduced)
    smell_weight: f64,           // Default: 0.6
}

impl Default for ScoreWeights {
    fn default() -> Self {
        ScoreWeights {
            complexity_weight: 1.0,
            coverage_weight: 1.0,
            structural_weight: 0.8,
            size_weight: 0.3,  // Reduced from previous ~1.5
            smell_weight: 0.6,
        }
    }
}
```

### Configuration File Support

```toml
# .debtmap.toml
[scoring]
# Adjust weights for your project priorities
complexity_weight = 1.2  # Emphasize complexity
coverage_weight = 1.0
structural_weight = 0.8
size_weight = 0.2        # De-emphasize pure size
smell_weight = 0.6

[scoring.presets]
# Quick preset configurations
preset = "quality-focused"  # Options: balanced, quality-focused, size-focused

# quality-focused: Prioritize complexity and coverage
# size-focused: Traditional size-heavy scoring (legacy)
# balanced: Default weights
```

## Dependencies

- **Prerequisites**:
  - Spec 134: Need consistent metrics to score reliably
  - Spec 135: Need context-aware thresholds for file size scoring
- **Affected Components**:
  - `src/debt/scoring.rs` - New module for scoring logic
  - `src/debt/prioritization.rs` - Use new scoring for ranking
  - `src/io/output.rs` - Display score breakdown
  - `src/config.rs` - Add scoring configuration
- **External Dependencies**: None (pure Rust)

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_complexity_outweighs_size() {
    let complex_issue = DebtIssue {
        kind: IssueKind::ComplexFunction {
            complexity: Complexity { cyclomatic: 42, cognitive: 77, .. },
            ..
        },
        coverage: Some(Coverage { percentage: 38.7, .. }),
        ..
    };

    let large_file_issue = DebtIssue {
        kind: IssueKind::LargeFile {
            lines: 7775,
            file_type: FileType::DeclarativeConfig,
            threshold: FileSizeThresholds { base_threshold: 1200, .. },
            ..
        },
        coverage: None,
        ..
    };

    let complex_score = DebtScore::calculate(&complex_issue, &ScoreWeights::default());
    let size_score = DebtScore::calculate(&large_file_issue, &ScoreWeights::default());

    assert!(complex_score.total > size_score.total,
            "Complex + untested should score higher than large file");
}

#[test]
fn test_coverage_gap_multiplier() {
    let weights = ScoreWeights::default();

    let complex_untested = DebtIssue {
        kind: IssueKind::ComplexFunction {
            complexity: Complexity { cyclomatic: 20, .. },
            ..
        },
        coverage: Some(Coverage { percentage: 40.0, .. }),
        ..
    };

    let simple_untested = DebtIssue {
        kind: IssueKind::ComplexFunction {
            complexity: Complexity { cyclomatic: 5, .. },
            ..
        },
        coverage: Some(Coverage { percentage: 40.0, .. }),
        ..
    };

    let complex_score = DebtScore::calculate(&complex_untested, &weights);
    let simple_score = DebtScore::calculate(&simple_untested, &weights);

    // Coverage score should be multiplied for complex functions
    assert!(complex_score.components.coverage_score > simple_score.components.coverage_score * 1.8);
}

#[test]
fn test_generated_code_minimal_score() {
    let generated = DebtIssue {
        kind: IssueKind::LargeFile {
            lines: 5000,
            file_type: FileType::GeneratedCode { tool: Some("prost".to_string()) },
            threshold: FileSizeThresholds { base_threshold: 5000, .. },
            ..
        },
        ..
    };

    let score = DebtScore::calculate(&generated, &ScoreWeights::default());
    assert!(score.total < 10.0, "Generated code should score very low");
    assert_eq!(score.severity, Severity::Low);
}
```

### Integration Tests

```rust
#[test]
fn test_ripgrep_prioritization() {
    let issues = analyze_project("../ripgrep").unwrap();
    let ranked = prioritize_issues(issues);

    // Complexity + coverage issues should rank higher than pure size
    let top_issues = &ranked[0..5];

    for issue in top_issues {
        if issue.severity == Severity::Critical {
            // Critical issues must have complexity OR structural problems
            assert!(
                issue.score.components.complexity_score > 40.0
                || issue.score.components.structural_score > 40.0,
                "Critical issues must have substance beyond size"
            );
        }
    }

    // The old #1 (pure size) should not be top 3
    let large_file_rank = ranked.iter().position(|i|
        matches!(i.kind, IssueKind::LargeFile { lines: 7775, .. })
    ).unwrap();

    assert!(large_file_rank > 2, "Pure size issue should not dominate ranking");
}
```

### Property-Based Tests

```rust
proptest! {
    #[test]
    fn complexity_always_weighted_higher_than_size(
        complexity in 10..50u32,
        lines in 500..5000usize
    ) {
        let weights = ScoreWeights::default();

        let complexity_component = score_complexity(
            &complexity_issue(complexity),
            &weights
        );

        let size_component = score_file_size(
            &size_issue(lines),
            &weights
        );

        // For equivalent "badness", complexity should always score higher
        prop_assert!(complexity_component > size_component);
    }
}
```

## Documentation Requirements

### Code Documentation

- Document each scoring component and its weight rationale
- Explain severity determination algorithm
- Provide examples of different score profiles
- Document configuration options and presets

### User Documentation

- **Scoring Guide**: Explain how debt scores are calculated
- **Interpreting Scores**: What different score ranges mean
- **Configuration**: How to tune scoring for your project
- **Migration**: How scores changed from previous version

### Architecture Updates

Update ARCHITECTURE.md:
- Add section on debt scoring and prioritization
- Document scoring component design
- Explain the balance between different debt types

## Implementation Notes

### Scoring Philosophy

The new scoring should reflect this priority order:
1. **Complexity + Coverage Gap**: Highest risk, hardest to maintain
2. **Structural Issues**: Architectural debt, affects whole codebase
3. **Code Smells**: Localized quality issues
4. **Contextual Size**: Only when excessive for file type
5. **Pure Size**: Lowest priority, context-dependent

### Calibration Process

1. Analyze diverse codebases (ripgrep, rust-analyzer, tokio, etc.)
2. Manually rank top 20 issues for each
3. Tune weights until automated ranking matches manual ranking
4. Validate on additional codebases
5. Iterate based on user feedback

### Common Pitfalls

- **Over-correction**: Don't make size irrelevant, just proportionate
- **Score inflation**: Keep total scores in reasonable range (0-200)
- **Severity creep**: Don't mark everything CRITICAL
- **Configuration complexity**: Too many knobs make it unusable

### Backward Compatibility Strategy

```rust
// Legacy scoring (for comparison/migration)
impl ScoreWeights {
    pub fn legacy() -> Self {
        ScoreWeights {
            complexity_weight: 0.5,
            coverage_weight: 0.4,
            structural_weight: 0.6,
            size_weight: 1.5,  // Old high weight
            smell_weight: 0.3,
        }
    }
}
```

## Migration and Compatibility

### Breaking Changes

- Debt scores will change significantly
- Issue ranking will change (some issues move up, others down)
- CRITICAL severity may apply to different issues

### Migration Path

1. **Dual Output Mode**: Show both old and new scores during transition
2. **Migration Report**: Explain score changes for top 20 issues
3. **Configuration**: Offer `legacy-scoring = true` option
4. **Documentation**: Publish migration guide with examples

### Communication

Release notes should clearly explain:
- Why scoring changed
- What kinds of issues are now prioritized
- How to restore old behavior if needed
- Expected impact on typical projects

## Success Metrics

- Complexity + coverage issues consistently rank in top 10
- Pure file size issues (no other problems) rank below complexity issues
- User reports indicate better prioritization alignment
- Fewer "false positive" critical issues
- Score rationale is clear and actionable
- Configuration adoption shows users fine-tuning for their needs
