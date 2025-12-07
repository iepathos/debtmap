//! Location-based grouping for debt items.
//!
//! Groups multiple debt items at the same location (file, function, line)
//! into a single entry for display, while preserving all individual items.

use crate::priority::classification::Severity;
use crate::priority::{TransitiveCoverage, UnifiedDebtItem};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use super::sort::SortCriteria;

/// A group of debt items at the same location
#[derive(Debug, Clone)]
pub struct LocationGroup<'a> {
    /// Representative location (from first item)
    pub location: &'a crate::priority::Location,
    /// All debt items at this location
    pub items: Vec<&'a UnifiedDebtItem>,
    /// Combined score (sum of all item scores)
    pub combined_score: f64,
    /// Highest severity among items (lowercase string for TUI display)
    pub max_severity: String,
}

/// Aggregated metrics across all items in a group
#[derive(Debug)]
pub struct AggregatedMetrics<'a> {
    pub cognitive_complexity: u32,
    pub nesting_depth: u32,
    pub function_length: usize,
    pub coverage: Option<&'a TransitiveCoverage>,
}

/// Group debt items by (file, function, line) location and sort by criteria
pub fn group_by_location<'a>(
    items: impl Iterator<Item = &'a UnifiedDebtItem>,
    sort_by: SortCriteria,
) -> Vec<LocationGroup<'a>> {
    let mut groups: HashMap<(&PathBuf, &str, usize), Vec<&UnifiedDebtItem>> = HashMap::new();

    for item in items {
        let key = (
            &item.location.file,
            item.location.function.as_str(),
            item.location.line,
        );
        groups.entry(key).or_default().push(item);
    }

    let mut result: Vec<LocationGroup> = groups
        .into_values()
        .map(|items| {
            let combined_score = items
                .iter()
                .map(|i| i.unified_score.final_score.value())
                .sum::<f64>();

            let max_severity = items
                .iter()
                .map(|i| {
                    Severity::from_score_100(i.unified_score.final_score.value())
                        .as_str()
                        .to_lowercase()
                })
                .max_by(|a, b| severity_rank(a).cmp(&severity_rank(b)))
                .unwrap_or_else(|| "low".to_string());

            LocationGroup {
                location: &items[0].location,
                items,
                combined_score,
                max_severity,
            }
        })
        .collect();

    // Sort groups based on criteria
    sort_groups(&mut result, sort_by);

    result
}

/// Sort groups based on the given criteria
fn sort_groups(groups: &mut [LocationGroup], criteria: SortCriteria) {
    groups.sort_by(|a, b| {
        let primary = match criteria {
            SortCriteria::Score => {
                // Sort by combined score descending
                b.combined_score
                    .partial_cmp(&a.combined_score)
                    .unwrap_or(Ordering::Equal)
            }
            SortCriteria::Coverage => {
                // Sort by coverage ascending (lowest coverage first)
                let cov_a = aggregate_metrics(a).coverage.map(|c| c.direct);
                let cov_b = aggregate_metrics(b).coverage.map(|c| c.direct);

                match (cov_a, cov_b) {
                    (None, None) => Ordering::Equal,
                    (None, Some(_)) => Ordering::Less, // No coverage is worst
                    (Some(_), None) => Ordering::Greater,
                    (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
                }
            }
            SortCriteria::Complexity => {
                // Sort by max complexity descending
                let comp_a = aggregate_metrics(a).cognitive_complexity;
                let comp_b = aggregate_metrics(b).cognitive_complexity;
                comp_b.cmp(&comp_a)
            }
            SortCriteria::FilePath => {
                // Sort by file path alphabetically
                a.location.file.cmp(&b.location.file)
            }
            SortCriteria::FunctionName => {
                // Sort by function name alphabetically
                a.location.function.cmp(&b.location.function)
            }
        };

        // Tiebreaker: compare by file path, then line number for stable ordering
        match primary {
            Ordering::Equal => match a.location.file.cmp(&b.location.file) {
                Ordering::Equal => a.location.line.cmp(&b.location.line),
                other => other,
            },
            other => other,
        }
    });
}

/// Extract all unique metrics across items in group
pub fn aggregate_metrics<'a>(group: &LocationGroup<'a>) -> AggregatedMetrics<'a> {
    let max_cog = group
        .items
        .iter()
        .map(|i| i.cognitive_complexity)
        .max()
        .unwrap_or(0);

    let max_nest = group
        .items
        .iter()
        .map(|i| i.nesting_depth)
        .max()
        .unwrap_or(0);

    let max_len = group
        .items
        .iter()
        .map(|i| i.function_length)
        .max()
        .unwrap_or(0);

    // Coverage same across all items at location
    let coverage = group.items[0].transitive_coverage.as_ref();

    AggregatedMetrics {
        cognitive_complexity: max_cog,
        nesting_depth: max_nest,
        function_length: max_len,
        coverage,
    }
}

/// Get numeric rank for severity (higher = more severe)
fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;
    use crate::priority::{
        ActionableRecommendation, DebtType, ImpactMetrics, Location, UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_item(file: &str, function: &str, line: usize, score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: function.to_string(),
                line,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: 10,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: Score0To100::new(score),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: crate::priority::semantic_classifier::FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
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
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 5,
            cognitive_complexity: 10,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
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
        }
    }

    #[test]
    fn test_group_by_location_single_item() {
        let items = vec![create_test_item("file.rs", "func", 10, 50.0)];
        let groups = group_by_location(items.iter(), SortCriteria::Score);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 1);
        assert_eq!(groups[0].combined_score, 50.0);
    }

    #[test]
    fn test_group_by_location_multiple_types() {
        let items = vec![
            create_test_item("file.rs", "func", 10, 75.0),
            create_test_item("file.rs", "func", 10, 60.0),
            create_test_item("file.rs", "func", 10, 45.0),
        ];
        let groups = group_by_location(items.iter(), SortCriteria::Score);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 3);
        assert_eq!(groups[0].combined_score, 180.0);
    }

    #[test]
    fn test_combined_score_calculation() {
        let items = vec![
            create_test_item("file.rs", "func", 10, 75.0),
            create_test_item("file.rs", "func", 10, 60.0),
            create_test_item("file.rs", "func", 10, 45.0),
        ];
        let groups = group_by_location(items.iter(), SortCriteria::Score);
        assert_eq!(groups[0].combined_score, 180.0);
    }

    #[test]
    fn test_separate_locations() {
        let items = vec![
            create_test_item("file.rs", "func1", 10, 50.0),
            create_test_item("file.rs", "func2", 20, 50.0),
            create_test_item("other.rs", "func1", 10, 50.0),
        ];
        let groups = group_by_location(items.iter(), SortCriteria::Score);
        assert_eq!(groups.len(), 3);
        for group in &groups {
            assert_eq!(group.items.len(), 1);
        }
    }

    #[test]
    fn test_max_severity() {
        let items = vec![
            create_test_item("file.rs", "func", 10, 75.0),  // high
            create_test_item("file.rs", "func", 10, 120.0), // critical
            create_test_item("file.rs", "func", 10, 45.0),  // medium
        ];
        let groups = group_by_location(items.iter(), SortCriteria::Score);
        assert_eq!(groups[0].max_severity, "critical");
    }
}
