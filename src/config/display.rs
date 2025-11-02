use super::GodObjectThresholds;
use serde::{Deserialize, Serialize};

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

/// Evidence verbosity level for multi-signal classification display (spec 148)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceVerbosity {
    /// Minimal - only category and confidence
    #[default]
    Minimal,
    /// Standard - signal summary
    Standard,
    /// Verbose - detailed breakdown
    Verbose,
    /// Very verbose - all signals including low-weight ones
    VeryVerbose,
}

impl From<u8> for EvidenceVerbosity {
    fn from(count: u8) -> Self {
        match count {
            0 => Self::Minimal,
            1 => Self::Standard,
            2 => Self::Verbose,
            _ => Self::VeryVerbose,
        }
    }
}

impl From<EvidenceVerbosity> for u8 {
    fn from(level: EvidenceVerbosity) -> Self {
        match level {
            EvidenceVerbosity::Minimal => 0,
            EvidenceVerbosity::Standard => 1,
            EvidenceVerbosity::Verbose => 2,
            EvidenceVerbosity::VeryVerbose => 3,
        }
    }
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

/// Output configuration for evidence display (spec 148)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Evidence verbosity level for multi-signal classification
    #[serde(default)]
    pub evidence_verbosity: EvidenceVerbosity,

    /// Minimum confidence for showing warning (0.0-1.0)
    #[serde(default = "default_min_confidence_warning")]
    pub min_confidence_warning: f64,

    /// Signal filters for evidence display
    #[serde(default)]
    pub signal_filters: SignalFilterConfig,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            evidence_verbosity: EvidenceVerbosity::default(),
            min_confidence_warning: default_min_confidence_warning(),
            signal_filters: SignalFilterConfig::default(),
        }
    }
}

fn default_min_confidence_warning() -> f64 {
    0.80
}

/// Signal filtering configuration for evidence display (spec 148)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalFilterConfig {
    /// Show I/O detection signal
    #[serde(default = "default_true")]
    pub show_io_detection: bool,

    /// Show call graph signal
    #[serde(default = "default_true")]
    pub show_call_graph: bool,

    /// Show type signatures signal
    #[serde(default = "default_true")]
    pub show_type_signatures: bool,

    /// Show purity signal
    #[serde(default = "default_true")]
    pub show_purity: bool,

    /// Show framework signal
    #[serde(default = "default_true")]
    pub show_framework: bool,

    /// Show name heuristics signal (usually low weight)
    #[serde(default)]
    pub show_name_heuristics: bool,
}

impl Default for SignalFilterConfig {
    fn default() -> Self {
        Self {
            show_io_detection: true,
            show_call_graph: true,
            show_type_signatures: true,
            show_purity: true,
            show_framework: true,
            show_name_heuristics: false, // Low-weight fallback, hidden by default
        }
    }
}

fn default_true() -> bool {
    true
}
