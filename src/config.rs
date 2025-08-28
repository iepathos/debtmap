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
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            coverage: default_coverage_weight(),
            complexity: default_complexity_weight(),
            semantic: default_semantic_weight(),
            dependency: default_dependency_weight(),
            security: default_security_weight(),
            organization: default_organization_weight(),
        }
    }
}

impl ScoringWeights {
    /// Validate that weights sum to 1.0 (with small tolerance for floating point)
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.coverage
            + self.complexity
            + self.semantic
            + self.dependency
            + self.security
            + self.organization;
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

        Ok(())
    }

    /// Normalize weights to ensure they sum to 1.0
    pub fn normalize(&mut self) {
        let sum = self.coverage
            + self.complexity
            + self.semantic
            + self.dependency
            + self.security
            + self.organization;
        if sum > 0.0 {
            self.coverage /= sum;
            self.complexity /= sum;
            self.semantic /= sum;
            self.dependency /= sum;
            self.security /= sum;
            self.organization /= sum;
        }
    }
}

// Default weights after spec 58 - removed double penalties and redundant factors
fn default_coverage_weight() -> f64 {
    0.40 // Increased from 0.30 to prioritize coverage after removing ROI and semantic factors
}
fn default_complexity_weight() -> f64 {
    0.35 // Increased from 0.20, absorbing organization factor's 5% (redundant with complexity)
}
fn default_semantic_weight() -> f64 {
    0.00 // Removed from scoring per spec 58 to avoid double penalty with role multipliers
}
fn default_dependency_weight() -> f64 {
    0.20 // Increased from 0.10, absorbing semantic factor's 5%
}
fn default_security_weight() -> f64 {
    0.05 // Unchanged
}
fn default_organization_weight() -> f64 {
    0.00 // Removed from scoring per spec 58 (redundant with complexity factor)
}

/// Role multipliers configuration for semantic classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMultipliers {
    /// Multiplier for PureLogic functions (default: 1.5)
    #[serde(default = "default_pure_logic_multiplier")]
    pub pure_logic: f64,

    /// Multiplier for Orchestrator functions (default: 0.6)
    #[serde(default = "default_orchestrator_multiplier")]
    pub orchestrator: f64,

    /// Multiplier for IOWrapper functions (default: 0.5)
    #[serde(default = "default_io_wrapper_multiplier")]
    pub io_wrapper: f64,

    /// Multiplier for EntryPoint functions (default: 0.8)
    #[serde(default = "default_entry_point_multiplier")]
    pub entry_point: f64,

    /// Multiplier for PatternMatch functions (default: 0.4)
    #[serde(default = "default_pattern_match_multiplier")]
    pub pattern_match: f64,

    /// Multiplier for Unknown functions (default: 1.0)
    #[serde(default = "default_unknown_multiplier")]
    pub unknown: f64,
}

impl Default for RoleMultipliers {
    fn default() -> Self {
        Self {
            pure_logic: default_pure_logic_multiplier(),
            orchestrator: default_orchestrator_multiplier(),
            io_wrapper: default_io_wrapper_multiplier(),
            entry_point: default_entry_point_multiplier(),
            pattern_match: default_pattern_match_multiplier(),
            unknown: default_unknown_multiplier(),
        }
    }
}

fn default_pure_logic_multiplier() -> f64 {
    1.2 // Prioritized but not extreme (was 1.5)
}

fn default_orchestrator_multiplier() -> f64 {
    0.8 // Reduced but not severely (was 0.6)
}

fn default_io_wrapper_multiplier() -> f64 {
    0.7 // Minor reduction (was 0.5)
}

fn default_entry_point_multiplier() -> f64 {
    0.9 // Slight reduction (was 0.8)
}

fn default_pattern_match_multiplier() -> f64 {
    0.6 // Moderate reduction (was 0.4)
}

fn default_unknown_multiplier() -> f64 {
    1.0 // No adjustment for unknown functions
}

/// Context-aware detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Whether context-aware detection is enabled
    #[serde(default = "default_context_enabled")]
    pub enabled: bool,

    /// Custom context rules
    #[serde(default)]
    pub rules: Vec<ContextRuleConfig>,

    /// Function role patterns
    #[serde(default)]
    pub function_patterns: Option<FunctionPatternConfig>,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enabled: default_context_enabled(),
            rules: Vec::new(),
            function_patterns: None,
        }
    }
}

fn default_context_enabled() -> bool {
    false // Opt-in by default
}

/// Configuration for a context rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRuleConfig {
    /// Name of the rule
    pub name: String,

    /// Pattern to match (e.g., "blocking_io", "security", "performance")
    pub pattern: String,

    /// Context matcher configuration
    pub context: ContextMatcherConfig,

    /// Action to take (allow, skip, warn, reduce_severity)
    pub action: String,

    /// Priority (higher number = higher priority)
    #[serde(default = "default_rule_priority")]
    pub priority: i32,

    /// Optional reason for the rule
    pub reason: Option<String>,
}

fn default_rule_priority() -> i32 {
    50
}

/// Context matcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMatcherConfig {
    /// Function role (main, test, handler, config_loader, etc.)
    pub role: Option<String>,

    /// File type (production, test, benchmark, example, etc.)
    pub file_type: Option<String>,

    /// Whether function is async
    pub is_async: Option<bool>,

    /// Framework pattern (rust_main, web_handler, cli_handler, etc.)
    pub framework_pattern: Option<String>,

    /// Function name pattern (regex)
    pub name_pattern: Option<String>,
}

/// Function pattern configuration for detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionPatternConfig {
    /// Additional test function patterns
    #[serde(default)]
    pub test_patterns: Vec<String>,

    /// Additional config loader patterns
    #[serde(default)]
    pub config_patterns: Vec<String>,

    /// Additional handler patterns
    #[serde(default)]
    pub handler_patterns: Vec<String>,

    /// Additional initialization patterns
    #[serde(default)]
    pub init_patterns: Vec<String>,
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

    /// Context-aware detection configuration
    #[serde(default)]
    pub context: Option<ContextConfig>,

    /// Entropy-based complexity scoring configuration
    #[serde(default)]
    pub entropy: Option<EntropyConfig>,

    /// Role multipliers for semantic classification
    #[serde(default)]
    pub role_multipliers: Option<RoleMultipliers>,

    /// Complexity thresholds for enhanced detection
    #[serde(default)]
    pub complexity_thresholds: Option<crate::complexity::threshold_manager::ComplexityThresholds>,
}

impl DebtmapConfig {
    /// Get ignore patterns from configuration
    ///
    /// Returns a vector of glob patterns that should be excluded from analysis.
    /// If no configuration is found or no patterns are specified, returns an empty vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::config::DebtmapConfig;
    /// let config = DebtmapConfig::default();
    /// let patterns = config.get_ignore_patterns();
    /// // patterns might contain ["tests/**/*", "*.test.rs"]
    /// ```
    pub fn get_ignore_patterns(&self) -> Vec<String> {
        self.ignore
            .as_ref()
            .map(|ig| ig.patterns.clone())
            .unwrap_or_default()
    }
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

fn default_entropy_enabled() -> bool {
    true // Enabled by default for better match statement handling
}

fn default_entropy_weight() -> f64 {
    1.0 // Full weight when enabled (user can adjust)
}

fn default_entropy_min_tokens() -> usize {
    20
}

fn default_entropy_pattern_threshold() -> f64 {
    0.7
}

fn default_entropy_threshold() -> f64 {
    0.4 // Below 0.4 entropy is considered low
}

fn default_branch_threshold() -> f64 {
    0.8 // Above 80% branch similarity triggers dampening
}

fn default_max_repetition_reduction() -> f64 {
    0.20 // Max 20% reduction for high repetition
}

fn default_max_entropy_reduction() -> f64 {
    0.15 // Max 15% reduction for low entropy
}

fn default_max_branch_reduction() -> f64 {
    0.25 // Max 25% reduction for similar branches
}

fn default_max_combined_reduction() -> f64 {
    0.30 // Max 30% total reduction (cap)
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

/// Get the configuration without panicking on errors
pub fn get_config_safe() -> Result<DebtmapConfig, std::io::Error> {
    Ok(load_config())
}

/// Get the scoring weights (with defaults if not configured)
pub fn get_scoring_weights() -> &'static ScoringWeights {
    SCORING_WEIGHTS.get_or_init(|| get_config().scoring.clone().unwrap_or_default())
}

/// Get entropy-based complexity scoring configuration
pub fn get_entropy_config() -> EntropyConfig {
    get_config().entropy.clone().unwrap_or_default()
}

/// Get role multipliers configuration
pub fn get_role_multipliers() -> RoleMultipliers {
    get_config().role_multipliers.clone().unwrap_or_default()
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

/// Get complexity thresholds configuration
pub fn get_complexity_thresholds() -> crate::complexity::threshold_manager::ComplexityThresholds {
    get_config()
        .complexity_thresholds
        .clone()
        .unwrap_or_default()
}

/// Get smart performance configuration
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ignore_patterns_with_patterns() {
        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig {
                patterns: vec![
                    "tests/**/*".to_string(),
                    "*.test.rs".to_string(),
                    "**/fixtures/**".to_string(),
                ],
            }),
            ..Default::default()
        };

        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"tests/**/*".to_string()));
        assert!(patterns.contains(&"*.test.rs".to_string()));
        assert!(patterns.contains(&"**/fixtures/**".to_string()));
    }

    #[test]
    fn test_get_ignore_patterns_without_config() {
        let config = DebtmapConfig::default();
        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_get_ignore_patterns_with_empty_patterns() {
        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig { patterns: vec![] }),
            ..Default::default()
        };

        let patterns = config.get_ignore_patterns();
        assert_eq!(patterns.len(), 0);
    }
}
