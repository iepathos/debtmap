pub mod cascade;
pub mod effort;
pub mod learning;
pub mod models;
pub mod reduction;

#[cfg(test)]
mod tests;

use crate::core::ComplexityMetrics;
use im::{HashMap, Vector};
use std::path::PathBuf;

pub use cascade::{CascadeCalculator, CascadeImpact};
pub use effort::{AdvancedEffortModel, EffortEstimate, EffortModel};
pub use learning::{ROILearningSystem, ROIOutcome};
pub use models::{ROIBreakdown, ROIComponent, ROI};
pub use reduction::{RiskReduction, RiskReductionModel};

use super::priority::TestTarget;
use super::RiskAnalyzer;

pub struct ROICalculator {
    effort_model: Box<dyn EffortModel>,
    risk_model: Box<dyn RiskReductionModel>,
    cascade_calculator: CascadeCalculator,
    learning_system: Option<ROILearningSystem>,
    config: ROIConfig,
}

#[derive(Clone, Debug)]
pub struct ROIConfig {
    pub cascade_weight: f64,
    pub confidence_threshold: f64,
    pub max_cascade_depth: usize,
    pub learning_enabled: bool,
}

impl Default for ROIConfig {
    fn default() -> Self {
        Self {
            cascade_weight: 0.5,
            confidence_threshold: 0.1,
            max_cascade_depth: 3,
            learning_enabled: false,
        }
    }
}

impl ROICalculator {
    pub fn new(risk_analyzer: RiskAnalyzer) -> Self {
        Self {
            effort_model: Box::new(AdvancedEffortModel::new()),
            risk_model: Box::new(reduction::AdvancedRiskReductionModel::new(risk_analyzer)),
            cascade_calculator: CascadeCalculator::new(),
            learning_system: None,
            config: ROIConfig::default(),
        }
    }

    pub fn with_learning(mut self, learning_system: ROILearningSystem) -> Self {
        self.learning_system = Some(learning_system);
        self
    }

    pub fn calculate(&self, target: &TestTarget, context: &Context) -> ROI {
        let effort = self.estimate_effort(target, context);
        let direct_impact = self.calculate_direct_impact(target);
        let cascade_impact = self.cascade_calculator.calculate(target, context);
        let confidence = self.calculate_confidence(target, &effort, &direct_impact);

        let total_impact = direct_impact.percentage
            + cascade_impact.total_risk_reduction * self.config.cascade_weight;
        let adjusted_effort = self.adjust_effort_with_learning(effort.hours, target);

        // Scale ROI to meaningful range (0.1 to 10.0)
        // High impact (>30%) with low effort (<2h) = ROI ~5-10
        // Moderate impact (10-30%) with moderate effort (2-5h) = ROI ~1-5
        // Low impact (<10%) or high effort (>5h) = ROI ~0.1-1
        let raw_roi = if adjusted_effort > 0.0 {
            total_impact / adjusted_effort
        } else {
            total_impact
        };

        let value = (raw_roi * confidence).clamp(0.1, 10.0);

        ROI {
            value,
            effort: effort.clone(),
            direct_impact: direct_impact.clone(),
            cascade_impact: cascade_impact.clone(),
            confidence,
            breakdown: self.generate_breakdown(target, &effort, &direct_impact, &cascade_impact),
        }
    }

    fn estimate_effort(&self, target: &TestTarget, _context: &Context) -> EffortEstimate {
        self.effort_model.estimate(target)
    }

    fn calculate_direct_impact(&self, target: &TestTarget) -> RiskReduction {
        self.risk_model.calculate(target)
    }

    fn calculate_confidence(
        &self,
        target: &TestTarget,
        effort: &EffortEstimate,
        _impact: &RiskReduction,
    ) -> f64 {
        let complexity_confidence = match target.complexity.cyclomatic_complexity {
            0..=5 => 0.9,
            6..=10 => 0.8,
            11..=20 => 0.7,
            _ => 0.6,
        };

        let coverage_confidence = if target.current_coverage == 0.0 {
            0.95
        } else {
            0.8
        };

        let effort_confidence = match effort.hours {
            h if h <= 2.0 => 0.9,
            h if h <= 5.0 => 0.8,
            h if h <= 10.0 => 0.7,
            _ => 0.6,
        };

        f64::max(
            complexity_confidence * coverage_confidence * effort_confidence,
            0.5,
        )
    }

    fn adjust_effort_with_learning(&self, base_effort: f64, target: &TestTarget) -> f64 {
        if let Some(ref learning) = self.learning_system {
            learning.adjust_estimate(base_effort, target)
        } else {
            base_effort
        }
    }

    fn generate_breakdown(
        &self,
        target: &TestTarget,
        effort: &EffortEstimate,
        direct: &RiskReduction,
        cascade: &CascadeImpact,
    ) -> ROIBreakdown {
        let mut components = Vec::new();

        components.push(ROIComponent {
            name: "Direct Risk Reduction".to_string(),
            value: direct.percentage,
            weight: 1.0,
            explanation: format!(
                "Reduces risk from {:.1} to {:.1}",
                target.current_risk,
                target.current_risk - direct.absolute
            ),
        });

        components.push(ROIComponent {
            name: "Cascade Impact".to_string(),
            value: cascade.total_risk_reduction,
            weight: self.config.cascade_weight,
            explanation: format!(
                "Affects {} dependent modules",
                cascade.affected_modules.len()
            ),
        });

        components.push(ROIComponent {
            name: "Effort Required".to_string(),
            value: effort.hours,
            weight: -1.0,
            explanation: format!(
                "{} test cases, {:.1} hours",
                effort.test_cases, effort.hours
            ),
        });

        let formula = format!(
            "ROI = (Direct[{:.1}%] + Cascade[{:.1}%] * {:.1}) / Effort[{:.1}h]",
            direct.percentage,
            cascade.total_risk_reduction,
            self.config.cascade_weight,
            effort.hours
        );

        let explanation = self.generate_explanation(target, effort, direct);

        ROIBreakdown {
            components,
            formula,
            explanation,
            confidence_factors: vec![],
        }
    }

    fn generate_explanation(
        &self,
        target: &TestTarget,
        effort: &EffortEstimate,
        impact: &RiskReduction,
    ) -> String {
        let coverage_str = if target.current_coverage == 0.0 {
            "completely untested".to_string()
        } else {
            format!("{:.0}% covered", target.current_coverage)
        };

        let complexity_str = format!(
            "cyclomatic {} / cognitive {}",
            target.complexity.cyclomatic_complexity, target.complexity.cognitive_complexity
        );

        let impact_str = if !target.dependents.is_empty() {
            format!(" affecting {} modules", target.dependents.len())
        } else {
            String::new()
        };

        format!(
            "Currently {coverage_str} with {complexity_str}{impact_str}. \
             Testing would reduce risk by {:.1}% with {:.1} hours effort ({} test cases)",
            impact.percentage, effort.hours, effort.test_cases
        )
    }
}

#[derive(Clone, Debug)]
pub struct Context {
    pub dependency_graph: DependencyGraph,
    pub critical_paths: Vec<PathBuf>,
    pub historical_data: Option<HistoricalData>,
}

#[derive(Clone, Debug)]
pub struct DependencyGraph {
    pub nodes: HashMap<String, DependencyNode>,
    pub edges: Vector<DependencyEdge>,
}

#[derive(Clone, Debug)]
pub struct DependencyNode {
    pub id: String,
    pub path: PathBuf,
    pub risk: f64,
    pub complexity: ComplexityMetrics,
}

#[derive(Clone, Debug)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub weight: f64,
}

#[derive(Clone, Debug)]
pub struct HistoricalData {
    pub change_frequency: HashMap<PathBuf, usize>,
    pub bug_density: HashMap<PathBuf, f64>,
    pub test_effectiveness: HashMap<PathBuf, f64>,
}
