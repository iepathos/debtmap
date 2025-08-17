use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read};
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

    /// Weight for security issues factor (0.0-1.0)
    #[serde(default = "default_security_weight")]
    pub security: f64,

    /// Weight for code organization issues factor (0.0-1.0)
    #[serde(default = "default_organization_weight")]
    pub organization: f64,

    /// Weight for performance issues factor (0.0-1.0)
    #[serde(default = "default_performance_weight")]
    pub performance: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            coverage: default_coverage_weight(),
            complexity: default_complexity_weight(),
            roi: default_roi_weight(),
            semantic: default_semantic_weight(),
            dependency: default_dependency_weight(),
            security: default_security_weight(),
            organization: default_organization_weight(),
            performance: default_performance_weight(),
        }
    }
}

impl ScoringWeights {
    /// Validate that weights sum to 1.0 (with small tolerance for floating point)
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.coverage
            + self.complexity
            + self.roi
            + self.semantic
            + self.dependency
            + self.security
            + self.organization
            + self.performance;
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
        if self.security < 0.0 || self.security > 1.0 {
            return Err("Security weight must be between 0.0 and 1.0".to_string());
        }
        if self.organization < 0.0 || self.organization > 1.0 {
            return Err("Organization weight must be between 0.0 and 1.0".to_string());
        }
        if self.performance < 0.0 || self.performance > 1.0 {
            return Err("Performance weight must be between 0.0 and 1.0".to_string());
        }

        Ok(())
    }

    /// Normalize weights to ensure they sum to 1.0
    pub fn normalize(&mut self) {
        let sum = self.coverage
            + self.complexity
            + self.roi
            + self.semantic
            + self.dependency
            + self.security
            + self.organization
            + self.performance;
        if sum > 0.0 {
            self.coverage /= sum;
            self.complexity /= sum;
            self.roi /= sum;
            self.semantic /= sum;
            self.dependency /= sum;
            self.security /= sum;
            self.organization /= sum;
            self.performance /= sum;
        }
    }
}

// Default weights - prioritize coverage but include dependency criticality
fn default_coverage_weight() -> f64 {
    0.35
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
    0.10
}
fn default_security_weight() -> f64 {
    0.05
}
fn default_organization_weight() -> f64 {
    0.05
}
fn default_performance_weight() -> f64 {
    0.0 // Default to 0 for backward compatibility
}

/// Performance detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Configuration for test performance detection
    #[serde(default)]
    pub tests: Option<TestPerformanceConfig>,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            tests: Some(TestPerformanceConfig::default()),
        }
    }
}

/// Test performance detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPerformanceConfig {
    /// Whether to detect performance issues in tests
    #[serde(default = "default_test_perf_enabled")]
    pub enabled: bool,

    /// Severity reduction for test performance issues
    /// 0 = no reduction, 1 = reduce by one level, 2 = reduce by two levels
    #[serde(default = "default_test_severity_reduction")]
    pub severity_reduction: u8,
}

impl Default for TestPerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: default_test_perf_enabled(),
            severity_reduction: default_test_severity_reduction(),
        }
    }
}

fn default_test_perf_enabled() -> bool {
    true // Detect test performance issues by default
}

fn default_test_severity_reduction() -> u8 {
    1 // Reduce severity by one level for test performance issues
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

    /// Performance detection configuration
    #[serde(default)]
    pub performance: Option<PerformanceConfig>,
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

    /// Validation thresholds - used by `debtmap validate` command
    #[serde(default)]
    pub validation: Option<ValidationThresholds>,
}

/// Validation thresholds for the validate command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationThresholds {
    /// Maximum allowed average complexity (default: 10.0)
    #[serde(default = "default_max_avg_complexity")]
    pub max_average_complexity: f64,

    /// Maximum allowed high complexity function count (default: 100)
    #[serde(default = "default_max_high_complexity_count")]
    pub max_high_complexity_count: usize,

    /// Maximum allowed technical debt items (default: 2000)
    #[serde(default = "default_max_debt_items")]
    pub max_debt_items: usize,

    /// Maximum allowed total debt score (default: 1000)
    /// Note: Uses unified scoring where each item is capped at 10.0
    #[serde(default = "default_max_total_debt_score")]
    pub max_total_debt_score: u32,

    /// Maximum allowed codebase risk score (default: 7.0)
    #[serde(default = "default_max_codebase_risk")]
    pub max_codebase_risk_score: f64,

    /// Maximum allowed high-risk function count (default: 50)
    #[serde(default = "default_max_high_risk_count")]
    pub max_high_risk_functions: usize,

    /// Minimum required code coverage percentage (default: 0.0 - no minimum)
    #[serde(default = "default_min_coverage")]
    pub min_coverage_percentage: f64,
}

impl Default for ValidationThresholds {
    fn default() -> Self {
        Self {
            max_average_complexity: default_max_avg_complexity(),
            max_high_complexity_count: default_max_high_complexity_count(),
            max_debt_items: default_max_debt_items(),
            max_total_debt_score: default_max_total_debt_score(),
            max_codebase_risk_score: default_max_codebase_risk(),
            max_high_risk_functions: default_max_high_risk_count(),
            min_coverage_percentage: default_min_coverage(),
        }
    }
}

// Default validation threshold values
fn default_max_avg_complexity() -> f64 {
    10.0
}
fn default_max_high_complexity_count() -> usize {
    100
}
fn default_max_debt_items() -> usize {
    2000
}
fn default_max_total_debt_score() -> u32 {
    1000 // Unified scoring: each item capped at 10.0, so ~100 high-priority items
}
fn default_max_codebase_risk() -> f64 {
    7.0
}
fn default_max_high_risk_count() -> usize {
    50
}
fn default_min_coverage() -> f64 {
    0.0
}

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
fn default_detect_dead_code() -> bool {
    true // Will be overridden for Rust
}

fn default_detect_complexity() -> bool {
    true
}

fn default_detect_duplication() -> bool {
    true
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
    let current = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!(
                "Failed to get current directory: {}. Using default config.",
                e
            );
            return DebtmapConfig::default();
        }
    };

    // Limit directory traversal to prevent excessive I/O
    const MAX_TRAVERSAL_DEPTH: usize = 10;
    let mut dir = current;
    let mut depth = 0;

    loop {
        // Check traversal limit
        if depth >= MAX_TRAVERSAL_DEPTH {
            log::debug!(
                "Reached maximum directory traversal depth ({}). Using default config.",
                MAX_TRAVERSAL_DEPTH
            );
            return DebtmapConfig::default();
        }

        let config_path = dir.join(".debtmap.toml");
        // Optimize I/O: Try to open directly instead of checking existence first
        match fs::File::open(&config_path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut contents = String::new();
                match reader.read_to_string(&mut contents) {
                    Ok(_) => {
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
                                log::debug!("Loaded config from {}", config_path.display());
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
                    Err(e) => {
                        log::warn!(
                            "Failed to read config file {}: {}",
                            config_path.display(),
                            e
                        );
                        // Continue searching in parent directories
                    }
                }
            }
            Err(e) => {
                // Only log actual errors, not "file not found"
                if e.kind() != std::io::ErrorKind::NotFound {
                    log::warn!(
                        "Failed to open config file {}: {}",
                        config_path.display(),
                        e
                    );
                }
                // Continue searching in parent directories
            }
        }

        if !dir.pop() {
            break;
        }
        depth += 1;
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

/// Get validation thresholds (with defaults)
pub fn get_validation_thresholds() -> ValidationThresholds {
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.validation.clone())
        .unwrap_or_default()
}

/// Get language-specific feature configuration
pub fn get_language_features(language: &crate::core::Language) -> LanguageFeatures {
    use crate::core::Language;

    let config = get_config();
    let languages_config = config.languages.as_ref();

    match language {
        Language::Rust => {
            languages_config
                .and_then(|lc| lc.rust.clone())
                .unwrap_or(LanguageFeatures {
                    detect_dead_code: false, // Rust dead code detection disabled by default
                    detect_complexity: true,
                    detect_duplication: true,
                })
        }
        Language::Python => languages_config
            .and_then(|lc| lc.python.clone())
            .unwrap_or_default(),
        Language::JavaScript => languages_config
            .and_then(|lc| lc.javascript.clone())
            .unwrap_or_default(),
        Language::TypeScript => languages_config
            .and_then(|lc| lc.typescript.clone())
            .unwrap_or_default(),
        Language::Unknown => LanguageFeatures::default(),
    }
}

/// Get test performance configuration
pub fn get_test_performance_config() -> TestPerformanceConfig {
    get_config()
        .performance
        .as_ref()
        .and_then(|p| p.tests.clone())
        .unwrap_or_default()
}
