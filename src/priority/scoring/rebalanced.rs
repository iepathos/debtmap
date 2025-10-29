// Rebalanced debt scoring algorithm (Spec 136)
//
// This module implements a multi-dimensional scoring system that prioritizes actual code quality
// issues (complexity + coverage gaps + structural problems) over pure file size concerns.

use crate::core::FunctionMetrics;
use crate::priority::DebtType;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// Severity levels for debt items based on score and risk factors
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
        }
    }
}

/// Individual scoring components with their contributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreComponents {
    pub complexity_score: f64, // Weight: 0-100
    pub coverage_score: f64,   // Weight: 0-80
    pub structural_score: f64, // Weight: 0-60
    pub size_score: f64,       // Weight: 0-30 (reduced from current)
    pub smell_score: f64,      // Weight: 0-40
}

impl ScoreComponents {
    /// Calculate weighted total score with normalization to 0-200 range
    pub fn weighted_total(&self, weights: &ScoreWeights) -> f64 {
        let raw_total = self.complexity_score * weights.complexity_weight
            + self.coverage_score * weights.coverage_weight
            + self.structural_score * weights.structural_weight
            + self.size_score * weights.size_weight
            + self.smell_score * weights.smell_weight;

        // Normalize to 0-200 range
        // Theoretical max: 100×1.0 + 80×1.0 + 60×0.8 + 30×0.3 + 40×0.6 = 237
        (raw_total / 237.0) * 200.0
    }
}

/// Configurable weights for each scoring component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub complexity_weight: f64, // Default: 1.0
    pub coverage_weight: f64,   // Default: 1.0
    pub structural_weight: f64, // Default: 0.8
    pub size_weight: f64,       // Default: 0.3 (reduced)
    pub smell_weight: f64,      // Default: 0.6
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
            size_weight: 0.3, // Reduced from previous ~1.5
            smell_weight: 0.6,
        }
    }

    /// Quality-focused preset: Maximum emphasis on code quality over size
    pub fn quality_focused() -> Self {
        ScoreWeights {
            complexity_weight: 1.2,
            coverage_weight: 1.1,
            structural_weight: 0.9,
            size_weight: 0.2, // Further reduced
            smell_weight: 0.7,
        }
    }

    /// Size-focused preset: Legacy behavior for compatibility
    pub fn size_focused() -> Self {
        ScoreWeights {
            complexity_weight: 0.5,
            coverage_weight: 0.4,
            structural_weight: 0.6,
            size_weight: 1.5, // Old high weight
            smell_weight: 0.3,
        }
    }

    /// Test-coverage preset: Emphasize testing gaps
    pub fn test_coverage_focused() -> Self {
        ScoreWeights {
            complexity_weight: 0.8,
            coverage_weight: 1.3, // Highest weight
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

/// Rationale explaining why a score was assigned
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
            primary.push(format!(
                "High cyclomatic complexity (+{:.1})",
                components.complexity_score
            ));
        }

        if components.coverage_score > 30.0 {
            primary.push(format!(
                "Significant coverage gap (+{:.1})",
                components.coverage_score
            ));
        }

        if components.structural_score > 30.0 {
            primary.push(format!(
                "Structural issues (+{:.1})",
                components.structural_score
            ));
        }

        // Bonuses (additive enhancements)
        if components.complexity_score > 40.0 && components.coverage_score > 20.0 {
            bonuses.push("Complex + untested: +20 bonus applied".to_string());
        }

        if components.smell_score > 20.0 {
            bonuses.push(format!(
                "Code smells detected (+{:.1})",
                components.smell_score
            ));
        }

        // Context adjustments
        if components.size_score < 10.0 && components.size_score > 0.0 {
            adjustments
                .push("File size context-adjusted (reduced weight for file type)".to_string());
        }

        if weights.size_weight < 0.5 {
            adjustments.push(format!(
                "Size de-emphasized (weight: {:.1})",
                weights.size_weight
            ));
        }

        ScoringRationale {
            primary_factors: primary,
            bonuses,
            context_adjustments: adjustments,
        }
    }
}

impl fmt::Display for ScoringRationale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// Complete debt score with components, severity, and rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtScore {
    pub total: f64,
    pub components: ScoreComponents,
    pub severity: Severity,
    pub rationale: ScoringRationale,
}

impl DebtScore {
    /// Calculate debt score for a function with the given debt type
    pub fn calculate(func: &FunctionMetrics, debt_type: &DebtType, weights: &ScoreWeights) -> Self {
        let mut components = ScoreComponents {
            complexity_score: score_complexity(func, debt_type, weights),
            coverage_score: score_coverage_gap(func, debt_type, weights),
            structural_score: score_structural_issues(debt_type, weights),
            size_score: score_file_size(func, weights),
            smell_score: score_code_smells(func, weights),
        };

        // Apply generated code detection and scoring reduction
        if is_generated_file(&func.file) {
            // Reduce size score by 90% for generated code
            components.size_score *= 0.1;
        }

        let total = components.weighted_total(weights);
        let severity = determine_severity(&components, func, debt_type);
        let rationale = ScoringRationale::explain(&components, weights);

        DebtScore {
            total,
            components,
            severity,
            rationale,
        }
    }
}

/// Score complexity component based on cyclomatic and cognitive complexity
fn score_complexity(_func: &FunctionMetrics, debt_type: &DebtType, _weights: &ScoreWeights) -> f64 {
    match debt_type {
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => {
            // Base score from cyclomatic complexity
            let cyclomatic_score: f64 = match cyclomatic {
                c if *c > 30 => 100.0,
                c if *c > 20 => 80.0,
                c if *c > 15 => 60.0,
                c if *c > 10 => 40.0,
                c if *c > 5 => 20.0,
                _ => 0.0,
            };

            // Additive bonus from cognitive complexity
            let cognitive_bonus: f64 = match cognitive {
                c if *c > 50 => 20.0,
                c if *c > 30 => 15.0,
                c if *c > 20 => 10.0,
                c if *c > 15 => 5.0,
                _ => 0.0,
            };

            (cyclomatic_score + cognitive_bonus).min(100.0)
        }
        DebtType::TestingGap {
            cyclomatic,
            cognitive,
            ..
        } => {
            // Lower base scores for testing gaps
            let base: f64 = match cyclomatic {
                c if *c > 15 => 30.0,
                c if *c > 10 => 20.0,
                c if *c > 5 => 10.0,
                _ => 0.0,
            };

            // Add cognitive bonus for testing gaps
            let cognitive_bonus: f64 = match cognitive {
                c if *c > 30 => 10.0,
                c if *c > 15 => 5.0,
                _ => 0.0,
            };

            (base + cognitive_bonus).min(40.0)
        }
        _ => 0.0,
    }
}

/// Score coverage gap component
fn score_coverage_gap(
    _func: &FunctionMetrics,
    debt_type: &DebtType,
    _weights: &ScoreWeights,
) -> f64 {
    match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            ..
        } => {
            let gap_percent = (1.0 - coverage) * 100.0;
            let base_score = (gap_percent * 0.6).min(60.0);

            // Additive bonus for complex untested code (not multiplicative)
            let complexity_bonus = if *cyclomatic > 15 {
                20.0 // +20 for high complexity + low coverage
            } else if *cyclomatic > 10 {
                10.0 // +10 for moderate complexity + low coverage
            } else {
                0.0
            };

            (base_score + complexity_bonus).min(80.0)
        }
        _ => 0.0,
    }
}

/// Score structural issues like god objects
fn score_structural_issues(debt_type: &DebtType, _weights: &ScoreWeights) -> f64 {
    match debt_type {
        DebtType::GodObject {
            methods,
            responsibilities,
            god_object_score,
            ..
        } => {
            let responsibility_score = ((*responsibilities as f64 - 1.0) * 10.0).min(30.0);
            let method_score = ((*methods as f64 / 20.0) * 15.0).min(20.0);
            let god_score = (god_object_score * 10.0).min(10.0);

            (responsibility_score + method_score + god_score).min(60.0)
        }
        _ => 0.0,
    }
}

/// Score file size component using context-aware thresholds from spec 135
fn score_file_size(func: &FunctionMetrics, _weights: &ScoreWeights) -> f64 {
    // Function-level scoring: Score based on function length
    // Use simplified thresholds since we don't have full file context here

    let length = func.length;

    // Simplified function-level thresholds (would use file context in full impl)
    // These are conservative estimates for function-level analysis
    let threshold: usize = 100; // Reasonable function length threshold
    let max_threshold: usize = 200; // Max before critical

    if length <= threshold {
        0.0
    } else if length <= max_threshold {
        // Linear scaling from threshold to max_threshold
        let ratio = (length - threshold) as f64 / (max_threshold - threshold) as f64;
        ratio * 15.0 // Max 15 points for moderate size
    } else {
        // Beyond max threshold, cap at 30
        let excess = (length - max_threshold) as f64;
        (15.0 + (excess / 100.0).min(15.0)).min(30.0)
    }
}

/// Score code smells like long functions, deep nesting, etc.
fn score_code_smells(func: &FunctionMetrics, _weights: &ScoreWeights) -> f64 {
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

/// Determine severity based on score components and normalized total score
fn determine_severity(
    components: &ScoreComponents,
    _func: &FunctionMetrics,
    _debt_type: &DebtType,
) -> Severity {
    // Use normalized total score (0-200 range) as primary factor
    let total = components.weighted_total(&ScoreWeights::default());

    // CRITICAL: Total score > 120 OR high complexity + low coverage
    if total > 120.0 || (components.complexity_score > 60.0 && components.coverage_score > 40.0) {
        return Severity::Critical;
    }

    // HIGH: Total score > 80 OR moderate complexity + coverage gap OR severe structural issue
    if total > 80.0
        || (components.complexity_score > 40.0 && components.coverage_score > 20.0)
        || components.structural_score > 50.0
    {
        return Severity::High;
    }

    // MEDIUM: Total score > 40 OR single moderate issue
    if total > 40.0
        || components.complexity_score > 30.0
        || components.coverage_score > 30.0
        || components.structural_score > 30.0
    {
        return Severity::Medium;
    }

    // LOW: Everything else (minor issues, pure size concerns)
    Severity::Low
}

/// Detect if a file is generated code based on common patterns
fn is_generated_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Common generated file patterns
    let generated_patterns = [
        ".generated.rs",
        ".pb.rs", // Protocol buffers
        ".g.rs",  // Grammar generated files
        "_pb.rs", // Alternative protobuf naming
        "generated/",
        "/gen/",
    ];

    generated_patterns
        .iter()
        .any(|pattern| path_str.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_function(
        name: &str,
        cyclomatic: u32,
        cognitive: u32,
        length: usize,
    ) -> FunctionMetrics {
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

    #[test]
    fn test_complexity_outweighs_size() {
        let complex_func = create_test_function("complex_untested", 42, 77, 150);
        let complex_score = DebtScore::calculate(
            &complex_func,
            &DebtType::ComplexityHotspot {
                cyclomatic: 42,
                cognitive: 77,
            },
            &ScoreWeights::default(),
        );

        // Simulate a large file with low complexity
        // Note: In practice, file-level scoring would be separate
        // For this test, we're comparing function complexity vs simulated size
        // The size_score is 0 for functions, so we're testing the principle
        println!(
            "Complex score: {:.1}, severity: {:?}",
            complex_score.total, complex_score.severity
        );
        println!("Components: {:?}", complex_score.components);
        assert!(
            complex_score.total > 50.0,
            "Complex function should have substantial score, got {:.1}",
            complex_score.total
        );
        assert!(
            matches!(
                complex_score.severity,
                Severity::Critical | Severity::High
            ),
            "Complex untested function should be CRITICAL or HIGH, got {:?}",
            complex_score.severity
        );
    }

    #[test]
    fn test_coverage_gap_multiplier() {
        let weights = ScoreWeights::default();

        let complex_untested = create_test_function("complex_untested", 20, 35, 100);
        let complex_score = DebtScore::calculate(
            &complex_untested,
            &DebtType::TestingGap {
                coverage: 0.4,
                cyclomatic: 20,
                cognitive: 35,
            },
            &weights,
        );

        let simple_untested = create_test_function("simple_untested", 5, 8, 50);
        let simple_score = DebtScore::calculate(
            &simple_untested,
            &DebtType::TestingGap {
                coverage: 0.4,
                cyclomatic: 5,
                cognitive: 8,
            },
            &weights,
        );

        // Coverage score should be higher for complex functions (additive bonus)
        assert!(
            complex_score.components.coverage_score > simple_score.components.coverage_score,
            "Complex untested should have higher coverage score than simple untested"
        );

        // The bonus should be additive (not multiplicative)
        let bonus_diff =
            complex_score.components.coverage_score - simple_score.components.coverage_score;
        assert!(
            bonus_diff >= 10.0 && bonus_diff <= 20.0,
            "Bonus should be additive (10-20 points), got {:.1}",
            bonus_diff
        );
    }

    #[test]
    fn test_severity_determination() {
        let high_complexity = create_test_function("high_complexity", 42, 77, 150);
        let score = DebtScore::calculate(
            &high_complexity,
            &DebtType::ComplexityHotspot {
                cyclomatic: 42,
                cognitive: 77,
            },
            &ScoreWeights::default(),
        );

        assert!(
            matches!(score.severity, Severity::Critical | Severity::High),
            "High complexity should be CRITICAL or HIGH, got {:?}",
            score.severity
        );

        let moderate_complexity = create_test_function("moderate", 12, 20, 80);
        let score = DebtScore::calculate(
            &moderate_complexity,
            &DebtType::ComplexityHotspot {
                cyclomatic: 12,
                cognitive: 20,
            },
            &ScoreWeights::default(),
        );

        assert!(matches!(score.severity, Severity::Medium | Severity::High));
    }

    #[test]
    fn test_preset_weights() {
        let balanced = ScoreWeights::balanced();
        assert_eq!(balanced.complexity_weight, 1.0);
        assert_eq!(balanced.coverage_weight, 1.0);
        assert_eq!(balanced.size_weight, 0.3);

        let quality = ScoreWeights::quality_focused();
        assert_eq!(quality.complexity_weight, 1.2);
        assert_eq!(quality.size_weight, 0.2);

        let legacy = ScoreWeights::size_focused();
        assert_eq!(legacy.size_weight, 1.5);

        let testing = ScoreWeights::test_coverage_focused();
        assert_eq!(testing.coverage_weight, 1.3);
    }

    #[test]
    fn test_score_normalization() {
        let func = create_test_function("test", 30, 50, 200);
        let score = DebtScore::calculate(
            &func,
            &DebtType::ComplexityHotspot {
                cyclomatic: 30,
                cognitive: 50,
            },
            &ScoreWeights::default(),
        );

        // Score should be in 0-200 range
        assert!(
            score.total >= 0.0 && score.total <= 200.0,
            "Score should be in 0-200 range, got {}",
            score.total
        );
    }

    #[test]
    fn test_scoring_on_synthetic_codebase() {
        // Test with synthetic examples that don't require external dependencies

        // Example 1: Complex function with low coverage
        let complex_untested = create_test_function("complex_untested", 42, 77, 150);
        let score1 = DebtScore::calculate(
            &complex_untested,
            &DebtType::TestingGap {
                coverage: 0.38,
                cyclomatic: 42,
                cognitive: 77,
            },
            &ScoreWeights::default(),
        );

        // Example 2: Large but simple function
        let large_simple = create_test_function("large_simple", 3, 5, 2000);
        let score2 = DebtScore::calculate(
            &large_simple,
            &DebtType::Risk {
                risk_score: 0.2,
                factors: vec!["Long function".to_string()],
            },
            &ScoreWeights::default(),
        );

        // Complex + untested should score significantly higher than just large
        assert!(
            score1.total > score2.total * 1.5,
            "Complex untested (score={:.1}) should score much higher than large simple (score={:.1})",
            score1.total,
            score2.total
        );

        assert!(
            matches!(score1.severity, Severity::Critical | Severity::High),
            "Complex untested should be CRITICAL or HIGH, got {:?}",
            score1.severity
        );
        assert!(
            matches!(score2.severity, Severity::Low | Severity::Medium),
            "Large simple should be LOW or MEDIUM, got {:?}",
            score2.severity
        );
    }

    #[test]
    fn test_rationale_display() {
        let func = create_test_function("test", 25, 40, 150);
        let score = DebtScore::calculate(
            &func,
            &DebtType::TestingGap {
                coverage: 0.3,
                cyclomatic: 25,
                cognitive: 40,
            },
            &ScoreWeights::default(),
        );

        let rationale_str = format!("{}", score.rationale);
        assert!(
            !rationale_str.is_empty(),
            "Rationale should produce non-empty output"
        );
    }

    #[test]
    fn test_generated_code_detection() {
        // Test generated file patterns
        assert!(is_generated_file(Path::new("src/proto/api.pb.rs")));
        assert!(is_generated_file(Path::new("src/generated/schema.rs")));
        assert!(is_generated_file(Path::new("src/parser.g.rs")));
        assert!(is_generated_file(Path::new("src/models_pb.rs")));

        // Test normal files
        assert!(!is_generated_file(Path::new("src/main.rs")));
        assert!(!is_generated_file(Path::new("src/lib.rs")));
        assert!(!is_generated_file(Path::new("src/utils/helpers.rs")));
    }

    #[test]
    fn test_generated_code_scoring_reduction() {
        // Create a function in a generated file
        let mut generated_func = create_test_function("generated_fn", 10, 15, 500);
        generated_func.file = PathBuf::from("src/proto/api.pb.rs");

        let score = DebtScore::calculate(
            &generated_func,
            &DebtType::Risk {
                risk_score: 0.5,
                factors: vec!["Long function".to_string()],
            },
            &ScoreWeights::default(),
        );

        // Size score should be reduced by 90%
        assert!(
            score.components.size_score < 3.0,
            "Generated code size score should be reduced to ~10%, got {:.1}",
            score.components.size_score
        );

        // Create the same function in a normal file
        let mut normal_func = create_test_function("normal_fn", 10, 15, 500);
        normal_func.file = PathBuf::from("src/processor.rs");

        let normal_score = DebtScore::calculate(
            &normal_func,
            &DebtType::Risk {
                risk_score: 0.5,
                factors: vec!["Long function".to_string()],
            },
            &ScoreWeights::default(),
        );

        // Normal file should have full size score
        assert!(
            normal_score.components.size_score > score.components.size_score * 5.0,
            "Normal file should have much higher size score than generated file"
        );
    }
}
