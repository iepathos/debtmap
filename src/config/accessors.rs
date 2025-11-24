use std::sync::OnceLock;

use super::core::DebtmapConfig;
use super::detection::ConstructorDetectionConfig;
use super::detection::{
    AccessorDetectionConfig, DataFlowClassificationConfig, ErrorHandlingConfig,
    OrchestratorDetectionConfig,
};
use super::display::DisplayConfig;
use super::languages::{EntropyConfig, LanguageFeatures};
use super::loader::load_config;
use super::scoring::{
    ContextMultipliers, RoleCoverageWeights, RoleMultiplierConfig, RoleMultipliers, ScoringWeights,
};
use super::thresholds::ValidationThresholds;

/// Cache the configuration
static CONFIG: OnceLock<DebtmapConfig> = OnceLock::new();
static SCORING_WEIGHTS: OnceLock<ScoringWeights> = OnceLock::new();

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

/// Get display configuration (with defaults)
pub fn get_display_config() -> DisplayConfig {
    get_config().display.clone().unwrap_or_default()
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

/// Get error handling configuration
pub fn get_error_handling_config() -> ErrorHandlingConfig {
    get_config().error_handling.clone().unwrap_or_default()
}

/// Get role-based coverage weight multipliers
pub fn get_role_coverage_weights() -> RoleCoverageWeights {
    get_config()
        .role_coverage_weights
        .clone()
        .unwrap_or_default()
}

/// Get role-based coverage expectations (spec 119)
pub fn get_coverage_expectations() -> crate::priority::scoring::CoverageExpectations {
    get_config()
        .coverage_expectations
        .clone()
        .unwrap_or_default()
}

/// Get role multiplier clamping configuration (spec 119)
pub fn get_role_multiplier_config() -> RoleMultiplierConfig {
    get_config()
        .role_multiplier_config
        .clone()
        .unwrap_or_default()
}

/// Get orchestrator detection configuration
pub fn get_orchestrator_detection_config() -> OrchestratorDetectionConfig {
    get_config()
        .orchestrator_detection
        .clone()
        .unwrap_or_default()
}

/// Get orchestration adjustment configuration (spec 110)
pub fn get_orchestration_adjustment_config(
) -> crate::priority::scoring::orchestration_adjustment::OrchestrationAdjustmentConfig {
    get_config()
        .orchestration_adjustment
        .clone()
        .unwrap_or_default()
}

/// Get constructor detection configuration (spec 117)
pub fn get_constructor_detection_config() -> ConstructorDetectionConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.constructors.clone())
        .unwrap_or_default()
}

/// Get accessor detection configuration (spec 125)
pub fn get_accessor_detection_config() -> AccessorDetectionConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.accessors.clone())
        .unwrap_or_default()
}

/// Get data flow classification configuration (spec 126)
pub fn get_data_flow_classification_config() -> DataFlowClassificationConfig {
    get_config()
        .classification
        .as_ref()
        .and_then(|c| c.data_flow.clone())
        .unwrap_or_default()
}

/// Get functional analysis configuration (spec 111)
pub fn get_functional_analysis_config() -> crate::analysis::FunctionalAnalysisConfig {
    get_config().functional_analysis.clone().unwrap_or_default()
}

/// Get context-aware dampening multipliers (spec 191)
pub fn get_context_multipliers() -> ContextMultipliers {
    get_config().context_multipliers.clone().unwrap_or_default()
}

/// Get minimum score threshold for filtering recommendations (spec 193)
/// Default: 3.0 (hide LOW severity items)
/// CLI flag --min-score overrides config file setting via DEBTMAP_MIN_SCORE_THRESHOLD env var
pub fn get_minimum_score_threshold() -> f64 {
    // Check environment variable first (CLI override)
    if let Ok(env_value) = std::env::var("DEBTMAP_MIN_SCORE_THRESHOLD") {
        if let Ok(threshold) = env_value.parse::<f64>() {
            return threshold;
        }
    }

    // Fall back to config file or default
    get_config()
        .thresholds
        .as_ref()
        .and_then(|t| t.min_score_threshold)
        .unwrap_or(3.0)
}
