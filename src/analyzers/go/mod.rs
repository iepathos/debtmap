//! Go analyzer support.

pub mod advanced;
pub mod analyzer;
pub mod debt;
pub mod dependencies;
pub mod generated;
pub(crate) mod imports;
pub mod metrics;
pub mod orchestration;
pub mod parser;
pub mod purity;
pub mod types;
pub mod visitor;
pub mod walk;

pub use analyzer::GoAnalyzer;
