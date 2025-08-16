use super::{
    ComparisonResult, CouplingEvidence, CouplingIssue, CouplingMetrics, DesignPattern,
    RemediationAction, RiskEvidence, RiskFactor, RiskSeverity, RiskType,
};
use crate::priority::call_graph::CallGraph;
use crate::priority::FunctionAnalysis;
use crate::risk::evidence::{ModuleType, RiskContext};
use crate::risk::thresholds::{CouplingThresholds, StatisticalThresholdProvider};

pub struct CouplingRiskAnalyzer {
    #[allow(dead_code)]
    thresholds: CouplingThresholds,
    threshold_provider: StatisticalThresholdProvider,
}

impl Default for CouplingRiskAnalyzer {
    fn default() -> Self {
        Self {
            thresholds: CouplingThresholds::default(),
            threshold_provider: StatisticalThresholdProvider::new(),
        }
    }
}

impl CouplingRiskAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze(
        &self,
        function: &FunctionAnalysis,
        context: &RiskContext,
        call_graph: &CallGraph,
    ) -> RiskFactor {
        let func_id = crate::priority::call_graph::FunctionId {
            file: function.file.clone(),
            name: function.function.clone(),
            line: function.line,
        };

        let afferent_coupling = call_graph.get_callers(&func_id).len() as u32;
        let efferent_coupling = call_graph.get_callees(&func_id).len() as u32;
        let instability = self.calculate_instability(afferent_coupling, efferent_coupling);
        let circular_dependencies = self.detect_circular_dependencies(&func_id, call_graph);

        // Module-type adjusted thresholds
        let adjusted_thresholds = self.adjust_for_module_type(&context.module_type);

        let coupling_score = self.calculate_coupling_risk(
            afferent_coupling,
            efferent_coupling,
            instability,
            circular_dependencies,
            &adjusted_thresholds,
        );

        let comparison = self.compare_to_baseline(
            (afferent_coupling + efferent_coupling) / 2,
            &context.module_type,
        );

        let evidence = CouplingEvidence {
            afferent_coupling,
            efferent_coupling,
            instability,
            circular_dependencies,
            comparison_to_baseline: comparison,
        };

        let severity = self.classify_coupling_severity(coupling_score, &adjusted_thresholds);
        let coupling_issues = self.identify_coupling_issues(
            afferent_coupling,
            efferent_coupling,
            instability,
            circular_dependencies,
        );

        let remediation_actions = self.get_coupling_actions(
            afferent_coupling,
            efferent_coupling,
            instability,
            &coupling_issues,
            &severity,
        );

        RiskFactor {
            risk_type: RiskType::Coupling {
                afferent_coupling,
                efferent_coupling,
                instability,
                circular_dependencies,
            },
            score: coupling_score,
            severity,
            evidence: RiskEvidence::Coupling(evidence),
            remediation_actions,
            weight: self.get_weight_for_module_type(&context.module_type),
            confidence: self.calculate_confidence(afferent_coupling + efferent_coupling),
        }
    }

    fn calculate_instability(&self, afferent: u32, efferent: u32) -> f64 {
        if afferent + efferent == 0 {
            return 0.0;
        }
        efferent as f64 / (afferent + efferent) as f64
    }

    fn detect_circular_dependencies(
        &self,
        func_id: &crate::priority::call_graph::FunctionId,
        call_graph: &CallGraph,
    ) -> u32 {
        // Simple circular dependency detection
        let mut visited = std::collections::HashSet::new();
        let mut circular_count = 0;

        for callee in call_graph.get_callees(func_id) {
            if self.has_path_back(&callee, func_id, call_graph, &mut visited, 0, 5) {
                circular_count += 1;
            }
            visited.clear();
        }

        circular_count
    }

    #[allow(clippy::only_used_in_recursion)]
    fn has_path_back(
        &self,
        from: &crate::priority::call_graph::FunctionId,
        to: &crate::priority::call_graph::FunctionId,
        call_graph: &CallGraph,
        visited: &mut std::collections::HashSet<crate::priority::call_graph::FunctionId>,
        depth: u32,
        max_depth: u32,
    ) -> bool {
        // Check termination conditions
        if Self::should_terminate_search(from, to, visited, depth, max_depth) {
            return from == to;
        }

        // Recursively check all callees
        Self::check_callees_for_path(self, from, to, call_graph, visited, depth, max_depth)
    }

    /// Determines if the graph traversal should terminate
    fn should_terminate_search(
        from: &crate::priority::call_graph::FunctionId,
        to: &crate::priority::call_graph::FunctionId,
        visited: &mut std::collections::HashSet<crate::priority::call_graph::FunctionId>,
        depth: u32,
        max_depth: u32,
    ) -> bool {
        depth > max_depth || from == to || !visited.insert(from.clone())
    }

    /// Recursively checks all callees for a path to the target
    fn check_callees_for_path(
        analyzer: &CouplingRiskAnalyzer,
        from: &crate::priority::call_graph::FunctionId,
        to: &crate::priority::call_graph::FunctionId,
        call_graph: &CallGraph,
        visited: &mut std::collections::HashSet<crate::priority::call_graph::FunctionId>,
        depth: u32,
        max_depth: u32,
    ) -> bool {
        for callee in call_graph.get_callees(from) {
            if analyzer.has_path_back(&callee, to, call_graph, visited, depth + 1, max_depth) {
                return true;
            }
        }
        false
    }

    fn adjust_for_module_type(&self, module_type: &ModuleType) -> CouplingThresholds {
        let base_thresholds = self.threshold_provider.get_coupling_thresholds(module_type);

        match module_type {
            ModuleType::Util => CouplingThresholds {
                // Utilities should have low coupling
                low: base_thresholds.low,
                moderate: base_thresholds.moderate,
                high: base_thresholds.high,
                critical: base_thresholds.critical,
            },
            ModuleType::Core => CouplingThresholds {
                // Core modules can have higher coupling
                low: base_thresholds.low * 2,
                moderate: base_thresholds.moderate * 2,
                high: base_thresholds.high * 2,
                critical: base_thresholds.critical * 2,
            },
            ModuleType::Api => CouplingThresholds {
                // API modules have moderate coupling tolerance
                low: (base_thresholds.low as f64 * 1.5) as u32,
                moderate: (base_thresholds.moderate as f64 * 1.5) as u32,
                high: (base_thresholds.high as f64 * 1.5) as u32,
                critical: (base_thresholds.critical as f64 * 1.5) as u32,
            },
            ModuleType::Test => CouplingThresholds {
                // Test modules can have high coupling
                low: base_thresholds.low * 3,
                moderate: base_thresholds.moderate * 3,
                high: base_thresholds.high * 3,
                critical: base_thresholds.critical * 3,
            },
            ModuleType::Infrastructure => base_thresholds,
        }
    }

    fn calculate_coupling_risk(
        &self,
        afferent: u32,
        efferent: u32,
        instability: f64,
        circular: u32,
        thresholds: &CouplingThresholds,
    ) -> f64 {
        let total_coupling = afferent + efferent;
        let coupling_score = self.score_coupling(total_coupling, thresholds);
        let instability_score = self.score_instability(instability);
        let circular_score = self.score_circular(circular);

        // Weighted average: coupling 50%, instability 30%, circular 20%
        coupling_score * 0.5 + instability_score * 0.3 + circular_score * 0.2
    }

    fn score_coupling(&self, coupling: u32, thresholds: &CouplingThresholds) -> f64 {
        Self::classify_coupling_risk(coupling, thresholds)
    }

    /// Pure function to classify coupling into risk score based on thresholds
    fn classify_coupling_risk(coupling: u32, thresholds: &CouplingThresholds) -> f64 {
        let (base_score, range_score, lower_bound, upper_bound) =
            Self::determine_coupling_tier(coupling, thresholds);
        Self::calculate_tier_score(coupling, base_score, range_score, lower_bound, upper_bound)
    }

    /// Determine which coupling tier the value falls into
    fn determine_coupling_tier(
        coupling: u32,
        thresholds: &CouplingThresholds,
    ) -> (f64, f64, u32, u32) {
        match coupling {
            c if c <= thresholds.low => (0.0, 2.5, 0, thresholds.low),
            c if c <= thresholds.moderate => (2.5, 2.5, thresholds.low, thresholds.moderate),
            c if c <= thresholds.high => (5.0, 2.5, thresholds.moderate, thresholds.high),
            c if c <= thresholds.critical => (7.5, 2.0, thresholds.high, thresholds.critical),
            _ => Self::calculate_overflow_score(coupling, thresholds.critical),
        }
    }

    /// Calculate the score within a tier
    fn calculate_tier_score(
        value: u32,
        base_score: f64,
        range_score: f64,
        lower_bound: u32,
        upper_bound: u32,
    ) -> f64 {
        if upper_bound == lower_bound {
            base_score
        } else {
            let progress = (value - lower_bound) as f64 / (upper_bound - lower_bound) as f64;
            base_score + progress * range_score
        }
    }

    /// Calculate score for values exceeding the critical threshold
    fn calculate_overflow_score(coupling: u32, critical_threshold: u32) -> (f64, f64, u32, u32) {
        let overflow_ratio = (coupling - critical_threshold) as f64 / critical_threshold as f64;
        let overflow_score = (overflow_ratio * 0.5).min(0.5);
        (9.5 + overflow_score, 0.0, 0, 0) // Return computed score directly
    }

    fn score_instability(&self, instability: f64) -> f64 {
        // Ideal instability is around 0.5 (balanced)
        // Too stable (0.0) or too unstable (1.0) are both risky
        let deviation = (instability - 0.5).abs();
        deviation * 20.0 // Scale to 0-10
    }

    fn score_circular(&self, circular: u32) -> f64 {
        match circular {
            0 => 0.0,
            1 => 3.0,
            2 => 6.0,
            3 => 8.0,
            _ => 10.0,
        }
    }

    fn classify_coupling_severity(
        &self,
        score: f64,
        _thresholds: &CouplingThresholds,
    ) -> RiskSeverity {
        match score {
            s if s <= 2.0 => RiskSeverity::None,
            s if s <= 4.0 => RiskSeverity::Low,
            s if s <= 6.0 => RiskSeverity::Moderate,
            s if s <= 8.0 => RiskSeverity::High,
            _ => RiskSeverity::Critical,
        }
    }

    fn compare_to_baseline(&self, avg_coupling: u32, module_type: &ModuleType) -> ComparisonResult {
        let baseline = self.threshold_provider.get_coupling_thresholds(module_type);
        Self::classify_coupling_level(avg_coupling, &baseline)
    }

    /// Classify coupling level based on thresholds
    fn classify_coupling_level(value: u32, thresholds: &CouplingThresholds) -> ComparisonResult {
        match value {
            v if v <= thresholds.low => ComparisonResult::BelowMedian,
            v if v <= thresholds.moderate => ComparisonResult::AboveMedian,
            v if v <= thresholds.high => ComparisonResult::AboveP75,
            v if v <= thresholds.critical => ComparisonResult::AboveP90,
            _ => ComparisonResult::AboveP95,
        }
    }

    fn identify_coupling_issues(
        &self,
        afferent: u32,
        efferent: u32,
        instability: f64,
        circular: u32,
    ) -> Vec<CouplingIssue> {
        let mut issues = Vec::new();

        if circular > 0 {
            issues.push(CouplingIssue::CircularDependency(format!(
                "{circular} circular dependencies detected"
            )));
        }

        if !(0.2..=0.8).contains(&instability) {
            issues.push(CouplingIssue::HighInstability);
        }

        if afferent + efferent > 20 {
            issues.push(CouplingIssue::TooManyDependencies);
        }

        if afferent > 15 {
            issues.push(CouplingIssue::GodClass);
        }

        issues
    }

    fn get_coupling_actions(
        &self,
        afferent: u32,
        efferent: u32,
        instability: f64,
        issues: &[CouplingIssue],
        severity: &RiskSeverity,
    ) -> Vec<RemediationAction> {
        match severity {
            RiskSeverity::None | RiskSeverity::Low => vec![],
            RiskSeverity::Moderate | RiskSeverity::High | RiskSeverity::Critical => {
                let metrics = Self::create_coupling_metrics(afferent, efferent, instability);
                let patterns = Self::get_suggested_patterns(severity);
                let effort = Self::get_estimated_effort(severity);

                let mut actions = vec![RemediationAction::ReduceCoupling {
                    current_coupling: metrics,
                    coupling_issues: issues.to_vec(),
                    suggested_patterns: patterns,
                    estimated_effort_hours: effort,
                }];

                if matches!(severity, RiskSeverity::Critical) {
                    actions.push(RemediationAction::ExtractLogic {
                        extraction_candidates: vec![],
                        pure_function_opportunities: efferent / 3,
                        testability_improvement: 0.4,
                    });
                }

                actions
            }
        }
    }

    fn create_coupling_metrics(afferent: u32, efferent: u32, instability: f64) -> CouplingMetrics {
        CouplingMetrics {
            afferent,
            efferent,
            instability,
        }
    }

    fn get_suggested_patterns(severity: &RiskSeverity) -> Vec<DesignPattern> {
        match severity {
            RiskSeverity::Moderate => vec![DesignPattern::DependencyInjection],
            RiskSeverity::High => vec![
                DesignPattern::DependencyInjection,
                DesignPattern::FacadePattern,
                DesignPattern::AdapterPattern,
            ],
            RiskSeverity::Critical => vec![
                DesignPattern::DependencyInjection,
                DesignPattern::StrategyPattern,
                DesignPattern::ObserverPattern,
                DesignPattern::FacadePattern,
                DesignPattern::AdapterPattern,
            ],
            _ => vec![],
        }
    }

    fn get_estimated_effort(severity: &RiskSeverity) -> u32 {
        match severity {
            RiskSeverity::Moderate => 2,
            RiskSeverity::High => 4,
            RiskSeverity::Critical => 8,
            _ => 0,
        }
    }

    fn get_weight_for_module_type(&self, module_type: &ModuleType) -> f64 {
        match module_type {
            ModuleType::Core => 1.0,           // Full weight for core modules
            ModuleType::Api => 0.9,            // High weight for API modules
            ModuleType::Infrastructure => 0.7, // Moderate weight for infrastructure
            ModuleType::Util => 0.8,           // Good weight for utilities
            ModuleType::Test => 0.3,           // Low weight for test modules
        }
    }

    /// Classify coupling confidence level based on total connections
    fn classify_confidence_level(total_coupling: u32) -> f64 {
        match total_coupling {
            0 => 0.5,       // Low confidence for isolated functions
            1..=4 => 0.7,   // Moderate confidence for lightly coupled
            5..=14 => 0.85, // High confidence for moderately coupled
            _ => 0.95,      // Very high confidence for highly coupled
        }
    }

    fn calculate_confidence(&self, total_coupling: u32) -> f64 {
        Self::classify_confidence_level(total_coupling)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_coupling_level_below_median() {
        let thresholds = CouplingThresholds {
            low: 5,
            moderate: 10,
            high: 15,
            critical: 20,
        };

        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(3, &thresholds),
            ComparisonResult::BelowMedian
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(5, &thresholds),
            ComparisonResult::BelowMedian
        );
    }

    #[test]
    fn test_classify_coupling_level_above_median() {
        let thresholds = CouplingThresholds {
            low: 5,
            moderate: 10,
            high: 15,
            critical: 20,
        };

        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(6, &thresholds),
            ComparisonResult::AboveMedian
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(10, &thresholds),
            ComparisonResult::AboveMedian
        );
    }

    #[test]
    fn test_classify_coupling_level_above_p75() {
        let thresholds = CouplingThresholds {
            low: 5,
            moderate: 10,
            high: 15,
            critical: 20,
        };

        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(11, &thresholds),
            ComparisonResult::AboveP75
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(15, &thresholds),
            ComparisonResult::AboveP75
        );
    }

    #[test]
    fn test_classify_coupling_level_above_p90() {
        let thresholds = CouplingThresholds {
            low: 5,
            moderate: 10,
            high: 15,
            critical: 20,
        };

        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(16, &thresholds),
            ComparisonResult::AboveP90
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(20, &thresholds),
            ComparisonResult::AboveP90
        );
    }

    #[test]
    fn test_classify_coupling_level_above_p95() {
        let thresholds = CouplingThresholds {
            low: 5,
            moderate: 10,
            high: 15,
            critical: 20,
        };

        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(21, &thresholds),
            ComparisonResult::AboveP95
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(100, &thresholds),
            ComparisonResult::AboveP95
        );
    }

    #[test]
    fn test_classify_coupling_level_edge_cases() {
        let thresholds = CouplingThresholds {
            low: 5,
            moderate: 10,
            high: 15,
            critical: 20,
        };

        // Test zero value
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(0, &thresholds),
            ComparisonResult::BelowMedian
        );

        // Test exact boundary values
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(5, &thresholds),
            ComparisonResult::BelowMedian
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(10, &thresholds),
            ComparisonResult::AboveMedian
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(15, &thresholds),
            ComparisonResult::AboveP75
        );
        assert_eq!(
            CouplingRiskAnalyzer::classify_coupling_level(20, &thresholds),
            ComparisonResult::AboveP90
        );
    }
}
