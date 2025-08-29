use crate::core::{DebtItem, FunctionMetrics};
use crate::priority::call_graph::FunctionId;
use crate::priority::debt_aggregator::DebtAggregator;
use crate::scoring::{CriticalityAnalyzer, ScoreBreakdown, ScoreNormalizer, ScoringContext};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedScore {
    pub base_score: f64,        // Issue severity (1-10)
    pub criticality: f64,       // Function importance (0-2)
    pub complexity_factor: f64, // Actual complexity (0-2)
    pub coverage_factor: f64,   // Coverage correlation (0-2)
    pub dependency_factor: f64, // Downstream impact (0-2)
    pub frequency_factor: f64,  // Call/change frequency (0-2)
    pub test_weight: f64,       // Production vs test (0.3-1.0)
    pub confidence: f64,        // Scoring confidence (0-1)
    pub final_score: f64,       // Normalized final score
    pub raw_score: f64,         // Pre-normalization score
}

impl EnhancedScore {
    pub fn calculate_raw(&self) -> f64 {
        self.base_score
            * self.criticality
            * self.complexity_factor
            * self.coverage_factor
            * self.dependency_factor
            * self.frequency_factor
            * self.test_weight
    }
}

pub struct EnhancedScorer<'a> {
    context: &'a ScoringContext,
    normalizer: ScoreNormalizer,
    criticality_analyzer: CriticalityAnalyzer<'a>,
}

impl<'a> EnhancedScorer<'a> {
    pub fn new(context: &'a ScoringContext) -> Self {
        Self {
            context,
            normalizer: ScoreNormalizer::new(),
            criticality_analyzer: CriticalityAnalyzer::new(context),
        }
    }

    pub fn with_normalizer(mut self, normalizer: ScoreNormalizer) -> Self {
        self.normalizer = normalizer;
        self
    }

    pub fn score_debt_item(
        &self,
        item: &DebtItem,
        function: Option<&FunctionMetrics>,
    ) -> ScoreBreakdown {
        let base_score = self.calculate_base_score(item);
        let is_test = self.is_test_code(&item.file);

        // If we have function metrics, calculate detailed factors
        let (criticality, complexity_factor, coverage_factor, dependency_factor, frequency_factor) =
            if let Some(func) = function {
                let func_id = FunctionId {
                    file: func.file.clone(),
                    name: func.name.clone(),
                    line: func.line,
                };

                let criticality = self.criticality_analyzer.calculate_criticality(func);
                let complexity_factor = self.calculate_complexity_factor(func);
                let coverage_factor = self.calculate_coverage_factor(func);
                let dependency_factor = self.calculate_dependency_factor(&func_id);
                let frequency_factor = self.calculate_frequency_factor(&func_id);

                (
                    criticality,
                    complexity_factor,
                    coverage_factor,
                    dependency_factor,
                    frequency_factor,
                )
            } else {
                // Default factors when function metrics not available
                (1.0, 1.0, 1.0, 1.0, 1.0)
            };

        // Apply test weight
        let test_weight = if is_test { 0.3 } else { 1.0 };

        // Build enhanced score
        let enhanced_score = EnhancedScore {
            base_score,
            criticality,
            complexity_factor,
            coverage_factor,
            dependency_factor,
            frequency_factor,
            test_weight,
            confidence: self.calculate_confidence(item),
            raw_score: 0.0,   // Will be calculated
            final_score: 0.0, // Will be normalized
        };

        // Calculate raw score
        let mut enhanced_score = enhanced_score;
        enhanced_score.raw_score = enhanced_score.calculate_raw();

        // Normalize the score
        enhanced_score.final_score = self.normalizer.normalize(enhanced_score.raw_score);

        // Add deterministic jitter to prevent identical scores
        let seed = Self::hash_item(item);
        enhanced_score.final_score = self.normalizer.add_jitter(enhanced_score.final_score, seed);

        // Build score breakdown
        let mut breakdown = ScoreBreakdown::new(enhanced_score.final_score);
        breakdown.add_component("base_severity", base_score);
        breakdown.add_component("criticality", criticality);
        breakdown.add_component("complexity", complexity_factor);
        breakdown.add_component("coverage", coverage_factor);
        breakdown.add_component("dependency", dependency_factor);
        breakdown.add_component("frequency", frequency_factor);
        breakdown.add_component("test_weight", test_weight);
        breakdown.add_component("raw_score", enhanced_score.raw_score);

        let explanation = self.explain_score(&enhanced_score, item);
        breakdown
            .with_explanation(explanation)
            .with_confidence(enhanced_score.confidence)
    }

    pub fn score_function_with_aggregator(
        &self,
        function: &FunctionMetrics,
        aggregator: &DebtAggregator,
    ) -> ScoreBreakdown {
        let func_id = FunctionId {
            file: function.file.clone(),
            name: function.name.clone(),
            line: function.line,
        };

        // Get aggregated debt scores
        let agg_func_id = crate::priority::debt_aggregator::FunctionId {
            file: function.file.clone(),
            name: function.name.clone(),
            start_line: function.line,
            end_line: function.line + function.length,
        };
        let debt_scores = aggregator.calculate_debt_scores(&agg_func_id);

        // Calculate base score from aggregated debt
        let base_score = self.calculate_aggregated_base_score(&debt_scores);

        // Calculate all factors
        let criticality = self.criticality_analyzer.calculate_criticality(function);
        let complexity_factor = self.calculate_complexity_factor(function);
        let coverage_factor = self.calculate_coverage_factor(function);
        let dependency_factor = self.calculate_dependency_factor(&func_id);
        let frequency_factor = self.calculate_frequency_factor(&func_id);

        let is_test = self.is_test_code(&function.file);
        let test_weight = if is_test { 0.3 } else { 1.0 };

        // Build enhanced score
        let mut enhanced_score = EnhancedScore {
            base_score,
            criticality,
            complexity_factor,
            coverage_factor,
            dependency_factor,
            frequency_factor,
            test_weight,
            confidence: 0.9, // High confidence with aggregated data
            raw_score: 0.0,
            final_score: 0.0,
        };

        enhanced_score.raw_score = enhanced_score.calculate_raw();
        enhanced_score.final_score = self.normalizer.normalize(enhanced_score.raw_score);

        // Add jitter based on function signature
        let seed = Self::hash_function(function);
        enhanced_score.final_score = self.normalizer.add_jitter(enhanced_score.final_score, seed);

        // Build breakdown
        let mut breakdown = ScoreBreakdown::new(enhanced_score.final_score);
        breakdown.add_component("base_severity", base_score);
        breakdown.add_component("criticality", criticality);
        breakdown.add_component("complexity", complexity_factor);
        breakdown.add_component("coverage", coverage_factor);
        breakdown.add_component("dependency", dependency_factor);
        breakdown.add_component("frequency", frequency_factor);
        breakdown.add_component("test_weight", test_weight);
        breakdown.add_component("organization_debt", debt_scores.organization);

        let explanation = format!(
            "Function {} has {} debt score with criticality {:.1}x",
            function.name,
            if enhanced_score.final_score > 7.0 {
                "high"
            } else if enhanced_score.final_score > 4.0 {
                "medium"
            } else {
                "low"
            },
            criticality
        );

        breakdown
            .with_explanation(explanation)
            .with_confidence(enhanced_score.confidence)
    }

    fn calculate_base_score(&self, item: &DebtItem) -> f64 {
        let base = Self::debt_type_base_severity(&item.debt_type);
        Self::apply_priority_multiplier(base, &item.priority)
    }

    fn debt_type_base_severity(debt_type: &crate::core::DebtType) -> f64 {
        use crate::core::DebtType;

        match debt_type {
            DebtType::ErrorSwallowing => 7.5,
            DebtType::Complexity => 6.5,
            DebtType::Duplication | DebtType::ResourceManagement => 6.0,
            DebtType::Dependency => 5.5,
            DebtType::CodeSmell => 5.0,
            DebtType::TestQuality => 4.5,
            DebtType::Todo | DebtType::Fixme => 4.0,
            DebtType::CodeOrganization => 3.5,
            DebtType::TestComplexity | DebtType::TestTodo | DebtType::TestDuplication => 3.0,
        }
    }

    fn apply_priority_multiplier(base: f64, priority: &crate::core::Priority) -> f64 {
        use crate::core::Priority;

        match priority {
            Priority::Critical => base * 1.5,
            Priority::High => base * 1.2,
            Priority::Medium => base,
            Priority::Low => base * 0.7,
        }
    }

    fn calculate_aggregated_base_score(
        &self,
        debt_scores: &crate::priority::debt_aggregator::DebtScores,
    ) -> f64 {
        // Combine all debt categories into a base score
        let organization_weight = if debt_scores.organization > 0.0 {
            1.0
        } else {
            0.0
        };
        let testing_weight = if debt_scores.testing > 0.0 { 1.2 } else { 0.0 };
        let resource_weight = if debt_scores.resource > 0.0 { 1.3 } else { 0.0 };

        let total_weight = organization_weight + testing_weight + resource_weight;

        if total_weight == 0.0 {
            return 0.0;
        }

        let weighted_sum = debt_scores.organization * organization_weight
            + debt_scores.testing * testing_weight
            + debt_scores.resource * resource_weight;

        (weighted_sum / total_weight).min(10.0)
    }

    fn calculate_complexity_factor(&self, func: &FunctionMetrics) -> f64 {
        // Map actual complexity to a multiplier
        let combined_complexity = (func.cyclomatic + func.cognitive) as f64 / 2.0;

        if combined_complexity <= 3.0 {
            0.8 // Simple functions get lower scores
        } else if combined_complexity <= 5.0 {
            1.0
        } else if combined_complexity <= 10.0 {
            1.2
        } else if combined_complexity <= 20.0 {
            1.5
        } else {
            1.8 // Very complex functions get higher scores
        }
    }

    fn calculate_coverage_factor(&self, func: &FunctionMetrics) -> f64 {
        if let Some(coverage) = &self.context.coverage_map {
            if let Some(coverage_pct) = coverage.get_function_coverage(&func.file, &func.name) {
                if coverage_pct == 0.0 {
                    1.8 // No coverage is bad
                } else if coverage_pct < 50.0 {
                    1.4
                } else if coverage_pct < 80.0 {
                    1.1
                } else {
                    0.9 // Well-covered code gets lower scores
                }
            } else {
                1.2 // Unknown coverage
            }
        } else {
            1.2 // No coverage data available
        }
    }

    fn calculate_dependency_factor(&self, func_id: &FunctionId) -> f64 {
        let downstream_count = self.context.call_graph.get_callees(func_id).len();

        if downstream_count == 0 {
            0.9 // Leaf functions are less critical
        } else if downstream_count <= 3 {
            1.0
        } else if downstream_count <= 10 {
            1.3
        } else {
            1.6 // Functions with many dependencies are critical
        }
    }

    fn calculate_frequency_factor(&self, func_id: &FunctionId) -> f64 {
        // Call frequency
        let call_freq = self
            .context
            .call_frequencies
            .get(func_id)
            .copied()
            .unwrap_or(0);

        let call_factor = if call_freq == 0 {
            0.9
        } else if call_freq <= 2 {
            1.0
        } else if call_freq <= 5 {
            1.2
        } else {
            1.4
        };

        // Change frequency (if git history available)
        let change_factor = if let Some(git_history) = &self.context.git_history {
            if let Some(changes) = git_history.change_counts.get(&func_id.file) {
                if *changes > 20 {
                    1.3
                } else if *changes > 10 {
                    1.15
                } else {
                    1.0
                }
            } else {
                1.0
            }
        } else {
            1.0
        };

        f64::min(call_factor * change_factor, 1.8)
    }

    fn calculate_confidence(&self, item: &DebtItem) -> f64 {
        // Higher confidence for concrete issues vs pattern-based detection
        use crate::core::DebtType;

        match item.debt_type {
            DebtType::Todo | DebtType::Fixme => 1.0, // Direct text match
            DebtType::Duplication => 0.95,           // Hash-based
            DebtType::CodeSmell => 0.85,             // Pattern-based
            DebtType::TestQuality => 0.8,            // Heuristic-based
            DebtType::ErrorSwallowing => 0.9,        // AST-based
            _ => 0.75,
        }
    }

    fn is_test_code(&self, file: &std::path::Path) -> bool {
        let path_str = file.to_string_lossy();

        // Check if in test files set
        if self.context.test_files.contains(file) {
            return true;
        }

        // Common test patterns
        path_str.contains("/tests/")
            || path_str.contains("/test/")
            || path_str.ends_with("_test.rs")
            || path_str.ends_with("_tests.rs")
            || path_str.ends_with(".test.")
            || path_str.ends_with(".spec.")
    }

    fn explain_score(&self, score: &EnhancedScore, item: &DebtItem) -> String {
        let mut parts = vec![format!(
            "{}: base {:.1}",
            Self::debt_type_name(&item.debt_type),
            score.base_score
        )];

        parts.extend(Self::collect_factor_explanations(score));
        parts.join(", ")
    }

    fn collect_factor_explanations(score: &EnhancedScore) -> Vec<String> {
        const THRESHOLD: f64 = 1.3;
        let mut explanations = Vec::new();

        let factors = [
            (score.criticality, "critical path"),
            (score.complexity_factor, "high complexity"),
            (score.coverage_factor, "low coverage"),
            (score.dependency_factor, "high impact"),
            (score.frequency_factor, "frequently used"),
        ];

        for (value, label) in factors {
            if value > THRESHOLD {
                explanations.push(format!("{} ({:.1}x)", label, value));
            }
        }

        if score.test_weight < 1.0 {
            explanations.push("test code (0.3x)".to_string());
        }

        explanations
    }

    fn debt_type_name(debt_type: &crate::core::DebtType) -> &'static str {
        use crate::core::DebtType;

        match debt_type {
            DebtType::Todo => "TODO",
            DebtType::Fixme => "FIXME",
            DebtType::CodeSmell => "Code smell",
            DebtType::ErrorSwallowing => "Error handling",
            DebtType::TestQuality => "Test quality",
            DebtType::Duplication => "Duplication",
            DebtType::Dependency => "Dependency",
            DebtType::Complexity => "Complexity",
            DebtType::ResourceManagement => "Resource",
            DebtType::CodeOrganization => "Organization",
            DebtType::TestComplexity => "Test complexity",
            DebtType::TestTodo => "Test TODO",
            DebtType::TestDuplication => "Test duplication",
        }
    }

    fn hash_item(item: &DebtItem) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        item.file.hash(&mut hasher);
        item.line.hash(&mut hasher);
        item.message.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_function(func: &FunctionMetrics) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        func.file.hash(&mut hasher);
        func.name.hash(&mut hasher);
        func.line.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DebtItem, DebtType, Priority};
    use std::path::PathBuf;

    #[test]
    fn test_debt_type_base_severity() {
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::ErrorSwallowing), 7.5);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::Complexity), 6.5);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::Duplication), 6.0);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::ResourceManagement), 6.0);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::Dependency), 5.5);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::CodeSmell), 5.0);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::TestQuality), 4.5);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::Todo), 4.0);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::Fixme), 4.0);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::CodeOrganization), 3.5);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::TestComplexity), 3.0);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::TestTodo), 3.0);
        assert_eq!(EnhancedScorer::debt_type_base_severity(&DebtType::TestDuplication), 3.0);
    }

    #[test]
    fn test_apply_priority_multiplier() {
        let base = 5.0;
        assert_eq!(EnhancedScorer::apply_priority_multiplier(base, &Priority::Critical), 7.5);
        assert_eq!(EnhancedScorer::apply_priority_multiplier(base, &Priority::High), 6.0);
        assert_eq!(EnhancedScorer::apply_priority_multiplier(base, &Priority::Medium), 5.0);
        assert_eq!(EnhancedScorer::apply_priority_multiplier(base, &Priority::Low), 3.5);
    }

    #[test]
    fn test_collect_factor_explanations() {
        let score = EnhancedScore {
            base_score: 5.0,
            criticality: 1.5,
            complexity_factor: 1.1,
            coverage_factor: 1.4,
            dependency_factor: 1.0,
            frequency_factor: 1.6,
            test_weight: 1.0,
            confidence: 0.9,
            final_score: 7.5,
            raw_score: 7.0,
        };

        let explanations = EnhancedScorer::collect_factor_explanations(&score);
        assert_eq!(explanations.len(), 3);
        assert!(explanations.contains(&"critical path (1.5x)".to_string()));
        assert!(explanations.contains(&"low coverage (1.4x)".to_string()));
        assert!(explanations.contains(&"frequently used (1.6x)".to_string()));
    }

    #[test]
    fn test_collect_factor_explanations_with_test_weight() {
        let score = EnhancedScore {
            base_score: 5.0,
            criticality: 1.0,
            complexity_factor: 1.0,
            coverage_factor: 1.0,
            dependency_factor: 1.0,
            frequency_factor: 1.0,
            test_weight: 0.3,
            confidence: 0.9,
            final_score: 1.5,
            raw_score: 1.5,
        };

        let explanations = EnhancedScorer::collect_factor_explanations(&score);
        assert_eq!(explanations.len(), 1);
        assert!(explanations.contains(&"test code (0.3x)".to_string()));
    }

    #[test]
    fn test_calculate_base_score_integration() {
        use crate::priority::call_graph::CallGraph;
        
        let call_graph = CallGraph::new();
        let context = ScoringContext::new(call_graph);
        let scorer = EnhancedScorer::new(&context);

        let item = DebtItem {
            id: "test_item".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            column: Some(5),
            debt_type: DebtType::Complexity,
            priority: Priority::High,
            message: "Test message".to_string(),
            context: None,
        };

        let score = scorer.calculate_base_score(&item);
        assert_eq!(score, 7.8); // 6.5 * 1.2
    }
}
