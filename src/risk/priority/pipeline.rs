use super::{stages::*, TestTarget};

pub trait PrioritizationStage {
    fn process(&self, targets: Vec<TestTarget>) -> Vec<TestTarget>;
    fn name(&self) -> &str;
}

pub struct PrioritizationPipeline {
    stages: Vec<Box<dyn PrioritizationStage>>,
}

impl Default for PrioritizationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl PrioritizationPipeline {
    pub fn new() -> Self {
        Self {
            stages: vec![
                Box::new(ZeroCoverageStage::new()),
                Box::new(CriticalPathStage::new()),
                Box::new(ComplexityRiskStage::new()),
                Box::new(DependencyImpactStage::new()),
                Box::new(EffortOptimizationStage::new()),
            ],
        }
    }

    pub fn process(&self, targets: Vec<TestTarget>) -> Vec<TestTarget> {
        self.stages
            .iter()
            .fold(targets, |acc, stage| stage.process(acc))
    }
}
