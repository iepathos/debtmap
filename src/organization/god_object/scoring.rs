//! # God Object Scoring (Pure Core)
//!
//! Pure functions for calculating god object scores and weights.
//!
//! ## Stillwater Architecture
//!
//! This is part of the **Pure Core** - deterministic math with no side effects.
//! All functions are:
//! - Deterministic: Same inputs → same outputs
//! - Side-effect free: No I/O, no mutations
//! - Composable: Can be chained together
//! - 100% testable: No mocks needed

use super::metrics_types::ComplexityMetrics;
use super::thresholds::{ComplexityThresholds, GodObjectThresholds};

/// Calculate god object score from method, field, responsibility counts, and LOC.
///
/// **Pure function** - deterministic, no side effects.
///
/// # Arguments
///
/// * `method_count` - Number of methods in the type
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `thresholds` - God object thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
///
/// # Scoring Logic
///
/// The score is calculated based on:
/// - Method factor: ratio of methods to threshold (capped at 3.0)
/// - Field factor: ratio of fields to threshold (capped at 3.0)
/// - Responsibility factor: ratio of responsibilities to 3.0 (capped at 3.0)
/// - Size factor: ratio of LOC to threshold (capped at 3.0)
///
/// Violation-based scaling:
/// - 1 violation: minimum score 30.0
/// - 2 violations: minimum score 50.0
/// - 3+ violations: minimum score 70.0
pub fn calculate_god_object_score(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    thresholds: &GodObjectThresholds,
) -> f64 {
    let method_factor = (method_count as f64 / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if method_count > thresholds.max_methods {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply appropriate scoring based on violation severity
    // More nuanced approach to prevent over-flagging moderate files
    if violation_count > 0 {
        // Graduated minimum scores based on violation count
        let base_min_score = match violation_count {
            1 => 30.0, // Single violation: Moderate score
            2 => 50.0, // Two violations: Borderline CRITICAL
            _ => 70.0, // Three+ violations: Likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0
    }
}

/// Calculate complexity-weighted god object score.
///
/// **Pure function** - deterministic, no side effects.
///
/// Unlike raw method counting, this function weights each method by its
/// cyclomatic complexity, ensuring that 100 simple functions (complexity 1-3)
/// score better than 10 complex functions (complexity 17+).
///
/// # Arguments
///
/// * `weighted_method_count` - Sum of complexity weights for all functions
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `avg_complexity` - Average cyclomatic complexity across functions
/// * `thresholds` - God object thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
///
/// # Complexity Factors
///
/// - Low complexity (< 3.0): 0.7x multiplier (reward simple functions)
/// - Medium complexity (3.0-10.0): 1.0x multiplier (neutral)
/// - High complexity (> 10.0): 1.5x multiplier (penalize complex functions)
pub fn calculate_god_object_score_weighted(
    weighted_method_count: f64,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    avg_complexity: f64,
    thresholds: &GodObjectThresholds,
) -> f64 {
    // Use weighted count instead of raw count
    let method_factor = (weighted_method_count / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Add complexity bonus/penalty
    let complexity_factor = if avg_complexity < 3.0 {
        0.7 // Reward simple functions
    } else if avg_complexity > 10.0 {
        1.5 // Penalize complex functions
    } else {
        1.0
    };

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if weighted_method_count > thresholds.max_methods as f64 {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply complexity factor and ensure appropriate score for violations
    // Scale scores more conservatively to prevent small files from being CRITICAL
    if violation_count > 0 {
        // More nuanced minimum scores based on violation severity
        // 1 violation (e.g., just responsibilities): 30-50 range
        // 2 violations: 50-70 range
        // 3+ violations: 70+ range (CRITICAL territory)
        let base_min_score = match violation_count {
            1 => 30.0, // Moderate threshold - won't trigger CRITICAL (< 50)
            2 => 50.0, // High threshold - borderline CRITICAL
            _ => 70.0, // Multiple violations - likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * complexity_factor * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0 * complexity_factor
    }
}

// ============================================================================
// Spec 211: Method Complexity Weighting
// ============================================================================

/// Calculate complexity factor for God Object scoring (Spec 211).
///
/// **Pure function** - deterministic, no side effects.
///
/// Returns a multiplier in range [0.5, 3.0]:
/// - 0.5-1.0: Low complexity methods (simple struct, rewards clean code)
/// - 1.0-1.5: Average complexity (neutral)
/// - 1.5-2.0: High complexity (penalty)
/// - 2.0-3.0: Very high complexity (severe God Object signal)
///
/// ## Weighting Strategy
///
/// The factor combines four signals:
/// - **Average complexity (40%)**: High averages indicate uniformly complex methods
/// - **Max complexity (30%)**: Penalizes having any extremely complex method
/// - **Total complexity (20%)**: Accounts for overall complexity budget
/// - **Variance (10%)**: High variance indicates inconsistent code quality
///
/// # Arguments
///
/// * `metrics` - Aggregated complexity metrics from `calculate_complexity_metrics`
/// * `thresholds` - Complexity thresholds for the language
///
/// # Returns
///
/// Complexity factor in range [0.5, 3.0]
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     ComplexityMetrics, ComplexityThresholds, calculate_complexity_factor,
/// };
///
/// // Low complexity - should produce factor < 1.0
/// let low = ComplexityMetrics {
///     avg_cyclomatic: 2.0,
///     max_cyclomatic: 5,
///     total_cyclomatic: 20,
///     complexity_variance: 1.0,
///     ..Default::default()
/// };
/// let factor = calculate_complexity_factor(&low, &ComplexityThresholds::default());
/// assert!(factor < 1.0);
///
/// // High complexity - should produce factor > 1.5
/// let high = ComplexityMetrics {
///     avg_cyclomatic: 15.0,
///     max_cyclomatic: 30,
///     total_cyclomatic: 200,
///     complexity_variance: 10.0,
///     ..Default::default()
/// };
/// let factor = calculate_complexity_factor(&high, &ComplexityThresholds::default());
/// assert!(factor > 1.5);
/// ```
pub fn calculate_complexity_factor(
    metrics: &ComplexityMetrics,
    thresholds: &ComplexityThresholds,
) -> f64 {
    // Handle empty metrics case
    if metrics.total_cyclomatic == 0 {
        return 1.0; // Neutral factor for no data
    }

    // Average complexity contribution (40% weight)
    // Range: [0.5, 2.0] based on how avg compares to target
    let avg_factor = (metrics.avg_cyclomatic / thresholds.target_avg_complexity).clamp(0.5, 2.0);

    // Max complexity contribution (30% weight)
    // Penalize having any extremely complex method
    // Range: [0.5, 2.5] - higher cap because single complex method is a strong signal
    let max_factor =
        (metrics.max_cyclomatic as f64 / thresholds.max_method_complexity as f64).clamp(0.5, 2.5);

    // Total complexity contribution (20% weight)
    // Range: [0.5, 2.0] based on total complexity budget
    let total_factor =
        (metrics.total_cyclomatic as f64 / thresholds.target_total_complexity).clamp(0.5, 2.0);

    // Variance contribution (10% weight)
    // High variance indicates inconsistent quality
    // Range: [0.8, 1.5] - narrower range since variance is a weaker signal
    // Normalized against expected std dev of 5.0 (typical for moderate variance)
    let variance_factor = (metrics.complexity_variance / 5.0).clamp(0.8, 1.5);

    // Weighted combination
    let combined = avg_factor * 0.4 + max_factor * 0.3 + total_factor * 0.2 + variance_factor * 0.1;

    combined.clamp(0.5, 3.0)
}

/// Calculate God Object score incorporating method complexity (Spec 211).
///
/// **Pure function** - deterministic, no side effects.
///
/// This is an enhanced version of `calculate_god_object_score_weighted` that
/// incorporates detailed complexity metrics. A struct with 15 highly complex
/// methods scores higher than one with 15 simple accessors.
///
/// # Arguments
///
/// * `weighted_method_count` - Sum of complexity weights for all functions (Spec 209)
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `complexity_metrics` - Aggregated complexity metrics (Spec 211)
/// * `thresholds` - God object thresholds for the language
/// * `complexity_thresholds` - Complexity thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
///
/// # Scoring Logic
///
/// The score is calculated by:
/// 1. Computing base factors (method, field, responsibility, size)
/// 2. Computing complexity factor from `calculate_complexity_factor`
/// 3. Applying complexity factor to the method contribution via square root
///    (to moderate its impact and prevent extreme scores)
/// 4. Applying violation-based minimum scores
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     ComplexityMetrics, ComplexityThresholds, GodObjectThresholds,
///     calculate_god_object_score_with_complexity,
/// };
///
/// // Simple struct with low complexity - should score lower
/// let simple_metrics = ComplexityMetrics {
///     avg_cyclomatic: 1.5,
///     max_cyclomatic: 3,
///     total_cyclomatic: 15,
///     ..Default::default()
/// };
/// let simple_score = calculate_god_object_score_with_complexity(
///     15.0, 5, 3, 200,
///     &simple_metrics,
///     &GodObjectThresholds::default(),
///     &ComplexityThresholds::default(),
/// );
///
/// // Complex struct - should score higher
/// let complex_metrics = ComplexityMetrics {
///     avg_cyclomatic: 12.0,
///     max_cyclomatic: 25,
///     total_cyclomatic: 180,
///     ..Default::default()
/// };
/// let complex_score = calculate_god_object_score_with_complexity(
///     15.0, 5, 3, 200,
///     &complex_metrics,
///     &GodObjectThresholds::default(),
///     &ComplexityThresholds::default(),
/// );
///
/// assert!(complex_score > simple_score);
/// ```
#[allow(clippy::too_many_arguments)]
pub fn calculate_god_object_score_with_complexity(
    weighted_method_count: f64,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_metrics: &ComplexityMetrics,
    thresholds: &GodObjectThresholds,
    complexity_thresholds: &ComplexityThresholds,
) -> f64 {
    // Existing factors (same as calculate_god_object_score_weighted)
    let method_factor = (weighted_method_count / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // NEW: Complexity factor from Spec 211
    let complexity_factor = calculate_complexity_factor(complexity_metrics, complexity_thresholds);

    // Apply complexity factor to method contribution via square root
    // This moderates the impact: sqrt(2.0) ≈ 1.41, sqrt(3.0) ≈ 1.73
    let adjusted_method_factor = method_factor * complexity_factor.sqrt();

    // Calculate violation count for minimum score determination
    let violation_count = count_violations_with_complexity(
        weighted_method_count,
        field_count,
        responsibility_count,
        lines_of_code,
        complexity_metrics,
        thresholds,
    );

    // Exponential scaling for severe violations
    let base_score = adjusted_method_factor * field_factor * responsibility_factor * size_factor;

    // Apply appropriate scoring based on violation severity
    if violation_count > 0 {
        let base_min_score = match violation_count {
            1 => 30.0,
            2 => 50.0,
            _ => 70.0,
        };
        let score = base_score * 20.0 * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0
    }
}

/// Count violations including complexity-based violations (Spec 211).
fn count_violations_with_complexity(
    weighted_method_count: f64,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_metrics: &ComplexityMetrics,
    thresholds: &GodObjectThresholds,
) -> usize {
    let mut violations = 0;

    // Standard violations
    if weighted_method_count > thresholds.max_methods as f64 {
        violations += 1;
    }
    if field_count > thresholds.max_fields {
        violations += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violations += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violations += 1;
    }

    // Total complexity violation
    if complexity_metrics.total_cyclomatic > thresholds.max_complexity {
        violations += 1;
    }

    // NEW: Complexity-based violations from Spec 211
    // Single extremely complex method (>25 cyclomatic)
    if complexity_metrics.max_cyclomatic > 25 {
        violations += 1;
    }
    // High average complexity (>10 average)
    if complexity_metrics.avg_cyclomatic > 10.0 {
        violations += 1;
    }

    violations
}

// ============================================================================
// Spec 213: Pure Function Method Weighting Integration
// ============================================================================

use super::classification_types::MethodSelfUsageBreakdown;

/// Calculate God Object score incorporating self-usage weighting (Spec 213).
///
/// **Pure function** - deterministic, no side effects.
///
/// This function integrates pure function detection into god object scoring.
/// Structs with many pure helper methods get significantly lower scores because
/// functional decomposition is good design, not god object sprawl.
///
/// # Arguments
///
/// * `method_breakdown` - Self-usage breakdown from `MethodSelfUsageBreakdown`
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `complexity_metrics` - Aggregated complexity metrics (Spec 211)
/// * `thresholds` - God object thresholds for the language
/// * `complexity_thresholds` - Complexity thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
///
/// # Scoring Adjustments
///
/// - `is_mostly_pure()`: Score reduced by 30% (functional design bonus)
/// - `is_highly_pure()`: Score reduced by 50% (strong functional design bonus)
/// - Uses `weighted_count()` instead of raw method count
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     ComplexityMetrics, ComplexityThresholds, GodObjectThresholds,
///     MethodSelfUsageBreakdown, calculate_god_object_score_with_self_usage,
/// };
///
/// // Struct with 24 methods, 21 of which are pure helpers
/// let breakdown = MethodSelfUsageBreakdown {
///     total_methods: 24,
///     instance_methods: 3,
///     pure_associated: 21,
///     unused_self: 0,
/// };
///
/// let metrics = ComplexityMetrics::default();
///
/// let score = calculate_god_object_score_with_self_usage(
///     &breakdown,
///     3, // fields
///     1, // responsibility
///     200, // lines
///     &metrics,
///     &GodObjectThresholds::default(),
///     &ComplexityThresholds::default(),
/// );
///
/// // With 21/24 pure helpers (87.5%), should have heavily reduced score
/// assert!(score < 30.0, "Highly pure struct should score LOW, got {}", score);
/// ```
#[allow(clippy::too_many_arguments)]
pub fn calculate_god_object_score_with_self_usage(
    method_breakdown: &MethodSelfUsageBreakdown,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_metrics: &ComplexityMetrics,
    thresholds: &GodObjectThresholds,
    complexity_thresholds: &ComplexityThresholds,
) -> f64 {
    // Use weighted method count from breakdown (Spec 213)
    let weighted_method_count = method_breakdown.weighted_count();

    // Calculate base score using existing logic (Spec 211)
    let base_score = calculate_god_object_score_with_complexity(
        weighted_method_count,
        field_count,
        responsibility_count,
        lines_of_code,
        complexity_metrics,
        thresholds,
        complexity_thresholds,
    );

    // Apply functional design bonus (Spec 213)
    // Structs with many pure helpers get score reduction
    let functional_factor = if method_breakdown.is_highly_pure() {
        0.5 // 50% reduction for highly pure (>70% pure methods)
    } else if method_breakdown.is_mostly_pure() {
        0.7 // 30% reduction for mostly pure (>50% pure methods)
    } else {
        1.0 // No adjustment
    };

    base_score * functional_factor
}

/// Calculate effective method count combining all weighting factors (Spec 209, 211, 213).
///
/// This provides the "effective methods" count shown in output, which represents
/// how many "equivalent substantive instance methods" the struct has.
///
/// # Arguments
///
/// * `method_breakdown` - Self-usage breakdown from Spec 213
/// * `avg_complexity` - Average cyclomatic complexity
///
/// # Returns
///
/// Effective method count considering all weighting factors.
///
/// # Weighting Formula
///
/// For each method type:
/// - Instance methods: `1.0 * complexity_factor`
/// - Pure associated: `0.2 * complexity_factor`
/// - Unused self: `0.3 * complexity_factor`
///
/// Where complexity_factor is:
/// - Low complexity (< 3.0): 0.8
/// - Normal complexity: 1.0
/// - High complexity (> 10.0): 1.5
pub fn calculate_effective_method_count(
    method_breakdown: &MethodSelfUsageBreakdown,
    avg_complexity: f64,
) -> f64 {
    let complexity_factor = if avg_complexity < 3.0 {
        0.8 // Simple functions get bonus
    } else if avg_complexity > 10.0 {
        1.5 // Complex functions get penalty
    } else {
        1.0
    };

    method_breakdown.weighted_count() * complexity_factor
}

// ============================================================================
// Spec 215: Functional Decomposition Recognition
// ============================================================================

use super::classification_types::FunctionalDecompositionMetrics;

/// Calculate God Object score with functional decomposition bonus (Spec 215).
///
/// **Pure function** - deterministic, no side effects.
///
/// This function applies a significant score reduction for code that follows
/// functional decomposition patterns. A struct with many small, pure helper
/// functions composing into a few orchestrators represents good design, not
/// a god object.
///
/// # Arguments
///
/// * `method_breakdown` - Self-usage breakdown from Spec 213
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `complexity_metrics` - Aggregated complexity metrics (Spec 211)
/// * `functional_metrics` - Functional decomposition metrics (Spec 215)
/// * `thresholds` - God object thresholds for the language
/// * `complexity_thresholds` - Complexity thresholds for the language
///
/// # Returns
///
/// God object score (0-100+) with functional bonus applied.
///
/// # Score Adjustments
///
/// | Functional Score | Multiplier | Description |
/// |------------------|------------|-------------|
/// | >= 0.7 | 0.3x | Strong functional design |
/// | >= 0.5 | 0.5x | Moderate functional style |
/// | >= 0.3 | 0.75x | Some functional elements |
/// | < 0.3 | 1.0x | Traditional OOP style |
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     ComplexityMetrics, ComplexityThresholds, GodObjectThresholds,
///     MethodSelfUsageBreakdown, FunctionalDecompositionMetrics,
///     calculate_god_object_score_with_functional_bonus,
/// };
///
/// // CallResolver example: 24 methods, 21 pure helpers, 3 orchestrators
/// let breakdown = MethodSelfUsageBreakdown {
///     total_methods: 24,
///     instance_methods: 3,
///     pure_associated: 21,
///     unused_self: 0,
/// };
///
/// let functional_metrics = FunctionalDecompositionMetrics {
///     pure_method_ratio: 0.875,
///     orchestrator_count: 3,
///     pure_helper_count: 21,
///     avg_pure_method_loc: 8.0,
///     composition_patterns: vec![],
///     functional_score: 0.75,
/// };
///
/// let score = calculate_god_object_score_with_functional_bonus(
///     &breakdown,
///     3, // fields
///     1, // responsibility
///     200, // lines
///     &ComplexityMetrics::default(),
///     &functional_metrics,
///     &GodObjectThresholds::default(),
///     &ComplexityThresholds::default(),
/// );
///
/// // Score should be low due to functional bonus
/// assert!(score < 30.0, "Functional decomposition should score low, got {}", score);
/// ```
#[allow(clippy::too_many_arguments)]
pub fn calculate_god_object_score_with_functional_bonus(
    method_breakdown: &MethodSelfUsageBreakdown,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_metrics: &ComplexityMetrics,
    functional_metrics: &FunctionalDecompositionMetrics,
    thresholds: &GodObjectThresholds,
    complexity_thresholds: &ComplexityThresholds,
) -> f64 {
    // Calculate base score using Spec 213 (self-usage weighting)
    let base_score = calculate_god_object_score_with_self_usage(
        method_breakdown,
        field_count,
        responsibility_count,
        lines_of_code,
        complexity_metrics,
        thresholds,
        complexity_thresholds,
    );

    // Apply functional decomposition bonus
    base_score * functional_metrics.score_multiplier()
}

/// Apply functional decomposition bonus to an existing score (Spec 215).
///
/// **Pure function** - deterministic, no side effects.
///
/// This is a simpler version that takes a pre-calculated base score and
/// applies the functional bonus multiplier.
///
/// # Arguments
///
/// * `base_score` - Pre-calculated god object score
/// * `functional_metrics` - Functional decomposition metrics
///
/// # Returns
///
/// Adjusted score with functional bonus applied.
pub fn apply_functional_bonus(
    base_score: f64,
    functional_metrics: &FunctionalDecompositionMetrics,
) -> f64 {
    base_score * functional_metrics.score_multiplier()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoring_deterministic() {
        let thresholds = GodObjectThresholds::default();
        let score1 = calculate_god_object_score(20, 15, 5, 500, &thresholds);
        let score2 = calculate_god_object_score(20, 15, 5, 500, &thresholds);
        assert_eq!(score1, score2);
    }

    #[test]
    fn test_weighted_scoring_deterministic() {
        let thresholds = GodObjectThresholds::default();
        let score1 = calculate_god_object_score_weighted(25.0, 15, 5, 500, 5.0, &thresholds);
        let score2 = calculate_god_object_score_weighted(25.0, 15, 5, 500, 5.0, &thresholds);
        assert_eq!(score1, score2);
    }

    #[test]
    fn test_scoring_zero_methods() {
        let thresholds = GodObjectThresholds::default();
        let score = calculate_god_object_score(0, 0, 0, 0, &thresholds);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_scoring_zero_responsibilities() {
        let thresholds = GodObjectThresholds::default();
        let score = calculate_god_object_score(10, 5, 0, 100, &thresholds);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_scoring_threshold_boundary() {
        let thresholds = GodObjectThresholds::default();
        // Exactly at threshold should not trigger violation
        let score = calculate_god_object_score(
            thresholds.max_methods,
            thresholds.max_fields,
            thresholds.max_traits,
            thresholds.max_lines,
            &thresholds,
        );
        // Should use non-violation scoring (multiplier of 10.0)
        assert!(score < 30.0); // Less than minimum violation score
    }

    #[test]
    fn test_scoring_single_violation() {
        let thresholds = GodObjectThresholds::default();
        // Just over method threshold
        let score = calculate_god_object_score(
            thresholds.max_methods + 1,
            thresholds.max_fields,
            thresholds.max_traits,
            thresholds.max_lines,
            &thresholds,
        );
        // Should have minimum score of 30.0 for single violation
        assert!(score >= 30.0);
    }

    #[test]
    fn test_scoring_multiple_violations() {
        let thresholds = GodObjectThresholds::default();
        // Three violations
        let score = calculate_god_object_score(
            thresholds.max_methods + 10,
            thresholds.max_fields + 10,
            thresholds.max_traits + 1,
            thresholds.max_lines,
            &thresholds,
        );
        // Should have minimum score of 70.0 for 3+ violations
        assert!(score >= 70.0);
    }

    #[test]
    fn test_weighted_vs_unweighted_consistency() {
        let thresholds = GodObjectThresholds::default();
        // When weighted_count == method_count and avg_complexity is neutral
        let method_count = 20;
        let field_count = 15;
        let resp_count = 5;
        let loc = 500;

        let unweighted =
            calculate_god_object_score(method_count, field_count, resp_count, loc, &thresholds);
        let weighted = calculate_god_object_score_weighted(
            method_count as f64,
            field_count,
            resp_count,
            loc,
            5.0, // Medium complexity
            &thresholds,
        );

        // Should be equal when complexity factor is 1.0
        assert_eq!(unweighted, weighted);
    }

    #[test]
    fn test_weighted_low_complexity_bonus() {
        let thresholds = GodObjectThresholds::default();
        let normal = calculate_god_object_score_weighted(20.0, 15, 5, 500, 5.0, &thresholds);
        let low_complexity =
            calculate_god_object_score_weighted(20.0, 15, 5, 500, 2.0, &thresholds);

        // Low complexity should score lower (better)
        assert!(low_complexity < normal);
    }

    #[test]
    fn test_weighted_high_complexity_penalty() {
        let thresholds = GodObjectThresholds::default();
        let normal = calculate_god_object_score_weighted(20.0, 15, 5, 500, 5.0, &thresholds);
        let high_complexity =
            calculate_god_object_score_weighted(20.0, 15, 5, 500, 15.0, &thresholds);

        // High complexity should score higher (worse)
        assert!(high_complexity > normal);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn score_never_negative(
            method_count in 0..1000usize,
            field_count in 0..200usize,
            resp_count in 0..100usize,
            loc in 0..10000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score(
                method_count,
                field_count,
                resp_count,
                loc,
                &thresholds
            );
            prop_assert!(score >= 0.0);
        }

        #[test]
        fn weighted_score_never_negative(
            weighted_count in 0.0..1000.0f64,
            field_count in 0..200usize,
            resp_count in 0..100usize,
            loc in 0..10000usize,
            avg_complexity in 1.0..30.0f64
        ) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                avg_complexity,
                &thresholds
            );
            prop_assert!(score >= 0.0);
        }

        #[test]
        fn score_monotonic_in_methods(
            base in 10..100usize,
            delta in 1..50usize,
            field_count in 5..50usize,
            resp_count in 1..10usize,
            loc in 100..1000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score1 = calculate_god_object_score(base, field_count, resp_count, loc, &thresholds);
            let score2 = calculate_god_object_score(base + delta, field_count, resp_count, loc, &thresholds);
            prop_assert!(score2 >= score1);
        }

        #[test]
        fn score_monotonic_in_fields(
            method_count in 10..100usize,
            base in 5..50usize,
            delta in 1..20usize,
            resp_count in 1..10usize,
            loc in 100..1000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score1 = calculate_god_object_score(method_count, base, resp_count, loc, &thresholds);
            let score2 = calculate_god_object_score(method_count, base + delta, resp_count, loc, &thresholds);
            prop_assert!(score2 >= score1);
        }

        #[test]
        fn score_monotonic_in_responsibilities(
            method_count in 10..100usize,
            field_count in 5..50usize,
            base in 1..10usize,
            delta in 1..5usize,
            loc in 100..1000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score1 = calculate_god_object_score(method_count, field_count, base, loc, &thresholds);
            let score2 = calculate_god_object_score(method_count, field_count, base + delta, loc, &thresholds);
            prop_assert!(score2 >= score1);
        }

        #[test]
        fn weighted_score_reasonable_bounds(
            weighted_count in 1.0..500.0f64,
            field_count in 1..100usize,
            resp_count in 1..20usize,
            loc in 100..5000usize,
            avg_complexity in 1.0..20.0f64
        ) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                avg_complexity,
                &thresholds
            );
            // Score should be reasonable (not infinity, not NaN)
            prop_assert!(score.is_finite());
            // Score should be positive
            prop_assert!(score >= 0.0);
        }

        #[test]
        fn complexity_factor_affects_score(
            weighted_count in 20.0..100.0f64,
            field_count in 10..50usize,
            resp_count in 3..10usize,
            loc in 500..2000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            // Use avg_complexity values that map to different categories:
            // 2.0 < 3.0 -> 0.7x multiplier (low complexity)
            // 15.0 > 10.0 -> 1.5x multiplier (high complexity)
            let low_complexity = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                2.0, // Low complexity: 0.7x
                &thresholds
            );
            let high_complexity = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                15.0, // High complexity: 1.5x
                &thresholds
            );
            // Property: High complexity (15.0) should score at least as high as low complexity (2.0)
            // The complexity factors are: low=0.7x, high=1.5x (ratio ~2.14)
            //
            // Note: Due to minimum score thresholds (30.0, 50.0, 70.0 based on violation count),
            // the actual scores might be clamped, which can reduce or eliminate the difference.
            // However, the high complexity should NEVER score lower than low complexity.
            prop_assert!(high_complexity >= low_complexity,
                "High complexity ({}) should be >= low complexity ({})",
                high_complexity, low_complexity);
        }
    }
}

// ============================================================================
// Spec 211: Method Complexity Weighting Tests
// ============================================================================

#[cfg(test)]
mod spec_211_tests {
    use super::*;

    // -------------------------------------------------------------------------
    // calculate_complexity_factor tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_complexity_factor_low() {
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 2.0,
            max_cyclomatic: 5,
            total_cyclomatic: 20,
            complexity_variance: 1.0,
            ..Default::default()
        };
        let thresholds = ComplexityThresholds::default();

        let factor = calculate_complexity_factor(&metrics, &thresholds);
        assert!(
            factor < 1.0,
            "Low complexity should produce factor < 1.0, got {}",
            factor
        );
    }

    #[test]
    fn test_complexity_factor_high() {
        // Use extreme values to ensure factor > 2.0
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 20.0,      // 4x target (5.0)
            max_cyclomatic: 50,        // 3.3x max threshold (15)
            total_cyclomatic: 300,     // 4x target (75)
            complexity_variance: 15.0, // 3x normalized value (5.0)
            ..Default::default()
        };
        let thresholds = ComplexityThresholds::default();

        let factor = calculate_complexity_factor(&metrics, &thresholds);
        assert!(
            factor > 2.0,
            "High complexity should produce factor > 2.0, got {}",
            factor
        );
    }

    #[test]
    fn test_complexity_factor_neutral() {
        // Metrics at exactly the target thresholds
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 5.0,  // target_avg_complexity
            max_cyclomatic: 15,   // max_method_complexity
            total_cyclomatic: 75, // target_total_complexity
            complexity_variance: 5.0,
            ..Default::default()
        };
        let thresholds = ComplexityThresholds::default();

        let factor = calculate_complexity_factor(&metrics, &thresholds);
        // Should be approximately 1.0 (neutral)
        assert!(
            (factor - 1.0).abs() < 0.15,
            "Neutral complexity should produce factor ≈ 1.0, got {}",
            factor
        );
    }

    #[test]
    fn test_complexity_factor_empty_metrics() {
        let metrics = ComplexityMetrics::default();
        let thresholds = ComplexityThresholds::default();

        let factor = calculate_complexity_factor(&metrics, &thresholds);
        assert!(
            (factor - 1.0).abs() < f64::EPSILON,
            "Empty metrics should produce factor = 1.0, got {}",
            factor
        );
    }

    #[test]
    fn test_complexity_factor_clamped_low() {
        // Extremely low complexity
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 0.5,
            max_cyclomatic: 1,
            total_cyclomatic: 2,
            complexity_variance: 0.1,
            ..Default::default()
        };
        let thresholds = ComplexityThresholds::default();

        let factor = calculate_complexity_factor(&metrics, &thresholds);
        assert!(
            factor >= 0.5,
            "Factor should be clamped to >= 0.5, got {}",
            factor
        );
    }

    #[test]
    fn test_complexity_factor_clamped_high() {
        // Extremely high complexity
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 50.0,
            max_cyclomatic: 100,
            total_cyclomatic: 1000,
            complexity_variance: 50.0,
            ..Default::default()
        };
        let thresholds = ComplexityThresholds::default();

        let factor = calculate_complexity_factor(&metrics, &thresholds);
        assert!(
            factor <= 3.0,
            "Factor should be clamped to <= 3.0, got {}",
            factor
        );
    }

    #[test]
    fn test_complexity_factor_deterministic() {
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 7.5,
            max_cyclomatic: 20,
            total_cyclomatic: 100,
            complexity_variance: 4.0,
            ..Default::default()
        };
        let thresholds = ComplexityThresholds::default();

        let factor1 = calculate_complexity_factor(&metrics, &thresholds);
        let factor2 = calculate_complexity_factor(&metrics, &thresholds);
        assert_eq!(factor1, factor2);
    }

    // -------------------------------------------------------------------------
    // calculate_god_object_score_with_complexity tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_score_with_complexity_simple_struct() {
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 1.5,
            max_cyclomatic: 3,
            total_cyclomatic: 15,
            ..Default::default()
        };

        let score = calculate_god_object_score_with_complexity(
            15.0,
            5,
            3,
            200,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // Simple methods should result in reasonable score
        // With low complexity, the score should be moderate
        assert!(
            score.is_finite() && score >= 0.0,
            "Score should be valid, got {}",
            score
        );
    }

    #[test]
    fn test_score_with_complexity_complex_struct() {
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 12.0,
            max_cyclomatic: 25,
            total_cyclomatic: 180,
            ..Default::default()
        };

        let score = calculate_god_object_score_with_complexity(
            15.0,
            5,
            3,
            200,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // Complex methods should result in higher score
        assert!(
            score.is_finite() && score >= 0.0,
            "Score should be valid, got {}",
            score
        );
    }

    #[test]
    fn test_complex_scores_higher_than_simple() {
        let simple_metrics = ComplexityMetrics {
            avg_cyclomatic: 1.5,
            max_cyclomatic: 3,
            total_cyclomatic: 15,
            complexity_variance: 0.5,
            ..Default::default()
        };

        let complex_metrics = ComplexityMetrics {
            avg_cyclomatic: 12.0,
            max_cyclomatic: 25,
            total_cyclomatic: 180,
            complexity_variance: 8.0,
            ..Default::default()
        };

        let simple_score = calculate_god_object_score_with_complexity(
            15.0,
            10,
            4,
            500,
            &simple_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        let complex_score = calculate_god_object_score_with_complexity(
            15.0,
            10,
            4,
            500,
            &complex_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        assert!(
            complex_score > simple_score,
            "Complex struct ({}) should score higher than simple struct ({})",
            complex_score,
            simple_score
        );
    }

    #[test]
    fn test_one_complex_method_scores_higher_than_many_simple() {
        // Spec 211: A struct with one 50-complexity method should score higher
        // than a struct with 10 5-complexity methods
        let one_complex = ComplexityMetrics {
            avg_cyclomatic: 50.0,
            max_cyclomatic: 50,
            total_cyclomatic: 50,
            complexity_variance: 0.0,
            ..Default::default()
        };

        let many_simple = ComplexityMetrics {
            avg_cyclomatic: 5.0,
            max_cyclomatic: 5,
            total_cyclomatic: 50, // Same total
            complexity_variance: 0.0,
            ..Default::default()
        };

        // For this test, use same parameters except complexity metrics
        let one_complex_score = calculate_god_object_score_with_complexity(
            1.0, // 1 method
            5,
            2,
            100,
            &one_complex,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        let many_simple_score = calculate_god_object_score_with_complexity(
            10.0, // 10 methods
            5,
            2,
            100,
            &many_simple,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // The struct with one super-complex method should score higher due to
        // the max_cyclomatic penalty and avg_cyclomatic being way above target
        assert!(
            one_complex_score > many_simple_score,
            "One 50-complexity method ({}) should score higher than 10 5-complexity methods ({})",
            one_complex_score,
            many_simple_score
        );
    }

    #[test]
    fn test_high_max_triggers_violation() {
        // max_cyclomatic > 25 should add a violation
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 5.0,
            max_cyclomatic: 30, // > 25 threshold
            total_cyclomatic: 50,
            ..Default::default()
        };

        let thresholds = GodObjectThresholds::default();
        let violations = count_violations_with_complexity(
            10.0, // under max_methods
            5,    // under max_fields
            3,    // under max_traits
            500,  // under max_lines
            &metrics,
            &thresholds,
        );

        assert!(
            violations >= 1,
            "max_cyclomatic > 25 should trigger a violation, got {} violations",
            violations
        );
    }

    #[test]
    fn test_high_avg_triggers_violation() {
        // avg_cyclomatic > 10 should add a violation
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 12.0, // > 10 threshold
            max_cyclomatic: 15,
            total_cyclomatic: 60,
            ..Default::default()
        };

        let thresholds = GodObjectThresholds::default();
        let violations = count_violations_with_complexity(10.0, 5, 3, 500, &metrics, &thresholds);

        assert!(
            violations >= 1,
            "avg_cyclomatic > 10.0 should trigger a violation, got {} violations",
            violations
        );
    }

    #[test]
    fn test_score_deterministic() {
        let metrics = ComplexityMetrics {
            avg_cyclomatic: 8.0,
            max_cyclomatic: 15,
            total_cyclomatic: 120,
            complexity_variance: 4.0,
            ..Default::default()
        };

        let score1 = calculate_god_object_score_with_complexity(
            20.0,
            12,
            5,
            800,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        let score2 = calculate_god_object_score_with_complexity(
            20.0,
            12,
            5,
            800,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        assert_eq!(score1, score2);
    }

    // -------------------------------------------------------------------------
    // Property-based tests for Spec 211
    // -------------------------------------------------------------------------

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn complexity_factor_in_range(
            avg in 0.5f64..30.0,
            max in 1u32..100,
            total in 1u32..500,
            variance in 0.0f64..20.0
        ) {
            let metrics = ComplexityMetrics {
                avg_cyclomatic: avg,
                max_cyclomatic: max,
                total_cyclomatic: total,
                complexity_variance: variance,
                ..Default::default()
            };
            let thresholds = ComplexityThresholds::default();

            let factor = calculate_complexity_factor(&metrics, &thresholds);
            prop_assert!((0.5..=3.0).contains(&factor),
                "Factor {} out of range [0.5, 3.0]", factor);
        }

        #[test]
        fn score_with_complexity_non_negative(
            weighted_count in 1.0..100.0f64,
            field_count in 1..50usize,
            resp_count in 1..10usize,
            loc in 100..2000usize,
            avg in 1.0f64..20.0,
            max in 1u32..50,
            total in 1u32..300
        ) {
            let metrics = ComplexityMetrics {
                avg_cyclomatic: avg,
                max_cyclomatic: max,
                total_cyclomatic: total,
                ..Default::default()
            };

            let score = calculate_god_object_score_with_complexity(
                weighted_count,
                field_count,
                resp_count,
                loc,
                &metrics,
                &GodObjectThresholds::default(),
                &ComplexityThresholds::default(),
            );

            prop_assert!(score >= 0.0 && score.is_finite(),
                "Score {} is invalid", score);
        }

        #[test]
        fn higher_complexity_means_higher_or_equal_score(
            weighted_count in 10.0..50.0f64,
            field_count in 5..20usize,
            resp_count in 2..6usize,
            loc in 300..1000usize,
            low_avg in 1.0f64..5.0,
            high_avg in 10.0f64..20.0
        ) {
            let low_metrics = ComplexityMetrics {
                avg_cyclomatic: low_avg,
                max_cyclomatic: 5,
                total_cyclomatic: 50,
                ..Default::default()
            };

            let high_metrics = ComplexityMetrics {
                avg_cyclomatic: high_avg,
                max_cyclomatic: 25,
                total_cyclomatic: 200,
                ..Default::default()
            };

            let low_score = calculate_god_object_score_with_complexity(
                weighted_count,
                field_count,
                resp_count,
                loc,
                &low_metrics,
                &GodObjectThresholds::default(),
                &ComplexityThresholds::default(),
            );

            let high_score = calculate_god_object_score_with_complexity(
                weighted_count,
                field_count,
                resp_count,
                loc,
                &high_metrics,
                &GodObjectThresholds::default(),
                &ComplexityThresholds::default(),
            );

            prop_assert!(high_score >= low_score,
                "High complexity score ({}) should be >= low complexity score ({})",
                high_score, low_score);
        }
    }
}

// ============================================================================
// Spec 213: Pure Function Method Weighting Tests
// ============================================================================

#[cfg(test)]
mod spec_213_tests {
    use super::*;

    #[test]
    fn test_method_self_usage_breakdown_from_classifications() {
        use super::super::classification_types::MethodSelfUsage;

        let classifications = vec![
            MethodSelfUsage::InstanceMethod,
            MethodSelfUsage::InstanceMethod,
            MethodSelfUsage::InstanceMethod,
            MethodSelfUsage::PureAssociated,
            MethodSelfUsage::PureAssociated,
            MethodSelfUsage::PureAssociated,
            MethodSelfUsage::PureAssociated,
            MethodSelfUsage::PureAssociated,
            MethodSelfUsage::UnusedSelf,
        ];

        let breakdown = MethodSelfUsageBreakdown::from_classifications(&classifications);

        assert_eq!(breakdown.total_methods, 9);
        assert_eq!(breakdown.instance_methods, 3);
        assert_eq!(breakdown.pure_associated, 5);
        assert_eq!(breakdown.unused_self, 1);
    }

    #[test]
    fn test_method_self_usage_breakdown_weighted_count() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 24,
            instance_methods: 3,
            pure_associated: 20,
            unused_self: 1,
        };

        // 3*1.0 + 20*0.2 + 1*0.3 = 3.0 + 4.0 + 0.3 = 7.3
        let weighted = breakdown.weighted_count();
        assert!(
            (weighted - 7.3).abs() < 0.01,
            "Expected 7.3, got {}",
            weighted
        );
    }

    #[test]
    fn test_method_self_usage_breakdown_pure_helper_count() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 24,
            instance_methods: 3,
            pure_associated: 20,
            unused_self: 1,
        };

        assert_eq!(breakdown.pure_helper_count(), 21);
    }

    #[test]
    fn test_method_self_usage_breakdown_pure_ratio() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 24,
            instance_methods: 3,
            pure_associated: 20,
            unused_self: 1,
        };

        // 21/24 = 0.875
        let ratio = breakdown.pure_ratio();
        assert!(
            (ratio - 0.875).abs() < 0.001,
            "Expected 0.875, got {}",
            ratio
        );
    }

    #[test]
    fn test_method_self_usage_breakdown_is_mostly_pure() {
        // >50% pure
        let mostly_pure = MethodSelfUsageBreakdown {
            total_methods: 10,
            instance_methods: 4,
            pure_associated: 6,
            unused_self: 0,
        };
        assert!(mostly_pure.is_mostly_pure());

        // <50% pure
        let not_mostly_pure = MethodSelfUsageBreakdown {
            total_methods: 10,
            instance_methods: 6,
            pure_associated: 4,
            unused_self: 0,
        };
        assert!(!not_mostly_pure.is_mostly_pure());
    }

    #[test]
    fn test_method_self_usage_breakdown_is_highly_pure() {
        // >70% pure
        let highly_pure = MethodSelfUsageBreakdown {
            total_methods: 10,
            instance_methods: 2,
            pure_associated: 8,
            unused_self: 0,
        };
        assert!(highly_pure.is_highly_pure());

        // <70% pure (but >50%)
        let not_highly_pure = MethodSelfUsageBreakdown {
            total_methods: 10,
            instance_methods: 4,
            pure_associated: 6,
            unused_self: 0,
        };
        assert!(!not_highly_pure.is_highly_pure());
        assert!(not_highly_pure.is_mostly_pure()); // Still mostly pure
    }

    #[test]
    fn test_method_self_usage_breakdown_display() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 24,
            instance_methods: 3,
            pure_associated: 20,
            unused_self: 1,
        };

        let display = format!("{}", breakdown);
        assert_eq!(display, "24 (3 instance, 21 pure helpers)");
    }

    #[test]
    fn test_method_self_usage_breakdown_empty() {
        let breakdown = MethodSelfUsageBreakdown::default();

        assert_eq!(breakdown.total_methods, 0);
        assert_eq!(breakdown.weighted_count(), 0.0);
        assert_eq!(breakdown.pure_ratio(), 0.0);
        assert!(!breakdown.is_mostly_pure());
    }

    #[test]
    fn test_calculate_god_object_score_with_self_usage_highly_pure() {
        // CallResolver example from spec: 24 methods, 21 pure
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 24,
            instance_methods: 3,
            pure_associated: 21,
            unused_self: 0,
        };

        let metrics = ComplexityMetrics::default();

        let score = calculate_god_object_score_with_self_usage(
            &breakdown,
            3,   // fields
            1,   // responsibility
            200, // lines
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // With 87.5% pure (highly pure), score should be significantly reduced
        // and well below CRITICAL threshold of 50
        assert!(
            score < 50.0,
            "Highly pure struct should score < 50 (CRITICAL threshold), got {}",
            score
        );
    }

    #[test]
    fn test_calculate_god_object_score_with_self_usage_mostly_pure() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 20,
            instance_methods: 8,
            pure_associated: 12,
            unused_self: 0,
        };

        let metrics = ComplexityMetrics {
            avg_cyclomatic: 5.0,
            max_cyclomatic: 10,
            total_cyclomatic: 100,
            ..Default::default()
        };

        let score = calculate_god_object_score_with_self_usage(
            &breakdown,
            5,   // fields
            2,   // responsibilities
            300, // lines
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // With 60% pure (mostly pure), should get 30% reduction
        assert!(
            score.is_finite() && score >= 0.0,
            "Score should be valid, got {}",
            score
        );
    }

    #[test]
    fn test_calculate_god_object_score_with_self_usage_instance_heavy() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 20,
            instance_methods: 18,
            pure_associated: 2,
            unused_self: 0,
        };

        let metrics = ComplexityMetrics {
            avg_cyclomatic: 8.0,
            max_cyclomatic: 15,
            total_cyclomatic: 160,
            ..Default::default()
        };

        let score = calculate_god_object_score_with_self_usage(
            &breakdown,
            10,  // fields
            5,   // responsibilities
            500, // lines
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // With only 10% pure, no functional bonus should apply
        // Score should be higher than pure equivalents
        assert!(
            score.is_finite() && score > 0.0,
            "Score should be valid and positive, got {}",
            score
        );
    }

    #[test]
    fn test_highly_pure_scores_lower_than_instance_heavy() {
        let metrics = ComplexityMetrics::default();

        // Same raw metrics, different pure ratios
        let highly_pure = MethodSelfUsageBreakdown {
            total_methods: 20,
            instance_methods: 3,
            pure_associated: 17,
            unused_self: 0,
        };

        let instance_heavy = MethodSelfUsageBreakdown {
            total_methods: 20,
            instance_methods: 17,
            pure_associated: 3,
            unused_self: 0,
        };

        let pure_score = calculate_god_object_score_with_self_usage(
            &highly_pure,
            5,
            3,
            300,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        let instance_score = calculate_god_object_score_with_self_usage(
            &instance_heavy,
            5,
            3,
            300,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        assert!(
            pure_score < instance_score,
            "Highly pure struct ({}) should score LOWER than instance heavy ({})",
            pure_score,
            instance_score
        );
    }

    #[test]
    fn test_calculate_effective_method_count_low_complexity() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 10,
            instance_methods: 5,
            pure_associated: 5,
            unused_self: 0,
        };

        // Weighted: 5*1.0 + 5*0.2 = 6.0
        // With low complexity (2.0), apply 0.8 factor: 6.0 * 0.8 = 4.8
        let effective = calculate_effective_method_count(&breakdown, 2.0);
        assert!(
            (effective - 4.8).abs() < 0.01,
            "Expected 4.8, got {}",
            effective
        );
    }

    #[test]
    fn test_calculate_effective_method_count_high_complexity() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 10,
            instance_methods: 5,
            pure_associated: 5,
            unused_self: 0,
        };

        // Weighted: 5*1.0 + 5*0.2 = 6.0
        // With high complexity (15.0), apply 1.5 factor: 6.0 * 1.5 = 9.0
        let effective = calculate_effective_method_count(&breakdown, 15.0);
        assert!(
            (effective - 9.0).abs() < 0.01,
            "Expected 9.0, got {}",
            effective
        );
    }

    #[test]
    fn test_calculate_effective_method_count_normal_complexity() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 10,
            instance_methods: 5,
            pure_associated: 5,
            unused_self: 0,
        };

        // Weighted: 5*1.0 + 5*0.2 = 6.0
        // With normal complexity (5.0), factor is 1.0: 6.0 * 1.0 = 6.0
        let effective = calculate_effective_method_count(&breakdown, 5.0);
        assert!(
            (effective - 6.0).abs() < 0.01,
            "Expected 6.0, got {}",
            effective
        );
    }

    #[test]
    fn test_score_with_self_usage_deterministic() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 15,
            instance_methods: 5,
            pure_associated: 10,
            unused_self: 0,
        };

        let metrics = ComplexityMetrics {
            avg_cyclomatic: 6.0,
            max_cyclomatic: 12,
            total_cyclomatic: 90,
            ..Default::default()
        };

        let score1 = calculate_god_object_score_with_self_usage(
            &breakdown,
            8,
            3,
            400,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        let score2 = calculate_god_object_score_with_self_usage(
            &breakdown,
            8,
            3,
            400,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        assert_eq!(score1, score2);
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_breakdown_weighted_count_non_negative(
            instance in 0usize..50,
            pure in 0usize..50,
            unused in 0usize..20
        ) {
            let breakdown = MethodSelfUsageBreakdown {
                total_methods: instance + pure + unused,
                instance_methods: instance,
                pure_associated: pure,
                unused_self: unused,
            };

            prop_assert!(breakdown.weighted_count() >= 0.0);
        }

        #[test]
        fn prop_pure_ratio_bounded(
            instance in 0usize..50,
            pure in 0usize..50,
            unused in 0usize..20
        ) {
            let breakdown = MethodSelfUsageBreakdown {
                total_methods: instance + pure + unused,
                instance_methods: instance,
                pure_associated: pure,
                unused_self: unused,
            };

            let ratio = breakdown.pure_ratio();
            prop_assert!((0.0..=1.0).contains(&ratio),
                "Pure ratio {} out of bounds", ratio);
        }

        #[test]
        fn prop_effective_count_non_negative(
            instance in 0usize..50,
            pure in 0usize..50,
            unused in 0usize..20,
            complexity in 1.0f64..25.0
        ) {
            let breakdown = MethodSelfUsageBreakdown {
                total_methods: instance + pure + unused,
                instance_methods: instance,
                pure_associated: pure,
                unused_self: unused,
            };

            let effective = calculate_effective_method_count(&breakdown, complexity);
            prop_assert!(effective >= 0.0 && effective.is_finite(),
                "Effective count {} invalid", effective);
        }

        #[test]
        fn prop_score_with_self_usage_finite(
            instance in 1usize..30,
            pure in 0usize..30,
            fields in 1usize..20,
            resp in 1usize..10,
            loc in 50usize..1000
        ) {
            let breakdown = MethodSelfUsageBreakdown {
                total_methods: instance + pure,
                instance_methods: instance,
                pure_associated: pure,
                unused_self: 0,
            };

            let metrics = ComplexityMetrics::default();

            let score = calculate_god_object_score_with_self_usage(
                &breakdown,
                fields,
                resp,
                loc,
                &metrics,
                &GodObjectThresholds::default(),
                &ComplexityThresholds::default(),
            );

            prop_assert!(score.is_finite() && score >= 0.0,
                "Score {} invalid", score);
        }
    }
}

// ============================================================================
// Spec 215: Functional Decomposition Recognition Tests
// ============================================================================

#[cfg(test)]
mod spec_215_tests {
    use super::*;

    #[test]
    fn test_functional_bonus_strong_functional_design() {
        // CallResolver example: 24 methods, 21 pure helpers, 3 orchestrators
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 24,
            instance_methods: 3,
            pure_associated: 21,
            unused_self: 0,
        };

        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.875,
            orchestrator_count: 3,
            pure_helper_count: 21,
            avg_pure_method_loc: 8.0,
            composition_patterns: vec![],
            functional_score: 0.80, // Strong functional (>= 0.7)
        };

        let metrics = ComplexityMetrics::default();

        let score = calculate_god_object_score_with_functional_bonus(
            &breakdown,
            3,   // fields
            1,   // responsibility
            200, // lines
            &metrics,
            &functional_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // Strong functional design should get 0.3x multiplier
        // Score should be low
        assert!(
            score < 30.0,
            "Strong functional design should score low, got {}",
            score
        );
    }

    #[test]
    fn test_functional_bonus_moderate_functional_style() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 20,
            instance_methods: 8,
            pure_associated: 12,
            unused_self: 0,
        };

        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.60,
            orchestrator_count: 5,
            pure_helper_count: 12,
            avg_pure_method_loc: 10.0,
            composition_patterns: vec![],
            functional_score: 0.55, // Moderate functional (0.5-0.7)
        };

        let metrics = ComplexityMetrics::default();

        let score = calculate_god_object_score_with_functional_bonus(
            &breakdown,
            5,
            2,
            300,
            &metrics,
            &functional_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // Moderate functional design should get 0.5x multiplier
        assert!(score.is_finite() && score >= 0.0);
    }

    #[test]
    fn test_functional_bonus_no_functional_pattern() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 20,
            instance_methods: 18,
            pure_associated: 2,
            unused_self: 0,
        };

        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.10,
            orchestrator_count: 10,
            pure_helper_count: 2,
            avg_pure_method_loc: 20.0,
            composition_patterns: vec![],
            functional_score: 0.20, // No functional pattern (< 0.3)
        };

        let metrics = ComplexityMetrics {
            avg_cyclomatic: 8.0,
            max_cyclomatic: 15,
            total_cyclomatic: 160,
            ..Default::default()
        };

        let score_with_bonus = calculate_god_object_score_with_functional_bonus(
            &breakdown,
            10,
            5,
            500,
            &metrics,
            &functional_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        let score_base = calculate_god_object_score_with_self_usage(
            &breakdown,
            10,
            5,
            500,
            &metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // No functional pattern = 1.0x multiplier = same score
        assert!(
            (score_with_bonus - score_base).abs() < 0.01,
            "No functional pattern should not affect score. Base: {}, With bonus: {}",
            score_base,
            score_with_bonus
        );
    }

    #[test]
    fn test_functional_bonus_deterministic() {
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 15,
            instance_methods: 5,
            pure_associated: 10,
            unused_self: 0,
        };

        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.67,
            orchestrator_count: 3,
            pure_helper_count: 10,
            avg_pure_method_loc: 6.0,
            composition_patterns: vec![],
            functional_score: 0.72,
        };

        let metrics = ComplexityMetrics::default();

        let score1 = calculate_god_object_score_with_functional_bonus(
            &breakdown,
            5,
            2,
            200,
            &metrics,
            &functional_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        let score2 = calculate_god_object_score_with_functional_bonus(
            &breakdown,
            5,
            2,
            200,
            &metrics,
            &functional_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        assert_eq!(score1, score2, "Scoring should be deterministic");
    }

    #[test]
    fn test_apply_functional_bonus_simple() {
        let high_functional = FunctionalDecompositionMetrics {
            functional_score: 0.80,
            ..Default::default()
        };

        let moderate_functional = FunctionalDecompositionMetrics {
            functional_score: 0.55,
            ..Default::default()
        };

        let weak_functional = FunctionalDecompositionMetrics {
            functional_score: 0.35,
            ..Default::default()
        };

        let no_functional = FunctionalDecompositionMetrics {
            functional_score: 0.20,
            ..Default::default()
        };

        let base_score = 100.0;

        // High functional: 0.3x
        let high_result = apply_functional_bonus(base_score, &high_functional);
        assert!(
            (high_result - 30.0).abs() < 0.01,
            "High functional should be 30.0, got {}",
            high_result
        );

        // Moderate functional: 0.5x
        let moderate_result = apply_functional_bonus(base_score, &moderate_functional);
        assert!(
            (moderate_result - 50.0).abs() < 0.01,
            "Moderate functional should be 50.0, got {}",
            moderate_result
        );

        // Weak functional: 0.75x
        let weak_result = apply_functional_bonus(base_score, &weak_functional);
        assert!(
            (weak_result - 75.0).abs() < 0.01,
            "Weak functional should be 75.0, got {}",
            weak_result
        );

        // No functional: 1.0x
        let no_result = apply_functional_bonus(base_score, &no_functional);
        assert!(
            (no_result - 100.0).abs() < 0.01,
            "No functional should be 100.0, got {}",
            no_result
        );
    }

    #[test]
    fn test_functional_bonus_reduces_high_scores() {
        // Scenario: A struct would normally score as CRITICAL (>50)
        // but has strong functional decomposition
        let breakdown = MethodSelfUsageBreakdown {
            total_methods: 30,
            instance_methods: 5,
            pure_associated: 25,
            unused_self: 0,
        };

        let functional_metrics = FunctionalDecompositionMetrics {
            pure_method_ratio: 0.833,
            orchestrator_count: 2,
            pure_helper_count: 25,
            avg_pure_method_loc: 5.0,
            composition_patterns: vec![],
            functional_score: 0.85, // Very strong functional
        };

        let metrics = ComplexityMetrics {
            avg_cyclomatic: 3.0,
            max_cyclomatic: 8,
            total_cyclomatic: 90,
            ..Default::default()
        };

        let score = calculate_god_object_score_with_functional_bonus(
            &breakdown,
            5,   // fields
            3,   // responsibilities
            400, // lines
            &metrics,
            &functional_metrics,
            &GodObjectThresholds::default(),
            &ComplexityThresholds::default(),
        );

        // With strong functional bonus (0.3x), should be well below CRITICAL
        assert!(
            score < 50.0,
            "Strong functional code should not be CRITICAL, got {}",
            score
        );
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_functional_bonus_never_increases_score(
            instance in 1usize..20,
            pure in 0usize..30,
            fields in 1usize..20,
            resp in 1usize..10,
            loc in 50usize..1000,
            functional_score in 0.0f64..1.0
        ) {
            let breakdown = MethodSelfUsageBreakdown {
                total_methods: instance + pure,
                instance_methods: instance,
                pure_associated: pure,
                unused_self: 0,
            };

            let functional_metrics = FunctionalDecompositionMetrics {
                functional_score,
                ..Default::default()
            };

            let metrics = ComplexityMetrics::default();

            let score_with_bonus = calculate_god_object_score_with_functional_bonus(
                &breakdown,
                fields,
                resp,
                loc,
                &metrics,
                &functional_metrics,
                &GodObjectThresholds::default(),
                &ComplexityThresholds::default(),
            );

            let score_base = calculate_god_object_score_with_self_usage(
                &breakdown,
                fields,
                resp,
                loc,
                &metrics,
                &GodObjectThresholds::default(),
                &ComplexityThresholds::default(),
            );

            // Functional bonus should never INCREASE the score
            prop_assert!(score_with_bonus <= score_base + 0.001,
                "Functional bonus should not increase score. Base: {}, With bonus: {}",
                score_base, score_with_bonus);
        }

        #[test]
        fn prop_functional_bonus_finite(
            instance in 1usize..20,
            pure in 0usize..30,
            fields in 1usize..20,
            resp in 1usize..10,
            loc in 50usize..1000,
            functional_score in 0.0f64..1.0
        ) {
            let breakdown = MethodSelfUsageBreakdown {
                total_methods: instance + pure,
                instance_methods: instance,
                pure_associated: pure,
                unused_self: 0,
            };

            let functional_metrics = FunctionalDecompositionMetrics {
                functional_score,
                ..Default::default()
            };

            let metrics = ComplexityMetrics::default();

            let score = calculate_god_object_score_with_functional_bonus(
                &breakdown,
                fields,
                resp,
                loc,
                &metrics,
                &functional_metrics,
                &GodObjectThresholds::default(),
                &ComplexityThresholds::default(),
            );

            prop_assert!(score.is_finite() && score >= 0.0,
                "Score {} is invalid", score);
        }
    }
}
