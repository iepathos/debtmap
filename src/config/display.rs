use serde::{Deserialize, Serialize};
use super::GodObjectThresholds;

/// Verbosity level for output formatting
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum VerbosityLevel {
    /// Summary output - only essential information
    Summary,
    /// Detailed output - includes module structure details
    #[default]
    Detailed,
    /// Comprehensive output - all available analysis data
    Comprehensive,
}

/// Display configuration for output formatting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Whether to use tiered priority display
    #[serde(default = "default_tiered_display")]
    pub tiered: bool,

    /// Maximum items to show per tier (default: 5)
    #[serde(default = "default_items_per_tier")]
    pub items_per_tier: usize,

    /// Verbosity level for output (summary/detailed/comprehensive)
    #[serde(default)]
    pub verbosity: VerbosityLevel,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            tiered: default_tiered_display(),
            items_per_tier: default_items_per_tier(),
            verbosity: VerbosityLevel::default(),
        }
    }
}

fn default_tiered_display() -> bool {
    true // Enable tiered display by default
}

fn default_items_per_tier() -> usize {
    5
}

/// God object detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectConfig {
    /// Whether god object detection is enabled
    #[serde(default = "default_god_object_enabled")]
    pub enabled: bool,

    /// Language-specific thresholds
    #[serde(default)]
    pub rust: GodObjectThresholds,

    #[serde(default)]
    pub python: GodObjectThresholds,

    #[serde(default)]
    pub javascript: GodObjectThresholds,
}

impl Default for GodObjectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rust: GodObjectThresholds::rust_defaults(),
            python: GodObjectThresholds::python_defaults(),
            javascript: GodObjectThresholds::javascript_defaults(),
        }
    }
}

fn default_god_object_enabled() -> bool {
    true
}
