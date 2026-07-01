//! Solidity analyzer support.

pub mod analyzer;
pub mod complexity;
pub mod debt;
pub mod dependencies;
pub mod metrics;
pub mod orchestration;
pub mod parser;
pub mod test_detection;
pub mod types;
pub mod visitor;

pub use analyzer::SolidityAnalyzer;
