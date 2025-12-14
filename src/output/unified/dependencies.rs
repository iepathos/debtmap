//! Shared dependency and purity types for function debt items
//!
//! Provides structures for tracking function-level dependencies (upstream/downstream)
//! and purity analysis results.

use serde::{Deserialize, Serialize};

/// Dependency information for function debt items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    pub upstream_count: usize,
    pub downstream_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream_callers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub downstream_callees: Vec<String>,
}

/// Purity analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityAnalysis {
    pub is_pure: bool,
    pub confidence: f32,
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
