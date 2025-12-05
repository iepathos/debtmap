//! Pipeline builder for composing analysis stages.
//!
//! This module provides a type-safe fluent API for building analysis pipelines.

use super::stage::{AnyStage, Stage};
use crate::errors::AnalysisError;
use std::any::Any;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

/// Builder for constructing pipelines.
///
/// The builder uses phantom types to track the output type of the pipeline
/// at compile time, enabling type-safe composition.
///
/// # Example
///
/// ```rust,ignore
/// let pipeline = PipelineBuilder::new()
///     .stage(file_discovery)  // Output: Vec<PathBuf>
///     .stage(parsing)         // Input: Vec<PathBuf>, Output: Vec<FunctionMetrics>
///     .stage(call_graph)      // Input: Vec<FunctionMetrics>, Output: CallGraph
///     .build();
/// ```
pub struct PipelineBuilder<T> {
    stages: Vec<Box<dyn AnyStage>>,
    progress_enabled: bool,
    _phantom: PhantomData<T>,
}

impl PipelineBuilder<()> {
    /// Create a new empty pipeline builder.
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            progress_enabled: false,
            _phantom: PhantomData,
        }
    }
}

impl Default for PipelineBuilder<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PipelineBuilder<T> {
    /// Add a stage to the pipeline.
    ///
    /// The stage's input type must match the current pipeline output type.
    /// Returns a new builder with the stage's output type.
    pub fn stage<S>(mut self, stage: S) -> PipelineBuilder<S::Output>
    where
        S: Stage<Input = T> + Send + Sync + 'static,
        S::Input: 'static,
        S::Output: 'static,
        S::Error: Into<AnalysisError>,
    {
        self.stages.push(Box::new(stage));
        PipelineBuilder {
            stages: self.stages,
            progress_enabled: self.progress_enabled,
            _phantom: PhantomData,
        }
    }

    /// Add a stage conditionally.
    ///
    /// If the condition is true, the stage is added. Otherwise, this is a no-op.
    /// This is useful for optional features like coverage or context loading.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let pipeline = PipelineBuilder::new()
    ///     .stage(file_discovery)
    ///     .when(config.enable_coverage, |p| {
    ///         p.stage(coverage_loading)
    ///     })
    ///     .build();
    /// ```
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }

    /// Enable progress reporting for this pipeline.
    pub fn with_progress(mut self) -> Self {
        self.progress_enabled = true;
        self
    }

    /// Build the final pipeline ready for execution.
    pub fn build(self) -> BuiltPipeline<T> {
        BuiltPipeline {
            stages: self.stages,
            progress_enabled: self.progress_enabled,
            _phantom: PhantomData,
        }
    }
}

/// A built pipeline ready for execution.
///
/// The pipeline can be executed multiple times with different inputs.
pub struct BuiltPipeline<T> {
    stages: Vec<Box<dyn AnyStage>>,
    progress_enabled: bool,
    _phantom: PhantomData<T>,
}

impl<T: 'static> BuiltPipeline<T> {
    /// Execute the pipeline.
    ///
    /// The pipeline starts with a unit value `()` and threads data through
    /// each stage sequentially.
    pub fn execute(&self) -> Result<T, AnalysisError> {
        let mut data: Box<dyn Any> = Box::new(());

        // Report total number of stages if progress enabled
        if self.progress_enabled {
            if let Ok(quiet) = std::env::var("DEBTMAP_QUIET") {
                if quiet != "true" {
                    log::info!("Pipeline: {} stages", self.stages.len());
                }
            }
        }

        // Execute each stage in sequence
        for (i, stage) in self.stages.iter().enumerate() {
            if self.progress_enabled {
                if let Ok(quiet) = std::env::var("DEBTMAP_QUIET") {
                    if quiet != "true" {
                        log::info!("Stage {}/{}: {}", i + 1, self.stages.len(), stage.name());
                    }
                }
            }

            data = stage.execute_any(data).map_err(|e| {
                AnalysisError::other(format!("Failed in stage '{}': {}", stage.name(), e))
            })?;
        }

        // Downcast final result
        data.downcast::<T>()
            .map(|b| *b)
            .map_err(|_| AnalysisError::other("Type mismatch in pipeline output"))
    }

    /// Execute the pipeline and collect timing information for each stage.
    ///
    /// Returns both the final result and timing data for performance analysis.
    pub fn execute_with_timing(&self) -> Result<(T, Vec<StageTiming>), AnalysisError> {
        let mut data: Box<dyn Any> = Box::new(());
        let mut timings = Vec::new();

        for (i, stage) in self.stages.iter().enumerate() {
            let start = Instant::now();

            if self.progress_enabled {
                if let Ok(quiet) = std::env::var("DEBTMAP_QUIET") {
                    if quiet != "true" {
                        log::info!("Stage {}/{}: {}", i + 1, self.stages.len(), stage.name());
                    }
                }
            }

            data = stage.execute_any(data).map_err(|e| {
                AnalysisError::other(format!("Failed in stage '{}': {}", stage.name(), e))
            })?;

            let elapsed = start.elapsed();
            timings.push(StageTiming {
                name: stage.name().to_string(),
                duration: elapsed,
            });
        }

        let result = data
            .downcast::<T>()
            .map(|b| *b)
            .map_err(|_| AnalysisError::other("Type mismatch in pipeline output"))?;

        Ok((result, timings))
    }

    /// Get the number of stages in this pipeline.
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

/// Timing information for a pipeline stage.
#[derive(Debug, Clone)]
pub struct StageTiming {
    /// Name of the stage
    pub name: String,

    /// Time taken to execute the stage
    pub duration: Duration,
}

impl StageTiming {
    /// Format the timing as a human-readable string.
    pub fn format(&self) -> String {
        format!("{}: {:.2}s", self.name, self.duration.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::stage::PureStage;

    #[test]
    fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new()
            .stage(PureStage::new("Add 1", |()| 1))
            .stage(PureStage::new("Double", |x: i32| x * 2))
            .stage(PureStage::new("To String", |x: i32| x.to_string()))
            .build();

        let result = pipeline.execute().unwrap();
        assert_eq!(result, "2");
    }

    #[test]
    fn test_pipeline_conditional() {
        let with_extra = PipelineBuilder::new()
            .stage(PureStage::new("Start", |()| 1))
            .when(true, |p| p.stage(PureStage::new("Add 10", |x: i32| x + 10)))
            .stage(PureStage::new("Double", |x: i32| x * 2))
            .build();

        let without_extra = PipelineBuilder::new()
            .stage(PureStage::new("Start", |()| 1))
            .when(false, |p| {
                p.stage(PureStage::new("Add 10", |x: i32| x + 10))
            })
            .stage(PureStage::new("Double", |x: i32| x * 2))
            .build();

        assert_eq!(with_extra.execute().unwrap(), 22); // (1 + 10) * 2
        assert_eq!(without_extra.execute().unwrap(), 2); // 1 * 2
    }

    #[test]
    fn test_pipeline_timing() {
        let pipeline = PipelineBuilder::new()
            .stage(PureStage::new("Stage 1", |()| 42))
            .stage(PureStage::new("Stage 2", |x: i32| x * 2))
            .build();

        let (result, timings) = pipeline.execute_with_timing().unwrap();

        assert_eq!(result, 84);
        assert_eq!(timings.len(), 2);
        assert_eq!(timings[0].name, "Stage 1");
        assert_eq!(timings[1].name, "Stage 2");
    }

    #[test]
    fn test_stage_count() {
        let pipeline = PipelineBuilder::new()
            .stage(PureStage::new("S1", |()| 1))
            .stage(PureStage::new("S2", |x: i32| x + 1))
            .stage(PureStage::new("S3", |x: i32| x * 2))
            .build();

        assert_eq!(pipeline.stage_count(), 3);
    }
}
