use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::OnceLock;

/// Scoring weights configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for coverage factor (0.0-1.0)
    #[serde(default = "default_coverage_weight")]
    pub coverage: f64,

    /// Weight for complexity factor (0.0-1.0)
    #[serde(default = "default_complexity_weight")]
    pub complexity: f64,

    /// Weight for ROI factor (0.0-1.0)
    #[serde(default = "default_roi_weight")]
    pub roi: f64,

    /// Weight for semantic factor (0.0-1.0)
    #[serde(default = "default_semantic_weight")]
    pub semantic: f64,

    /// Weight for dependency criticality factor (0.0-1.0)
    #[serde(default = "default_dependency_weight")]
    pub dependency: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            coverage: default_coverage_weight(),
            complexity: default_complexity_weight(),
            roi: default_roi_weight(),
            semantic: default_semantic_weight(),
            dependency: default_dependency_weight(),
        }
    }
}

impl ScoringWeights {
    /// Validate that weights sum to 1.0 (with small tolerance for floating point)
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.coverage + self.complexity + self.roi + self.semantic + self.dependency;
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!(
                "Scoring weights must sum to 1.0, but sum to {:.3}",
                sum
            ));
        }

        // Check each weight is between 0 and 1
        if self.coverage < 0.0 || self.coverage > 1.0 {
            return Err("Coverage weight must be between 0.0 and 1.0".to_string());
        }
        if self.complexity < 0.0 || self.complexity > 1.0 {
            return Err("Complexity weight must be between 0.0 and 1.0".to_string());
        }
        if self.roi < 0.0 || self.roi > 1.0 {
            return Err("ROI weight must be between 0.0 and 1.0".to_string());
        }
        if self.semantic < 0.0 || self.semantic > 1.0 {
            return Err("Semantic weight must be between 0.0 and 1.0".to_string());
        }
        if self.dependency < 0.0 || self.dependency > 1.0 {
            return Err("Dependency weight must be between 0.0 and 1.0".to_string());
        }

        Ok(())
    }

    /// Normalize weights to ensure they sum to 1.0
    pub fn normalize(&mut self) {
        let sum = self.coverage + self.complexity + self.roi + self.semantic + self.dependency;
        if sum > 0.0 {
            self.coverage /= sum;
            self.complexity /= sum;
            self.roi /= sum;
            self.semantic /= sum;
            self.dependency /= sum;
        }
    }
}

// Default weights - prioritize coverage but include dependency criticality
fn default_coverage_weight() -> f64 {
    0.40
}
fn default_complexity_weight() -> f64 {
    0.15
}
fn default_roi_weight() -> f64 {
    0.25
}
fn default_semantic_weight() -> f64 {
    0.05
}
fn default_dependency_weight() -> f64 {
    0.15
}

/// Root configuration structure for debtmap
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebtmapConfig {
    /// Scoring weights configuration
    #[serde(default)]
    pub scoring: Option<ScoringWeights>,

    /// External API detection configuration
    #[serde(default)]
    pub external_api: Option<crate::priority::external_api_detector::ExternalApiConfig>,

    /// Thresholds configuration
    #[serde(default)]
    pub thresholds: Option<ThresholdsConfig>,

    /// Language configuration
    #[serde(default)]
    pub languages: Option<LanguagesConfig>,

    /// Ignore patterns
    #[serde(default)]
    pub ignore: Option<IgnoreConfig>,

    /// Output configuration
    #[serde(default)]
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdsConfig {
    pub complexity: Option<u32>,
    pub duplication: Option<u32>,
    pub max_file_length: Option<usize>,
    pub max_function_length: Option<usize>,

    /// Minimum thresholds for including items in debt analysis
    #[serde(default)]
    pub minimum_debt_score: Option<f64>,

    /// Minimum complexity thresholds for considering something as debt
    #[serde(default)]
    pub minimum_cyclomatic_complexity: Option<u32>,
    #[serde(default)]
    pub minimum_cognitive_complexity: Option<u32>,

    /// Minimum risk score for including items (0.0-10.0)
    #[serde(default)]
    pub minimum_risk_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagesConfig {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub default_format: Option<String>,
}

/// Cache the configuration
static CONFIG: OnceLock<DebtmapConfig> = OnceLock::new();
static SCORING_WEIGHTS: OnceLock<ScoringWeights> = OnceLock::new();

/// Load configuration from .debtmap.toml if it exists
pub fn load_config() -> DebtmapConfig {
    // Try to find .debtmap.toml in current directory or parent directories
    let current = std::env::current_dir().ok();
    if let Some(mut dir) = current {
        loop {
            let config_path = dir.join(".debtmap.toml");
            if config_path.exists() {
                if let Ok(contents) = fs::read_to_string(&config_path) {
                    match toml::from_str::<DebtmapConfig>(&contents) {
                        Ok(mut config) => {
                            // Validate and normalize scoring weights if present
                            if let Some(ref mut scoring) = config.scoring {
                                if let Err(e) = scoring.validate() {
                                    eprintln!(
                                        "Warning: Invalid scoring weights: {}. Using defaults.",
                                        e
                                    );
                                    config.scoring = Some(ScoringWeights::default());
                                } else {
                                    scoring.normalize(); // Ensure exact sum of 1.0
                                }
                            }
                            return config;
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to parse .debtmap.toml: {}. Using defaults.",
                                e
                            );
                            return DebtmapConfig::default();
                        }
                    }
                }
            }

            if !dir.pop() {
                break;
            }
        }
    }

    // Default configuration
    DebtmapConfig::default()
}

/// Get the cached configuration
pub fn get_config() -> &'static DebtmapConfig {
    CONFIG.get_or_init(load_config)
}

/// Get the scoring weights (with defaults if not configured)
pub fn get_scoring_weights() -> &'static ScoringWeights {
    SCORING_WEIGHTS.get_or_init(|| get_config().scoring.clone().unwrap_or_default())
}

/// Get minimum debt score threshold (default: 1.0)
pub fn get_minimum_debt_score() -> f64 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_debt_score)
        .unwrap_or(1.0)
}

/// Get minimum cyclomatic complexity threshold (default: 2)
pub fn get_minimum_cyclomatic_complexity() -> u32 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_cyclomatic_complexity)
        .unwrap_or(2)
}

/// Get minimum cognitive complexity threshold (default: 3)
pub fn get_minimum_cognitive_complexity() -> u32 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_cognitive_complexity)
        .unwrap_or(3)
}

/// Get minimum risk score threshold (default: 1.0)
pub fn get_minimum_risk_score() -> f64 {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.minimum_risk_score)
        .unwrap_or(1.0)
}
