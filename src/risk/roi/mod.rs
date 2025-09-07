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

        // Apply module type bonus to impact
        let module_multiplier = self.get_module_type_multiplier(target);

        // Calculate total impact with module bonus and cascade effects
        let total_impact = (direct_impact.percentage * module_multiplier)
            + (cascade_impact.total_risk_reduction * self.config.cascade_weight);

        let adjusted_effort = self.adjust_effort_with_learning(effort.hours, target);

        // Enhanced ROI formula with dependency awareness
        // Include dependency count as a factor
        let dependency_factor = 1.0 + (target.dependents.len() as f64 * 0.1).min(1.0);

        // Apply complexity weighting to penalize trivial functions
        let complexity_weight = self.get_complexity_weight(target);

        let raw_roi = if adjusted_effort > 0.0 {
            (total_impact * dependency_factor * complexity_weight) / adjusted_effort
        } else {
            total_impact * dependency_factor * complexity_weight
        };

        // Apply diminishing returns for very high ROI values
        let scaled_roi = if raw_roi > 20.0 {
            10.0 + (raw_roi - 20.0).ln()
        } else if raw_roi > 10.0 {
            5.0 + (raw_roi - 10.0) * 0.5
        } else {
            raw_roi
        };

        let value = (scaled_roi * confidence).max(0.1);

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

    fn get_module_type_multiplier(&self, target: &TestTarget) -> f64 {
        use super::priority::ModuleType;
        match target.module_type {
            ModuleType::EntryPoint => 2.0, // Highest multiplier for entry points
            ModuleType::Core => 1.5,       // High multiplier for core modules
            ModuleType::Api => 1.2,        // Moderate multiplier for API modules
            ModuleType::Model => 1.1,      // Slight bonus for data models
            ModuleType::IO => 1.0,         // Standard for I/O modules
            _ => 1.0,                      // No bonus for utilities and unknown
        }
    }

    fn get_complexity_weight(&self, target: &TestTarget) -> f64 {
        // Apply complexity weighting to heavily penalize trivial functions
        // This prevents delegation functions from dominating ROI recommendations
        match (
            target.complexity.cyclomatic_complexity,
            target.complexity.cognitive_complexity,
        ) {
            (1, 0..=1) => 0.1, // Trivial delegation - 90% reduction
            (1, 2..=3) => 0.3, // Very simple - 70% reduction
            (2..=3, _) => 0.5, // Simple - 50% reduction
            (4..=5, _) => 0.7, // Moderate - 30% reduction
            _ => 1.0,          // Complex - no reduction
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
            explanation: if cascade.affected_modules.is_empty() && !target.dependents.is_empty() {
                format!(
                    "Potentially affects {} dependent modules",
                    target.dependents.len()
                )
            } else if !cascade.affected_modules.is_empty() {
                format!(
                    "Affects {} dependent modules (depth: {})",
                    cascade.affected_modules.len(),
                    cascade.propagation_depth
                )
            } else {
                "No cascade impact detected".to_string()
            },
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

        let module_multiplier = self.get_module_type_multiplier(target);
        let dependency_factor = 1.0 + (target.dependents.len() as f64 * 0.1).min(1.0);
        let complexity_weight = self.get_complexity_weight(target);

        let formula = format!(
            "ROI = ((Direct[{:.1}%] * {:.1}) + (Cascade[{:.1}%] * {:.1})) * DependencyFactor[{:.1}] * ComplexityWeight[{:.1}] / Effort[{:.1}h]",
            direct.percentage,
            module_multiplier,
            cascade.total_risk_reduction,
            self.config.cascade_weight,
            dependency_factor,
            complexity_weight,
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
            format!(" affecting {} dependent modules", target.dependents.len())
        } else if !target.dependencies.is_empty() {
            format!(" with {} dependencies", target.dependencies.len())
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
