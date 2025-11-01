use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguagesConfig {
    pub enabled: Vec<String>,

    /// Rust-specific configuration
    #[serde(default)]
    pub rust: Option<LanguageFeatures>,

    /// Python-specific configuration
    #[serde(default)]
    pub python: Option<LanguageFeatures>,

    /// JavaScript-specific configuration
    #[serde(default)]
    pub javascript: Option<LanguageFeatures>,

    /// TypeScript-specific configuration
    #[serde(default)]
    pub typescript: Option<LanguageFeatures>,
}

/// Language-specific feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageFeatures {
    /// Whether to detect dead code for this language
    #[serde(default = "default_detect_dead_code")]
    pub detect_dead_code: bool,

    /// Whether to detect complexity issues for this language
    #[serde(default = "default_detect_complexity")]
    pub detect_complexity: bool,

    /// Whether to detect duplication for this language
    #[serde(default = "default_detect_duplication")]
    pub detect_duplication: bool,
}

impl Default for LanguageFeatures {
    fn default() -> Self {
        Self {
            detect_dead_code: default_detect_dead_code(),
            detect_complexity: default_detect_complexity(),
            detect_duplication: default_detect_duplication(),
        }
    }
}

// Default feature flags - all enabled except Rust dead code detection
pub fn default_detect_dead_code() -> bool {
    true // Will be overridden for Rust
}

pub fn default_detect_complexity() -> bool {
    true
}

pub fn default_detect_duplication() -> bool {
    true
}

/// Entropy-based complexity scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyConfig {
    /// Whether entropy-based scoring is enabled
    #[serde(default = "default_entropy_enabled")]
    pub enabled: bool,

    /// Weight of entropy in complexity adjustment (0.0-1.0)
    #[serde(default = "default_entropy_weight")]
    pub weight: f64,

    /// Minimum tokens required for entropy calculation
    #[serde(default = "default_entropy_min_tokens")]
    pub min_tokens: usize,

    /// Pattern similarity threshold for repetition detection (0.0-1.0)
    #[serde(default = "default_entropy_pattern_threshold")]
    pub pattern_threshold: f64,

    /// Entropy threshold for low entropy detection (0.0-1.0)
    #[serde(default = "default_entropy_threshold")]
    pub entropy_threshold: f64,

    /// Whether to use smarter token classification (false by default for backward compatibility)
    #[serde(default)]
    pub use_classification: Option<bool>,

    /// Branch similarity threshold for detection (0.0-1.0)
    #[serde(default = "default_branch_threshold")]
    pub branch_threshold: f64,

    /// Maximum reduction for high repetition (0.0-1.0)
    #[serde(default = "default_max_repetition_reduction")]
    pub max_repetition_reduction: f64,

    /// Maximum reduction for low entropy (0.0-1.0)
    #[serde(default = "default_max_entropy_reduction")]
    pub max_entropy_reduction: f64,

    /// Maximum reduction for similar branches (0.0-1.0)
    #[serde(default = "default_max_branch_reduction")]
    pub max_branch_reduction: f64,

    /// Maximum combined reduction (0.0-1.0)
    #[serde(default = "default_max_combined_reduction")]
    pub max_combined_reduction: f64,
}

impl Default for EntropyConfig {
    fn default() -> Self {
        Self {
            enabled: default_entropy_enabled(),
            weight: default_entropy_weight(),
            min_tokens: default_entropy_min_tokens(),
            pattern_threshold: default_entropy_pattern_threshold(),
            entropy_threshold: default_entropy_threshold(),
            use_classification: None, // Default to None for backward compatibility
            branch_threshold: default_branch_threshold(),
            max_repetition_reduction: default_max_repetition_reduction(),
            max_entropy_reduction: default_max_entropy_reduction(),
            max_branch_reduction: default_max_branch_reduction(),
            max_combined_reduction: default_max_combined_reduction(),
        }
    }
}

pub fn default_entropy_enabled() -> bool {
    true // Enabled by default for better match statement handling
}

pub fn default_entropy_weight() -> f64 {
    1.0 // Full weight when enabled (user can adjust)
}

pub fn default_entropy_min_tokens() -> usize {
    20
}

pub fn default_entropy_pattern_threshold() -> f64 {
    0.7
}

pub fn default_entropy_threshold() -> f64 {
    0.4 // Below 0.4 entropy is considered low
}

pub fn default_branch_threshold() -> f64 {
    0.8 // Above 80% branch similarity triggers dampening
}

pub fn default_max_repetition_reduction() -> f64 {
    0.20 // Max 20% reduction for high repetition
}

pub fn default_max_entropy_reduction() -> f64 {
    0.15 // Max 15% reduction for low entropy
}

pub fn default_max_branch_reduction() -> f64 {
    0.25 // Max 25% reduction for similar branches
}

pub fn default_max_combined_reduction() -> f64 {
    0.30 // Max 30% total reduction (cap)
}
