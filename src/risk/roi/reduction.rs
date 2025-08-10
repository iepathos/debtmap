use super::super::priority::TestTarget;
use super::super::RiskAnalyzer;

pub trait RiskReductionModel: Send + Sync {
    fn calculate(&self, target: &TestTarget) -> RiskReduction;
}

#[derive(Clone, Debug)]
pub struct RiskReduction {
    pub absolute: f64,
    pub percentage: f64,
    pub coverage_increase: f64,
    pub confidence: f64,
}

pub struct AdvancedRiskReductionModel {
    risk_analyzer: RiskAnalyzer,
}

impl AdvancedRiskReductionModel {
    pub fn new(risk_analyzer: RiskAnalyzer) -> Self {
        Self { risk_analyzer }
    }

    fn project_coverage_increase(&self, target: &TestTarget) -> f64 {
        if target.current_coverage == 0.0 {
            match target.complexity.cyclomatic_complexity {
                0..=5 => 80.0,
                6..=10 => 70.0,
                11..=20 => 60.0,
                _ => 50.0,
            }
        } else {
            let remaining = 100.0 - target.current_coverage;
            let achievable = match target.complexity.cognitive_complexity {
                0..=10 => 0.7,
                11..=20 => 0.5,
                21..=40 => 0.3,
                _ => 0.2,
            };
            remaining * achievable
        }
    }

    fn test_effectiveness(&self, coverage_delta: f64) -> f64 {
        let normalized_delta = coverage_delta / 100.0;

        match normalized_delta {
            d if d <= 0.2 => d * 2.0,
            d if d <= 0.5 => 0.4 + (d - 0.2) * 1.333,
            d if d <= 0.8 => 0.8 + (d - 0.5) * 0.667,
            d => 1.0 + (d - 0.8) * 0.5,
        }
    }

    fn risk_reduction_multiplier(&self, target: &TestTarget) -> f64 {
        let coverage_factor = if target.current_coverage == 0.0 {
            2.0
        } else if target.current_coverage < 20.0 {
            1.5
        } else if target.current_coverage < 50.0 {
            1.2
        } else {
            1.0
        };

        let complexity_factor = match target.complexity.cyclomatic_complexity {
            0..=5 => 0.8,
            6..=10 => 1.0,
            11..=20 => 1.2,
            21..=40 => 1.5,
            _ => 2.0,
        };

        let debt_factor = if target.debt_items > 0 {
            1.0 + (target.debt_items as f64 * 0.05).min(0.5)
        } else {
            1.0
        };

        coverage_factor * complexity_factor * debt_factor
    }

    fn calculate_confidence(&self, target: &TestTarget, coverage_delta: f64) -> f64 {
        let base_confidence = match target.complexity.cyclomatic_complexity {
            0..=5 => 0.95,
            6..=10 => 0.90,
            11..=20 => 0.85,
            21..=40 => 0.75,
            _ => 0.65,
        };

        let coverage_confidence = if target.current_coverage == 0.0 {
            0.95
        } else {
            0.85 - (target.current_coverage / 200.0).min(0.3)
        };

        let delta_confidence = match coverage_delta {
            d if d >= 50.0 => 0.90,
            d if d >= 30.0 => 0.85,
            d if d >= 20.0 => 0.80,
            d if d >= 10.0 => 0.75,
            _ => 0.70,
        };

        (base_confidence * coverage_confidence * delta_confidence).max(0.5)
    }
}

impl RiskReductionModel for AdvancedRiskReductionModel {
    fn calculate(&self, target: &TestTarget) -> RiskReduction {
        let current_risk = target.current_risk;
        let coverage_delta = self.project_coverage_increase(target);
        let new_coverage = (target.current_coverage + coverage_delta).min(100.0);

        let projected_risk = self.risk_analyzer.calculate_risk_score(
            target.complexity.cyclomatic_complexity,
            target.complexity.cognitive_complexity,
            Some(new_coverage),
        );

        let effectiveness = self.test_effectiveness(coverage_delta);
        let multiplier = self.risk_reduction_multiplier(target);

        let risk_reduction = (current_risk - projected_risk).max(0.0);
        let adjusted_reduction = risk_reduction * effectiveness * multiplier;

        let percentage = if current_risk > 0.0 {
            (adjusted_reduction / current_risk * 100.0).min(95.0)
        } else {
            0.0
        };

        RiskReduction {
            absolute: adjusted_reduction,
            percentage,
            coverage_increase: coverage_delta,
            confidence: self.calculate_confidence(target, coverage_delta),
        }
    }
}
