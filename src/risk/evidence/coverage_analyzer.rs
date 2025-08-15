use super::{
    ComparisonResult, CoverageEvidence, RemediationAction, RiskEvidence, RiskFactor, RiskSeverity,
    RiskType, TestQuality, TestType,
};
use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::FunctionAnalysis;
use crate::risk::evidence::RiskContext;
use crate::risk::lcov::LcovData;
use crate::risk::thresholds::{CoverageThresholds, StatisticalThresholdProvider};

pub struct CoverageRiskAnalyzer {
    #[allow(dead_code)]
    thresholds: CoverageThresholds,
    threshold_provider: StatisticalThresholdProvider,
}

impl Default for CoverageRiskAnalyzer {
    fn default() -> Self {
        Self {
            thresholds: CoverageThresholds::default(),
            threshold_provider: StatisticalThresholdProvider::new(),
        }
    }
}

impl CoverageRiskAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pure function to classify coverage level against thresholds
    fn classify_coverage_level(coverage: f64, thresholds: &CoverageThresholds) -> ComparisonResult {
        match () {
            _ if coverage >= thresholds.excellent => ComparisonResult::BelowMedian, // Better than median (inverted for coverage)
            _ if coverage >= thresholds.good => ComparisonResult::AboveMedian,
            _ if coverage >= thresholds.moderate => ComparisonResult::AboveP75,
            _ if coverage >= thresholds.poor => ComparisonResult::AboveP90,
            _ => ComparisonResult::AboveP95,
        }
    }

    /// Pure function to classify test quality based on coverage and complexity
    fn classify_test_quality(coverage: f64, complexity: u32) -> TestQuality {
        match () {
            _ if coverage >= 90.0 && complexity <= 5 => TestQuality::Excellent,
            _ if coverage >= 80.0 => TestQuality::Good,
            _ if coverage >= 60.0 => TestQuality::Adequate,
            _ if coverage > 0.0 => TestQuality::Poor,
            _ => TestQuality::Missing,
        }
    }

    fn classify_confidence_from_coverage(coverage: f64) -> f64 {
        match () {
            _ if coverage == 0.0 => 0.9, // High confidence that uncovered code is risky
            _ if coverage < 50.0 => 0.85,
            _ if coverage < 80.0 => 0.8,
            _ => 0.95, // Very high confidence for well-covered code
        }
    }

    fn classify_role_weight(role: &FunctionRole) -> f64 {
        match role {
            FunctionRole::PureLogic => 1.0,    // Full weight for business logic
            FunctionRole::EntryPoint => 0.9,   // High weight for entry points
            FunctionRole::Orchestrator => 0.6, // Moderate weight for orchestration
            FunctionRole::IOWrapper => 0.4,    // Lower weight for I/O
            FunctionRole::Unknown => 0.8,      // Default weight
        }
    }

    pub fn analyze(
        &self,
        function: &FunctionAnalysis,
        context: &RiskContext,
        coverage_data: Option<&LcovData>,
    ) -> RiskFactor {
        // Test functions don't need coverage themselves
        if function.is_test {
            return self.create_test_function_factor(function);
        }

        let coverage_percentage = self.get_coverage_percentage(function, coverage_data);
        let critical_paths_uncovered =
            self.count_uncovered_critical_paths(function, coverage_percentage, context.role);
        let test_quality =
            self.assess_test_quality(coverage_percentage, function.cyclomatic_complexity);

        // Role-adjusted coverage thresholds
        let adjusted_thresholds = self.adjust_for_role(&context.role);

        let coverage_score = self.calculate_coverage_risk(
            coverage_percentage,
            critical_paths_uncovered,
            &test_quality,
            &adjusted_thresholds,
        );

        let comparison = self.compare_to_baseline(coverage_percentage, &context.role);

        let evidence = CoverageEvidence {
            coverage_percentage,
            critical_paths_uncovered,
            test_count: self
                .estimate_test_count(coverage_percentage, function.cyclomatic_complexity),
            test_quality,
            comparison_to_baseline: comparison,
        };

        let severity = self.classify_coverage_severity(coverage_score, &adjusted_thresholds);
        let remediation_actions = self.get_coverage_actions(
            coverage_percentage,
            function.cyclomatic_complexity,
            critical_paths_uncovered,
            &severity,
            &context.role,
        );

        RiskFactor {
            risk_type: RiskType::Coverage {
                coverage_percentage,
                critical_paths_uncovered,
                test_quality,
            },
            score: coverage_score,
            severity,
            evidence: RiskEvidence::Coverage(evidence),
            remediation_actions,
            weight: Self::classify_role_weight(&context.role),
            confidence: Self::classify_confidence_from_coverage(coverage_percentage),
        }
    }

    fn create_test_function_factor(&self, _function: &FunctionAnalysis) -> RiskFactor {
        // Test functions have no coverage risk by definition
        RiskFactor {
            risk_type: RiskType::Coverage {
                coverage_percentage: 100.0,
                critical_paths_uncovered: 0,
                test_quality: TestQuality::Excellent,
            },
            score: 0.0,
            severity: RiskSeverity::None,
            evidence: RiskEvidence::Coverage(CoverageEvidence {
                coverage_percentage: 100.0,
                critical_paths_uncovered: 0,
                test_count: 0,
                test_quality: TestQuality::Excellent,
                comparison_to_baseline: ComparisonResult::BelowMedian,
            }),
            remediation_actions: vec![],
            weight: 0.0,
            confidence: 1.0,
        }
    }

    fn get_coverage_percentage(
        &self,
        function: &FunctionAnalysis,
        coverage_data: Option<&LcovData>,
    ) -> f64 {
        if let Some(coverage) = coverage_data {
            coverage
                .get_function_coverage_with_line(&function.file, &function.function, function.line)
                .unwrap_or(0.0)
        } else {
            0.0 // No coverage data means untested
        }
    }

    fn count_uncovered_critical_paths(
        &self,
        function: &FunctionAnalysis,
        coverage_percentage: f64,
        role: FunctionRole,
    ) -> u32 {
        if coverage_percentage >= 100.0 {
            return 0;
        }

        let uncovered_paths =
            Self::calculate_uncovered_paths(function.cyclomatic_complexity, coverage_percentage);

        let multiplier = Self::get_role_criticality_multiplier(role);
        (uncovered_paths as f64 * multiplier) as u32
    }

    /// Calculate the number of uncovered paths based on complexity and coverage
    fn calculate_uncovered_paths(complexity: u32, coverage_percentage: f64) -> u32 {
        let uncovered_ratio = 1.0 - (coverage_percentage / 100.0);
        (complexity as f64 * uncovered_ratio) as u32
    }

    /// Get the criticality multiplier for a given function role
    fn get_role_criticality_multiplier(role: FunctionRole) -> f64 {
        match role {
            FunctionRole::PureLogic => 2.0, // All paths critical in business logic
            FunctionRole::EntryPoint => 2.0, // Entry points are critical
            FunctionRole::Orchestrator => 1.0, // Normal criticality
            FunctionRole::IOWrapper => 0.5, // Less critical paths
            FunctionRole::Unknown => 1.0,
        }
    }

    fn assess_test_quality(&self, coverage: f64, complexity: u32) -> TestQuality {
        Self::classify_test_quality(coverage, complexity)
    }

    fn estimate_test_count(&self, coverage: f64, complexity: u32) -> u32 {
        if coverage == 0.0 {
            return 0;
        }

        // Estimate based on coverage and complexity
        let coverage_ratio = coverage / 100.0;
        let estimated = (complexity as f64 * coverage_ratio * 1.5) as u32;
        estimated.max(1)
    }

    fn adjust_for_role(&self, role: &FunctionRole) -> CoverageThresholds {
        let base_thresholds = self.threshold_provider.get_coverage_thresholds(role);

        match role {
            FunctionRole::PureLogic | FunctionRole::EntryPoint => base_thresholds, // Strict requirements
            FunctionRole::Orchestrator => CoverageThresholds {
                excellent: base_thresholds.excellent * 0.9,
                good: base_thresholds.good * 0.9,
                moderate: base_thresholds.moderate * 0.9,
                poor: base_thresholds.poor * 0.9,
                critical: base_thresholds.critical * 0.9,
            },
            FunctionRole::IOWrapper => CoverageThresholds {
                excellent: base_thresholds.excellent * 0.8,
                good: base_thresholds.good * 0.8,
                moderate: base_thresholds.moderate * 0.8,
                poor: base_thresholds.poor * 0.8,
                critical: base_thresholds.critical * 0.8,
            },
            FunctionRole::Unknown => base_thresholds,
        }
    }

    fn calculate_coverage_risk(
        &self,
        coverage: f64,
        critical_paths: u32,
        quality: &TestQuality,
        thresholds: &CoverageThresholds,
    ) -> f64 {
        // Invert coverage for risk (low coverage = high risk)
        let coverage_risk = self.score_coverage(coverage, thresholds);
        let path_risk = self.score_critical_paths(critical_paths);
        let quality_risk = self.score_quality(quality);

        // Weighted average: coverage 60%, paths 25%, quality 15%
        coverage_risk * 0.6 + path_risk * 0.25 + quality_risk * 0.15
    }

    fn score_coverage(&self, coverage: f64, thresholds: &CoverageThresholds) -> f64 {
        Self::classify_coverage_risk(coverage, thresholds)
    }

    /// Pure function to classify coverage into risk score based on thresholds
    fn classify_coverage_risk(coverage: f64, thresholds: &CoverageThresholds) -> f64 {
        let (base_score, range_score, lower_bound, upper_bound) =
            Self::determine_coverage_tier(coverage, thresholds);

        Self::calculate_tier_score(coverage, base_score, range_score, lower_bound, upper_bound)
    }

    /// Determine which coverage tier the value falls into
    fn determine_coverage_tier(
        coverage: f64,
        thresholds: &CoverageThresholds,
    ) -> (f64, f64, f64, f64) {
        match () {
            _ if coverage >= thresholds.excellent => (10.0, 2.0, thresholds.excellent, 100.0),
            _ if coverage >= thresholds.good => (7.5, 2.5, thresholds.good, thresholds.excellent),
            _ if coverage >= thresholds.moderate => {
                (5.0, 2.5, thresholds.moderate, thresholds.good)
            }
            _ if coverage >= thresholds.poor => (2.5, 2.0, thresholds.poor, thresholds.moderate),
            _ if coverage > 0.0 => (0.5, 1.0, 0.0, thresholds.poor),
            _ => (10.0, 0.0, 0.0, 0.0), // Zero coverage = maximum risk
        }
    }

    /// Calculate the final score within a tier
    fn calculate_tier_score(
        coverage: f64,
        base_score: f64,
        range_score: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> f64 {
        if range_score == 0.0 {
            base_score // Special case for zero coverage
        } else {
            let position_in_tier = (coverage - lower_bound) / (upper_bound - lower_bound);
            base_score - position_in_tier * range_score
        }
    }

    fn score_critical_paths(&self, paths: u32) -> f64 {
        match paths {
            0 => 0.0,
            1..=2 => 2.5,
            3..=5 => 5.0,
            6..=10 => 7.5,
            _ => 10.0,
        }
    }

    fn score_quality(&self, quality: &TestQuality) -> f64 {
        match quality {
            TestQuality::Excellent => 0.0,
            TestQuality::Good => 2.5,
            TestQuality::Adequate => 5.0,
            TestQuality::Poor => 7.5,
            TestQuality::Missing => 10.0,
        }
    }

    fn classify_coverage_severity(
        &self,
        score: f64,
        _thresholds: &CoverageThresholds,
    ) -> RiskSeverity {
        match score {
            s if s <= 2.0 => RiskSeverity::None,
            s if s <= 4.0 => RiskSeverity::Low,
            s if s <= 6.0 => RiskSeverity::Moderate,
            s if s <= 8.0 => RiskSeverity::High,
            _ => RiskSeverity::Critical,
        }
    }

    fn compare_to_baseline(&self, coverage: f64, role: &FunctionRole) -> ComparisonResult {
        let baseline = self.threshold_provider.get_coverage_thresholds(role);
        Self::classify_coverage_level(coverage, &baseline)
    }

    fn get_coverage_actions(
        &self,
        coverage: f64,
        complexity: u32,
        critical_paths: u32,
        severity: &RiskSeverity,
        role: &FunctionRole,
    ) -> Vec<RemediationAction> {
        match severity {
            RiskSeverity::None | RiskSeverity::Low => vec![],
            RiskSeverity::Moderate => vec![RemediationAction::AddTestCoverage {
                current_coverage: coverage,
                target_coverage: 80.0,
                critical_paths: self.identify_critical_paths(critical_paths, role),
                test_types_needed: vec![TestType::Unit, TestType::EdgeCase],
                estimated_effort_hours: 2,
            }],
            RiskSeverity::High => vec![RemediationAction::AddTestCoverage {
                current_coverage: coverage,
                target_coverage: 90.0,
                critical_paths: self.identify_critical_paths(critical_paths, role),
                test_types_needed: vec![
                    TestType::Unit,
                    TestType::Integration,
                    TestType::EdgeCase,
                    TestType::Parameterized,
                ],
                estimated_effort_hours: 4,
            }],
            RiskSeverity::Critical => vec![
                RemediationAction::AddTestCoverage {
                    current_coverage: coverage,
                    target_coverage: 95.0,
                    critical_paths: self.identify_critical_paths(critical_paths, role),
                    test_types_needed: vec![
                        TestType::Unit,
                        TestType::Integration,
                        TestType::Property,
                        TestType::Parameterized,
                        TestType::EdgeCase,
                    ],
                    estimated_effort_hours: 8,
                },
                RemediationAction::ExtractLogic {
                    extraction_candidates: vec![],
                    pure_function_opportunities: (complexity / 5).max(1),
                    testability_improvement: 0.5,
                },
            ],
        }
    }

    fn identify_critical_paths(&self, count: u32, role: &FunctionRole) -> Vec<String> {
        let mut paths = Vec::new();

        for i in 0..count.min(5) {
            let path_name = match role {
                FunctionRole::PureLogic => format!("Business logic path {}", i + 1),
                FunctionRole::EntryPoint => format!("Entry point flow {}", i + 1),
                FunctionRole::Orchestrator => format!("Orchestration path {}", i + 1),
                FunctionRole::IOWrapper => format!("I/O operation {}", i + 1),
                FunctionRole::Unknown => format!("Execution path {}", i + 1),
            };
            paths.push(path_name);
        }

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::FunctionVisibility;
    use std::path::PathBuf;

    #[test]
    fn test_classify_test_quality_excellent() {
        // Coverage >= 90% and complexity <= 5 should be Excellent
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(90.0, 5),
            TestQuality::Excellent
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(95.0, 3),
            TestQuality::Excellent
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(100.0, 1),
            TestQuality::Excellent
        );
        // Edge case: exactly at boundaries
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(90.0, 5),
            TestQuality::Excellent
        );
    }

    #[test]
    fn test_classify_test_quality_good() {
        // Coverage >= 80% should be Good (when not Excellent)
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(80.0, 10),
            TestQuality::Good
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(85.0, 6),
            TestQuality::Good
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(89.9, 6),
            TestQuality::Good
        );
        // High coverage with high complexity is still Good
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(95.0, 20),
            TestQuality::Good
        );
    }

    #[test]
    fn test_classify_test_quality_adequate() {
        // Coverage >= 60% but < 80% should be Adequate
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(60.0, 5),
            TestQuality::Adequate
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(70.0, 10),
            TestQuality::Adequate
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(79.9, 15),
            TestQuality::Adequate
        );
    }

    #[test]
    fn test_classify_test_quality_poor() {
        // Coverage > 0% but < 60% should be Poor
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(0.1, 5),
            TestQuality::Poor
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(30.0, 10),
            TestQuality::Poor
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(59.9, 20),
            TestQuality::Poor
        );
    }

    #[test]
    fn test_classify_test_quality_missing() {
        // Coverage == 0% should be Missing
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(0.0, 1),
            TestQuality::Missing
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(0.0, 10),
            TestQuality::Missing
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(0.0, 100),
            TestQuality::Missing
        );
    }

    #[test]
    fn test_classify_test_quality_boundary_conditions() {
        // Test exact boundary values
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(89.99, 5),
            TestQuality::Good
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(90.01, 5),
            TestQuality::Excellent
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(79.99, 10),
            TestQuality::Adequate
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(80.01, 10),
            TestQuality::Good
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(59.99, 10),
            TestQuality::Poor
        );
        assert_eq!(
            CoverageRiskAnalyzer::classify_test_quality(60.01, 10),
            TestQuality::Adequate
        );
    }

    #[test]
    fn test_calculate_uncovered_paths_zero_coverage() {
        assert_eq!(CoverageRiskAnalyzer::calculate_uncovered_paths(10, 0.0), 10);
    }

    #[test]
    fn test_calculate_uncovered_paths_partial_coverage() {
        assert_eq!(CoverageRiskAnalyzer::calculate_uncovered_paths(10, 50.0), 5);
        assert_eq!(CoverageRiskAnalyzer::calculate_uncovered_paths(10, 75.0), 2);
    }

    #[test]
    fn test_calculate_uncovered_paths_full_coverage() {
        assert_eq!(
            CoverageRiskAnalyzer::calculate_uncovered_paths(10, 100.0),
            0
        );
    }

    #[test]
    fn test_get_role_criticality_multiplier_pure_logic() {
        assert_eq!(
            CoverageRiskAnalyzer::get_role_criticality_multiplier(FunctionRole::PureLogic),
            2.0
        );
    }

    #[test]
    fn test_get_role_criticality_multiplier_entry_point() {
        assert_eq!(
            CoverageRiskAnalyzer::get_role_criticality_multiplier(FunctionRole::EntryPoint),
            2.0
        );
    }

    #[test]
    fn test_get_role_criticality_multiplier_orchestrator() {
        assert_eq!(
            CoverageRiskAnalyzer::get_role_criticality_multiplier(FunctionRole::Orchestrator),
            1.0
        );
    }

    #[test]
    fn test_get_role_criticality_multiplier_io_wrapper() {
        assert_eq!(
            CoverageRiskAnalyzer::get_role_criticality_multiplier(FunctionRole::IOWrapper),
            0.5
        );
    }

    #[test]
    fn test_get_role_criticality_multiplier_unknown() {
        assert_eq!(
            CoverageRiskAnalyzer::get_role_criticality_multiplier(FunctionRole::Unknown),
            1.0
        );
    }

    #[test]
    fn test_count_uncovered_critical_paths_full_coverage() {
        let analyzer = CoverageRiskAnalyzer::new();
        let function = FunctionAnalysis {
            file: PathBuf::from("test.rs"),
            function: "test_func".to_string(),
            line: 1,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 5,
            nesting_depth: 2,
            is_test: false,
            visibility: FunctionVisibility::Private,
        };

        assert_eq!(
            analyzer.count_uncovered_critical_paths(&function, 100.0, FunctionRole::PureLogic),
            0
        );
    }

    #[test]
    fn test_count_uncovered_critical_paths_pure_logic() {
        let analyzer = CoverageRiskAnalyzer::new();
        let function = FunctionAnalysis {
            file: PathBuf::from("test.rs"),
            function: "test_func".to_string(),
            line: 1,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 5,
            nesting_depth: 2,
            is_test: false,
            visibility: FunctionVisibility::Private,
        };

        // 50% coverage means 5 uncovered paths, times 2.0 multiplier = 10
        assert_eq!(
            analyzer.count_uncovered_critical_paths(&function, 50.0, FunctionRole::PureLogic),
            10
        );
    }

    #[test]
    fn test_count_uncovered_critical_paths_io_wrapper() {
        let analyzer = CoverageRiskAnalyzer::new();
        let function = FunctionAnalysis {
            file: PathBuf::from("test.rs"),
            function: "test_func".to_string(),
            line: 1,
            function_length: 50,
            cyclomatic_complexity: 10,
            cognitive_complexity: 5,
            nesting_depth: 2,
            is_test: false,
            visibility: FunctionVisibility::Private,
        };

        // 50% coverage means 5 uncovered paths, times 0.5 multiplier = 2
        assert_eq!(
            analyzer.count_uncovered_critical_paths(&function, 50.0, FunctionRole::IOWrapper),
            2
        );
    }
}
