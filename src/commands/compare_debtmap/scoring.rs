//! Pure scoring functions for validation results.
//!
//! All functions in this module are pure and compute scores
//! without side effects. Functions are kept under 20 lines.

use super::types::{
    AnalysisSummary, ImprovedItems, NewItems, ResolvedItems, ScoringComponents, UnchangedCritical,
};

// =============================================================================
// Component Score Calculations
// =============================================================================

/// Pure: Calculate high priority resolution progress percentage
pub fn score_high_priority_progress(
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
    resolved: &ResolvedItems,
) -> f64 {
    if before_summary.high_priority_items == 0 {
        return 100.0;
    }

    let resolved_count = resolved.high_priority_count as f64;
    let addressed_count = before_summary
        .high_priority_items
        .saturating_sub(after_summary.high_priority_items) as f64;

    (addressed_count.max(resolved_count) / before_summary.high_priority_items as f64) * 100.0
}

/// Pure: Calculate overall score improvement percentage
pub fn score_overall_improvement(
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
) -> f64 {
    if before_summary.average_score <= 0.0 {
        return 0.0;
    }
    ((before_summary.average_score - after_summary.average_score) / before_summary.average_score)
        * 100.0
}

/// Pure: Calculate complexity reduction score (0-100)
pub fn score_complexity_reduction(improved: &ImprovedItems) -> f64 {
    improved.complexity_reduction * 100.0
}

/// Pure: Calculate regression penalty score (100 if no regressions, 0 otherwise)
pub fn score_regression_penalty(new_items: &NewItems) -> f64 {
    if new_items.critical_count == 0 {
        100.0
    } else {
        0.0
    }
}

// =============================================================================
// Score Adjustments
// =============================================================================

/// Pure: Apply penalty for unchanged critical items
pub fn apply_unchanged_penalty(
    score: f64,
    unchanged_critical: &UnchangedCritical,
    has_improvements: bool,
) -> f64 {
    if unchanged_critical.count == 0 {
        return score;
    }

    let (penalty_rate, max_penalty) = if has_improvements {
        (0.05, 0.25) // Lighter penalty when there are improvements
    } else {
        (0.1, 0.5)
    };

    let penalty_factor = 1.0 - (unchanged_critical.count as f64 * penalty_rate).min(max_penalty);
    score * penalty_factor
}

/// Pure: Apply minimum threshold for significant improvements
pub fn apply_minimum_threshold(score: f64, has_improvements: bool, score_improvement: f64) -> f64 {
    if has_improvements && score < 40.0 && score_improvement > 5.0 {
        40.0
    } else {
        score.clamp(0.0, 100.0)
    }
}

/// Pure: Calculate weighted score from scoring components
pub fn calculate_weighted_score(components: &ScoringComponents) -> f64 {
    components.high_priority * 0.4
        + components.improvement.max(0.0) * 0.3
        + components.complexity * 0.2
        + components.regression * 0.1
}

// =============================================================================
// Main Scoring Orchestration
// =============================================================================

/// Pure: Build scoring components from analysis results
pub fn build_scoring_components(
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
    resolved: &ResolvedItems,
    improved: &ImprovedItems,
    new_items: &NewItems,
) -> ScoringComponents {
    ScoringComponents {
        high_priority: score_high_priority_progress(before_summary, after_summary, resolved),
        improvement: score_overall_improvement(before_summary, after_summary),
        complexity: score_complexity_reduction(improved),
        regression: score_regression_penalty(new_items),
    }
}

/// Pure: Calculate overall improvement score
pub fn calculate_improvement_score(
    resolved: &ResolvedItems,
    improved: &ImprovedItems,
    new_items: &NewItems,
    unchanged_critical: &UnchangedCritical,
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
) -> f64 {
    if before_summary.total_items == 0 && after_summary.total_items == 0 {
        return 100.0;
    }

    let components =
        build_scoring_components(before_summary, after_summary, resolved, improved, new_items);
    let weighted_score = calculate_weighted_score(&components);
    let has_improvements = components.complexity > 0.0 || components.improvement > 0.0;
    let penalized = apply_unchanged_penalty(weighted_score, unchanged_critical, has_improvements);

    apply_minimum_threshold(penalized, has_improvements, components.improvement)
}

// =============================================================================
// Status Determination
// =============================================================================

/// Pure: Determine validation status based on score and metrics
pub fn determine_status(
    improvement_score: f64,
    new_items: &NewItems,
    before_summary: &AnalysisSummary,
    after_summary: &AnalysisSummary,
) -> String {
    let has_regressions = new_items.critical_count > 0;
    let all_high_priority_addressed =
        before_summary.high_priority_items > 0 && after_summary.high_priority_items == 0;
    let meets_score_threshold = improvement_score >= 75.0;

    if has_regressions {
        "failed"
    } else if all_high_priority_addressed || meets_score_threshold {
        "complete"
    } else if improvement_score >= 40.0 {
        "incomplete"
    } else {
        "failed"
    }
    .to_string()
}
