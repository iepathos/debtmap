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

impl AdvancedRiskReductionModel {
    // Pure function to calculate base reduction for untested code
    fn reduction_for_untested(coverage_delta: f64) -> f64 {
        match coverage_delta {
            d if d >= 60.0 => 60.0,
            d if d >= 40.0 => 50.0,
            d if d >= 20.0 => 40.0,
            _ => 30.0,
        }
    }

    // Pure function to calculate base reduction for low coverage code
    fn reduction_for_low_coverage(coverage_delta: f64) -> f64 {
        match coverage_delta {
            d if d >= 40.0 => 45.0,
            d if d >= 20.0 => 35.0,
            _ => 25.0,
        }
    }

    // Pure function to calculate base reduction for moderate coverage code
    fn reduction_for_moderate_coverage(coverage_delta: f64) -> f64 {
        match coverage_delta {
            d if d >= 30.0 => 25.0,
            d if d >= 15.0 => 20.0,
            _ => 15.0,
        }
    }

    // Pure function to calculate base reduction for high coverage code
    fn reduction_for_high_coverage(current_coverage: f64, coverage_delta: f64) -> f64 {
        let remaining_gap = 100.0 - current_coverage;
        let coverage_ratio = coverage_delta / remaining_gap.max(1.0);
        coverage_ratio * 15.0
    }

    // Pure function to get base reduction percentage
    fn get_base_reduction_percentage(current_coverage: f64, coverage_delta: f64) -> f64 {
        match current_coverage {
            0.0 => Self::reduction_for_untested(coverage_delta),
            c if c < 20.0 => Self::reduction_for_low_coverage(coverage_delta),
            c if c < 50.0 => Self::reduction_for_moderate_coverage(coverage_delta),
            c => Self::reduction_for_high_coverage(c, coverage_delta),
        }
    }

    // Pure function to get complexity multiplier
    fn get_complexity_multiplier(cyclomatic_complexity: u32) -> f64 {
        match cyclomatic_complexity {
            0..=5 => 0.9,
            6..=10 => 1.0,
            11..=20 => 1.2,
            _ => 1.4,
        }
    }
}

impl RiskReductionModel for AdvancedRiskReductionModel {
    fn calculate(&self, target: &TestTarget) -> RiskReduction {
        let coverage_delta = self.project_coverage_increase(target);

        // Use functional composition to calculate the reduction
        let base_reduction_percentage =
            Self::get_base_reduction_percentage(target.current_coverage, coverage_delta);

        let complexity_multiplier =
            Self::get_complexity_multiplier(target.complexity.cyclomatic_complexity);

        let percentage = (base_reduction_percentage * complexity_multiplier).min(85.0);
        let absolute = target.current_risk * (percentage / 100.0);

        RiskReduction {
            absolute,
            percentage,
            coverage_increase: coverage_delta,
            confidence: self.calculate_confidence(target, coverage_delta),
        }
    }
}
