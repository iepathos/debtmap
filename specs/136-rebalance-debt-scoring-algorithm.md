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

### Type Definitions

This section defines the core types referenced throughout the implementation. These align with existing debtmap types where applicable.

```rust
// Existing types from src/priority/mod.rs (reference only)
pub enum DebtType {
    TestingGap { coverage: f64, cyclomatic: u32, cognitive: u32 },
    ComplexityHotspot { cyclomatic: u32, cognitive: u32 },
    DeadCode { visibility: FunctionVisibility, cyclomatic: u32, cognitive: u32, usage_hints: Vec<String> },
    GodObject { methods: u32, fields: u32, responsibilities: u32, god_object_score: f64 },
    // ... other variants
}

pub enum FunctionVisibility {
    Private,
    Crate,
    Public,
}

// Existing types from src/core/mod.rs (reference only)
pub struct FunctionMetrics {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub length: usize,
    // ... other fields
}

// New types for Spec 136
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileCategory {
    BusinessLogic,
    TestCode,
    DeclarativeConfig,
    GeneratedCode { tool: Option<String> },
    Infrastructure,
}

// Note: This simplified Complexity type is for examples only
// The actual FunctionMetrics already contains cyclomatic and cognitive fields
#[derive(Debug, Clone)]
pub struct Complexity {
    pub cyclomatic: u32,
    pub cognitive: u32,
}
```

### Implementation Approach

1. **Scoring Components**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct DebtScore {
       pub total: f64,
       pub components: ScoreComponents,
       pub severity: Severity,
       pub rationale: ScoringRationale,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ScoreComponents {
       pub complexity_score: f64,      // Weight: 0-100
       pub coverage_score: f64,        // Weight: 0-80
       pub structural_score: f64,      // Weight: 0-60
       pub size_score: f64,            // Weight: 0-30 (reduced from current)
       pub smell_score: f64,           // Weight: 0-40
   }

   impl ScoreComponents {
       /// Calculate total score with normalization to 0-200 range
       pub fn weighted_total(&self, weights: &ScoreWeights) -> f64 {
           let raw_total =
               self.complexity_score * weights.complexity_weight +
               self.coverage_score * weights.coverage_weight +
               self.structural_score * weights.structural_weight +
               self.size_score * weights.size_weight +
               self.smell_score * weights.smell_weight;

           // Normalize to 0-200 range
           // Theoretical max: 100×1.0 + 80×1.0 + 60×0.8 + 30×0.3 + 40×0.6 = 237
           (raw_total / 237.0) * 200.0
       }
   }

   impl DebtScore {
       pub fn calculate(func: &FunctionMetrics, debt_type: &DebtType, weights: &ScoreWeights) -> Self {
           let components = ScoreComponents {
               complexity_score: score_complexity(func, debt_type, weights),
               coverage_score: score_coverage_gap(func, debt_type, weights),
               structural_score: score_structural_issues(debt_type, weights),
               size_score: score_file_size(func, weights),
               smell_score: score_code_smells(func, weights),
           };

           let total = components.weighted_total(weights);
           let severity = determine_severity(&components, func, debt_type);
           let rationale = ScoringRationale::explain(&components, weights);

           DebtScore { total, components, severity, rationale }
       }
   }
   ```

2. **Component Scoring Functions**

   These functions calculate individual component scores. They use **additive bonuses** rather than multiplicative factors for better predictability.

   ```rust
   fn score_complexity(func: &FunctionMetrics, debt_type: &DebtType, weights: &ScoreWeights) -> f64 {
       match debt_type {
           DebtType::ComplexityHotspot { cyclomatic, cognitive } => {
               // Base score from cyclomatic complexity
               let cyclomatic_score = match cyclomatic {
                   c if *c > 30 => 100.0,
                   c if *c > 20 => 80.0,
                   c if *c > 15 => 60.0,
                   c if *c > 10 => 40.0,
                   c if *c > 5 => 20.0,
                   _ => 0.0,
               };

               // Additive bonus from cognitive complexity
               let cognitive_bonus = match cognitive {
                   c if *c > 50 => 20.0,
                   c if *c > 30 => 15.0,
                   c if *c > 20 => 10.0,
                   c if *c > 15 => 5.0,
                   _ => 0.0,
               };

               (cyclomatic_score + cognitive_bonus).min(100.0)
           }
           DebtType::TestingGap { cyclomatic, cognitive, .. } => {
               // Lower base scores for testing gaps
               let base = match cyclomatic {
                   c if *c > 15 => 30.0,
                   c if *c > 10 => 20.0,
                   c if *c > 5 => 10.0,
                   _ => 0.0,
               };
               base.min(40.0)
           }
           _ => 0.0,
       }
   }

   fn score_coverage_gap(func: &FunctionMetrics, debt_type: &DebtType, weights: &ScoreWeights) -> f64 {
       match debt_type {
           DebtType::TestingGap { coverage, cyclomatic, .. } => {
               let gap_percent = (1.0 - coverage) * 100.0;
               let base_score = (gap_percent * 0.6).min(60.0);

               // Additive bonus for complex untested code (not multiplicative)
               let complexity_bonus = if *cyclomatic > 15 {
                   20.0  // +20 for high complexity + low coverage
               } else if *cyclomatic > 10 {
                   10.0  // +10 for moderate complexity + low coverage
               } else {
                   0.0
               };

               (base_score + complexity_bonus).min(80.0)
           }
           _ => 0.0,
       }
   }

   fn score_file_size(func: &FunctionMetrics, weights: &ScoreWeights) -> f64 {
       // File size scoring is typically done at file-level analysis
       // For function-level debt items, this returns 0
       // This would be implemented in a separate file-level scoring module
       0.0
   }

   fn score_structural_issues(debt_type: &DebtType, weights: &ScoreWeights) -> f64 {
       match debt_type {
           DebtType::GodObject { methods, responsibilities, god_object_score, .. } => {
               let responsibility_score = ((*responsibilities as f64 - 1.0) * 10.0).min(30.0);
               let method_score = ((*methods as f64 / 20.0) * 15.0).min(20.0);
               let god_score = (god_object_score * 10.0).min(10.0);

               (responsibility_score + method_score + god_score).min(60.0)
           }
           _ => 0.0,
       }
   }

   fn score_code_smells(func: &FunctionMetrics, weights: &ScoreWeights) -> f64 {
       let mut smell_score = 0.0;

       // Long function smell
       if func.length > 100 {
           smell_score += ((func.length as f64 - 100.0) / 20.0).min(15.0);
       }

       // Deep nesting smell
       if func.nesting > 3 {
           smell_score += ((func.nesting as f64 - 3.0) * 5.0).min(15.0);
       }

       // Impure function in logic (potential side effect smell)
       if let Some(false) = func.is_pure {
           smell_score += 10.0;
       }

       smell_score.min(40.0)
   }
   ```

3. **Severity Determination**

   Severity is determined based on score components and normalized total score:

   ```rust
   #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
   pub enum Severity {
       Critical,
       High,
       Medium,
       Low,
   }

   fn determine_severity(components: &ScoreComponents, func: &FunctionMetrics, debt_type: &DebtType) -> Severity {
       // Use normalized total score (0-200 range) as primary factor
       let total = components.weighted_total(&ScoreWeights::default());

       // CRITICAL: Total score > 150 OR high complexity + low coverage
       if total > 150.0 || (components.complexity_score > 60.0 && components.coverage_score > 40.0) {
           return Severity::Critical;
       }

       // HIGH: Total score > 100 OR moderate complexity + coverage gap OR severe structural issue
       if total > 100.0
           || (components.complexity_score > 40.0 && components.coverage_score > 20.0)
           || components.structural_score > 50.0 {
           return Severity::High;
       }

       // MEDIUM: Total score > 50 OR single moderate issue
       if total > 50.0
           || components.complexity_score > 30.0
           || components.coverage_score > 30.0
           || components.structural_score > 30.0 {
           return Severity::Medium;
       }

       // LOW: Everything else (minor issues, pure size concerns)
       Severity::Low
   }
   ```

4. **Scoring Rationale**

   The rationale explains why a particular score was assigned, providing transparency to users.

   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ScoringRationale {
       pub primary_factors: Vec<String>,
       pub bonuses: Vec<String>,
       pub context_adjustments: Vec<String>,
   }

   impl ScoringRationale {
       pub fn explain(components: &ScoreComponents, weights: &ScoreWeights) -> Self {
           let mut primary = Vec::new();
           let mut bonuses = Vec::new();
           let mut adjustments = Vec::new();

           // Primary factors (main contributors to score)
           if components.complexity_score > 40.0 {
               primary.push(format!("High cyclomatic complexity (+{:.1})", components.complexity_score));
           }

           if components.coverage_score > 30.0 {
               primary.push(format!("Significant coverage gap (+{:.1})", components.coverage_score));
           }

           if components.structural_score > 30.0 {
               primary.push(format!("Structural issues (+{:.1})", components.structural_score));
           }

           // Bonuses (additive enhancements)
           if components.complexity_score > 40.0 && components.coverage_score > 20.0 {
               bonuses.push("Complex + untested: +20 bonus applied".to_string());
           }

           if components.smell_score > 20.0 {
               bonuses.push(format!("Code smells detected (+{:.1})", components.smell_score));
           }

           // Context adjustments
           if components.size_score < 10.0 && components.size_score > 0.0 {
               adjustments.push("File size context-adjusted (reduced weight for file type)".to_string());
           }

           if weights.size_weight < 0.5 {
               adjustments.push(format!("Size de-emphasized (weight: {:.1})", weights.size_weight));
           }

           ScoringRationale {
               primary_factors: primary,
               bonuses,
               context_adjustments: adjustments,
           }
       }
   }

   impl std::fmt::Display for ScoringRationale {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           if !self.primary_factors.is_empty() {
               writeln!(f, "  Primary factors:")?;
               for factor in &self.primary_factors {
                   writeln!(f, "    - {}", factor)?;
               }
           }

           if !self.bonuses.is_empty() {
               writeln!(f, "  Bonuses:")?;
               for bonus in &self.bonuses {
                   writeln!(f, "    - {}", bonus)?;
               }
           }

           if !self.context_adjustments.is_empty() {
               writeln!(f, "  Context adjustments:")?;
               for adj in &self.context_adjustments {
                   writeln!(f, "    - {}", adj)?;
               }
           }

           Ok(())
       }
   }
   ```

   **Example Output Format:**

   ```
   Issue #1: analyze_project (src/main.rs:42)
   Score: 156.3 (CRITICAL)
   Components:
     - Complexity: 80.0
     - Coverage: 60.0
     - Structural: 12.0
     - Size: 0.0
     - Smells: 15.0

   Rationale:
     Primary factors:
       - High cyclomatic complexity (+80.0)
       - Significant coverage gap (+60.0)
     Bonuses:
       - Complex + untested: +20 bonus applied
       - Code smells detected (+15.0)
     Context adjustments:
       - Size de-emphasized (weight: 0.3)
   ```

### Scoring Weights Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub complexity_weight: f64,       // Default: 1.0
    pub coverage_weight: f64,         // Default: 1.0
    pub structural_weight: f64,       // Default: 0.8
    pub size_weight: f64,            // Default: 0.3 (reduced)
    pub smell_weight: f64,           // Default: 0.6
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self::balanced()
    }
}

impl ScoreWeights {
    /// Balanced preset: Default weights prioritizing complexity and coverage
    pub fn balanced() -> Self {
        ScoreWeights {
            complexity_weight: 1.0,
            coverage_weight: 1.0,
            structural_weight: 0.8,
            size_weight: 0.3,  // Reduced from previous ~1.5
            smell_weight: 0.6,
        }
    }

    /// Quality-focused preset: Maximum emphasis on code quality over size
    pub fn quality_focused() -> Self {
        ScoreWeights {
            complexity_weight: 1.2,
            coverage_weight: 1.1,
            structural_weight: 0.9,
            size_weight: 0.2,  // Further reduced
            smell_weight: 0.7,
        }
    }

    /// Size-focused preset: Legacy behavior for compatibility
    pub fn size_focused() -> Self {
        ScoreWeights {
            complexity_weight: 0.5,
            coverage_weight: 0.4,
            structural_weight: 0.6,
            size_weight: 1.5,  // Old high weight
            smell_weight: 0.3,
        }
    }

    /// Test-coverage preset: Emphasize testing gaps
    pub fn test_coverage_focused() -> Self {
        ScoreWeights {
            complexity_weight: 0.8,
            coverage_weight: 1.3,  // Highest weight
            structural_weight: 0.6,
            size_weight: 0.2,
            smell_weight: 0.5,
        }
    }

    /// From preset name
    pub fn from_preset(preset: &str) -> Option<Self> {
        match preset.to_lowercase().as_str() {
            "balanced" => Some(Self::balanced()),
            "quality-focused" | "quality_focused" | "quality" => Some(Self::quality_focused()),
            "size-focused" | "size_focused" | "legacy" => Some(Self::size_focused()),
            "test-coverage" | "test_coverage" | "testing" => Some(Self::test_coverage_focused()),
            _ => None,
        }
    }
}
```

### Configuration File Support

```toml
# .debtmap.toml or debtmap.toml
[scoring]
# Option 1: Use a preset
preset = "quality-focused"  # Options: balanced, quality-focused, size-focused, test-coverage

# Option 2: Custom weights (overrides preset if both specified)
# complexity_weight = 1.2
# coverage_weight = 1.0
# structural_weight = 0.8
# size_weight = 0.2
# smell_weight = 0.6

# Preset descriptions:
# - balanced: Default weights prioritizing complexity and coverage (1.0, 1.0, 0.8, 0.3, 0.6)
# - quality-focused: Maximum emphasis on code quality over size (1.2, 1.1, 0.9, 0.2, 0.7)
# - size-focused: Legacy behavior for compatibility (0.5, 0.4, 0.6, 1.5, 0.3)
# - test-coverage: Emphasize testing gaps (0.8, 1.3, 0.6, 0.2, 0.5)
```

**Loading Configuration in Code:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ScoringConfig {
    #[serde(default)]
    pub preset: Option<String>,
    pub complexity_weight: Option<f64>,
    pub coverage_weight: Option<f64>,
    pub structural_weight: Option<f64>,
    pub size_weight: Option<f64>,
    pub smell_weight: Option<f64>,
}

impl ScoringConfig {
    pub fn to_weights(&self) -> ScoreWeights {
        // Start with preset if specified, otherwise default
        let mut weights = self.preset
            .as_ref()
            .and_then(|p| ScoreWeights::from_preset(p))
            .unwrap_or_default();

        // Override with custom values if specified
        if let Some(w) = self.complexity_weight {
            weights.complexity_weight = w;
        }
        if let Some(w) = self.coverage_weight {
            weights.coverage_weight = w;
        }
        if let Some(w) = self.structural_weight {
            weights.structural_weight = w;
        }
        if let Some(w) = self.size_weight {
            weights.size_weight = w;
        }
        if let Some(w) = self.smell_weight {
            weights.smell_weight = w;
        }

        weights
    }
}
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
#[ignore] // Run with: cargo test --ignored test_ripgrep_prioritization
fn test_ripgrep_prioritization() {
    // This test requires ripgrep source code to be available
    // Clone it with: git clone https://github.com/BurntSushi/ripgrep ../ripgrep
    let ripgrep_path = std::path::Path::new("../ripgrep");

    if !ripgrep_path.exists() {
        eprintln!("Skipping ripgrep integration test: ripgrep source not found at ../ripgrep");
        eprintln!("To run this test, clone ripgrep: git clone https://github.com/BurntSushi/ripgrep ../ripgrep");
        return;
    }

    let analysis = analyze_project(ripgrep_path).expect("Failed to analyze ripgrep");
    let ranked = prioritize_issues(&analysis);

    // Complexity + coverage issues should rank higher than pure size
    let top_issues = &ranked[0..5.min(ranked.len())];

    let critical_issues: Vec<_> = top_issues.iter()
        .filter(|i| i.severity == Severity::Critical)
        .collect();

    for issue in critical_issues {
        // Critical issues must have complexity OR structural problems OR significant coverage gaps
        let has_substance =
            issue.score.components.complexity_score > 40.0
            || issue.score.components.structural_score > 40.0
            || issue.score.components.coverage_score > 40.0;

        assert!(
            has_substance,
            "Critical issue at {}:{} lacks substance (complexity={}, structural={}, coverage={})",
            issue.location.file.display(),
            issue.location.line,
            issue.score.components.complexity_score,
            issue.score.components.structural_score,
            issue.score.components.coverage_score
        );
    }

    // Verify score distribution is reasonable
    let max_score = ranked.first().map(|i| i.score.total).unwrap_or(0.0);
    assert!(max_score <= 200.0, "Max score should not exceed 200: {}", max_score);
    assert!(max_score > 50.0, "Max score should indicate real issues: {}", max_score);

    println!("Ripgrep prioritization test passed:");
    println!("  Total issues: {}", ranked.len());
    println!("  Top score: {:.1}", max_score);
    println!("  Critical count: {}", critical_issues.len());
}

#[test]
fn test_scoring_on_synthetic_codebase() {
    // Test with synthetic examples that don't require external dependencies

    // Example 1: Complex function with low coverage
    let complex_untested = create_test_function(
        "complex_untested",
        42,  // cyclomatic
        77,  // cognitive
        150, // length
    );
    let score1 = DebtScore::calculate(
        &complex_untested,
        &DebtType::ComplexityHotspot { cyclomatic: 42, cognitive: 77 },
        &ScoreWeights::default()
    );

    // Example 2: Large file (simulated via function length)
    let large_simple = create_test_function(
        "large_simple",
        3,    // cyclomatic
        5,    // cognitive
        2000, // length
    );
    let score2 = DebtScore::calculate(
        &large_simple,
        &DebtType::Risk { risk_score: 0.2, factors: vec!["Long function".to_string()] },
        &ScoreWeights::default()
    );

    // Complex + untested should score significantly higher than just large
    assert!(
        score1.total > score2.total * 1.5,
        "Complex untested (score={:.1}) should score much higher than large simple (score={:.1})",
        score1.total,
        score2.total
    );

    assert_eq!(score1.severity, Severity::Critical, "Complex untested should be CRITICAL");
    assert!(
        matches!(score2.severity, Severity::Low | Severity::Medium),
        "Large simple should be LOW or MEDIUM, got {:?}",
        score2.severity
    );
}

// Helper for synthetic tests
fn create_test_function(name: &str, cyclomatic: u32, cognitive: u32, length: usize) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic,
        cognitive,
        nesting: (cognitive / 10).min(5),
        length,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.5),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
    }
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

### Key Changes from Evaluation

Based on the specification evaluation, the following improvements were made:

1. **Type Definitions Added**: Complete type definitions for `DebtScore`, `ScoreComponents`, `Severity`, and `FileCategory` with references to existing debtmap types
2. **Score Normalization**: Added explicit normalization to 0-200 range in `weighted_total()` method to resolve score inconsistency
3. **Additive Bonuses**: Changed from multiplicative factors to additive bonuses (e.g., +20 for complex+untested instead of ×2.0) for more predictable scoring
4. **Display Format**: Added `Display` implementation for `ScoringRationale` with example output format
5. **Preset System**: Implemented 4 presets (balanced, quality-focused, size-focused, test-coverage) with `from_preset()` method
6. **Integration Tests**: Updated ripgrep test to be `#[ignore]`-gated with graceful skipping if source not available, plus synthetic test that always runs

### Scoring Philosophy

The new scoring reflects this priority order:
1. **Complexity + Coverage Gap**: Highest risk, hardest to maintain
2. **Structural Issues**: Architectural debt, affects whole codebase
3. **Code Smells**: Localized quality issues
4. **Contextual Size**: Only when excessive for file type
5. **Pure Size**: Lowest priority, context-dependent

### Score Range Design

- **Theoretical Maximum**: 100×1.0 + 80×1.0 + 60×0.8 + 30×0.3 + 40×0.6 = 237 (raw)
- **Normalized Range**: 0-200 (via `(raw / 237.0) * 200.0`)
- **Severity Thresholds**:
  - CRITICAL: > 150
  - HIGH: > 100
  - MEDIUM: > 50
  - LOW: ≤ 50

### Calibration Process

1. Analyze diverse codebases (ripgrep, rust-analyzer, tokio, etc.)
2. Manually rank top 20 issues for each
3. Tune weights until automated ranking matches manual ranking
4. Validate on additional codebases
5. Iterate based on user feedback

### Common Pitfalls

- **Over-correction**: Don't make size irrelevant, just proportionate (addressed: size_weight = 0.3 in default)
- **Score inflation**: Keep total scores in reasonable range (✓ addressed: explicit 0-200 normalization)
- **Multiplicative explosion**: Avoid multiplying scores together (✓ addressed: using additive bonuses instead)
- **Severity creep**: Don't mark everything CRITICAL (addressed: threshold > 150 for CRITICAL)
- **Configuration complexity**: Too many knobs make it unusable (addressed: 4 simple presets + optional custom weights)
- **Type confusion**: Mixing file-level and function-level concerns (addressed: clear type definitions and separation)

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
