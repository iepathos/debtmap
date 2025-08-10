use super::super::priority::TestTarget;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ROILearningSystem {
    history: Vec<ROIOutcome>,
    adjustment_factors: HashMap<String, f64>,
    confidence_threshold: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ROIOutcome {
    pub prediction: ROIPrediction,
    pub actual: ROIActual,
    pub timestamp: DateTime<Utc>,
    pub context: OutcomeContext,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ROIPrediction {
    pub effort: f64,
    pub risk_reduction: f64,
    pub roi: f64,
    pub target_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ROIActual {
    pub effort: f64,
    pub risk_reduction: f64,
    pub test_cases_written: usize,
    pub coverage_achieved: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutcomeContext {
    pub module_type: String,
    pub complexity_level: String,
    pub initial_coverage: f64,
    pub dependencies_count: usize,
}

impl Default for ROILearningSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ROILearningSystem {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            adjustment_factors: HashMap::new(),
            confidence_threshold: 0.7,
        }
    }

    pub fn record_outcome(
        &mut self,
        prediction: ROIPrediction,
        actual: ROIActual,
        target: &TestTarget,
    ) {
        let outcome = ROIOutcome {
            prediction,
            actual,
            timestamp: Utc::now(),
            context: self.capture_context(target),
        };

        self.history.push(outcome.clone());
        self.update_adjustment_factors(&outcome);
    }

    pub fn adjust_estimate(&self, base_estimate: f64, target: &TestTarget) -> f64 {
        let key = self.generate_key(target);

        if let Some(&factor) = self.adjustment_factors.get(&key) {
            (base_estimate * factor).max(0.1)
        } else {
            let similar_outcomes = self.find_similar_outcomes(target);

            if similar_outcomes.is_empty() {
                base_estimate
            } else {
                let adjustment = self.calculate_adjustment(&similar_outcomes);
                (base_estimate * adjustment).max(0.1)
            }
        }
    }

    fn capture_context(&self, target: &TestTarget) -> OutcomeContext {
        OutcomeContext {
            module_type: format!("{:?}", target.module_type),
            complexity_level: self.categorize_complexity(target),
            initial_coverage: target.current_coverage,
            dependencies_count: target.dependencies.len(),
        }
    }

    fn categorize_complexity(&self, target: &TestTarget) -> String {
        match target.complexity.cyclomatic_complexity {
            0..=5 => "Low".to_string(),
            6..=10 => "Medium".to_string(),
            11..=20 => "High".to_string(),
            _ => "VeryHigh".to_string(),
        }
    }

    fn generate_key(&self, target: &TestTarget) -> String {
        format!(
            "{:?}_{}_{}_{}",
            target.module_type,
            self.categorize_complexity(target),
            if target.current_coverage == 0.0 {
                "zero"
            } else {
                "partial"
            },
            target.dependencies.len() / 3
        )
    }

    fn find_similar_outcomes(&self, target: &TestTarget) -> Vec<&ROIOutcome> {
        let target_context = self.capture_context(target);

        self.history
            .iter()
            .filter(|outcome| {
                outcome.context.module_type == target_context.module_type
                    && outcome.context.complexity_level == target_context.complexity_level
                    && (outcome.context.initial_coverage - target_context.initial_coverage).abs()
                        < 20.0
            })
            .collect()
    }

    fn calculate_adjustment(&self, outcomes: &[&ROIOutcome]) -> f64 {
        if outcomes.is_empty() {
            return 1.0;
        }

        let total_ratio: f64 = outcomes
            .iter()
            .map(|o| {
                if o.prediction.effort > 0.0 {
                    o.actual.effort / o.prediction.effort
                } else {
                    1.0
                }
            })
            .sum();

        let avg_adjustment = total_ratio / outcomes.len() as f64;

        avg_adjustment.clamp(0.5, 2.0)
    }

    fn update_adjustment_factors(&mut self, outcome: &ROIOutcome) {
        let key = format!(
            "{}_{}_{}_{}",
            outcome.context.module_type,
            outcome.context.complexity_level,
            if outcome.context.initial_coverage == 0.0 {
                "zero"
            } else {
                "partial"
            },
            outcome.context.dependencies_count / 3
        );

        let effort_ratio = if outcome.prediction.effort > 0.0 {
            outcome.actual.effort / outcome.prediction.effort
        } else {
            1.0
        };

        self.adjustment_factors
            .entry(key)
            .and_modify(|factor| {
                *factor = (*factor * 0.7 + effort_ratio * 0.3).clamp(0.5, 2.0);
            })
            .or_insert(effort_ratio);
    }

    pub fn get_confidence(&self, target: &TestTarget) -> f64 {
        let similar_count = self.find_similar_outcomes(target).len();

        match similar_count {
            0 => 0.5,
            1..=2 => 0.6,
            3..=5 => 0.7,
            6..=10 => 0.8,
            11..=20 => 0.9,
            _ => 0.95,
        }
    }

    pub fn export(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn import(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }
}
