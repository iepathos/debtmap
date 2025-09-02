use super::ComplexityAttribution;
use crate::analysis::multi_pass::MultiPassResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tracks changes between analysis runs
pub struct ChangeTracker {
    previous_results: HashMap<String, MultiPassResult>,
}

impl ChangeTracker {
    pub fn new() -> Self {
        Self {
            previous_results: HashMap::new(),
        }
    }

    pub fn track_changes(
        &mut self,
        file_id: String,
        current: &MultiPassResult,
    ) -> Option<ChangeAnalysis> {
        if let Some(previous) = self.previous_results.get(&file_id) {
            let analysis = self.analyze_changes(previous, current);
            self.previous_results.insert(file_id, current.clone());
            Some(analysis)
        } else {
            self.previous_results.insert(file_id, current.clone());
            None
        }
    }

    fn analyze_changes(
        &self,
        previous: &MultiPassResult,
        current: &MultiPassResult,
    ) -> ChangeAnalysis {
        let complexity_delta = ChangeMetrics {
            raw_change: current.raw_complexity.total_complexity as i32
                - previous.raw_complexity.total_complexity as i32,
            normalized_change: current.normalized_complexity.total_complexity as i32
                - previous.normalized_complexity.total_complexity as i32,
            cognitive_change: current.raw_complexity.cognitive_complexity as i32
                - previous.raw_complexity.cognitive_complexity as i32,
        };

        let attribution_changes =
            self.analyze_attribution_changes(&previous.attribution, &current.attribution);

        let change_categories = self.categorize_changes(&complexity_delta, &attribution_changes);
        let improvement_score = self.calculate_improvement_score(&complexity_delta);

        ChangeAnalysis {
            complexity_delta,
            attribution_changes,
            change_categories,
            improvement_score,
        }
    }

    fn analyze_attribution_changes(
        &self,
        previous: &ComplexityAttribution,
        current: &ComplexityAttribution,
    ) -> AttributionChanges {
        AttributionChanges {
            logical_change: current.logical_complexity.total as i32
                - previous.logical_complexity.total as i32,
            formatting_change: current.formatting_artifacts.total as i32
                - previous.formatting_artifacts.total as i32,
            pattern_change: current.pattern_complexity.total as i32
                - previous.pattern_complexity.total as i32,
            confidence_change: self.calculate_confidence_change(previous, current),
        }
    }

    fn calculate_confidence_change(
        &self,
        previous: &ComplexityAttribution,
        current: &ComplexityAttribution,
    ) -> f32 {
        let prev_avg = (previous.logical_complexity.confidence
            + previous.formatting_artifacts.confidence
            + previous.pattern_complexity.confidence)
            / 3.0;
        let curr_avg = (current.logical_complexity.confidence
            + current.formatting_artifacts.confidence
            + current.pattern_complexity.confidence)
            / 3.0;
        curr_avg - prev_avg
    }

    fn categorize_changes(
        &self,
        metrics: &ChangeMetrics,
        attribution: &AttributionChanges,
    ) -> Vec<ChangeCategory> {
        let mut categories = Vec::new();

        // Categorize based on raw change
        if metrics.raw_change < -5 {
            categories.push(ChangeCategory::SignificantImprovement);
        } else if metrics.raw_change > 5 {
            categories.push(ChangeCategory::SignificantRegression);
        }

        // Check if change is primarily formatting
        if attribution.formatting_change.abs() > attribution.logical_change.abs() {
            categories.push(ChangeCategory::FormattingRelated);
        } else {
            categories.push(ChangeCategory::LogicalChange);
        }

        // Check for pattern improvements
        if attribution.pattern_change > 2 {
            categories.push(ChangeCategory::PatternImprovement);
        }

        // Check for refactoring
        if metrics.raw_change < 0 && metrics.cognitive_change < 0 {
            categories.push(ChangeCategory::SuccessfulRefactoring);
        }

        if categories.is_empty() {
            categories.push(ChangeCategory::MinorChange);
        }

        categories
    }

    fn calculate_improvement_score(&self, metrics: &ChangeMetrics) -> f32 {
        // Negative changes are improvements (reduced complexity)
        let raw_improvement = (-metrics.raw_change as f32) / 10.0;
        let cognitive_improvement = (-metrics.cognitive_change as f32) / 10.0;
        let normalized_improvement = (-metrics.normalized_change as f32) / 10.0;

        // Weighted average favoring cognitive complexity
        (raw_improvement * 0.3 + cognitive_improvement * 0.5 + normalized_improvement * 0.2)
            .max(-1.0)
            .min(1.0)
    }
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Analysis of changes between runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeAnalysis {
    pub complexity_delta: ChangeMetrics,
    pub attribution_changes: AttributionChanges,
    pub change_categories: Vec<ChangeCategory>,
    pub improvement_score: f32,
}

/// Metrics describing changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMetrics {
    pub raw_change: i32,
    pub normalized_change: i32,
    pub cognitive_change: i32,
}

/// Attribution-specific changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionChanges {
    pub logical_change: i32,
    pub formatting_change: i32,
    pub pattern_change: i32,
    pub confidence_change: f32,
}

/// Categories of changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeCategory {
    SignificantImprovement,
    SignificantRegression,
    FormattingRelated,
    LogicalChange,
    PatternImprovement,
    SuccessfulRefactoring,
    MinorChange,
}

impl ChangeCategory {
    pub fn description(&self) -> &str {
        match self {
            Self::SignificantImprovement => "Significant complexity reduction",
            Self::SignificantRegression => "Significant complexity increase",
            Self::FormattingRelated => "Primarily formatting-related changes",
            Self::LogicalChange => "Changes to logical structure",
            Self::PatternImprovement => "Improved pattern recognition",
            Self::SuccessfulRefactoring => "Successful refactoring",
            Self::MinorChange => "Minor changes only",
        }
    }

    pub fn is_positive(&self) -> bool {
        matches!(
            self,
            Self::SignificantImprovement | Self::PatternImprovement | Self::SuccessfulRefactoring
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::attribution::AttributedComplexity;
    use crate::analysis::multi_pass::{AnalysisType, ComplexityResult};

    #[test]
    fn test_change_tracker_new() {
        let tracker = ChangeTracker::new();
        assert!(tracker.previous_results.is_empty());
    }

    #[test]
    fn test_track_changes_first_run() {
        let mut tracker = ChangeTracker::new();
        let result = create_test_result(10, 8);

        let analysis = tracker.track_changes("test.rs".to_string(), &result);
        assert!(analysis.is_none());
        assert!(tracker.previous_results.contains_key("test.rs"));
    }

    #[test]
    fn test_track_changes_second_run() {
        let mut tracker = ChangeTracker::new();
        let result1 = create_test_result(10, 8);
        let result2 = create_test_result(8, 6);

        tracker.track_changes("test.rs".to_string(), &result1);
        let analysis = tracker.track_changes("test.rs".to_string(), &result2);

        assert!(analysis.is_some());
        let analysis = analysis.unwrap();
        assert_eq!(analysis.complexity_delta.raw_change, -2);
        assert!(analysis.improvement_score > 0.0);
    }

    #[test]
    fn test_categorize_improvement() {
        let tracker = ChangeTracker::new();
        let metrics = ChangeMetrics {
            raw_change: -8,
            normalized_change: -6,
            cognitive_change: -5,
        };
        let attribution = AttributionChanges {
            logical_change: -7,
            formatting_change: -1,
            pattern_change: 0,
            confidence_change: 0.1,
        };

        let categories = tracker.categorize_changes(&metrics, &attribution);
        assert!(categories.contains(&ChangeCategory::SignificantImprovement));
        assert!(categories.contains(&ChangeCategory::SuccessfulRefactoring));
    }

    #[test]
    fn test_calculate_improvement_score() {
        let tracker = ChangeTracker::new();

        let improvement = ChangeMetrics {
            raw_change: -10,
            normalized_change: -8,
            cognitive_change: -12,
        };
        let score = tracker.calculate_improvement_score(&improvement);
        assert!(score > 0.0);

        let regression = ChangeMetrics {
            raw_change: 10,
            normalized_change: 8,
            cognitive_change: 12,
        };
        let score = tracker.calculate_improvement_score(&regression);
        assert!(score < 0.0);
    }

    #[test]
    fn test_change_category_descriptions() {
        assert_eq!(
            ChangeCategory::SignificantImprovement.description(),
            "Significant complexity reduction"
        );
        assert!(ChangeCategory::SignificantImprovement.is_positive());
        assert!(!ChangeCategory::SignificantRegression.is_positive());
    }

    fn create_test_result(complexity: u32, cognitive: u32) -> MultiPassResult {
        MultiPassResult {
            raw_complexity: ComplexityResult {
                total_complexity: complexity,
                cognitive_complexity: cognitive,
                functions: vec![],
                analysis_type: AnalysisType::Raw,
            },
            normalized_complexity: ComplexityResult {
                total_complexity: complexity - 2,
                cognitive_complexity: cognitive - 1,
                functions: vec![],
                analysis_type: AnalysisType::Normalized,
            },
            attribution: ComplexityAttribution {
                logical_complexity: AttributedComplexity {
                    total: complexity - 3,
                    breakdown: vec![],
                    confidence: 0.8,
                },
                formatting_artifacts: AttributedComplexity {
                    total: 3,
                    breakdown: vec![],
                    confidence: 0.7,
                },
                pattern_complexity: AttributedComplexity {
                    total: 0,
                    breakdown: vec![],
                    confidence: 0.5,
                },
                source_mappings: vec![],
            },
            insights: vec![],
            recommendations: vec![],
        }
    }
}
