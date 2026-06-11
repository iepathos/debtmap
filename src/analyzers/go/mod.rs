//! Go analyzer support.

pub mod analyzer;
pub mod debt;
pub mod dependencies;
pub mod generated;
pub mod metrics;
pub mod orchestration;
pub mod parser;
pub mod purity;
pub mod types;
pub mod visitor;

pub use analyzer::GoAnalyzer;
