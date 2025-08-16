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
            FunctionRole::Unknown => "general",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
