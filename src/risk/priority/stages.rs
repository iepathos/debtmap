use super::{
    module_detection::ModuleType,
    pipeline::PrioritizationStage,
    scoring::{CriticalityScorer, EffortEstimator},
    TestTarget,
};

pub struct ZeroCoverageStage {
    boost_factor: f64,
}

impl Default for ZeroCoverageStage {
    fn default() -> Self {
        Self::new()
    }
}

impl ZeroCoverageStage {
    pub fn new() -> Self {
        Self {
            boost_factor: 100.0,
        }
    }
}

impl PrioritizationStage for ZeroCoverageStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            if target.current_coverage == 0.0 {
                let criticality_factor = match target.module_type {
                    ModuleType::EntryPoint => 10.0,
                    ModuleType::Core => 8.0,
                    ModuleType::Api => 6.0,
                    ModuleType::Model => 4.0,
                    ModuleType::IO => 3.0,
                    ModuleType::Utility => 2.0,
                    _ => 1.0,
                };

                let size_factor = (target.lines as f64).ln().max(1.0);
                target.priority_score += self.boost_factor * criticality_factor * size_factor;
            }
        }
        targets
    }

    fn name(&self) -> &str {
        "ZeroCoverageStage"
    }
}

pub struct CriticalPathStage {
    scorer: CriticalityScorer,
}

impl Default for CriticalPathStage {
    fn default() -> Self {
        Self::new()
    }
}

impl CriticalPathStage {
    pub fn new() -> Self {
        Self {
            scorer: CriticalityScorer::new(),
        }
    }
}

impl PrioritizationStage for CriticalPathStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            let criticality = self.scorer.score(target);
            target.priority_score += criticality * 10.0;
        }
        targets
    }

    fn name(&self) -> &str {
        "CriticalPathStage"
    }
}

pub struct ComplexityRiskStage;

impl Default for ComplexityRiskStage {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplexityRiskStage {
    pub fn new() -> Self {
        Self
    }
}

impl PrioritizationStage for ComplexityRiskStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            let complexity_score = (target.complexity.cyclomatic_complexity as f64
                + target.complexity.cognitive_complexity as f64)
                / 2.0;
            target.priority_score += complexity_score * target.current_risk / 10.0;
        }
        targets
    }

    fn name(&self) -> &str {
        "ComplexityRiskStage"
    }
}

pub struct DependencyImpactStage;

impl Default for DependencyImpactStage {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyImpactStage {
    pub fn new() -> Self {
        Self
    }
}

impl PrioritizationStage for DependencyImpactStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            let impact_factor = (target.dependents.len() as f64).sqrt();
            target.priority_score += impact_factor * 5.0;
        }
        targets
    }

    fn name(&self) -> &str {
        "DependencyImpactStage"
    }
}

pub struct EffortOptimizationStage;

impl Default for EffortOptimizationStage {
    fn default() -> Self {
        Self::new()
    }
}

impl EffortOptimizationStage {
    pub fn new() -> Self {
        Self
    }
}

impl PrioritizationStage for EffortOptimizationStage {
    fn process(&self, mut targets: Vec<TestTarget>) -> Vec<TestTarget> {
        for target in &mut targets {
            let effort = EffortEstimator::new().estimate(target);
            if effort > 0.0 {
                target.priority_score /= effort.sqrt();
            }
        }
        targets
    }

    fn name(&self) -> &str {
        "EffortOptimizationStage"
    }
}
