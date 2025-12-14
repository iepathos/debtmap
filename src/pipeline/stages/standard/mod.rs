//! Standard pipeline stages for technical debt analysis.
//!
//! This module implements the 9 core stages of the analysis pipeline as
//! reusable, composable units following Spec 209.
//!
//! # Module Structure
//!
//! - `analysis`: Call graph, trait resolution, and purity analysis stages (thin wrappers)
//! - `context`: Project context loading from README, Cargo.toml, etc.
//! - `coverage`: Coverage loading with LCOV parsing (pure core + I/O shell)
//! - `discovery`: File discovery with filesystem walking
//! - `parsing`: File parsing stage (placeholder)
//! - `scoring`: Debt detection and scoring stages (thin wrappers)
//!
//! # Design Principles
//!
//! 1. **Pure core, imperative shell**: LCOV parsing and other transformations are pure
//! 2. **Thin stage wrappers**: Stages delegate to pure functions in sibling modules
//! 3. **Single responsibility**: Each submodule handles one cohesive area

pub mod analysis;
pub mod context;
pub mod coverage;
pub mod discovery;
pub mod parsing;
pub mod scoring;

// Re-export all stages for backward compatibility
pub use analysis::{CallGraphStage, PurityAnalysisStage, TraitResolutionStage};
pub use context::ContextLoadingStage;
pub use coverage::CoverageLoadingStage;
pub use discovery::FileDiscoveryStage;
pub use parsing::ParsingStage;
pub use scoring::{DebtDetectionStage, ScoringStage};
