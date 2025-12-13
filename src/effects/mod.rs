//! Effect type aliases and helpers for debtmap analysis.
//!
//! This module provides type aliases that integrate stillwater's effect system
//! with debtmap's environment and error types. Using these aliases:
//!
//! - Reduces boilerplate in function signatures
//! - Centralizes the environment and error types
//! - Makes it easy to refactor if types change
//!
//! # Effect vs Validation
//!
//! - **Effect**: Represents a computation that may perform I/O and may fail.
//!   Use for operations like reading files or loading coverage data.
//!
//! - **Validation**: Represents a validation check that accumulates ALL errors
//!   instead of failing at the first one. Use for configuration validation,
//!   input checking, and anywhere you want comprehensive error reporting.
//!
//! # Reader Pattern (Spec 199)
//!
//! This module also provides **Reader pattern** helpers using stillwater 0.11.0's
//! zero-cost `ask`, `asks`, and `local` primitives. The Reader pattern eliminates
//! config parameter threading by making configuration available through the
//! environment.
//!
//! # Progress Effects (Spec 262)
//!
//! The [`progress`] submodule provides combinators for composable progress reporting:
//!
//! - [`progress::with_stage`]: Wrap an effect with stage tracking
//! - [`progress::traverse_with_progress`]: Sequential traversal with progress
//! - [`progress::par_traverse_with_progress`]: Parallel traversal with atomic progress
//! - [`progress::report_progress`]: Direct progress reporting effect
//!
//! ## Reader Pattern Benefits
//!
//! **Before (parameter threading):**
//! ```rust,ignore
//! fn analyze(ast: &Ast, config: &Config) -> Metrics {
//!     calculate_complexity(ast, &config.thresholds)
//! }
//! ```
//!
//! **After (Reader pattern):**
//! ```rust,ignore
//! use debtmap::effects::asks_config;
//!
//! fn analyze_effect<Env>(ast: Ast) -> impl Effect<...>
//! where Env: AnalysisEnv + Clone + Send + Sync
//! {
//!     asks_config(move |config| calculate_complexity(&ast, &config.thresholds))
//! }
//! ```
//!
//! ## Available Reader Helpers
//!
//! - [`asks_config`]: Access the full config via closure
//! - [`asks_thresholds`]: Access thresholds config section
//! - [`asks_scoring`]: Access scoring weights config section
//! - [`asks_entropy`]: Access entropy config section
//! - [`local_with_config`]: Run effect with modified config (temporary override)
//!
//! # Example: Using Effects
//!
//! ```rust,ignore
//! use debtmap::effects::AnalysisEffect;
//! use debtmap::env::AnalysisEnv;
//! use stillwater::Effect;
//!
//! fn read_source(path: PathBuf) -> AnalysisEffect<String> {
//!     Effect::from_fn(move |env: &dyn AnalysisEnv| {
//!         env.file_system()
//!             .read_to_string(&path)
//!             .map_err(Into::into)
//!     })
//! }
//! ```
//!
//! # Example: Using Progress Effects
//!
//! ```rust,ignore
//! use debtmap::effects::progress::{with_stage, traverse_with_progress};
//!
//! fn analyze_files(files: Vec<PathBuf>) -> AnalysisEffect<Vec<FileMetrics>> {
//!     with_stage("Analysis", traverse_with_progress(
//!         files,
//!         "File Processing",
//!         |path| analyze_file_effect(path)
//!     ))
//! }
//! ```

mod core;
pub mod progress;

// Re-export everything from core
pub use core::*;
