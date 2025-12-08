pub mod context;
pub mod correlation;
pub mod coverage_gap;
pub mod coverage_index;
pub mod delegation;
pub mod effects;
pub mod evidence;
pub mod evidence_calculator;
pub mod function_name_matching;
pub mod insights;
pub mod lcov;
pub mod path_normalization;
pub mod priority;
pub mod roi;
pub mod strategy;
pub mod thresholds;

use crate::core::ComplexityMetrics;
use im::Vector;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionRisk {
    pub file: PathBuf,
    pub function_name: String,
    pub line_range: (usize, usize),
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub coverage_percentage: Option<f64>,
    pub risk_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual_risk: Option<context::ContextualRisk>,
    pub test_effort: TestEffort,
    pub risk_category: RiskCategory,
    pub is_test_function: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RiskCategory {
    Critical,   // High complexity (>15), low coverage (<30%)
    High,       // High complexity (>10), moderate coverage (<60%)
    Medium,     // Moderate complexity (>5), low coverage (<50%)
    Low,        // Low complexity or high coverage
    WellTested, // High complexity with high coverage (good examples)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestEffort {
    pub estimated_difficulty: Difficulty,
    pub cognitive_load: u32,
    pub branch_count: u32,
    pub recommended_test_cases: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Difficulty {
    Trivial,     // Cognitive < 5
    Simple,      // Cognitive 5-10
    Moderate,    // Cognitive 10-20
    Complex,     // Cognitive 20-40
    VeryComplex, // Cognitive > 40
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskInsight {
    pub top_risks: Vector<FunctionRisk>,
    pub risk_reduction_opportunities: Vector<TestingRecommendation>,
    pub codebase_risk_score: f64,
    pub complexity_coverage_correlation: Option<f64>,
    pub risk_distribution: RiskDistribution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestingRecommendation {
    pub function: String,
    pub file: PathBuf,
    pub line: usize,
    pub current_risk: f64,
    pub potential_risk_reduction: f64,
    pub test_effort_estimate: TestEffort,
    pub rationale: String,
    pub roi: Option<f64>,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskDistribution {
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub well_tested_count: usize,
    pub total_functions: usize,
}

use self::context::{AnalysisTarget, ContextAggregator, ContextualRisk};
use self::strategy::{EnhancedRiskStrategy, RiskCalculator, RiskContext};
use std::sync::Arc;

pub struct RiskAnalyzer {
    strategy: Box<dyn RiskCalculator>,
    debt_score: Option<f64>,
    debt_threshold: Option<f64>,
    context_aggregator: Option<Arc<ContextAggregator>>,
}

impl Clone for RiskAnalyzer {
    /// Clone the risk analyzer, preserving context aggregator.
    ///
    /// The context aggregator is wrapped in Arc, so cloning is cheap (just
    /// an atomic reference count increment) and preserves the shared cache.
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy.box_clone(),
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            context_aggregator: self.context_aggregator.clone(), // Arc::clone is cheap!
        }
    }
}

impl Default for RiskAnalyzer {
    fn default() -> Self {
        Self {
            strategy: Box::new(EnhancedRiskStrategy::default()),
            debt_score: None,
            debt_threshold: None,
            context_aggregator: None,
        }
    }
}

impl RiskAnalyzer {
    pub fn with_debt_context(mut self, debt_score: f64, debt_threshold: f64) -> Self {
        self.debt_score = Some(debt_score);
        self.debt_threshold = Some(debt_threshold);
        self
    }

    pub fn with_context_aggregator(mut self, aggregator: ContextAggregator) -> Self {
        self.context_aggregator = Some(Arc::new(aggregator));
        self
    }

    pub fn has_context(&self) -> bool {
        self.context_aggregator.is_some()
    }

    pub fn analyze_function(
        &self,
        file: PathBuf,
        function_name: String,
        line_range: (usize, usize),
        complexity: &ComplexityMetrics,
        coverage: Option<f64>,
        is_test: bool,
    ) -> FunctionRisk {
        let context = RiskContext {
            file,
            function_name,
            line_range,
            complexity: complexity.clone(),
            coverage,
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            is_test,
            is_recognized_pattern: false,
            pattern_type: None,
            pattern_confidence: 0.0,
        };

        self.strategy.calculate(&context)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn analyze_function_with_context(
        &self,
        file: PathBuf,
        function_name: String,
        line_range: (usize, usize),
        complexity: &ComplexityMetrics,
        coverage: Option<f64>,
        is_test: bool,
        root_path: PathBuf,
    ) -> (FunctionRisk, Option<ContextualRisk>) {
        let mut base_risk = self.analyze_function(
            file.clone(),
            function_name.clone(),
            line_range,
            complexity,
            coverage,
            is_test,
        );

        let contextual_risk = if let Some(ref aggregator) = self.context_aggregator {
            let target = AnalysisTarget {
                root_path,
                file_path: file,
                function_name: function_name.clone(),
                line_range,
            };

            let context_map = aggregator.analyze(&target);
            let ctx_risk = ContextualRisk::new(base_risk.risk_score, &context_map);

            // Update the FunctionRisk with contextual data
            base_risk.contextual_risk = Some(ctx_risk.clone());
            base_risk.risk_score = ctx_risk.contextual_risk;

            // Verbose logging for context contributions
            if log::log_enabled!(log::Level::Debug) {
                log::debug!(
                    "Context analysis for {}::{}: base_risk={:.1}, contextual_risk={:.1}, multiplier={:.2}x",
                    base_risk.file.display(),
                    function_name,
                    ctx_risk.base_risk,
                    ctx_risk.contextual_risk,
                    ctx_risk.contextual_risk / ctx_risk.base_risk.max(0.1)
                );

                for context in &ctx_risk.contexts {
                    log::debug!(
                        "  └─ {}: contribution={:.2}, weight={:.1}, impact=+{:.1}",
                        context.provider,
                        context.contribution,
                        context.weight,
                        context.contribution * context.weight
                    );
                }
            }

            Some(ctx_risk)
        } else {
            None
        };

        (base_risk, contextual_risk)
    }

    pub fn calculate_risk_score(
        &self,
        cyclomatic: u32,
        cognitive: u32,
        coverage: Option<f64>,
    ) -> f64 {
        let context = RiskContext {
            file: PathBuf::new(),
            function_name: String::new(),
            line_range: (0, 0),
            complexity: ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: cyclomatic,
                cognitive_complexity: cognitive,
            },
            coverage,
            debt_score: self.debt_score,
            debt_threshold: self.debt_threshold,
            is_test: false,
            is_recognized_pattern: false,
            pattern_type: None,
            pattern_confidence: 0.0,
        };

        self.strategy.calculate_risk_score(&context)
    }

    pub fn calculate_risk_reduction(
        &self,
        current_risk: f64,
        complexity: u32,
        target_coverage: f64,
    ) -> f64 {
        self.strategy
            .calculate_risk_reduction(current_risk, complexity, target_coverage)
    }

    /// Analyze file-level contextual risk for god objects.
    ///
    /// This method specifically handles file-level analysis where there is no
    /// specific function being analyzed. It's designed for god objects where
    /// the entire file represents the technical debt unit.
    ///
    /// # Arguments
    /// * `file_path` - Path to the file being analyzed
    /// * `base_risk` - Base risk score for the god object (from god object scoring)
    /// * `root_path` - Project root path
    ///
    /// # Returns
    /// `Some(ContextualRisk)` if context analysis is enabled, `None` otherwise
    pub fn analyze_file_context(
        &self,
        file_path: PathBuf,
        base_risk: f64,
        root_path: PathBuf,
    ) -> Option<ContextualRisk> {
        let aggregator = self.context_aggregator.as_ref()?;

        let target = AnalysisTarget {
            root_path,
            file_path,
            function_name: String::new(), // Empty for file-level analysis
            line_range: (0, 0),           // Not applicable for file-level
        };

        let context_map = aggregator.analyze(&target);
        Some(ContextualRisk::new(base_risk, &context_map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_analyzer_clone_preserves_context() {
        let aggregator = ContextAggregator::new();

        let analyzer = RiskAnalyzer::default().with_context_aggregator(aggregator);

        let cloned = analyzer.clone();

        assert!(cloned.has_context());
    }
}
