//! Pattern detection for Rust analysis
//!
//! Contains modules for detecting various code patterns.

pub mod functional;
pub mod language_specific;
pub mod mapping;
pub mod parallel;
pub mod signals;

pub use functional::analyze_functional_composition;
pub use language_specific::build_language_specific;
pub use mapping::detect_mapping_pattern;
pub use parallel::detect_parallel_patterns;
pub use signals::detect_pattern_signals;
