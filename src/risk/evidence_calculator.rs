use crate::priority::call_graph::CallGraph;
use crate::priority::semantic_classifier::{classify_function_role, FunctionRole};
use crate::priority::{FunctionAnalysis, FunctionVisibility};
use crate::risk::evidence::change_analyzer::ChangeRiskAnalyzer;
use crate::risk::evidence::complexity_analyzer::ComplexityRiskAnalyzer;
use crate::risk::evidence::coupling_analyzer::CouplingRiskAnalyzer;
use crate::risk::evidence::coverage_analyzer::CoverageRiskAnalyzer;
use crate::risk::evidence::{
    ModuleType, RiskAssessment, RiskClassification, RiskContext, RiskFactor,
};
use crate::risk::lcov::LcovData;
use std::path::Path;

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

    pub fn calculate_risk(
        &self,
        function: &FunctionAnalysis,
        call_graph: &CallGraph,
        coverage_data: Option<&LcovData>,
    ) -> RiskAssessment {
        let role = self.classify_function_role(function, call_graph);
        let context = self.build_risk_context(function, role);

        let risk_factors = vec![
            self.complexity_analyzer.analyze(function, &context),
            self.coverage_analyzer
                .analyze(function, &context, coverage_data),
            self.coupling_analyzer
                .analyze(function, &context, call_graph),
            self.change_analyzer.analyze(function, &context),
        ];

        let risk_score = self.aggregate_risk_factors(&risk_factors, &role);
        let risk_classification = self.classify_risk_level(risk_score, &role);
        let recommendations = self.generate_recommendations(&risk_factors, &role);
        let confidence = self.calculate_confidence(&risk_factors);
        let explanation = self.generate_explanation(&risk_factors, &role, risk_score);

        RiskAssessment {
            score: risk_score,
            classification: risk_classification,
            factors: risk_factors,
            role_context: role,
            recommendations,
            confidence,
            explanation,
        }
    }

    fn classify_function_role(
        &self,
        function: &FunctionAnalysis,
        call_graph: &CallGraph,
    ) -> FunctionRole {
        let func_id = crate::priority::call_graph::FunctionId {
            file: function.file.clone(),
            name: function.function.clone(),
            line: function.line,
        };

        let func_metrics = crate::core::FunctionMetrics {
            file: function.file.clone(),
            name: function.function.clone(),
            line: function.line,
            length: function.function_length,
            cyclomatic: function.cyclomatic_complexity,
            cognitive: function.cognitive_complexity,
            nesting: function.nesting_depth,
            is_test: function.is_test,
            visibility: Some(self.visibility_to_string(&function.visibility)),
            is_trait_method: false, // Default to false, can be updated if needed
            in_test_module: false,  // Default to false for risk evidence
            entropy_score: None,    // TODO: Add entropy scoring for risk analysis
            is_pure: None,          // TODO: Add purity detection for risk analysis
            purity_confidence: None,
        };

        classify_function_role(&func_metrics, &func_id, call_graph)
    }

    fn visibility_to_string(&self, visibility: &FunctionVisibility) -> String {
        match visibility {
            FunctionVisibility::Public => "pub".to_string(),
            FunctionVisibility::Crate => "pub(crate)".to_string(),
            FunctionVisibility::Private => "".to_string(),
        }
    }

    fn build_risk_context(&self, function: &FunctionAnalysis, role: FunctionRole) -> RiskContext {
        RiskContext {
            role,
            visibility: function.visibility.clone(),
            module_type: self.determine_module_type(&function.file),
        }
    }

    fn determine_module_type(&self, file: &Path) -> ModuleType {
        let path_str = file.to_string_lossy();

        // Use pure classification functions
        if Self::is_test_module(&path_str) {
            ModuleType::Test
        } else if let Some(module_type) = Self::classify_by_directory(&path_str) {
            module_type
        } else {
            Self::classify_by_filename(&path_str)
        }
    }

    // Pure function to check if path is a test module
    fn is_test_module(path_str: &str) -> bool {
        path_str.contains("/tests/") || path_str.contains("_test.rs")
    }

    // Pure function to classify module by directory structure
    fn classify_by_directory(path_str: &str) -> Option<ModuleType> {
        match () {
            _ if path_str.contains("/core/") || path_str.contains("/domain/") => {
                Some(ModuleType::Core)
            }
            _ if path_str.contains("/api/") || path_str.contains("/handlers/") => {
                Some(ModuleType::Api)
            }
            _ if path_str.contains("/utils/") || path_str.contains("/helpers/") => {
                Some(ModuleType::Util)
            }
            _ if path_str.contains("/infra/") || path_str.contains("/db/") => {
                Some(ModuleType::Infrastructure)
            }
            _ => None,
        }
    }

    // Pure function to classify module by filename
    fn classify_by_filename(path_str: &str) -> ModuleType {
        match () {
            _ if path_str.ends_with("mod.rs") || path_str.ends_with("lib.rs") => ModuleType::Core,
            _ if path_str.contains("main.rs") => ModuleType::Infrastructure,
            _ => ModuleType::Util,
        }
    }

    fn calculate_weighted_average(factors: &[RiskFactor]) -> f64 {
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        for factor in factors {
            // Skip factors with zero weight (e.g., disabled change analysis)
            if factor.weight > 0.0 {
                total_score += factor.score * factor.weight;
                total_weight += factor.weight;
            }
        }

        if total_weight == 0.0 {
            return 0.0;
        }

        total_score / total_weight
    }

    fn get_role_multiplier(role: &FunctionRole) -> f64 {
        match role {
            FunctionRole::PureLogic => 1.2,    // Business logic is more critical
            FunctionRole::EntryPoint => 1.1,   // Entry points are important
            FunctionRole::Orchestrator => 0.9, // Orchestration is less risky
            FunctionRole::IOWrapper => 0.7,    // I/O wrappers are expected to be simple
            FunctionRole::PatternMatch => 0.5, // Pattern matching is very low risk
            FunctionRole::Unknown => 1.0,      // Default multiplier
        }
    }

    fn aggregate_risk_factors(&self, factors: &[RiskFactor], role: &FunctionRole) -> f64 {
        let base_score = Self::calculate_weighted_average(factors);
        let role_multiplier = Self::get_role_multiplier(role);
        (base_score * role_multiplier).min(10.0)
    }

    fn calculate_role_adjustment(role: &FunctionRole) -> f64 {
        match role {
            FunctionRole::IOWrapper => 1.0,    // More lenient for I/O
            FunctionRole::Orchestrator => 0.5, // Slightly more lenient
            FunctionRole::PatternMatch => 1.5, // Very lenient for pattern matching
            _ => 0.0,                          // Standard thresholds
        }
    }

    fn classify_by_score(adjusted_score: f64) -> RiskClassification {
        match adjusted_score {
            s if s <= 2.0 => RiskClassification::WellDesigned,
            s if s <= 4.0 => RiskClassification::Acceptable,
            s if s <= 7.0 => RiskClassification::NeedsImprovement,
            s if s <= 9.0 => RiskClassification::Risky,
            _ => RiskClassification::Critical,
        }
    }

    fn classify_risk_level(&self, score: f64, role: &FunctionRole) -> RiskClassification {
        let adjustment = Self::calculate_role_adjustment(role);
        let adjusted_score = (score - adjustment).max(0.0);
        Self::classify_by_score(adjusted_score)
    }

    fn generate_recommendations(
        &self,
        factors: &[RiskFactor],
        _role: &FunctionRole,
    ) -> Vec<crate::risk::evidence::RemediationAction> {
        let mut all_actions = Vec::new();

        for factor in factors {
            all_actions.extend(factor.remediation_actions.clone());
        }

        // Sort by expected effort (lowest first) and take top 3
        all_actions.sort_by(|a, b| {
            let effort_a = self.get_effort_estimate(a);
            let effort_b = self.get_effort_estimate(b);
            effort_a.cmp(&effort_b)
        });

        all_actions.into_iter().take(3).collect()
    }

    fn get_effort_estimate(&self, action: &crate::risk::evidence::RemediationAction) -> u32 {
        use crate::risk::evidence::RemediationAction;

        match action {
            RemediationAction::RefactorComplexity {
                estimated_effort_hours,
                ..
            } => *estimated_effort_hours,
            RemediationAction::AddTestCoverage {
                estimated_effort_hours,
                ..
            } => *estimated_effort_hours,
            RemediationAction::ReduceCoupling {
                estimated_effort_hours,
                ..
            } => *estimated_effort_hours,
            RemediationAction::ExtractLogic { .. } => 2, // Default low effort for extraction
        }
    }

    fn calculate_confidence(&self, factors: &[RiskFactor]) -> f64 {
        if factors.is_empty() {
            return 0.0;
        }

        let mut total_confidence = 0.0;
        let mut total_weight = 0.0;

        for factor in factors {
            if factor.weight > 0.0 {
                total_confidence += factor.confidence * factor.weight;
                total_weight += factor.weight;
            }
        }

        if total_weight == 0.0 {
            return 0.5;
        }

        total_confidence / total_weight
    }

    fn format_risk_type(risk_type: &crate::risk::evidence::RiskType) -> String {
        match risk_type {
            crate::risk::evidence::RiskType::Complexity {
                cyclomatic,
                cognitive,
                ..
            } => {
                format!("High complexity (cyclomatic: {cyclomatic}, cognitive: {cognitive})")
            }
            crate::risk::evidence::RiskType::Coverage {
                coverage_percentage,
                ..
            } => {
                format!("Low test coverage ({coverage_percentage:.0}%)")
            }
            crate::risk::evidence::RiskType::Coupling {
                afferent_coupling,
                efferent_coupling,
                ..
            } => {
                format!(
                    "High coupling (incoming: {afferent_coupling}, outgoing: {efferent_coupling})"
                )
            }
            crate::risk::evidence::RiskType::ChangeFrequency {
                commits_last_month, ..
            } => {
                format!("Frequent changes ({commits_last_month} commits last month)")
            }
            crate::risk::evidence::RiskType::Architecture { .. } => {
                "Architectural issues detected".to_string()
            }
        }
    }

    fn format_risk_severity(severity: crate::risk::evidence::RiskSeverity) -> &'static str {
        match severity {
            crate::risk::evidence::RiskSeverity::None => "no significant issues",
            crate::risk::evidence::RiskSeverity::Low => "minor issues",
            crate::risk::evidence::RiskSeverity::Moderate => "moderate issues requiring attention",
            crate::risk::evidence::RiskSeverity::High => {
                "significant issues requiring prompt action"
            }
            crate::risk::evidence::RiskSeverity::Critical => {
                "critical issues requiring immediate attention"
            }
        }
    }

    fn find_highest_risk_factor(factors: &[RiskFactor]) -> Option<&RiskFactor> {
        factors.iter().filter(|f| f.weight > 0.0).max_by(|a, b| {
            (a.score * a.weight)
                .partial_cmp(&(b.score * b.weight))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn generate_explanation(
        &self,
        factors: &[RiskFactor],
        role: &FunctionRole,
        score: f64,
    ) -> String {
        let mut explanation = format!(
            "Risk score {:.1}/10 for {} function. ",
            score,
            self.role_to_string(role)
        );

        if let Some(highest) = Self::find_highest_risk_factor(factors) {
            let factor_desc = Self::format_risk_type(&highest.risk_type);
            let severity_desc = Self::format_risk_severity(highest.severity);

            explanation.push_str(&format!(
                "Primary factor: {factor_desc} with {severity_desc}."
            ));
        }

        explanation
    }

    fn role_to_string(&self, role: &FunctionRole) -> &str {
        match role {
            FunctionRole::PureLogic => "pure logic",
            FunctionRole::Orchestrator => "orchestrator",
            FunctionRole::IOWrapper => "I/O wrapper",
            FunctionRole::EntryPoint => "entry point",
            FunctionRole::PatternMatch => "pattern matching",
            FunctionRole::Unknown => "general",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::evidence::{
        ComparisonResult, ComplexityEvidence, ComplexityThreshold, CouplingEvidence,
        CoverageEvidence, RiskEvidence, RiskSeverity, RiskType, TestQuality,
    };

    #[test]
    fn test_is_test_module() {
        assert!(EvidenceBasedRiskCalculator::is_test_module(
            "src/tests/foo.rs"
        ));
        assert!(EvidenceBasedRiskCalculator::is_test_module(
            "src/module_test.rs"
        ));
        assert!(EvidenceBasedRiskCalculator::is_test_module(
            "path/to/tests/bar.rs"
        ));
        assert!(!EvidenceBasedRiskCalculator::is_test_module("src/main.rs"));
        assert!(!EvidenceBasedRiskCalculator::is_test_module("src/lib.rs"));
    }

    #[test]
    fn test_classify_by_directory() {
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/core/logic.rs"),
            Some(ModuleType::Core)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/domain/model.rs"),
            Some(ModuleType::Core)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/api/handler.rs"),
            Some(ModuleType::Api)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/handlers/route.rs"),
            Some(ModuleType::Api)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/utils/helper.rs"),
            Some(ModuleType::Util)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/helpers/format.rs"),
            Some(ModuleType::Util)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/infra/db.rs"),
            Some(ModuleType::Infrastructure)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/db/connection.rs"),
            Some(ModuleType::Infrastructure)
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_directory("src/other/file.rs"),
            None
        );
    }

    #[test]
    fn test_classify_by_filename() {
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_filename("src/mod.rs"),
            ModuleType::Core
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_filename("src/lib.rs"),
            ModuleType::Core
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_filename("src/main.rs"),
            ModuleType::Infrastructure
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_filename("src/foo/main.rs"),
            ModuleType::Infrastructure
        );
        assert_eq!(
            EvidenceBasedRiskCalculator::classify_by_filename("src/something.rs"),
            ModuleType::Util
        );
    }

    #[test]
    fn test_determine_module_type_integration() {
        use std::path::PathBuf;

        let calculator = EvidenceBasedRiskCalculator::new();

        // Test modules should be identified
        assert_eq!(
            calculator.determine_module_type(&PathBuf::from("src/tests/test.rs")),
            ModuleType::Test
        );

        // Core modules from directory
        assert_eq!(
            calculator.determine_module_type(&PathBuf::from("src/core/engine.rs")),
            ModuleType::Core
        );

        // API modules
        assert_eq!(
            calculator.determine_module_type(&PathBuf::from("src/api/routes.rs")),
            ModuleType::Api
        );

        // Fallback to filename classification
        assert_eq!(
            calculator.determine_module_type(&PathBuf::from("src/main.rs")),
            ModuleType::Infrastructure
        );

        // Default to Util for unknown patterns
        assert_eq!(
            calculator.determine_module_type(&PathBuf::from("src/random/file.rs")),
            ModuleType::Util
        );
    }

    #[test]
    fn test_calculate_weighted_average_normal_factors() {
        let factors = create_test_factors_normal();
        // (8.0 * 0.5 + 6.0 * 0.3 + 4.0 * 0.2) / (0.5 + 0.3 + 0.2)
        // = (4.0 + 1.8 + 0.8) / 1.0 = 6.6
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert!((result - 6.6).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_empty_factors() {
        let factors = vec![];
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_weighted_average_all_zero_weights() {
        let factors = create_test_factors_zero_weights();
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_weighted_average_mixed_zero_weights() {
        let factors = create_test_factors_mixed_weights();
        // Only non-zero weights: (5.0 * 0.6 + 10.0 * 0.4) / (0.6 + 0.4)
        // = (3.0 + 4.0) / 1.0 = 7.0
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert!((result - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_single_factor() {
        let factors = create_test_factors_single();
        // Single factor: 9.5 * 1.0 / 1.0 = 9.5
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert!((result - 9.5).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_high_scores() {
        let factors = create_test_factors_high_scores();
        // Maximum scores: (10.0 * 0.7 + 10.0 * 0.3) / (0.7 + 0.3)
        // = (7.0 + 3.0) / 1.0 = 10.0
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert!((result - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_low_scores() {
        let factors = create_test_factors_low_scores();
        // Low scores: (0.5 * 0.5 + 1.0 * 0.5) / (0.5 + 0.5)
        // = (0.25 + 0.5) / 1.0 = 0.75
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert!((result - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_calculate_weighted_average_unequal_weights() {
        let factors = create_test_factors_unequal_weights();
        // (2.0 * 0.9 + 10.0 * 0.1) / (0.9 + 0.1)
        // = (1.8 + 1.0) / 1.0 = 2.8
        let result = EvidenceBasedRiskCalculator::calculate_weighted_average(&factors);
        assert!((result - 2.8).abs() < 0.001);
    }

    // Helper functions to create test data
    fn create_test_factors_normal() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(8.0, 0.5, 20, 15),
            create_coverage_factor(6.0, 0.3, 30.0),
            create_coupling_factor(4.0, 0.2, 10, 8),
        ]
    }

    fn create_test_factors_zero_weights() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(8.0, 0.0, 15, 10),
            create_coverage_factor(6.0, 0.0, 40.0),
        ]
    }

    fn create_test_factors_mixed_weights() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(8.0, 0.0, 15, 10),
            create_coverage_factor(5.0, 0.6, 50.0),
            create_coupling_factor(10.0, 0.4, 20, 15),
        ]
    }

    fn create_test_factors_single() -> Vec<RiskFactor> {
        vec![create_complexity_factor(9.5, 1.0, 30, 25)]
    }

    fn create_test_factors_high_scores() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(10.0, 0.7, 40, 35),
            create_coverage_factor(10.0, 0.3, 0.0),
        ]
    }

    fn create_test_factors_low_scores() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(0.5, 0.5, 3, 2),
            create_coverage_factor(1.0, 0.5, 95.0),
        ]
    }

    fn create_test_factors_unequal_weights() -> Vec<RiskFactor> {
        vec![
            create_complexity_factor(2.0, 0.9, 5, 3),
            create_coverage_factor(10.0, 0.1, 10.0),
        ]
    }

    fn classify_complexity_threshold(cyclomatic: u32) -> ComplexityThreshold {
        match () {
            _ if cyclomatic > 20 => ComplexityThreshold::Critical,
            _ if cyclomatic > 10 => ComplexityThreshold::High,
            _ if cyclomatic > 5 => ComplexityThreshold::Moderate,
            _ => ComplexityThreshold::Low,
        }
    }

    fn create_complexity_factor(
        score: f64,
        weight: f64,
        cyclomatic: u32,
        cognitive: u32,
    ) -> RiskFactor {
        RiskFactor {
            risk_type: RiskType::Complexity {
                cyclomatic,
                cognitive,
                lines: 100,
                threshold_type: classify_complexity_threshold(cyclomatic),
            },
            score,
            severity: score_to_severity(score),
            evidence: RiskEvidence::Complexity(ComplexityEvidence {
                cyclomatic_complexity: cyclomatic,
                cognitive_complexity: cognitive,
                lines_of_code: 100,
                nesting_depth: 3,
                threshold_exceeded: cyclomatic > 10,
                role_adjusted: false,
                comparison_to_baseline: score_to_comparison(score),
            }),
            remediation_actions: vec![],
            weight,
            confidence: 0.8,
        }
    }

    fn create_coverage_factor(score: f64, weight: f64, coverage: f64) -> RiskFactor {
        RiskFactor {
            risk_type: RiskType::Coverage {
                coverage_percentage: coverage,
                critical_paths_uncovered: ((100.0 - coverage) / 5.0) as u32,
                test_quality: coverage_to_quality(coverage),
            },
            score,
            severity: score_to_severity(score),
            evidence: RiskEvidence::Coverage(CoverageEvidence {
                coverage_percentage: coverage,
                critical_paths_uncovered: ((100.0 - coverage) / 5.0) as u32,
                test_count: (coverage / 5.0) as u32,
                test_quality: coverage_to_quality(coverage),
                comparison_to_baseline: score_to_comparison(score),
            }),
            remediation_actions: vec![],
            weight,
            confidence: 0.7,
        }
    }

    fn create_coupling_factor(score: f64, weight: f64, afferent: u32, efferent: u32) -> RiskFactor {
        let instability = efferent as f64 / (afferent + efferent) as f64;
        RiskFactor {
            risk_type: RiskType::Coupling {
                afferent_coupling: afferent,
                efferent_coupling: efferent,
                instability,
                circular_dependencies: 0,
            },
            score,
            severity: score_to_severity(score),
            evidence: RiskEvidence::Coupling(CouplingEvidence {
                afferent_coupling: afferent,
                efferent_coupling: efferent,
                instability,
                circular_dependencies: 0,
                comparison_to_baseline: score_to_comparison(score),
            }),
            remediation_actions: vec![],
            weight,
            confidence: 0.9,
        }
    }

    fn score_to_severity(score: f64) -> RiskSeverity {
        match score {
            s if s >= 9.0 => RiskSeverity::Critical,
            s if s >= 7.0 => RiskSeverity::High,
            s if s >= 4.0 => RiskSeverity::Moderate,
            s if s >= 2.0 => RiskSeverity::Low,
            _ => RiskSeverity::None,
        }
    }

    fn score_to_comparison(score: f64) -> ComparisonResult {
        match score {
            s if s >= 9.5 => ComparisonResult::AboveP95,
            s if s >= 9.0 => ComparisonResult::AboveP90,
            s if s >= 7.5 => ComparisonResult::AboveP75,
            s if s >= 5.0 => ComparisonResult::AboveMedian,
            _ => ComparisonResult::BelowMedian,
        }
    }

    fn coverage_to_quality(coverage: f64) -> TestQuality {
        match coverage {
            c if c >= 90.0 => TestQuality::Excellent,
            c if c >= 70.0 => TestQuality::Good,
            c if c >= 50.0 => TestQuality::Adequate,
            c if c > 0.0 => TestQuality::Poor,
            _ => TestQuality::Missing,
        }
    }
}
