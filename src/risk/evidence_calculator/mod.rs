//! Evidence-based risk calculator for function analysis.
//!
//! This module provides a comprehensive risk assessment for functions
//! based on multiple evidence sources: complexity, coverage, coupling,
//! and change frequency.
//!
//! # Architecture
//!
//! The calculator follows the Stillwater "pure core, imperative shell" pattern:
//!
//! - **module_classifier**: Pure functions for module type classification
//! - **role_utils**: Pure functions for function role handling
//! - **risk_aggregation**: Pure functions for combining risk factors
//! - **risk_classification**: Pure functions for risk level determination
//! - **explanation**: Pure functions for generating human-readable output
//!
//! The main `EvidenceBasedRiskCalculator` is a thin orchestration layer
//! that composes these pure functions.

mod explanation;
mod module_classifier;
mod risk_aggregation;
mod risk_classification;
mod role_utils;

use crate::priority::call_graph::CallGraph;
use crate::priority::FunctionAnalysis;
use crate::risk::evidence::change_analyzer::ChangeRiskAnalyzer;
use crate::risk::evidence::complexity_analyzer::ComplexityRiskAnalyzer;
use crate::risk::evidence::coupling_analyzer::CouplingRiskAnalyzer;
use crate::risk::evidence::coverage_analyzer::CoverageRiskAnalyzer;
use crate::risk::evidence::RiskAssessment;
use crate::risk::lcov::LcovData;

// Re-export pure functions for external use
pub use explanation::{
    find_highest_risk_factor, format_risk_severity, format_risk_type, generate_explanation,
    generate_recommendations, get_effort_estimate,
};
pub use module_classifier::{
    classify_by_directory, classify_by_filename, classify_module_type, is_test_module,
};
pub use risk_aggregation::{
    aggregate_risk_factors, calculate_confidence, calculate_weighted_average, get_role_multiplier,
};
pub use risk_classification::{calculate_role_adjustment, classify_by_score, classify_risk_level};
pub use role_utils::{classify_role, role_to_display_string, visibility_to_string};

/// Evidence-based risk calculator that combines multiple risk analyzers.
///
/// This struct is a thin orchestration layer that coordinates:
/// - Complexity analysis
/// - Coverage analysis
/// - Coupling analysis
/// - Change frequency analysis
///
/// All business logic is delegated to pure functions in submodules.
pub struct EvidenceBasedRiskCalculator {
    complexity_analyzer: ComplexityRiskAnalyzer,
    coverage_analyzer: CoverageRiskAnalyzer,
    coupling_analyzer: CouplingRiskAnalyzer,
    change_analyzer: ChangeRiskAnalyzer,
}

impl Default for EvidenceBasedRiskCalculator {
    fn default() -> Self {
        Self {
            complexity_analyzer: ComplexityRiskAnalyzer::new(),
            coverage_analyzer: CoverageRiskAnalyzer::new(),
            coupling_analyzer: CouplingRiskAnalyzer::new(),
            change_analyzer: ChangeRiskAnalyzer::new(),
        }
    }
}

impl EvidenceBasedRiskCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculates a comprehensive risk assessment for a function.
    ///
    /// This method orchestrates the analysis pipeline:
    /// 1. Classifies the function's role
    /// 2. Builds the risk context
    /// 3. Gathers risk factors from all analyzers
    /// 4. Aggregates factors into a composite score
    /// 5. Classifies the risk level
    /// 6. Generates recommendations and explanation
    pub fn calculate_risk(
        &self,
        function: &FunctionAnalysis,
        call_graph: &CallGraph,
        coverage_data: Option<&LcovData>,
    ) -> RiskAssessment {
        // Step 1: Classify function role
        let role = role_utils::classify_role(function, call_graph);

        // Step 2: Build risk context
        let context = role_utils::build_risk_context(function, role, &function.file);

        // Step 3: Gather risk factors from all analyzers
        let risk_factors = vec![
            self.complexity_analyzer.analyze(function, &context),
            self.coverage_analyzer
                .analyze(function, &context, coverage_data, call_graph),
            self.coupling_analyzer
                .analyze(function, &context, call_graph),
            self.change_analyzer.analyze(function, &context),
        ];

        // Step 4: Aggregate factors into composite score
        let risk_score = risk_aggregation::aggregate_risk_factors(&risk_factors, &role);

        // Step 5: Classify risk level
        let risk_classification = risk_classification::classify_risk_level(risk_score, &role);

        // Step 6: Generate recommendations and explanation
        let recommendations = explanation::generate_recommendations(&risk_factors);
        let confidence = risk_aggregation::calculate_confidence(&risk_factors);
        let explanation_text = explanation::generate_explanation(&risk_factors, &role, risk_score);

        RiskAssessment {
            score: risk_score,
            classification: risk_classification,
            factors: risk_factors,
            role_context: role,
            recommendations,
            confidence,
            explanation: explanation_text,
        }
    }
}
