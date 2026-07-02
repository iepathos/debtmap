//! Solidity analyzer support.

pub mod advanced;
pub mod analyzer;
pub mod call_graph;
pub mod calls;
pub mod complexity;
pub mod debt;
pub mod dependencies;
pub mod effects;
pub mod entropy;
pub mod generated;
pub mod metrics;
pub mod orchestration;
pub mod parser;
pub mod remappings;
pub mod test_detection;
pub mod types;
pub mod visitor;

pub use analyzer::SolidityAnalyzer;
