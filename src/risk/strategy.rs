use super::{Difficulty, FunctionRisk, RiskCategory, TestEffort};
use crate::core::ComplexityMetrics;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskWeights {
    pub coverage: f64,
    pub complexity: f64,
    pub cognitive: f64,
    pub debt: f64,
    pub untested_penalty: f64,
    pub debt_threshold_multiplier: f64,
}

impl Default for RiskWeights {
    fn default() -> Self {
        Self {
            coverage: 0.5,
            complexity: 0.3,
            cognitive: 0.45,
            debt: 0.2,
            untested_penalty: 2.0,
            debt_threshold_multiplier: 1.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskComponents {
    pub base: f64,
    pub debt_factor: f64,
    pub coverage_penalty: f64,
    pub breakdown: Vec<RiskFactor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskFactor {
    pub name: String,
    pub weight: f64,
    pub raw_value: f64,
    pub contribution: f64,
}

#[derive(Clone, Debug)]
pub struct RiskContext {
    pub file: PathBuf,
    pub function_name: String,
    pub line_range: (usize, usize),
    pub complexity: ComplexityMetrics,
    pub coverage: Option<f64>,
    pub debt_score: Option<f64>,
    pub debt_threshold: Option<f64>,
}

pub trait RiskCalculator: Send + Sync {
    fn box_clone(&self) -> Box<dyn RiskCalculator>;
    fn calculate(&self, context: &RiskContext) -> FunctionRisk;
    fn calculate_risk_score(&self, context: &RiskContext) -> f64;
    fn calculate_risk_reduction(
        &self,
        current_risk: f64,
        complexity: u32,
        target_coverage: f64,
    ) -> f64;
}

#[derive(Clone, Default)]
pub struct EnhancedRiskStrategy {
    pub weights: RiskWeights,
}

impl RiskCalculator for EnhancedRiskStrategy {
    fn box_clone(&self) -> Box<dyn RiskCalculator> {
        Box::new(self.clone())
    }

    fn calculate(&self, context: &RiskContext) -> FunctionRisk {
        let risk_score = self.calculate_risk_score(context);
        let test_effort = self.estimate_test_effort(&context.complexity);
        let risk_category = self.categorize_risk(&context.complexity, context.coverage, risk_score);

        FunctionRisk {
            file: context.file.clone(),
            function_name: context.function_name.clone(),
            line_range: context.line_range,
            cyclomatic_complexity: context.complexity.cyclomatic_complexity,
            cognitive_complexity: context.complexity.cognitive_complexity,
            coverage_percentage: context.coverage,
            risk_score,
            test_effort,
            risk_category,
        }
    }

    fn calculate_risk_score(&self, context: &RiskContext) -> f64 {
        let base_risk = self.calculate_base_risk(context);
        let debt_factor = self.calculate_debt_factor(context.debt_score, context.debt_threshold);
        let coverage_penalty = self.calculate_coverage_penalty(context.coverage);

        let final_risk = base_risk * debt_factor * coverage_penalty;
        final_risk.min(10.0)
    }

    fn calculate_risk_reduction(
        &self,
        current_risk: f64,
        _complexity: u32,
        target_coverage: f64,
    ) -> f64 {
        let coverage_factor = target_coverage / 100.0;
        let reduction_rate = if target_coverage >= 80.0 { 0.8 } else { 0.6 };
        current_risk * coverage_factor * reduction_rate
    }
}

impl EnhancedRiskStrategy {
    fn calculate_base_risk(&self, context: &RiskContext) -> f64 {
        let cyclomatic = context.complexity.cyclomatic_complexity as f64;
        let cognitive = context.complexity.cognitive_complexity as f64;

        let complexity_component =
            (cyclomatic * self.weights.complexity + cognitive * self.weights.cognitive) / 50.0;

        let coverage_component = match context.coverage {
            Some(cov) => (100.0 - cov) / 100.0 * self.weights.coverage,
            None => 0.0, // Don't add coverage weight when coverage is unknown
        };

        (complexity_component + coverage_component) * 5.0
    }

    fn calculate_debt_factor(&self, debt_score: Option<f64>, debt_threshold: Option<f64>) -> f64 {
        match (debt_score, debt_threshold) {
            (Some(score), Some(threshold)) if threshold > 0.0 => {
                let ratio = score / threshold;
                match ratio {
                    r if r <= 1.0 => 1.0,
                    r if r <= 2.0 => 1.2,
                    r if r <= 5.0 => 1.5,
                    r if r <= 10.0 => 2.0,
                    _ => 2.5,
                }
            }
            _ => 1.0,
        }
    }

    fn calculate_coverage_penalty(&self, coverage: Option<f64>) -> f64 {
        match coverage {
            None => 1.0, // No penalty when coverage is unknown (not untested, just unknown)
            Some(c) if c < 20.0 => 3.0,
            Some(c) if c < 40.0 => 2.0,
            Some(c) if c < 60.0 => 1.5,
            Some(c) if c < 80.0 => 1.2,
            Some(_) => 0.8,
        }
    }

    fn estimate_test_effort(&self, complexity: &ComplexityMetrics) -> TestEffort {
        TestEffort {
            estimated_difficulty: Self::classify_difficulty(complexity.cognitive_complexity),
            cognitive_load: complexity.cognitive_complexity,
            branch_count: complexity.cyclomatic_complexity,
            recommended_test_cases: Self::calculate_test_cases(complexity.cyclomatic_complexity),
        }
    }

    fn classify_difficulty(cognitive: u32) -> Difficulty {
        const THRESHOLDS: [(u32, Difficulty); 5] = [
            (4, Difficulty::Trivial),
            (10, Difficulty::Simple),
            (20, Difficulty::Moderate),
            (40, Difficulty::Complex),
            (u32::MAX, Difficulty::VeryComplex),
        ];

        THRESHOLDS
            .iter()
            .find(|(threshold, _)| cognitive <= *threshold)
            .map(|(_, difficulty)| difficulty.clone())
            .unwrap_or(Difficulty::VeryComplex)
    }

    fn calculate_test_cases(cyclomatic: u32) -> u32 {
        const MAPPINGS: [(u32, u32); 6] =
            [(3, 1), (7, 2), (10, 3), (15, 5), (20, 7), (u32::MAX, 10)];

        MAPPINGS
            .iter()
            .find(|(threshold, _)| cyclomatic <= *threshold)
            .map(|(_, cases)| *cases)
            .unwrap_or(10)
    }

    fn categorize_risk(
        &self,
        complexity: &ComplexityMetrics,
        coverage: Option<f64>,
        risk_score: f64,
    ) -> RiskCategory {
        let avg_complexity =
            (complexity.cyclomatic_complexity + complexity.cognitive_complexity) / 2;

        if let Some(cov) = coverage {
            if avg_complexity > 10 && cov > 80.0 {
                return RiskCategory::WellTested;
            }
        }

        // When coverage is unknown, also consider complexity for categorization
        if coverage.is_none() {
            // Use complexity-based categorization when coverage is unknown
            return match avg_complexity {
                c if c > 15 => RiskCategory::Critical,
                c if c > 10 => RiskCategory::High,
                c if c > 5 => RiskCategory::Medium,
                _ => RiskCategory::Low,
            };
        }

        // When coverage is known, use risk score based categorization
        match risk_score {
            r if r >= 8.0 => RiskCategory::Critical,
            r if r >= 6.0 => RiskCategory::High,
            r if r >= 4.0 => RiskCategory::Medium,
            _ => RiskCategory::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComplexityMetrics;
    use std::path::PathBuf;

    fn create_test_context(
        cyclomatic: u32,
        cognitive: u32,
        coverage: Option<f64>,
        debt_score: Option<f64>,
    ) -> RiskContext {
        RiskContext {
            file: PathBuf::from("test.rs"),
            function_name: "test_function".to_string(),
            line_range: (1, 100),
            complexity: ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: cyclomatic,
                cognitive_complexity: cognitive,
            },
            coverage,
            debt_score,
            debt_threshold: Some(100.0),
        }
    }

    #[test]
    fn test_enhanced_strategy_no_coverage_high_complexity() {
        let strategy = EnhancedRiskStrategy::default();
        let context = create_test_context(20, 25, None, None);

        let risk = strategy.calculate(&context);

        // With no coverage, risk score is based on complexity only
        // No penalty is applied, so risk score should be moderate
        assert!(
            risk.risk_score > 1.0,
            "High complexity should still have some risk even without coverage data (got {})",
            risk.risk_score
        );
        // Average complexity (20+25)/2 = 22.5, which is > 15, so Critical
        assert_eq!(risk.risk_category, RiskCategory::Critical);
    }

    #[test]
    fn test_enhanced_strategy_low_coverage_high_debt() {
        let strategy = EnhancedRiskStrategy::default();
        let context = create_test_context(15, 20, Some(20.0), Some(500.0));

        let risk = strategy.calculate(&context);

        assert!(
            risk.risk_score > 7.0,
            "Low coverage with high debt should have very high risk"
        );
        assert_eq!(risk.risk_category, RiskCategory::Critical);
    }

    #[test]
    fn test_enhanced_strategy_good_coverage_low_complexity() {
        let strategy = EnhancedRiskStrategy::default();
        let context = create_test_context(5, 5, Some(85.0), Some(50.0));

        let risk = strategy.calculate(&context);

        assert!(
            risk.risk_score < 2.0,
            "Good coverage with low complexity should have low risk"
        );
        assert_eq!(risk.risk_category, RiskCategory::Low);
    }

    #[test]
    fn test_enhanced_strategy_well_tested_complex() {
        let strategy = EnhancedRiskStrategy::default();
        let context = create_test_context(15, 15, Some(90.0), Some(50.0));

        let risk = strategy.calculate(&context);

        assert_eq!(risk.risk_category, RiskCategory::WellTested);
        assert!(
            risk.risk_score < 3.0,
            "Well-tested complex code should have reduced risk"
        );
    }

    #[test]
    fn test_coverage_penalty_calculations() {
        let strategy = EnhancedRiskStrategy::default();

        assert_eq!(strategy.calculate_coverage_penalty(None), 1.0);
        assert_eq!(strategy.calculate_coverage_penalty(Some(10.0)), 3.0);
        assert_eq!(strategy.calculate_coverage_penalty(Some(30.0)), 2.0);
        assert_eq!(strategy.calculate_coverage_penalty(Some(50.0)), 1.5);
        assert_eq!(strategy.calculate_coverage_penalty(Some(70.0)), 1.2);
        assert_eq!(strategy.calculate_coverage_penalty(Some(85.0)), 0.8);
    }

    #[test]
    fn test_debt_factor_calculations() {
        let strategy = EnhancedRiskStrategy::default();

        assert_eq!(strategy.calculate_debt_factor(Some(50.0), Some(100.0)), 1.0);
        assert_eq!(
            strategy.calculate_debt_factor(Some(150.0), Some(100.0)),
            1.2
        );
        assert_eq!(
            strategy.calculate_debt_factor(Some(400.0), Some(100.0)),
            1.5
        );
        assert_eq!(
            strategy.calculate_debt_factor(Some(900.0), Some(100.0)),
            2.0
        );
        assert_eq!(
            strategy.calculate_debt_factor(Some(1500.0), Some(100.0)),
            2.5
        );
    }

    #[test]
    fn test_risk_score_max_cap() {
        let strategy = EnhancedRiskStrategy::default();
        // Extreme values to test max cap
        let context = create_test_context(100, 100, None, Some(10000.0));

        let risk_score = strategy.calculate_risk_score(&context);

        assert!(risk_score <= 10.0, "Risk score should be capped at 10.0");
    }

    #[test]
    fn test_risk_reduction_calculation() {
        let enhanced = EnhancedRiskStrategy::default();

        let enhanced_reduction = enhanced.calculate_risk_reduction(8.0, 20, 80.0);

        assert!(
            enhanced_reduction < 8.0,
            "Enhanced should show significant reduction"
        );
    }

    #[test]
    fn test_test_effort_estimation() {
        let strategy = EnhancedRiskStrategy::default();
        let complexity = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 15,
            cognitive_complexity: 25,
        };

        let effort = strategy.estimate_test_effort(&complexity);

        assert_eq!(effort.cognitive_load, 25);
        assert_eq!(effort.branch_count, 15);
        assert_eq!(effort.recommended_test_cases, 5);
        assert!(matches!(effort.estimated_difficulty, Difficulty::Complex));
    }

    #[test]
    fn test_risk_categorization_thresholds() {
        let strategy = EnhancedRiskStrategy::default();

        // Test various risk score thresholds
        let cases = vec![
            (9.0, RiskCategory::Critical),
            (7.0, RiskCategory::High),
            (5.0, RiskCategory::Medium),
            (2.0, RiskCategory::Low),
        ];

        for (score, expected_category) in cases {
            let complexity = ComplexityMetrics {
                functions: vec![],
                cyclomatic_complexity: 10,
                cognitive_complexity: 10,
            };
            let category = strategy.categorize_risk(&complexity, Some(50.0), score);
            assert_eq!(
                category, expected_category,
                "Score {score} should map to {expected_category:?}"
            );
        }
    }
}
