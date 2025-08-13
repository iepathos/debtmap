use super::{
    ComparisonResult, ComplexityEvidence, ComplexityThreshold, RefactoringTechnique,
    RemediationAction, RiskEvidence, RiskFactor, RiskSeverity, RiskType,
};
use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::FunctionAnalysis;
use crate::risk::evidence::RiskContext;
use crate::risk::thresholds::{ComplexityThresholds, StatisticalThresholdProvider};

pub struct ComplexityRiskAnalyzer {
    thresholds: ComplexityThresholds,
    threshold_provider: StatisticalThresholdProvider,
}

impl Default for ComplexityRiskAnalyzer {
    fn default() -> Self {
        Self {
            thresholds: ComplexityThresholds::default(),
            threshold_provider: StatisticalThresholdProvider::new(),
        }
    }
}

impl ComplexityRiskAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze(&self, function: &FunctionAnalysis, context: &RiskContext) -> RiskFactor {
        let cyclomatic = function.cyclomatic_complexity;
        let cognitive = function.cognitive_complexity;
        let lines = function.function_length as u32;

        // Role-adjusted complexity thresholds
        let adjusted_thresholds = self.adjust_for_role(&context.role);

        let complexity_score =
            self.calculate_complexity_risk(cyclomatic, cognitive, lines, &adjusted_thresholds);

        let comparison = self.compare_to_baseline(cyclomatic, cognitive, &context.role);
        let threshold_type = self.classify_threshold(complexity_score, &adjusted_thresholds);

        let evidence = ComplexityEvidence {
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            lines_of_code: lines,
            nesting_depth: function.nesting_depth,
            threshold_exceeded: complexity_score > adjusted_thresholds.moderate,
            role_adjusted: context.role != FunctionRole::PureLogic,
            comparison_to_baseline: comparison,
        };

        let severity = self.classify_complexity_severity(complexity_score, &adjusted_thresholds);
        let remediation_actions =
            self.get_complexity_actions(complexity_score, cyclomatic, cognitive, &severity);

        RiskFactor {
            risk_type: RiskType::Complexity {
                cyclomatic,
                cognitive,
                lines,
                threshold_type,
            },
            score: complexity_score,
            severity,
            evidence: RiskEvidence::Complexity(evidence),
            remediation_actions,
            weight: self.get_weight_for_role(&context.role),
            confidence: self.calculate_confidence(cyclomatic, cognitive),
        }
    }

    fn adjust_for_role(&self, role: &FunctionRole) -> ComplexityThresholds {
        let base_thresholds = self.threshold_provider.get_complexity_thresholds(role);

        match role {
            FunctionRole::PureLogic => base_thresholds, // Strict thresholds
            FunctionRole::Orchestrator => ComplexityThresholds {
                low: base_thresholds.low * 1.5,
                moderate: base_thresholds.moderate * 1.5,
                high: base_thresholds.high * 1.5,
                critical: base_thresholds.critical * 1.5,
            },
            FunctionRole::IOWrapper => ComplexityThresholds {
                low: base_thresholds.low * 2.0, // Very lenient for I/O
                moderate: base_thresholds.moderate * 2.0,
                high: base_thresholds.high * 2.0,
                critical: base_thresholds.critical * 2.0,
            },
            FunctionRole::EntryPoint => ComplexityThresholds {
                low: base_thresholds.low * 1.2,
                moderate: base_thresholds.moderate * 1.2,
                high: base_thresholds.high * 1.2,
                critical: base_thresholds.critical * 1.2,
            },
            FunctionRole::Unknown => base_thresholds,
        }
    }

    fn calculate_complexity_risk(
        &self,
        cyclomatic: u32,
        cognitive: u32,
        lines: u32,
        thresholds: &ComplexityThresholds,
    ) -> f64 {
        let cyclo_score = self.score_metric(cyclomatic as f64, thresholds);
        let cog_score = self.score_metric(cognitive as f64, thresholds);
        let lines_score = self.score_lines(lines);

        // Weighted average: cyclomatic 40%, cognitive 45%, lines 15%
        cyclo_score * 0.4 + cog_score * 0.45 + lines_score * 0.15
    }

    fn score_metric(&self, value: f64, thresholds: &ComplexityThresholds) -> f64 {
        if value <= thresholds.low {
            value / thresholds.low * 2.5 // 0-2.5 for low complexity
        } else if value <= thresholds.moderate {
            2.5 + (value - thresholds.low) / (thresholds.moderate - thresholds.low) * 2.5
        // 2.5-5.0
        } else if value <= thresholds.high {
            5.0 + (value - thresholds.moderate) / (thresholds.high - thresholds.moderate) * 2.5
        // 5.0-7.5
        } else if value <= thresholds.critical {
            7.5 + (value - thresholds.high) / (thresholds.critical - thresholds.high) * 2.0
        // 7.5-9.5
        } else {
            9.5 + ((value - thresholds.critical) / thresholds.critical * 0.5).min(0.5)
            // 9.5-10.0
        }
    }

    fn score_lines(&self, lines: u32) -> f64 {
        // Score based on lines of code
        match lines {
            0..=20 => lines as f64 / 20.0 * 2.5,                   // 0-2.5
            21..=50 => 2.5 + (lines - 20) as f64 / 30.0 * 2.5,     // 2.5-5.0
            51..=100 => 5.0 + (lines - 50) as f64 / 50.0 * 2.5,    // 5.0-7.5
            101..=200 => 7.5 + (lines - 100) as f64 / 100.0 * 2.0, // 7.5-9.5
            _ => 9.5 + ((lines - 200) as f64 / 200.0 * 0.5).min(0.5), // 9.5-10.0
        }
    }

    fn classify_complexity_severity(
        &self,
        score: f64,
        thresholds: &ComplexityThresholds,
    ) -> RiskSeverity {
        if score <= thresholds.low / 2.0 {
            RiskSeverity::None
        } else if score <= thresholds.low {
            RiskSeverity::Low
        } else if score <= thresholds.moderate {
            RiskSeverity::Moderate
        } else if score <= thresholds.high {
            RiskSeverity::High
        } else {
            RiskSeverity::Critical
        }
    }

    fn classify_threshold(
        &self,
        score: f64,
        thresholds: &ComplexityThresholds,
    ) -> ComplexityThreshold {
        if score <= thresholds.low {
            ComplexityThreshold::Low
        } else if score <= thresholds.moderate {
            ComplexityThreshold::Moderate
        } else if score <= thresholds.high {
            ComplexityThreshold::High
        } else {
            ComplexityThreshold::Critical
        }
    }

    fn compare_to_baseline(
        &self,
        cyclomatic: u32,
        cognitive: u32,
        role: &FunctionRole,
    ) -> ComparisonResult {
        let baseline = self.threshold_provider.get_complexity_thresholds(role);
        let avg_complexity = (cyclomatic + cognitive) as f64 / 2.0;

        if avg_complexity <= baseline.low {
            ComparisonResult::BelowMedian
        } else if avg_complexity <= baseline.moderate {
            ComparisonResult::AboveMedian
        } else if avg_complexity <= baseline.high {
            ComparisonResult::AboveP75
        } else if avg_complexity <= baseline.critical {
            ComparisonResult::AboveP90
        } else {
            ComparisonResult::AboveP95
        }
    }

    fn get_complexity_actions(
        &self,
        score: f64,
        cyclomatic: u32,
        cognitive: u32,
        severity: &RiskSeverity,
    ) -> Vec<RemediationAction> {
        match severity {
            RiskSeverity::None | RiskSeverity::Low => vec![],
            RiskSeverity::Moderate => vec![RemediationAction::RefactorComplexity {
                current_complexity: cyclomatic,
                target_complexity: 10,
                suggested_techniques: vec![
                    RefactoringTechnique::ExtractMethod,
                    RefactoringTechnique::ReduceNesting,
                ],
                estimated_effort_hours: 2,
                expected_risk_reduction: score * 0.3,
            }],
            RiskSeverity::High => vec![
                RemediationAction::RefactorComplexity {
                    current_complexity: cyclomatic,
                    target_complexity: 7,
                    suggested_techniques: vec![
                        RefactoringTechnique::ExtractMethod,
                        RefactoringTechnique::ReduceNesting,
                        RefactoringTechnique::EliminateElseAfterReturn,
                        RefactoringTechnique::ReplaceConditionalWithPolymorphism,
                    ],
                    estimated_effort_hours: 4,
                    expected_risk_reduction: score * 0.5,
                },
                RemediationAction::ExtractLogic {
                    extraction_candidates: self
                        .identify_extraction_candidates(cyclomatic, cognitive),
                    pure_function_opportunities: (cyclomatic / 5).max(1),
                    testability_improvement: 0.4,
                },
            ],
            RiskSeverity::Critical => vec![
                RemediationAction::RefactorComplexity {
                    current_complexity: cyclomatic,
                    target_complexity: 5,
                    suggested_techniques: vec![
                        RefactoringTechnique::ExtractMethod,
                        RefactoringTechnique::ExtractClass,
                        RefactoringTechnique::IntroduceParameterObject,
                        RefactoringTechnique::ReplaceConditionalWithPolymorphism,
                    ],
                    estimated_effort_hours: 8,
                    expected_risk_reduction: score * 0.7,
                },
                RemediationAction::ExtractLogic {
                    extraction_candidates: self
                        .identify_extraction_candidates(cyclomatic, cognitive),
                    pure_function_opportunities: (cyclomatic / 3).max(2),
                    testability_improvement: 0.6,
                },
            ],
        }
    }

    fn identify_extraction_candidates(
        &self,
        cyclomatic: u32,
        cognitive: u32,
    ) -> Vec<super::ExtractionCandidate> {
        let num_candidates = ((cyclomatic + cognitive) / 10).max(1).min(5);

        (0..num_candidates)
            .map(|i| super::ExtractionCandidate {
                start_line: i as usize * 10 + 1,
                end_line: (i + 1) as usize * 10,
                description: format!("Extract logical block {}", i + 1),
                complexity_reduction: (cyclomatic / num_candidates).max(1),
            })
            .collect()
    }

    fn get_weight_for_role(&self, role: &FunctionRole) -> f64 {
        match role {
            FunctionRole::PureLogic => 1.0,    // Full weight for business logic
            FunctionRole::Orchestrator => 0.7, // Reduced weight for orchestration
            FunctionRole::IOWrapper => 0.5,    // Lower weight for I/O
            FunctionRole::EntryPoint => 0.8,   // Moderate weight for entry points
            FunctionRole::Unknown => 0.9,      // Default weight
        }
    }

    fn calculate_confidence(&self, cyclomatic: u32, cognitive: u32) -> f64 {
        // Higher confidence for more complex functions (more data points)
        let complexity_points = cyclomatic + cognitive;

        if complexity_points < 5 {
            0.6 // Low confidence for very simple functions
        } else if complexity_points < 15 {
            0.8 // Moderate confidence
        } else if complexity_points < 30 {
            0.9 // High confidence
        } else {
            0.95 // Very high confidence for complex functions
        }
    }
}
