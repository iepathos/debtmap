pub mod cognitive;
pub mod cyclomatic;
pub mod entropy;
pub mod entropy_analysis;
pub mod entropy_core;
pub mod entropy_traits;
pub mod if_else_analyzer;
pub mod languages;
pub mod match_patterns;
pub mod message_generator;
pub mod pattern_adjustments;
pub mod patterns;
pub mod pure_mapping_patterns;
pub mod python_pattern_adjustments;
pub mod python_patterns;
pub mod python_specific_patterns;
pub mod recursive_detector;
pub mod rust_normalizer;
pub mod semantic_normalizer;
pub mod threshold_manager;
pub mod token_classifier;
pub mod visitor_detector;
pub mod weighted;

use crate::core::FunctionMetrics;

// Re-export weighted complexity types (spec 121)
pub use weighted::{ComplexityNormalization, ComplexityWeights, WeightedComplexity};

#[derive(Clone, Debug)]
pub struct ComplexityCalculator {
    cyclomatic_threshold: u32,
    cognitive_threshold: u32,
}

impl ComplexityCalculator {
    pub fn new(cyclomatic_threshold: u32, cognitive_threshold: u32) -> Self {
        Self {
            cyclomatic_threshold,
            cognitive_threshold,
        }
    }

    pub fn is_complex(&self, metrics: &FunctionMetrics) -> bool {
        metrics.cyclomatic > self.cyclomatic_threshold
            || metrics.cognitive > self.cognitive_threshold
    }

    pub fn calculate_score(&self, metrics: &FunctionMetrics) -> u32 {
        let cyclo_score = (metrics.cyclomatic as f64 / self.cyclomatic_threshold as f64) * 50.0;
        let cognitive_score = (metrics.cognitive as f64 / self.cognitive_threshold as f64) * 50.0;
        (cyclo_score + cognitive_score) as u32
    }
}

pub fn combine_complexity(a: u32, b: u32) -> u32 {
    a + b
}

pub fn max_complexity(complexities: &[u32]) -> u32 {
    complexities.iter().copied().max().unwrap_or(0)
}

pub fn average_complexity(complexities: &[u32]) -> f64 {
    if complexities.is_empty() {
        return 0.0;
    }
    let sum: u32 = complexities.iter().sum();
    sum as f64 / complexities.len() as f64
}
