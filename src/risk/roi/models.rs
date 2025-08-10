use super::{CascadeImpact, EffortEstimate, RiskReduction};

#[derive(Clone, Debug)]
pub struct ROI {
    pub value: f64,
    pub effort: EffortEstimate,
    pub direct_impact: RiskReduction,
    pub cascade_impact: CascadeImpact,
    pub confidence: f64,
    pub breakdown: ROIBreakdown,
}

#[derive(Clone, Debug)]
pub struct ROIBreakdown {
    pub components: Vec<ROIComponent>,
    pub formula: String,
    pub explanation: String,
    pub confidence_factors: Vec<ConfidenceFactor>,
}

#[derive(Clone, Debug)]
pub struct ROIComponent {
    pub name: String,
    pub value: f64,
    pub weight: f64,
    pub explanation: String,
}

#[derive(Clone, Debug)]
pub struct ConfidenceFactor {
    pub name: String,
    pub value: f64,
    pub reason: String,
}

impl ROI {
    pub fn effective_value(&self) -> f64 {
        self.value * self.confidence
    }

    pub fn is_high_priority(&self) -> bool {
        self.value > 2.0 && self.confidence > 0.7
    }

    pub fn total_impact(&self) -> f64 {
        self.direct_impact.percentage + self.cascade_impact.total_risk_reduction * 0.5
    }

    pub fn summary(&self) -> String {
        format!(
            "ROI: {:.2} | Risk Reduction: {:.1}% direct + {:.1}% cascade | Effort: {:.1}h | Confidence: {:.0}%",
            self.value,
            self.direct_impact.percentage,
            self.cascade_impact.total_risk_reduction,
            self.effort.hours,
            self.confidence * 100.0
        )
    }
}
