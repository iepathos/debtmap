//! Pure functional pipeline for technical debt analysis.
//!
//! This module contains pure functions for analyzing code and detecting technical debt.
//! All business logic is separated from I/O operations following the Stillwater philosophy.
//!
//! # Composable Pipeline Architecture (Spec 209)
//!
//! The pipeline system provides a type-safe way to compose analysis stages:
//!
//! ```rust,ignore
//! use debtmap::pipeline::{PipelineBuilder, stage::PureStage};
//!
//! let pipeline = PipelineBuilder::new()
//!     .stage(file_discovery)
//!     .stage(parsing)
//!     .stage(call_graph)
//!     .when(config.enable_coverage, |p| p.stage(coverage))
//!     .stage(debt_detection)
//!     .with_progress()
//!     .build();
//!
//! let result = pipeline.execute()?;
//! ```

pub mod builder;
pub mod configs;
pub mod data;
pub mod stage;
pub mod stages;

// Re-export main types
pub use builder::{BuiltPipeline, PipelineBuilder, StageTiming};
pub use data::{CoverageData, PipelineData, ProjectContext, PurityScores, ScoredDebtItem};
pub use stage::{FallibleStage, PureStage, Stage};
