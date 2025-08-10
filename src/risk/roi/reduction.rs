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
    _risk_analyzer: RiskAnalyzer,
}

impl AdvancedRiskReductionModel {
    pub fn new(risk_analyzer: RiskAnalyzer) -> Self {
        Self {
            _risk_analyzer: risk_analyzer,
        }
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

        (base_confidence * coverage_confidence * delta_confidence).max(0.5_f64)
    }
}

impl RiskReductionModel for AdvancedRiskReductionModel {
    fn calculate(&self, target: &TestTarget) -> RiskReduction {
        let current_risk = target.current_risk;
        let coverage_delta = self.project_coverage_increase(target);
        let _new_coverage = (target.current_coverage + coverage_delta).min(100.0);

        // Calculate risk reduction directly based on coverage improvement
        // For untested code (0% coverage), testing can reduce risk by 50-80%
        // For partially tested code, diminishing returns apply
        let base_reduction_percentage = if target.current_coverage == 0.0 {
            // First tests on untested code have highest impact
            match coverage_delta {
                d if d >= 60.0 => 60.0, // High coverage achieved
                d if d >= 40.0 => 50.0, // Moderate coverage achieved
                d if d >= 20.0 => 40.0, // Basic coverage achieved
                _ => 30.0,              // Minimal coverage
            }
        } else if target.current_coverage < 20.0 {
            // Low coverage - still high returns
            match coverage_delta {
                d if d >= 40.0 => 45.0,
                d if d >= 20.0 => 35.0,
                _ => 25.0,
            }
        } else if target.current_coverage < 50.0 {
            // Moderate coverage - moderate returns
            match coverage_delta {
                d if d >= 30.0 => 25.0,
                d if d >= 15.0 => 20.0,
                _ => 15.0,
            }
        } else {
            // High coverage - diminishing returns
            let remaining_gap = 100.0 - target.current_coverage;
            let coverage_ratio = coverage_delta / remaining_gap.max(1.0_f64);
            coverage_ratio * 15.0 // Max 15% additional reduction for well-tested code
        };

        // Apply complexity multiplier - more complex code benefits more from testing
        let complexity_multiplier = match target.complexity.cyclomatic_complexity {
            0..=5 => 0.9,   // Simple code - less benefit
            6..=10 => 1.0,  // Moderate complexity - normal benefit
            11..=20 => 1.2, // Complex code - more benefit
            _ => 1.4,       // Very complex - highest benefit
        };

        let percentage = (base_reduction_percentage * complexity_multiplier).min(85.0);
        let absolute = current_risk * (percentage / 100.0);

        RiskReduction {
            absolute,
            percentage,
            coverage_increase: coverage_delta,
            confidence: self.calculate_confidence(target, coverage_delta),
        }
    }
}
