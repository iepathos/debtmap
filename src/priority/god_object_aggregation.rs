//! Pure aggregation functions for god object metrics.
//!
//! This module provides composable functions to aggregate metrics from
//! member functions into god object-level metrics.
//!
//! # Aggregation Strategies
//!
//! - **Complexity**: SUM of all functions (total burden)
//! - **Coverage**: Weighted average by function length
//! - **Dependencies**: Aggregated from ALL raw FunctionMetrics (complete architectural view)
//! - **Contextual Risk**: Average across member functions
//!
//! ## Dependency Aggregation
//!
//! Dependencies are aggregated from raw FunctionMetrics to provide a complete
//! architectural view of god object dependencies. This ensures that even if
//! individual functions don't exceed complexity thresholds, the god object
//! still shows all its cross-file dependencies for proper assessment.
//!
//! ## Complexity Distribution Analysis (Spec 268)
//!
//! For file-scope items, we now analyze how complexity is distributed:
//! - **Concentrated**: Max complexity > 50% of total → likely god function
//! - **Distributed**: Max complexity < 20% of total → well-structured file
//! - **Mixed**: Between 20-50% → needs investigation
//!
//! This helps distinguish "many simple functions" from "one god function".
//!
//! # Examples
//!
//! ```rust,ignore
//! let members = extract_member_functions(items.iter(), &file_path);
//! let metrics = aggregate_god_object_metrics(&members);
//!
//! assert!(metrics.total_cyclomatic > 0);
//! assert!(metrics.weighted_coverage.is_some());
//! ```

use crate::complexity::entropy_core::{EntropyConfig, EntropyScore, UniversalEntropyCalculator};
use crate::complexity::EntropyAnalysis;
use crate::core::FunctionMetrics;
use crate::priority::{TransitiveCoverage, UnifiedDebtItem};
use crate::risk::context::ContextualRisk;
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Default threshold for flagging individual functions as complex (Spec 268).
pub const FUNCTION_COMPLEXITY_THRESHOLD: u32 = 15;

/// Classification of how complexity is distributed across functions in a file.
///
/// Used to distinguish between:
/// - Files with one dominant god function (Concentrated)
/// - Files with many small, well-structured functions (Distributed)
/// - Files that need further investigation (Mixed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplexityDistribution {
    /// Max complexity > 50% of total - likely contains god function(s)
    Concentrated,
    /// Max complexity 20-50% of total - needs investigation
    Mixed,
    /// Max complexity < 20% of total - well-structured file
    Distributed,
}

impl ComplexityDistribution {
    /// Human-readable name for display
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Concentrated => "Concentrated",
            Self::Mixed => "Mixed",
            Self::Distributed => "Distributed",
        }
    }

    /// Explanation of what the classification means for refactoring
    pub fn classification_explanation(&self) -> &'static str {
        match self {
            Self::Concentrated => "Contains god function(s) - refactoring recommended",
            Self::Mixed => "Some complexity concentration - review recommended",
            Self::Distributed => "Well-Structured File - complexity evenly distributed",
        }
    }
}

/// Distribution metrics for file-scope complexity analysis (Spec 268).
///
/// These metrics help distinguish between:
/// - A file with many simple functions (low max, distributed complexity)
/// - A file with one or more god functions (high max, concentrated complexity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionMetrics {
    /// Number of functions in the file
    pub function_count: usize,
    /// Highest cyclomatic complexity among all functions
    pub max_complexity: u32,
    /// Average cyclomatic complexity per function
    pub avg_complexity: f64,
    /// Median cyclomatic complexity (robust to outliers)
    pub median_complexity: u32,
    /// Number of functions exceeding the complexity threshold
    pub exceeding_threshold: usize,
    /// Classification based on complexity distribution
    pub distribution: ComplexityDistribution,
    /// Production code lines (excluding test modules)
    pub production_loc: usize,
    /// Test code lines (inside #[cfg(test)] modules)
    pub test_loc: usize,
}

/// Aggregated metrics from member functions.
#[derive(Debug, Clone)]
pub struct GodObjectAggregatedMetrics {
    pub total_cyclomatic: u32,
    pub total_cognitive: u32,
    pub max_nesting_depth: u32,
    pub weighted_coverage: Option<TransitiveCoverage>,
    pub unique_upstream_callers: Vec<String>,
    pub unique_downstream_callees: Vec<String>,
    pub upstream_dependencies: usize,
    pub downstream_dependencies: usize,
    pub aggregated_contextual_risk: Option<ContextualRisk>,
    /// Total count of error swallowing patterns across all functions
    pub total_error_swallowing_count: u32,
    /// Unique error swallowing pattern types found
    pub error_swallowing_patterns: Vec<String>,
    /// Aggregated entropy analysis from member functions (Spec 218)
    pub aggregated_entropy: Option<EntropyAnalysis>,
    /// Distribution metrics for file-scope analysis (Spec 268)
    pub distribution_metrics: Option<DistributionMetrics>,
}

/// Extract member functions for a file.
///
/// Pure function that filters items by file path.
#[inline]
pub fn extract_member_functions<'a>(
    items: impl Iterator<Item = &'a UnifiedDebtItem>,
    file_path: &Path,
) -> Vec<&'a UnifiedDebtItem> {
    items
        .filter(|item| item.location.file == file_path)
        .collect()
}

/// Aggregate complexity: sum cyclomatic/cognitive, max nesting.
pub fn aggregate_complexity_metrics(members: &[&UnifiedDebtItem]) -> (u32, u32, u32) {
    let total_cyclomatic = members.iter().map(|m| m.cyclomatic_complexity).sum();
    let total_cognitive = members.iter().map(|m| m.cognitive_complexity).sum();
    let max_nesting = members.iter().map(|m| m.nesting_depth).max().unwrap_or(0);

    (total_cyclomatic, total_cognitive, max_nesting)
}

/// Aggregate coverage: weighted average by function length.
pub fn aggregate_coverage_metrics(members: &[&UnifiedDebtItem]) -> Option<TransitiveCoverage> {
    let coverages: Vec<_> = members
        .iter()
        .filter_map(|m| {
            m.transitive_coverage
                .as_ref()
                .map(|c| (c, m.function_length))
        })
        .collect();

    if coverages.is_empty() {
        return None;
    }

    let total_length: usize = coverages.iter().map(|(_, len)| len).sum();
    if total_length == 0 {
        return None;
    }

    let weighted_direct = coverages
        .iter()
        .map(|(cov, len)| cov.direct * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    let weighted_transitive = coverages
        .iter()
        .map(|(cov, len)| cov.transitive * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Collect all unique uncovered lines
    let uncovered_lines: Vec<usize> = coverages
        .iter()
        .flat_map(|(cov, _)| &cov.uncovered_lines)
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Collect all unique propagated_from function IDs
    let propagated_from = coverages
        .iter()
        .flat_map(|(cov, _)| &cov.propagated_from)
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Some(TransitiveCoverage {
        direct: weighted_direct,
        transitive: weighted_transitive,
        propagated_from,
        uncovered_lines,
    })
}

// =============================================================================
// Distribution Metrics Functions (Spec 268)
// =============================================================================

/// Calculate the median of a slice of complexity values.
///
/// Pure function that computes the median without modifying input.
/// Returns 0 for empty slices.
pub fn calculate_median(values: &[u32]) -> u32 {
    if values.is_empty() {
        return 0;
    }

    let mut sorted: Vec<u32> = values.to_vec();
    sorted.sort_unstable();

    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        // Even number of elements: average of two middle values
        (sorted[mid - 1] + sorted[mid]) / 2
    } else {
        // Odd number of elements: middle value
        sorted[mid]
    }
}

/// Classify complexity distribution based on max/total ratio.
///
/// - Concentrated: max > 50% of total (one function dominates)
/// - Distributed: max < 20% of total (well-structured file)
/// - Mixed: 20-50% (needs investigation)
pub fn classify_distribution(max_complexity: u32, total_complexity: u32) -> ComplexityDistribution {
    if total_complexity == 0 {
        return ComplexityDistribution::Distributed;
    }

    let ratio = max_complexity as f64 / total_complexity as f64;

    if ratio > 0.5 {
        ComplexityDistribution::Concentrated
    } else if ratio > 0.2 {
        ComplexityDistribution::Mixed
    } else {
        ComplexityDistribution::Distributed
    }
}

/// Calculate distribution metrics from UnifiedDebtItem members (Spec 268).
///
/// Returns metrics describing how complexity is distributed across functions,
/// helping distinguish well-structured files from those with god functions.
pub fn aggregate_distribution_metrics(members: &[&UnifiedDebtItem]) -> DistributionMetrics {
    let complexities: Vec<u32> = members.iter().map(|m| m.cyclomatic_complexity).collect();

    let total: u32 = complexities.iter().sum();
    let max = complexities.iter().max().copied().unwrap_or(0);
    let count = complexities.len();

    let avg = if count > 0 {
        total as f64 / count as f64
    } else {
        0.0
    };

    let median = calculate_median(&complexities);

    let exceeding = complexities
        .iter()
        .filter(|&&c| c > FUNCTION_COMPLEXITY_THRESHOLD)
        .count();

    let distribution = classify_distribution(max, total);

    // Calculate LOC from function lengths
    let production_loc: usize = members.iter().map(|m| m.function_length).sum();

    DistributionMetrics {
        function_count: count,
        max_complexity: max,
        avg_complexity: avg,
        median_complexity: median,
        exceeding_threshold: exceeding,
        distribution,
        production_loc,
        test_loc: 0, // Updated separately via AST analysis
    }
}

/// Calculate distribution metrics from raw FunctionMetrics (Spec 268).
///
/// This version works with raw metrics before filtering, providing complete
/// distribution analysis including test code separation.
pub fn aggregate_distribution_metrics_from_raw(
    functions: &[FunctionMetrics],
) -> DistributionMetrics {
    // Separate production and test functions
    let production_functions: Vec<_> = functions.iter().filter(|f| !f.is_test).collect();
    let test_functions: Vec<_> = functions.iter().filter(|f| f.is_test).collect();

    let complexities: Vec<u32> = production_functions.iter().map(|f| f.cyclomatic).collect();

    let total: u32 = complexities.iter().sum();
    let max = complexities.iter().max().copied().unwrap_or(0);
    let count = complexities.len();

    let avg = if count > 0 {
        total as f64 / count as f64
    } else {
        0.0
    };

    let median = calculate_median(&complexities);

    let exceeding = complexities
        .iter()
        .filter(|&&c| c > FUNCTION_COMPLEXITY_THRESHOLD)
        .count();

    let distribution = classify_distribution(max, total);

    // Calculate LOC separately for production and test code
    let production_loc: usize = production_functions.iter().map(|f| f.length).sum();
    let test_loc: usize = test_functions.iter().map(|f| f.length).sum();

    DistributionMetrics {
        function_count: count,
        max_complexity: max,
        avg_complexity: avg,
        median_complexity: median,
        exceeding_threshold: exceeding,
        distribution,
        production_loc,
        test_loc,
    }
}

/// Aggregate dependencies: unique set deduplication.
pub fn aggregate_dependency_metrics(
    members: &[&UnifiedDebtItem],
) -> (Vec<String>, Vec<String>, usize, usize) {
    let mut unique_callers: HashSet<String> = HashSet::new();
    let mut unique_callees: HashSet<String> = HashSet::new();

    for item in members {
        unique_callers.extend(item.upstream_callers.iter().cloned());
        unique_callees.extend(item.downstream_callees.iter().cloned());
    }

    let upstream_count = unique_callers.len();
    let downstream_count = unique_callees.len();

    (
        unique_callers.into_iter().collect(),
        unique_callees.into_iter().collect(),
        upstream_count,
        downstream_count,
    )
}

/// Aggregate contextual risk: combine base and contextual risk from members.
pub fn aggregate_contextual_risk(members: &[&UnifiedDebtItem]) -> Option<ContextualRisk> {
    let risks: Vec<_> = members
        .iter()
        .filter_map(|m| m.contextual_risk.as_ref())
        .collect();

    if risks.is_empty() {
        return None;
    }

    // Average base risk
    let avg_base_risk = risks.iter().map(|r| r.base_risk).sum::<f64>() / risks.len() as f64;

    // Average contextual risk
    let avg_contextual_risk =
        risks.iter().map(|r| r.contextual_risk).sum::<f64>() / risks.len() as f64;

    // Collect all unique contexts
    let all_contexts: Vec<_> = risks.iter().flat_map(|r| &r.contexts).cloned().collect();

    let explanation = format!(
        "Aggregated from {} functions (avg base: {:.1}, avg contextual: {:.1})",
        risks.len(),
        avg_base_risk,
        avg_contextual_risk
    );

    Some(ContextualRisk {
        base_risk: avg_base_risk,
        contextual_risk: avg_contextual_risk,
        contexts: all_contexts,
        explanation,
    })
}

/// Aggregate error swallowing metrics from FunctionMetrics.
pub fn aggregate_error_swallowing(functions: &[FunctionMetrics]) -> (u32, Vec<String>) {
    let total_count = functions
        .iter()
        .filter_map(|f| f.error_swallowing_count)
        .sum();

    let mut unique_patterns: HashSet<String> = HashSet::new();
    for func in functions {
        if let Some(ref patterns) = func.error_swallowing_patterns {
            unique_patterns.extend(patterns.iter().cloned());
        }
    }

    (total_count, unique_patterns.into_iter().collect())
}

/// Aggregate dependency metrics from raw FunctionMetrics.
///
/// This provides a complete architectural view of dependencies by aggregating
/// from ALL functions in the file, not just those that became debt items.
/// This ensures god objects show their true blast radius.
pub fn aggregate_dependency_metrics_from_raw(
    functions: &[FunctionMetrics],
) -> (Vec<String>, Vec<String>, usize, usize) {
    let mut unique_callers: HashSet<String> = HashSet::new();
    let mut unique_callees: HashSet<String> = HashSet::new();

    for func in functions {
        if let Some(ref callers) = func.upstream_callers {
            unique_callers.extend(callers.iter().cloned());
        }
        if let Some(ref callees) = func.downstream_callees {
            unique_callees.extend(callees.iter().cloned());
        }
    }

    let upstream_count = unique_callers.len();
    let downstream_count = unique_callees.len();

    (
        unique_callers.into_iter().collect(),
        unique_callees.into_iter().collect(),
        upstream_count,
        downstream_count,
    )
}

/// Aggregate entropy analysis from member UnifiedDebtItems (Spec 218).
///
/// Returns weighted average entropy metrics based on function length.
/// Uses original (undampened) complexity values for the aggregate summary.
pub fn aggregate_entropy_metrics(members: &[&UnifiedDebtItem]) -> Option<EntropyAnalysis> {
    let entropy_data: Vec<_> = members
        .iter()
        .filter_map(|m| m.entropy_analysis.as_ref().map(|e| (e, m.function_length)))
        .collect();

    if entropy_data.is_empty() {
        return None;
    }

    let total_length: usize = entropy_data.iter().map(|(_, len)| len).sum();
    if total_length == 0 {
        return None;
    }

    // Weighted average of entropy scores
    let weighted_entropy = entropy_data
        .iter()
        .map(|(e, len)| e.entropy_score * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Weighted average of pattern repetition
    let weighted_repetition = entropy_data
        .iter()
        .map(|(e, len)| e.pattern_repetition * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Weighted average of branch similarity
    let weighted_branch_similarity = entropy_data
        .iter()
        .map(|(e, len)| e.branch_similarity * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Weighted average of dampening factor
    let weighted_dampening = entropy_data
        .iter()
        .map(|(e, len)| e.dampening_factor * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Sum original complexity across all members
    let total_original: u32 = entropy_data
        .iter()
        .map(|(e, _)| e.original_complexity)
        .sum();
    let total_adjusted: u32 = entropy_data
        .iter()
        .map(|(e, _)| e.adjusted_complexity)
        .sum();

    // Aggregate reasoning from all members
    let mut reasoning: Vec<String> = entropy_data
        .iter()
        .flat_map(|(e, _)| e.reasoning.iter().cloned())
        .collect();
    reasoning.dedup();
    reasoning.truncate(5); // Limit to top 5 reasons

    Some(EntropyAnalysis {
        entropy_score: weighted_entropy,
        pattern_repetition: weighted_repetition,
        branch_similarity: weighted_branch_similarity,
        dampening_factor: weighted_dampening,
        dampening_was_applied: weighted_dampening < 1.0,
        original_complexity: total_original,
        adjusted_complexity: total_adjusted,
        reasoning,
    })
}

/// Aggregate all metrics (composition of above functions).
pub fn aggregate_god_object_metrics(members: &[&UnifiedDebtItem]) -> GodObjectAggregatedMetrics {
    let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(members);
    let weighted_cov = aggregate_coverage_metrics(members);
    let (callers, callees, up_count, down_count) = aggregate_dependency_metrics(members);
    let contextual_risk = aggregate_contextual_risk(members);
    let entropy = aggregate_entropy_metrics(members);
    let distribution = aggregate_distribution_metrics(members);

    // Note: Error swallowing is aggregated from raw FunctionMetrics, not UnifiedDebtItem
    // This function sets defaults; use aggregate_from_raw_metrics for full error swallowing data
    GodObjectAggregatedMetrics {
        total_cyclomatic: total_cyc,
        total_cognitive: total_cog,
        max_nesting_depth: max_nest,
        weighted_coverage: weighted_cov,
        unique_upstream_callers: callers,
        unique_downstream_callees: callees,
        upstream_dependencies: up_count,
        downstream_dependencies: down_count,
        aggregated_contextual_risk: contextual_risk,
        total_error_swallowing_count: 0,
        error_swallowing_patterns: Vec::new(),
        aggregated_entropy: entropy,
        distribution_metrics: Some(distribution),
    }
}

// =============================================================================
// Pure helper functions for entropy aggregation (Stillwater principles)
// =============================================================================

/// Extracts entropy data tuples from function metrics.
///
/// Pure function - filters functions that have entropy scores and returns
/// tuples of (entropy_score, length, cognitive_complexity).
fn extract_entropy_data(functions: &[FunctionMetrics]) -> Vec<(&EntropyScore, usize, u32)> {
    functions
        .iter()
        .filter_map(|f| f.entropy_score.as_ref().map(|e| (e, f.length, f.cognitive)))
        .collect()
}

/// Calculates weighted average of a metric from entropy data.
///
/// Pure function that computes a length-weighted average using the provided
/// extractor function.
fn weighted_average<F>(data: &[(&EntropyScore, usize, u32)], total_length: usize, f: F) -> f64
where
    F: Fn(&EntropyScore) -> f64,
{
    data.iter()
        .map(|(e, len, _)| f(e) * (*len as f64))
        .sum::<f64>()
        / total_length as f64
}

/// Sums a u32 field from entropy data tuples.
///
/// Pure function for aggregating cognitive complexity values.
fn sum_cognitive(data: &[(&EntropyScore, usize, u32)]) -> u32 {
    data.iter().map(|(_, _, cog)| cog).sum()
}

/// Calculates total length from entropy data tuples.
fn total_length(data: &[(&EntropyScore, usize, u32)]) -> usize {
    data.iter().map(|(_, len, _)| *len).sum()
}

/// Aggregate entropy from raw FunctionMetrics (Spec 218).
///
/// Returns weighted average entropy based on function length from ALL functions,
/// not just those that became debt items.
///
/// Composed from pure helper functions following Stillwater principles.
pub fn aggregate_entropy_from_raw(functions: &[FunctionMetrics]) -> Option<EntropyAnalysis> {
    let data = extract_entropy_data(functions);
    let len = total_length(&data);

    if data.is_empty() || len == 0 {
        return None;
    }

    let entropy = weighted_average(&data, len, |e| e.token_entropy);
    let repetition = weighted_average(&data, len, |e| e.pattern_repetition);
    let branch_similarity = weighted_average(&data, len, |e| e.branch_similarity);
    let total_cognitive = sum_cognitive(&data);

    let calculator = UniversalEntropyCalculator::new(EntropyConfig::default());
    let dampening_factor = calculator.calculate_dampening_factor(entropy, repetition);
    let adjusted_complexity = (total_cognitive as f64 * dampening_factor) as u32;

    Some(EntropyAnalysis {
        entropy_score: entropy,
        pattern_repetition: repetition,
        branch_similarity,
        dampening_factor,
        dampening_was_applied: dampening_factor < 1.0,
        original_complexity: total_cognitive,
        adjusted_complexity,
        reasoning: vec![format!("Aggregated from {} functions", functions.len())],
    })
}

/// Aggregate coverage from raw FunctionMetrics using LCOV data.
///
/// This function looks up coverage for ALL functions in the file from LCOV data,
/// not just those that became UnifiedDebtItems. This ensures god objects show
/// accurate coverage metrics even when member functions are filtered out by
/// complexity thresholds.
///
/// Returns a weighted average coverage based on function length.
pub fn aggregate_coverage_from_raw_metrics(
    functions: &[FunctionMetrics],
    coverage: &LcovData,
) -> Option<TransitiveCoverage> {
    if functions.is_empty() {
        return None;
    }

    // Collect coverage data for each function
    let mut coverage_data: Vec<(f64, usize, Vec<usize>)> = Vec::with_capacity(functions.len());

    for func in functions {
        let end_line = func.line + func.length.saturating_sub(1);
        // Use get_function_coverage_with_bounds for accurate AST-based matching
        let direct_coverage = coverage
            .get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line)
            .unwrap_or(0.0);

        let uncovered = coverage
            .get_function_uncovered_lines(&func.file, &func.name, func.line)
            .unwrap_or_default();

        coverage_data.push((direct_coverage, func.length, uncovered));
    }

    let total_length: usize = coverage_data.iter().map(|(_, len, _)| len).sum();
    if total_length == 0 {
        return None;
    }

    // Calculate weighted average coverage
    let weighted_direct = coverage_data
        .iter()
        .map(|(cov, len, _)| cov * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Collect all uncovered lines (deduplicated)
    let all_uncovered: Vec<usize> = coverage_data
        .iter()
        .flat_map(|(_, _, uncovered)| uncovered.iter().copied())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Some(TransitiveCoverage {
        direct: weighted_direct,
        transitive: weighted_direct, // For god objects, transitive == direct
        propagated_from: vec![],
        uncovered_lines: all_uncovered,
    })
}

/// Aggregate metrics directly from raw FunctionMetrics (for ALL functions including tests).
///
/// This function aggregates complexity and dependencies from raw function metrics
/// before any filtering, ensuring god objects show:
/// - TRUE complexity of all their functions
/// - TRUE architectural dependencies (complete blast radius)
/// - Distribution metrics with production/test LOC separation (Spec 268)
///
/// Note: Coverage is NOT aggregated here. Use `aggregate_coverage_from_raw_metrics`
/// separately with LCOV data for coverage metrics.
pub fn aggregate_from_raw_metrics(functions: &[FunctionMetrics]) -> GodObjectAggregatedMetrics {
    let total_cyclomatic = functions.iter().map(|f| f.cyclomatic).sum();
    let total_cognitive = functions.iter().map(|f| f.cognitive).sum();
    let max_nesting = functions.iter().map(|f| f.nesting).max().unwrap_or(0);

    // Aggregate error swallowing from raw metrics
    let (total_error_swallowing, error_patterns) = aggregate_error_swallowing(functions);

    // Aggregate entropy from raw metrics (available for all functions)
    let aggregated_entropy = aggregate_entropy_from_raw(functions);

    // Aggregate dependencies from raw metrics (complete architectural view)
    let (
        unique_upstream_callers,
        unique_downstream_callees,
        upstream_dependencies,
        downstream_dependencies,
    ) = aggregate_dependency_metrics_from_raw(functions);

    // Aggregate distribution metrics with production/test LOC separation (Spec 268)
    let distribution_metrics = aggregate_distribution_metrics_from_raw(functions);

    GodObjectAggregatedMetrics {
        total_cyclomatic,
        total_cognitive,
        max_nesting_depth: max_nesting,
        weighted_coverage: None,
        unique_upstream_callers,
        unique_downstream_callees,
        upstream_dependencies,
        downstream_dependencies,
        aggregated_contextual_risk: None,
        total_error_swallowing_count: total_error_swallowing,
        error_swallowing_patterns: error_patterns,
        aggregated_entropy,
        distribution_metrics: Some(distribution_metrics),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::complexity::EntropyAnalysis;
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_item(
        file: &str,
        cyc: u32,
        cog: u32,
        nest: u32,
        length: usize,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: "test_fn".to_string(),
                line: 1,
            },
            debt_type: DebtType::Complexity {
                cyclomatic: cyc,
                cognitive: cog,
            },
            unified_score: UnifiedScore {
                final_score: 50.0,
                complexity_factor: 5.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: Vec::new(),
                related_items: Vec::new(),
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: Vec::new(),
            downstream_callees: Vec::new(),
            nesting_depth: nest,
            function_length: length,
            cyclomatic_complexity: cyc,
            cognitive_complexity: cog,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }
    }

    #[test]
    fn test_complexity_aggregation_sums_and_maxes() {
        let members = vec![
            create_test_item("file.rs", 5, 10, 2, 50),
            create_test_item("file.rs", 10, 15, 5, 100),
            create_test_item("file.rs", 15, 20, 3, 75),
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(&member_refs);

        assert_eq!(total_cyc, 30); // 5 + 10 + 15
        assert_eq!(total_cog, 45); // 10 + 15 + 20
        assert_eq!(max_nest, 5); // max(2, 5, 3)
    }

    #[test]
    fn test_coverage_weighted_average() {
        let mut members = vec![
            create_test_item("file.rs", 0, 0, 0, 10),
            create_test_item("file.rs", 0, 0, 0, 50),
            create_test_item("file.rs", 0, 0, 0, 40),
        ];

        // Add coverage data
        members[0].transitive_coverage = Some(TransitiveCoverage {
            direct: 0.8,
            transitive: 0.9,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        members[1].transitive_coverage = Some(TransitiveCoverage {
            direct: 0.2,
            transitive: 0.3,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        members[2].transitive_coverage = Some(TransitiveCoverage {
            direct: 0.5,
            transitive: 0.6,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        let member_refs: Vec<_> = members.iter().collect();
        let cov = aggregate_coverage_metrics(&member_refs).unwrap();

        // (10*0.8 + 50*0.2 + 40*0.5) / 100 = 38/100 = 0.38
        assert!((cov.direct - 0.38).abs() < 0.01);
        assert!((cov.transitive - 0.48).abs() < 0.01); // (10*0.9 + 50*0.3 + 40*0.6) / 100
    }

    #[test]
    fn test_dependencies_deduplicate() {
        let mut members = vec![
            create_test_item("file.rs", 0, 0, 0, 10),
            create_test_item("file.rs", 0, 0, 0, 20),
        ];

        members[0].upstream_callers = vec!["main".to_string(), "init".to_string()];
        members[0].downstream_callees = vec!["log".to_string()];
        members[1].upstream_callers = vec!["main".to_string(), "process".to_string()]; // "main" is duplicate
        members[1].downstream_callees = vec!["log".to_string(), "db".to_string()]; // "log" is duplicate

        let member_refs: Vec<_> = members.iter().collect();
        let (callers, _callees, up_count, down_count) = aggregate_dependency_metrics(&member_refs);

        assert_eq!(up_count, 3); // main, init, process (deduplicated)
        assert_eq!(down_count, 2); // log, db (deduplicated)
        assert!(callers.contains(&"main".to_string()));
        assert!(callers.contains(&"init".to_string()));
        assert!(callers.contains(&"process".to_string()));
    }

    #[test]
    fn test_extract_member_functions() {
        let items = vec![
            create_test_item("file1.rs", 5, 10, 2, 50),
            create_test_item("file2.rs", 10, 15, 3, 100),
            create_test_item("file1.rs", 15, 20, 4, 75),
        ];

        let members = extract_member_functions(items.iter(), Path::new("file1.rs"));

        assert_eq!(members.len(), 2);
        assert!(members
            .iter()
            .all(|m| m.location.file == Path::new("file1.rs")));
    }

    #[test]
    fn test_aggregate_god_object_metrics_composition() {
        let members = vec![
            create_test_item("file.rs", 5, 10, 2, 50),
            create_test_item("file.rs", 10, 15, 5, 100),
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let metrics = aggregate_god_object_metrics(&member_refs);

        assert_eq!(metrics.total_cyclomatic, 15);
        assert_eq!(metrics.total_cognitive, 25);
        assert_eq!(metrics.max_nesting_depth, 5);
        assert!(metrics.weighted_coverage.is_none()); // No coverage data
    }

    #[test]
    fn test_aggregate_error_swallowing() {
        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 5,
                cognitive: 5,
                nesting: 1,
                length: 20,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: Some(2),
                error_swallowing_patterns: Some(vec![
                    "if let Ok(...) without else branch".to_string()
                ]),
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 25,
                cyclomatic: 3,
                cognitive: 3,
                nesting: 1,
                length: 15,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: Some(3),
                error_swallowing_patterns: Some(vec![
                    "if let Ok(...) without else branch".to_string(),
                    "let _ = discarding Result".to_string(),
                ]),
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func3".to_string(),
                file: PathBuf::from("test.rs"),
                line: 50,
                cyclomatic: 2,
                cognitive: 2,
                nesting: 1,
                length: 10,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None, // No error swallowing
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let (total, patterns) = aggregate_error_swallowing(&functions);

        assert_eq!(total, 5); // 2 + 3 = 5
        assert_eq!(patterns.len(), 2); // 2 unique patterns
        assert!(patterns.contains(&"if let Ok(...) without else branch".to_string()));
        assert!(patterns.contains(&"let _ = discarding Result".to_string()));
    }

    #[test]
    fn test_aggregate_entropy_metrics_weighted_average() {
        // Create items with different entropy details and lengths
        let mut item1 = create_test_item("file.rs", 10, 20, 2, 100); // length 100
        item1.entropy_analysis = Some(EntropyAnalysis {
            entropy_score: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.3,
            original_complexity: 20,
            adjusted_complexity: 16,
            dampening_factor: 0.8,
            dampening_was_applied: true,
            reasoning: vec![],
        });

        let mut item2 = create_test_item("file.rs", 15, 30, 3, 200); // length 200
        item2.entropy_analysis = Some(EntropyAnalysis {
            entropy_score: 0.5,
            pattern_repetition: 0.3,
            branch_similarity: 0.2,
            original_complexity: 30,
            adjusted_complexity: 27,
            dampening_factor: 0.9,
            dampening_was_applied: true,
            reasoning: vec![],
        });

        let members = vec![&item1, &item2];
        let result = aggregate_entropy_metrics(&members).expect("should have entropy");

        // Weighted average: (100*0.4 + 200*0.5) / 300 = 140/300 ≈ 0.467
        assert!((result.entropy_score - 0.467).abs() < 0.01);

        // Weighted repetition: (100*0.6 + 200*0.3) / 300 = 120/300 = 0.4
        assert!((result.pattern_repetition - 0.4).abs() < 0.01);

        // Weighted dampening: (100*0.8 + 200*0.9) / 300 = 260/300 ≈ 0.867
        assert!((result.dampening_factor - 0.867).abs() < 0.01);

        // Sums: 20 + 30 = 50, 16 + 27 = 43
        assert_eq!(result.original_complexity, 50);
        assert_eq!(result.adjusted_complexity, 43);
    }

    #[test]
    fn test_aggregate_entropy_metrics_empty() {
        let item1 = create_test_item("file.rs", 10, 20, 2, 100);
        let item2 = create_test_item("file.rs", 15, 30, 3, 200);
        // Neither has entropy_details

        let members = vec![&item1, &item2];
        let result = aggregate_entropy_metrics(&members);

        assert!(result.is_none());
    }

    #[test]
    fn test_aggregate_entropy_metrics_partial() {
        // Only one item has entropy
        let mut item1 = create_test_item("file.rs", 10, 20, 2, 100);
        item1.entropy_analysis = Some(EntropyAnalysis {
            entropy_score: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.3,
            original_complexity: 20,
            adjusted_complexity: 16,
            dampening_factor: 0.8,
            dampening_was_applied: true,
            reasoning: vec![],
        });

        let item2 = create_test_item("file.rs", 15, 30, 3, 200); // No entropy

        let members = vec![&item1, &item2];
        let result = aggregate_entropy_metrics(&members).expect("should have entropy from item1");

        // Only item1 contributes, so values are from item1 only
        assert!((result.entropy_score - 0.4).abs() < 0.001);
        assert_eq!(result.original_complexity, 20);
    }

    #[test]
    fn test_aggregate_god_object_metrics_includes_entropy() {
        let mut item1 = create_test_item("file.rs", 10, 20, 2, 100);
        item1.entropy_analysis = Some(EntropyAnalysis {
            entropy_score: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.3,
            original_complexity: 20,
            adjusted_complexity: 16,
            dampening_factor: 0.8,
            dampening_was_applied: true,
            reasoning: vec![],
        });

        let members = vec![&item1];
        let metrics = aggregate_god_object_metrics(&members);

        assert!(metrics.aggregated_entropy.is_some());
        let entropy = metrics.aggregated_entropy.unwrap();
        assert!((entropy.entropy_score - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_aggregate_from_raw_metrics_includes_error_swallowing() {
        let functions = vec![FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: Some(4),
            error_swallowing_patterns: Some(vec!["match with ignored Err variant".to_string()]),
            entropy_analysis: None,
        }];

        let metrics = aggregate_from_raw_metrics(&functions);

        assert_eq!(metrics.total_cyclomatic, 10);
        assert_eq!(metrics.total_cognitive, 15);
        assert_eq!(metrics.total_error_swallowing_count, 4);
        assert_eq!(metrics.error_swallowing_patterns.len(), 1);
        assert!(metrics
            .error_swallowing_patterns
            .contains(&"match with ignored Err variant".to_string()));
    }

    #[test]
    fn test_aggregate_entropy_from_raw() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 10,
                cognitive: 20,
                nesting: 2,
                length: 100,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(RawEntropyScore {
                    token_entropy: 0.4,
                    pattern_repetition: 0.6,
                    branch_similarity: 0.0,
                    effective_complexity: 5.0,
                    unique_variables: 0,
                    max_nesting: 0,
                    dampening_applied: 0.0,
                }),
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 50,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(RawEntropyScore {
                    token_entropy: 0.5,
                    pattern_repetition: 0.3,
                    branch_similarity: 0.0,
                    effective_complexity: 3.0,
                    unique_variables: 0,
                    max_nesting: 0,
                    dampening_applied: 0.0,
                }),
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let result = aggregate_entropy_from_raw(&functions).expect("should have entropy");

        // Weighted average: (100*0.4 + 50*0.5) / 150 = 65/150 ≈ 0.433
        assert!((result.entropy_score - 0.433).abs() < 0.01);

        // Weighted repetition: (100*0.6 + 50*0.3) / 150 = 75/150 = 0.5
        assert!((result.pattern_repetition - 0.5).abs() < 0.01);

        // Original complexity: 20 + 10 = 30
        assert_eq!(result.original_complexity, 30);
    }

    #[test]
    fn test_aggregate_from_raw_metrics_includes_entropy() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let functions = vec![FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 10,
            cognitive: 20,
            nesting: 2,
            length: 100,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: Some(RawEntropyScore {
                token_entropy: 0.4,
                pattern_repetition: 0.6,
                branch_similarity: 0.0,
                effective_complexity: 5.0,
                unique_variables: 0,
                max_nesting: 0,
                dampening_applied: 0.0,
            }),
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }];

        let metrics = aggregate_from_raw_metrics(&functions);

        assert!(metrics.aggregated_entropy.is_some());
        let entropy = metrics.aggregated_entropy.unwrap();
        assert!((entropy.entropy_score - 0.4).abs() < 0.001);
        assert_eq!(entropy.original_complexity, 20);
    }

    // =========================================================================
    // Tests for pure helper functions (Stillwater refactoring)
    // =========================================================================

    #[test]
    fn test_extract_entropy_data_filters_correctly() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let functions = vec![
            FunctionMetrics {
                name: "with_entropy".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(RawEntropyScore {
                    token_entropy: 0.4,
                    pattern_repetition: 0.6,
                    branch_similarity: 0.0,
                    effective_complexity: 5.0,
                    unique_variables: 0,
                    max_nesting: 0,
                    dampening_applied: 0.0,
                }),
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "without_entropy".to_string(),
                file: PathBuf::from("test.rs"),
                line: 50,
                cyclomatic: 3,
                cognitive: 5,
                nesting: 1,
                length: 25,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None, // No entropy
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let data = extract_entropy_data(&functions);
        assert_eq!(data.len(), 1); // Only one function has entropy
        assert_eq!(data[0].1, 50); // Length of function with entropy
        assert_eq!(data[0].2, 10); // Cognitive of function with entropy
    }

    #[test]
    fn test_weighted_average_calculation() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let e1 = RawEntropyScore {
            token_entropy: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.0,
            effective_complexity: 5.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };
        let e2 = RawEntropyScore {
            token_entropy: 0.6,
            pattern_repetition: 0.2,
            branch_similarity: 0.0,
            effective_complexity: 3.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };

        // length 100, cognitive 20 and length 50, cognitive 10
        let data: Vec<(&RawEntropyScore, usize, u32)> = vec![(&e1, 100, 20), (&e2, 50, 10)];

        // (100*0.4 + 50*0.6) / 150 = 70/150 ≈ 0.467
        let avg = weighted_average(&data, 150, |e| e.token_entropy);
        assert!((avg - 0.467).abs() < 0.01);

        // (100*0.6 + 50*0.2) / 150 = 70/150 ≈ 0.467
        let rep = weighted_average(&data, 150, |e| e.pattern_repetition);
        assert!((rep - 0.467).abs() < 0.01);
    }

    #[test]
    fn test_sum_cognitive() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let e = RawEntropyScore {
            token_entropy: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.0,
            effective_complexity: 5.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };

        let data: Vec<(&RawEntropyScore, usize, u32)> =
            vec![(&e, 100, 20), (&e, 50, 15), (&e, 25, 5)];

        let total = sum_cognitive(&data);
        assert_eq!(total, 40); // 20 + 15 + 5
    }

    #[test]
    fn test_total_length() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let e = RawEntropyScore {
            token_entropy: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.0,
            effective_complexity: 5.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };

        let data: Vec<(&RawEntropyScore, usize, u32)> = vec![(&e, 100, 20), (&e, 50, 15)];

        let total = total_length(&data);
        assert_eq!(total, 150); // 100 + 50
    }

    #[test]
    fn test_calculate_dampening_factor_direct() {
        use crate::complexity::entropy_core::{EntropyConfig, UniversalEntropyCalculator};

        let calculator = UniversalEntropyCalculator::new(EntropyConfig::default());

        // Test with various entropy/repetition combinations
        let dampening = calculator.calculate_dampening_factor(0.4, 0.6);
        assert!((0.5..=1.0).contains(&dampening));

        // Low entropy, high repetition should result in lower effective complexity
        let low_dampening = calculator.calculate_dampening_factor(0.2, 0.8);
        assert!(low_dampening >= 0.5);

        // High entropy, low repetition should result in higher effective complexity
        let high_dampening = calculator.calculate_dampening_factor(0.8, 0.2);
        assert!(high_dampening <= 1.0);
    }

    #[test]
    fn test_aggregate_dependency_metrics_from_raw() {
        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: Some(vec!["caller1".to_string(), "caller2".to_string()]),
                downstream_callees: Some(vec!["callee1".to_string()]),
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                cyclomatic: 3,
                cognitive: 5,
                nesting: 1,
                length: 30,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: Some(vec!["caller2".to_string(), "caller3".to_string()]), // caller2 is duplicate
                downstream_callees: Some(vec!["callee2".to_string(), "callee3".to_string()]),
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let (callers, callees, upstream_count, downstream_count) =
            aggregate_dependency_metrics_from_raw(&functions);

        // Should deduplicate: caller1, caller2, caller3 = 3 unique
        assert_eq!(upstream_count, 3);
        // Should deduplicate: callee1, callee2, callee3 = 3 unique
        assert_eq!(downstream_count, 3);

        assert!(callers.contains(&"caller1".to_string()));
        assert!(callers.contains(&"caller2".to_string()));
        assert!(callers.contains(&"caller3".to_string()));

        assert!(callees.contains(&"callee1".to_string()));
        assert!(callees.contains(&"callee2".to_string()));
        assert!(callees.contains(&"callee3".to_string()));
    }

    #[test]
    fn test_aggregate_from_raw_metrics_includes_dependencies() {
        let functions = vec![FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 1,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: Some(vec!["caller1".to_string(), "caller2".to_string()]),
            downstream_callees: Some(vec!["callee1".to_string()]),
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }];

        let metrics = aggregate_from_raw_metrics(&functions);

        // Dependencies should be populated from raw metrics
        assert_eq!(metrics.upstream_dependencies, 2);
        assert_eq!(metrics.downstream_dependencies, 1);
        assert_eq!(metrics.unique_upstream_callers.len(), 2);
        assert_eq!(metrics.unique_downstream_callees.len(), 1);
    }

    // =========================================================================
    // Tests for Distribution Metrics (Spec 268)
    // =========================================================================

    #[test]
    fn test_calculate_median_odd_count() {
        assert_eq!(calculate_median(&[1, 2, 3, 4, 5]), 3);
        assert_eq!(calculate_median(&[5, 1, 3]), 3); // Tests sorting
        assert_eq!(calculate_median(&[100]), 100);
    }

    #[test]
    fn test_calculate_median_even_count() {
        // For even count, returns average of two middle values
        assert_eq!(calculate_median(&[1, 2, 3, 4]), 2); // (2+3)/2 = 2 (integer division)
        assert_eq!(calculate_median(&[1, 3, 5, 7]), 4); // (3+5)/2 = 4
    }

    #[test]
    fn test_calculate_median_empty() {
        assert_eq!(calculate_median(&[]), 0);
    }

    #[test]
    fn test_classify_distribution_concentrated() {
        // Single function has 60% of complexity
        assert_eq!(
            classify_distribution(60, 100),
            ComplexityDistribution::Concentrated
        );
        // Edge case: exactly 51%
        assert_eq!(
            classify_distribution(51, 100),
            ComplexityDistribution::Concentrated
        );
    }

    #[test]
    fn test_classify_distribution_mixed() {
        // Max function has 30% of complexity
        assert_eq!(
            classify_distribution(30, 100),
            ComplexityDistribution::Mixed
        );
        // Edge case: exactly 50%
        assert_eq!(
            classify_distribution(50, 100),
            ComplexityDistribution::Mixed
        );
        // Edge case: just above 20%
        assert_eq!(
            classify_distribution(21, 100),
            ComplexityDistribution::Mixed
        );
    }

    #[test]
    fn test_classify_distribution_distributed() {
        // Max function has only 10% of complexity
        assert_eq!(
            classify_distribution(10, 100),
            ComplexityDistribution::Distributed
        );
        // Edge case: exactly 20%
        assert_eq!(
            classify_distribution(20, 100),
            ComplexityDistribution::Distributed
        );
        // Zero total should be Distributed
        assert_eq!(
            classify_distribution(0, 0),
            ComplexityDistribution::Distributed
        );
    }

    #[test]
    fn test_aggregate_distribution_metrics() {
        // Create items with varied complexity to test distribution calculation
        let members = vec![
            create_test_item("file.rs", 5, 10, 2, 50), // Low complexity
            create_test_item("file.rs", 8, 15, 3, 100), // Medium complexity
            create_test_item("file.rs", 12, 20, 4, 75), // Medium complexity
            create_test_item("file.rs", 6, 12, 2, 60), // Low complexity
            create_test_item("file.rs", 7, 14, 3, 80), // Low complexity
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let dist = aggregate_distribution_metrics(&member_refs);

        // Total: 5+8+12+6+7 = 38, max = 12
        // Ratio: 12/38 = 0.316 -> Mixed
        assert_eq!(dist.function_count, 5);
        assert_eq!(dist.max_complexity, 12);
        assert!((dist.avg_complexity - 7.6).abs() < 0.01); // 38/5 = 7.6
        assert_eq!(dist.median_complexity, 7); // sorted: 5,6,7,8,12 -> median = 7
        assert_eq!(dist.exceeding_threshold, 0); // none exceed 15
        assert_eq!(dist.distribution, ComplexityDistribution::Mixed);
        assert_eq!(dist.production_loc, 365); // 50+100+75+60+80
    }

    #[test]
    fn test_aggregate_distribution_metrics_distributed_file() {
        // Create items simulating a well-structured file with many small functions
        let mut members = Vec::new();
        for i in 0..30 {
            // 30 functions with complexity 3-7 (average 5)
            members.push(create_test_item("file.rs", 3 + (i % 5), 10, 2, 20));
        }
        let member_refs: Vec<_> = members.iter().collect();

        let dist = aggregate_distribution_metrics(&member_refs);

        // Total: 30 functions with avg complexity 5 = total ~150
        // Max complexity = 7 (3 + 4)
        // Ratio: 7/~150 = ~0.047 -> Distributed
        assert_eq!(dist.function_count, 30);
        assert_eq!(dist.max_complexity, 7); // max of 3,4,5,6,7
        assert_eq!(dist.exceeding_threshold, 0);
        assert_eq!(dist.distribution, ComplexityDistribution::Distributed);
    }

    #[test]
    fn test_aggregate_distribution_metrics_concentrated_file() {
        // Create items simulating a god function dominating the file
        let members = vec![
            create_test_item("file.rs", 60, 100, 5, 500), // God function
            create_test_item("file.rs", 5, 10, 2, 30),    // Small helper
            create_test_item("file.rs", 5, 10, 2, 30),    // Small helper
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let dist = aggregate_distribution_metrics(&member_refs);

        // Total: 60+5+5 = 70, max = 60
        // Ratio: 60/70 = 0.857 -> Concentrated
        assert_eq!(dist.function_count, 3);
        assert_eq!(dist.max_complexity, 60);
        assert_eq!(dist.exceeding_threshold, 1); // 60 exceeds 15
        assert_eq!(dist.distribution, ComplexityDistribution::Concentrated);
    }

    #[test]
    fn test_aggregate_distribution_metrics_from_raw() {
        let functions = vec![
            FunctionMetrics {
                name: "prod_func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 10,
                cognitive: 15,
                nesting: 2,
                length: 100,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "prod_func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 50,
                cyclomatic: 8,
                cognitive: 12,
                nesting: 1,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "test_something".to_string(),
                file: PathBuf::from("test.rs"),
                line: 100,
                cyclomatic: 5,
                cognitive: 8,
                nesting: 1,
                length: 200, // Test function with more LOC
                is_test: true,
                visibility: None,
                is_trait_method: false,
                in_test_module: true,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let dist = aggregate_distribution_metrics_from_raw(&functions);

        // Should only count production functions (2 functions)
        assert_eq!(dist.function_count, 2);
        // Total complexity from production: 10+8 = 18, max = 10
        assert_eq!(dist.max_complexity, 10);
        assert!((dist.avg_complexity - 9.0).abs() < 0.01); // 18/2 = 9.0
                                                           // Production LOC: 100 + 50 = 150
        assert_eq!(dist.production_loc, 150);
        // Test LOC: 200
        assert_eq!(dist.test_loc, 200);
        // Ratio: 10/18 = 0.556 -> Concentrated
        assert_eq!(dist.distribution, ComplexityDistribution::Concentrated);
    }

    #[test]
    fn test_distributed_file_gets_correct_classification() {
        // Real-world scenario: 30 small functions (like overflow.rs example)
        let mut functions = Vec::new();
        for i in 0..30 {
            functions.push(FunctionMetrics {
                name: format!("func_{}", i),
                file: PathBuf::from("overflow.rs"),
                line: i * 20 + 1,
                cyclomatic: 5 + (i as u32 % 4), // 5, 6, 7, 8, 5, 6, ...
                cognitive: 4 + (i as u32 % 3),
                nesting: 2,
                length: 20, // ~20 lines each
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            });
        }

        let dist = aggregate_distribution_metrics_from_raw(&functions);

        // Should be classified as Distributed (well-structured file)
        assert_eq!(dist.function_count, 30);
        assert_eq!(dist.max_complexity, 8); // max of 5,6,7,8
        assert_eq!(dist.distribution, ComplexityDistribution::Distributed);
        assert_eq!(dist.production_loc, 600); // 30 * 20
    }

    #[test]
    fn test_complexity_distribution_display_names() {
        assert_eq!(
            ComplexityDistribution::Concentrated.display_name(),
            "Concentrated"
        );
        assert_eq!(ComplexityDistribution::Mixed.display_name(), "Mixed");
        assert_eq!(
            ComplexityDistribution::Distributed.display_name(),
            "Distributed"
        );
    }

    #[test]
    fn test_complexity_distribution_explanations() {
        assert!(ComplexityDistribution::Concentrated
            .classification_explanation()
            .contains("god function"));
        assert!(ComplexityDistribution::Mixed
            .classification_explanation()
            .contains("review"));
        assert!(ComplexityDistribution::Distributed
            .classification_explanation()
            .contains("Well-Structured"));
    }

    #[test]
    fn test_aggregate_god_object_metrics_includes_distribution() {
        let members = vec![
            create_test_item("file.rs", 10, 20, 2, 100),
            create_test_item("file.rs", 8, 15, 3, 80),
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let metrics = aggregate_god_object_metrics(&member_refs);

        assert!(metrics.distribution_metrics.is_some());
        let dist = metrics.distribution_metrics.unwrap();
        assert_eq!(dist.function_count, 2);
        assert_eq!(dist.max_complexity, 10);
    }

    #[test]
    fn test_aggregate_from_raw_metrics_includes_distribution() {
        let functions = vec![FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 2,
            length: 100,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }];

        let metrics = aggregate_from_raw_metrics(&functions);

        assert!(metrics.distribution_metrics.is_some());
        let dist = metrics.distribution_metrics.unwrap();
        assert_eq!(dist.function_count, 1);
        assert_eq!(dist.max_complexity, 10);
        assert_eq!(dist.production_loc, 100);
    }
}
