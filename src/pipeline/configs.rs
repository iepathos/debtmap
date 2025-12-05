//! Standard pipeline configurations for common analysis workflows.
//!
//! This module provides pre-configured pipelines for typical use cases:
//! - `example_pipeline()`: Simple example demonstrating the architecture
//! - `standard_pipeline()`: Full analysis with all stages
//! - `fast_pipeline()`: Quick analysis skipping coverage and context
//! - `complexity_only_pipeline()`: Just complexity metrics
//! - `call_graph_pipeline()`: Call graph and purity analysis
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use debtmap::pipeline::configs::standard_pipeline;
//! use std::path::Path;
//! use debtmap::core::Language;
//!
//! let pipeline = standard_pipeline(Path::new("."), &[Language::Rust], None, false);
//! let result = pipeline.execute()?;
//! ```

use super::{stage::PureStage, stages::*, PipelineBuilder};
use crate::core::Language;
use crate::pipeline::data::PipelineData;
use std::path::{Path, PathBuf};

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

/// Standard full analysis pipeline (all 9 stages).
///
/// This pipeline includes all analysis stages:
/// 1. File discovery
/// 2. Parsing
/// 3. Call graph construction
/// 4. Trait resolution
/// 5. Coverage loading (optional)
/// 6. Purity analysis
/// 7. Context loading (optional)
/// 8. Debt detection
/// 9. Scoring & prioritization
///
/// # Arguments
///
/// * `project_path` - Root path of the project to analyze
/// * `languages` - Languages to analyze (currently only Rust supported)
/// * `coverage_file` - Optional path to LCOV coverage file
/// * `enable_context` - Whether to load project context (README, etc.)
///
/// # Example
///
/// ```rust,ignore
/// let pipeline = standard_pipeline(
///     Path::new("."),
///     &[Language::Rust],
///     Some(&PathBuf::from("coverage.info")),
///     true
/// );
/// let result = pipeline.execute()?;
/// ```
pub fn standard_pipeline(
    project_path: &Path,
    languages: &[Language],
    coverage_file: Option<&PathBuf>,
    enable_context: bool,
) -> super::BuiltPipeline<PipelineData> {
    let mut builder = PipelineBuilder::new()
        .stage(FileDiscoveryStage::new(project_path, languages))
        .stage(ParsingStage::new())
        .stage(CallGraphStage::new())
        .stage(TraitResolutionStage::new(project_path));

    // Conditionally add coverage stage
    if let Some(coverage_path) = coverage_file {
        builder = builder.stage(CoverageLoadingStage::new(coverage_path));
    }

    builder = builder
        .stage(PurityAnalysisStage::new());

    // Conditionally add context stage
    if enable_context {
        builder = builder.stage(ContextLoadingStage::new(project_path));
    }

    builder
        .stage(DebtDetectionStage::new())
        .stage(ScoringStage::new())
        .with_progress()
        .build()
}

/// Fast pipeline (skips coverage and context).
///
/// This pipeline provides quick analysis without optional stages:
/// 1. File discovery
/// 2. Parsing
/// 3. Call graph construction
/// 4. Purity analysis
/// 5. Debt detection
/// 6. Scoring & prioritization
///
/// Use this when you need faster results and don't need coverage or context analysis.
///
/// # Arguments
///
/// * `project_path` - Root path of the project to analyze
/// * `languages` - Languages to analyze (currently only Rust supported)
pub fn fast_pipeline(
    project_path: &Path,
    languages: &[Language],
) -> super::BuiltPipeline<PipelineData> {
    PipelineBuilder::new()
        .stage(FileDiscoveryStage::new(project_path, languages))
        .stage(ParsingStage::new())
        .stage(CallGraphStage::new())
        .stage(PurityAnalysisStage::new())
        .stage(DebtDetectionStage::new())
        .stage(ScoringStage::new())
        .with_progress()
        .build()
}

/// Complexity-only pipeline (minimal analysis).
///
/// This pipeline focuses only on complexity metrics:
/// 1. File discovery
/// 2. Parsing
/// 3. Debt detection (complexity-based)
///
/// Use this when you only need complexity metrics without call graph or purity analysis.
///
/// # Arguments
///
/// * `project_path` - Root path of the project to analyze
/// * `languages` - Languages to analyze (currently only Rust supported)
pub fn complexity_only_pipeline(
    project_path: &Path,
    languages: &[Language],
) -> super::BuiltPipeline<PipelineData> {
    PipelineBuilder::new()
        .stage(FileDiscoveryStage::new(project_path, languages))
        .stage(ParsingStage::new())
        .stage(DebtDetectionStage::new())
        .with_progress()
        .build()
}

/// Call graph pipeline (call graph + purity).
///
/// This pipeline focuses on call graph and purity analysis:
/// 1. File discovery
/// 2. Parsing
/// 3. Call graph construction
/// 4. Trait resolution
/// 5. Purity analysis
///
/// Use this when you need to understand function relationships and purity.
///
/// # Arguments
///
/// * `project_path` - Root path of the project to analyze
/// * `languages` - Languages to analyze (currently only Rust supported)
pub fn call_graph_pipeline(
    project_path: &Path,
    languages: &[Language],
) -> super::BuiltPipeline<PipelineData> {
    PipelineBuilder::new()
        .stage(FileDiscoveryStage::new(project_path, languages))
        .stage(ParsingStage::new())
        .stage(CallGraphStage::new())
        .stage(TraitResolutionStage::new(project_path))
        .stage(PurityAnalysisStage::new())
        .with_progress()
        .build()
}

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

    #[test]
    fn test_standard_pipeline_builds() {
        let pipeline = standard_pipeline(
            Path::new("."),
            &[Language::Rust],
            None,
            false,
        );
        // Should build without panicking
        assert!(pipeline.stage_count() >= 6);
    }

    #[test]
    fn test_fast_pipeline_builds() {
        let pipeline = fast_pipeline(
            Path::new("."),
            &[Language::Rust],
        );
        // Should have fewer stages than standard
        assert_eq!(pipeline.stage_count(), 6);
    }

    #[test]
    fn test_complexity_only_pipeline_builds() {
        let pipeline = complexity_only_pipeline(
            Path::new("."),
            &[Language::Rust],
        );
        // Should have minimal stages
        assert_eq!(pipeline.stage_count(), 3);
    }

    #[test]
    fn test_call_graph_pipeline_builds() {
        let pipeline = call_graph_pipeline(
            Path::new("."),
            &[Language::Rust],
        );
        assert_eq!(pipeline.stage_count(), 5);
    }
}
