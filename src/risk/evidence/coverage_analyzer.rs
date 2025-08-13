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
            weight: self.get_weight_for_role(&context.role),
            confidence: self.calculate_confidence(coverage_percentage),
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

        let base_paths = function.cyclomatic_complexity;
        let uncovered_ratio = 1.0 - (coverage_percentage / 100.0);
        let uncovered_paths = (base_paths as f64 * uncovered_ratio) as u32;

        // Adjust for role - some roles have more critical paths
        match role {
            FunctionRole::PureLogic => uncovered_paths * 2, // All paths critical in business logic
            FunctionRole::EntryPoint => uncovered_paths * 2, // Entry points are critical
            FunctionRole::Orchestrator => uncovered_paths,  // Normal criticality
            FunctionRole::IOWrapper => uncovered_paths / 2, // Less critical paths
            FunctionRole::Unknown => uncovered_paths,
        }
    }

    fn assess_test_quality(&self, coverage: f64, complexity: u32) -> TestQuality {
        if coverage >= 90.0 && complexity <= 5 {
            TestQuality::Excellent
        } else if coverage >= 80.0 {
            TestQuality::Good
        } else if coverage >= 60.0 {
            TestQuality::Adequate
        } else if coverage > 0.0 {
            TestQuality::Poor
        } else {
            TestQuality::Missing
        }
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
        if coverage >= thresholds.excellent {
            10.0 - (coverage - thresholds.excellent) / (100.0 - thresholds.excellent) * 2.0
        // 0-2 for excellent
        } else if coverage >= thresholds.good {
            7.5 - (coverage - thresholds.good) / (thresholds.excellent - thresholds.good) * 2.5
        // 2-4.5 for good
        } else if coverage >= thresholds.moderate {
            5.0 - (coverage - thresholds.moderate) / (thresholds.good - thresholds.moderate) * 2.5
        // 4.5-7 for moderate
        } else if coverage >= thresholds.poor {
            2.5 - (coverage - thresholds.poor) / (thresholds.moderate - thresholds.poor) * 2.0
        // 7-9 for poor
        } else if coverage > 0.0 {
            0.5 - coverage / thresholds.poor * 1.0 // 9-10 for critical
        } else {
            10.0 // Maximum risk for zero coverage
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

        if coverage >= baseline.excellent {
            ComparisonResult::BelowMedian // Better than median (inverted for coverage)
        } else if coverage >= baseline.good {
            ComparisonResult::AboveMedian
        } else if coverage >= baseline.moderate {
            ComparisonResult::AboveP75
        } else if coverage >= baseline.poor {
            ComparisonResult::AboveP90
        } else {
            ComparisonResult::AboveP95
        }
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

    fn get_weight_for_role(&self, role: &FunctionRole) -> f64 {
        match role {
            FunctionRole::PureLogic => 1.0,    // Full weight for business logic
            FunctionRole::EntryPoint => 0.9,   // High weight for entry points
            FunctionRole::Orchestrator => 0.6, // Moderate weight for orchestration
            FunctionRole::IOWrapper => 0.4,    // Lower weight for I/O
            FunctionRole::Unknown => 0.8,      // Default weight
        }
    }

    fn calculate_confidence(&self, coverage: f64) -> f64 {
        if coverage == 0.0 {
            0.9 // High confidence that uncovered code is risky
        } else if coverage < 50.0 {
            0.85
        } else if coverage < 80.0 {
            0.8
        } else {
            0.95 // Very high confidence for well-covered code
        }
    }
}
