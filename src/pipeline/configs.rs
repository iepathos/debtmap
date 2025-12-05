//! Standard pipeline configurations for common analysis workflows.
//!
//! This module provides pre-configured pipelines for typical use cases:
//! - `example_pipeline()`: Simple example demonstrating the architecture
//! - Future: `standard_pipeline()`, `fast_pipeline()`, `complexity_only_pipeline()`
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use debtmap::pipeline::configs::example_pipeline;
//!
//! let pipeline = example_pipeline();
//! let result = pipeline.execute()?;
//! ```

use super::{stage::PureStage, PipelineBuilder};

/// Example pipeline demonstrating the composable architecture.
///
/// This is a simple example showing how to build a pipeline with multiple stages.
/// It doesn't perform real analysis but demonstrates the type-safe composition.
///
/// # Pipeline Stages
///
/// 1. **Initialize**: Create initial data structure
/// 2. **Process**: Transform the data
/// 3. **Finalize**: Produce final result
///
/// # Example
///
/// ```rust,ignore
/// let pipeline = example_pipeline();
/// let (result, timings) = pipeline.execute_with_timing()?;
///
/// // Show timing for each stage
/// for timing in timings {
///     println!("{}", timing.format());
/// }
/// ```
pub fn example_pipeline() -> super::BuiltPipeline<String> {
    PipelineBuilder::new()
        .stage(PureStage::new("Initialize", |()| {
            vec!["analysis", "started"]
        }))
        .stage(PureStage::new("Process", |words: Vec<&str>| {
            words
                .into_iter()
                .map(|w| w.to_uppercase())
                .collect::<Vec<_>>()
        }))
        .stage(PureStage::new("Finalize", |words: Vec<String>| {
            words.join(" ")
        }))
        .with_progress()
        .build()
}

// Future pipeline configurations will be added here as the architecture matures:
//
// pub fn standard_pipeline(config: &AnalyzeConfig) -> BuiltPipeline<UnifiedAnalysis> { ... }
// pub fn fast_pipeline(config: &AnalyzeConfig) -> BuiltPipeline<UnifiedAnalysis> { ... }
// pub fn complexity_only_pipeline(config: &AnalyzeConfig) -> BuiltPipeline<ComplexityReport> { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_pipeline() {
        let pipeline = example_pipeline();
        let result = pipeline.execute().unwrap();
        assert_eq!(result, "ANALYSIS STARTED");
    }

    #[test]
    fn test_example_pipeline_with_timing() {
        let pipeline = example_pipeline();
        let (result, timings) = pipeline.execute_with_timing().unwrap();

        assert_eq!(result, "ANALYSIS STARTED");
        assert_eq!(timings.len(), 3);
        assert_eq!(timings[0].name, "Initialize");
        assert_eq!(timings[1].name, "Process");
        assert_eq!(timings[2].name, "Finalize");
    }
}
