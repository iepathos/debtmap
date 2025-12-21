//! Shared dependency and purity types for function debt items
//!
//! Provides structures for tracking function-level dependencies (upstream/downstream)
//! and purity analysis results.

use serde::{Deserialize, Serialize};

/// Dependency information for function debt items
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dependencies {
    pub upstream_count: usize,
    pub downstream_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream_callers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downstream_callees: Vec<String>,
    /// Blast radius: upstream + downstream (impact of changes)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub blast_radius: usize,
    /// Whether this function is on a critical path (high upstream or downstream)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub critical_path: bool,
    /// Coupling classification: "Stable Core", "Leaf Module", "Hub", "Connector"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupling_classification: Option<String>,
    /// Instability metric: Ce/(Ca+Ce) where Ce=downstream, Ca=upstream (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instability: Option<f64>,
}

fn is_zero(val: &usize) -> bool {
    *val == 0
}

/// Purity analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityAnalysis {
    pub is_pure: bool,
    pub confidence: f32,
    /// Purity level: "StrictlyPure", "LocallyPure", "ReadOnly", "Impure"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purity_level: Option<String>,
    /// Reasons why the function is not strictly pure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_effects: Option<Vec<String>>,
}

/// Recommendation output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationOutput {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implementation_steps: Vec<String>,
}
