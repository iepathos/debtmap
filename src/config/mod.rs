// Sub-modules
mod classification;
mod detection;
mod display;
mod languages;
mod parallel;
pub mod presets;
pub mod retry;
mod scoring;
mod thresholds;

// Core configuration types
mod accessors;
pub mod analysis_config;
pub mod cli_validation;
mod core;
mod loader;
pub mod multi_source;
pub mod validation;

// Re-export scoring types for backward compatibility
pub use scoring::{
    default_benchmarks_multiplier, default_build_scripts_multiplier, default_cognitive_weight,
    default_complexity_weight, default_coverage_weight, default_cyclomatic_weight,
    default_data_flow_enabled, default_dead_store_boost, default_debug_coverage_weight,
    default_debug_multiplier, default_dependency_weight, default_documentation_multiplier,
    default_enable_context_dampening, default_enable_role_clamping,
    default_entry_point_coverage_weight, default_entry_point_multiplier,
    default_examples_multiplier, default_io_wrapper_coverage_weight, default_io_wrapper_multiplier,
    default_linear_threshold, default_log_multiplier, default_logarithmic_threshold,
    default_max_cognitive, default_max_cyclomatic, default_min_dead_store_ratio,
    default_orchestrator_coverage_weight, default_orchestrator_multiplier,
    default_organization_weight, default_pattern_match_coverage_weight,
    default_pattern_match_multiplier, default_pattern_weight, default_pure_logic_coverage_weight,
    default_pure_logic_multiplier, default_purity_weight, default_refactorability_weight,
    default_role_clamp_max, default_role_clamp_min, default_security_weight,
    default_semantic_weight, default_show_raw_scores, default_sqrt_multiplier,
    default_tests_multiplier, default_unknown_coverage_weight, default_unknown_multiplier,
    ComplexityWeightsConfig, ContextMultipliers, DataFlowScoringConfig, NormalizationConfig,
    RebalancedScoringConfig, RoleCoverageWeights, RoleMultiplierConfig, RoleMultipliers,
    ScoringWeights,
};

// Re-export threshold types for backward compatibility
pub use thresholds::{
    FileSizeThresholds, GodObjectThresholds, ThresholdsConfig, ValidationThresholds,
};

// Re-export detection types for backward compatibility
pub use detection::{
    AccessorDetectionConfig, ConstructorDetectionConfig, DataFlowClassificationConfig,
    ErrorHandlingConfig, ErrorPatternConfig, OrchestratorDetectionConfig, SeverityOverride,
};

// Re-export classification types for backward compatibility
pub use classification::{
    CallerCalleeConfig, ClassificationConfig, ContextConfig, ContextMatcherConfig,
    ContextRuleConfig, FunctionPatternConfig,
};

// Re-export language types for backward compatibility
pub use languages::{EntropyConfig, LanguageFeatures, LanguagesConfig};

// Re-export display types for backward compatibility
pub use display::{
    DisplayConfig, EvidenceVerbosity, GodObjectConfig, SignalFilterConfig, VerbosityLevel,
};

// Pure mapping pattern detection config (spec 118)
pub use crate::complexity::pure_mapping_patterns::MappingPatternConfig;

// Re-export parallel config types (spec 203)
pub use parallel::{BatchAnalysisConfig, ParallelConfig};

// Re-export retry config types (spec 205)
pub use retry::{RetryConfig, RetryStrategy};

// Re-export core types
pub use core::{AnalysisSettings, DebtmapConfig, IgnoreConfig, OutputConfig};

// Re-export loader functions
pub use loader::{
    directory_ancestors, load_config, load_config_from_path_result,
    load_config_from_path_validated, load_config_validated, load_config_validated_result,
    parse_and_validate_config,
};

// Re-export multi-source config types (spec 201)
pub use multi_source::{
    display_config_sources, load_multi_source_config, load_multi_source_config_from,
    load_multi_source_config_validated, user_config_path, ConfigSource, TracedConfig, TracedValue,
};

// Re-export accessor functions
pub use accessors::*;

// Re-export preset types (spec 205)
pub use presets::{merge_preset_with_config, PresetLevel};

// Re-export analysis config types (spec 201)
pub use analysis_config::{
    format_config_errors, AnalysisConfig, AnalysisConfigBuilder, ConfigValidationError,
    TracedAnalysisValue,
};

// Re-export CLI validation types (spec 201)
pub use cli_validation::{
    build_analysis_config_from_cli, format_cli_errors, validate_analyze_args, CliValidationError,
    CliValidationResult,
};

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

    #[test]
    fn test_parse_and_validate_config_valid_toml() {
        let toml_content = r#"
[context]
critical_paths = ["/src/main.rs"]

[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
"#;
        let result = parse_and_validate_config(toml_content);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.scoring.is_some());
        let scoring = config.scoring.unwrap();
        // Active weights should sum to 1.0
        let active_sum = scoring.coverage + scoring.complexity + scoring.dependency;
        assert!((active_sum - 1.0).abs() < 0.001);
        // Check the values with floating point tolerance
        assert!((scoring.coverage - 0.50).abs() < 0.001);
        assert!((scoring.complexity - 0.35).abs() < 0.001);
        assert!((scoring.dependency - 0.15).abs() < 0.001);
        // Unused weights should be 0
        assert!((scoring.semantic - 0.0).abs() < 0.001);
        assert!((scoring.security - 0.0).abs() < 0.001);
        assert!((scoring.organization - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_and_validate_config_invalid_toml() {
        let toml_content = "invalid toml [[ content";
        let result = parse_and_validate_config(toml_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn test_parse_and_validate_config_invalid_weights_replaced_with_defaults() {
        let toml_content = r#"
[scoring]
coverage = 0.5
complexity = 0.5
semantic = 0.5
dependency = 0.5
security = 0.5
organization = 0.5
"#;
        let result = parse_and_validate_config(toml_content);
        assert!(result.is_ok());
        let config = result.unwrap();
        let scoring = config.scoring.unwrap();
        // Invalid weights (sum > 1.0) should be replaced with defaults
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.semantic, 0.00);
        assert_eq!(scoring.dependency, 0.15);
        assert_eq!(scoring.security, 0.00);
        assert_eq!(scoring.organization, 0.00);
        let active_sum = scoring.coverage + scoring.complexity + scoring.dependency;
        assert!((active_sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_directory_ancestors_generates_correct_sequence() {
        use std::path::PathBuf;

        let start = PathBuf::from("/a/b/c/d");
        let ancestors: Vec<PathBuf> = directory_ancestors(start, 3).collect();

        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], PathBuf::from("/a/b/c/d"));
        assert_eq!(ancestors[1], PathBuf::from("/a/b/c"));
        assert_eq!(ancestors[2], PathBuf::from("/a/b"));
    }

    #[test]
    fn test_directory_ancestors_respects_max_depth() {
        use std::path::PathBuf;

        let start = PathBuf::from("/a/b/c/d/e/f/g/h");
        let ancestors: Vec<PathBuf> = directory_ancestors(start, 2).collect();

        assert_eq!(ancestors.len(), 2);
    }

    #[test]
    fn test_directory_ancestors_handles_root() {
        use std::path::PathBuf;

        let start = PathBuf::from("/");
        let ancestors: Vec<PathBuf> = directory_ancestors(start, 5).collect();

        // Root directory has no parent, so we only get the root itself
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], PathBuf::from("/"));
    }

    #[test]
    fn test_try_load_config_from_path_with_valid_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("debtmap.toml");

        // Write a valid config file
        fs::write(
            &config_path,
            r#"
[thresholds]
complexity = 15
max_file_length = 1000

[scoring]
complexity_weight = 0.4
coverage_weight = 0.3
inheritance_weight = 0.15
interface_weight = 0.15
"#,
        )
        .unwrap();

        let result = loader::try_load_config_from_path(&config_path);
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.thresholds.as_ref().unwrap().complexity, Some(15));
        assert_eq!(
            config.thresholds.as_ref().unwrap().max_file_length,
            Some(1000)
        );
    }

    #[test]
    fn test_try_load_config_from_path_with_invalid_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("debtmap.toml");

        // Write an invalid config file
        fs::write(&config_path, "invalid toml content").unwrap();

        let result = loader::try_load_config_from_path(&config_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_load_config_from_path_with_nonexistent_file() {
        use std::path::PathBuf;

        let config_path = PathBuf::from("/nonexistent/path/to/config.toml");
        let result = loader::try_load_config_from_path(&config_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_read_error_with_not_found() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path");
        let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

        // This should not panic and should not log a warning for NotFound
        loader::handle_read_error(&path, &error);
    }

    #[test]
    fn test_handle_read_error_with_permission_denied() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path");
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");

        // This should log a warning but not panic
        loader::handle_read_error(&path, &error);
    }

    #[test]
    fn test_get_validation_thresholds_with_defaults() {
        // Test that get_validation_thresholds returns expected values
        // The config might override these, so we test flexible values
        let thresholds = get_validation_thresholds();

        // Primary quality metrics
        assert_eq!(thresholds.max_average_complexity, 10.0);
        assert_eq!(thresholds.max_debt_density, 50.0);
        assert_eq!(thresholds.max_codebase_risk_score, 7.0);
        assert_eq!(thresholds.min_coverage_percentage, 0.0);

        // Safety net - high ceiling
        assert_eq!(thresholds.max_total_debt_score, 10000);

        // Deprecated metrics should be None by default
        #[allow(deprecated)]
        {
            assert_eq!(thresholds.max_high_complexity_count, None);
            assert_eq!(thresholds.max_debt_items, None);
            assert_eq!(thresholds.max_high_risk_functions, None);
        }
    }

    #[test]
    fn test_default_linear_threshold() {
        assert_eq!(default_linear_threshold(), 10.0);
    }

    #[test]
    fn test_default_logarithmic_threshold() {
        assert_eq!(default_logarithmic_threshold(), 100.0);
    }

    #[test]
    fn test_default_sqrt_multiplier() {
        assert_eq!(default_sqrt_multiplier(), 3.33);
    }

    #[test]
    fn test_default_log_multiplier() {
        assert_eq!(default_log_multiplier(), 10.0);
    }

    #[test]
    fn test_default_show_raw_scores() {
        assert!(default_show_raw_scores());
    }

    #[test]
    fn test_normalization_config_default() {
        let config = NormalizationConfig::default();
        assert_eq!(config.linear_threshold, 10.0);
        assert_eq!(config.logarithmic_threshold, 100.0);
        assert_eq!(config.sqrt_multiplier, 3.33);
        assert_eq!(config.log_multiplier, 10.0);
        assert!(config.show_raw_scores);
    }

    #[test]
    fn test_role_multipliers_default() {
        let multipliers = RoleMultipliers::default();
        assert_eq!(multipliers.pure_logic, 1.2);
        assert_eq!(multipliers.orchestrator, 0.8);
        assert_eq!(multipliers.io_wrapper, 0.7);
        assert_eq!(multipliers.entry_point, 0.9);
        assert_eq!(multipliers.pattern_match, 0.6);
        assert_eq!(multipliers.unknown, 1.0);
    }

    #[test]
    fn test_scoring_weights_default() {
        let weights = ScoringWeights::default();
        assert_eq!(weights.coverage, 0.50);
        assert_eq!(weights.complexity, 0.35);
        assert_eq!(weights.semantic, 0.00);
        assert_eq!(weights.dependency, 0.15);
        assert_eq!(weights.security, 0.00);
        assert_eq!(weights.organization, 0.00);
    }

    #[test]
    fn test_scoring_weights_validate_success() {
        let weights = ScoringWeights {
            coverage: 0.50,
            complexity: 0.35,
            semantic: 0.0,
            dependency: 0.15,
            security: 0.0,
            organization: 0.0,
        };
        assert!(weights.validate().is_ok());
    }

    #[test]
    fn test_scoring_weights_validate_invalid_sum() {
        let weights = ScoringWeights {
            coverage: 0.60,
            complexity: 0.60,
            semantic: 0.0,
            dependency: 0.0,
            security: 0.0,
            organization: 0.0,
        };
        assert!(weights.validate().is_err());
    }

    #[test]
    fn test_scoring_weights_normalize() {
        let mut weights = ScoringWeights {
            coverage: 0.40,
            complexity: 0.30,
            semantic: 0.0,
            dependency: 0.10,
            security: 0.0,
            organization: 0.0,
        };
        weights.normalize();
        // After normalization, active weights should sum to 1.0
        let sum = weights.coverage + weights.complexity + weights.dependency;
        assert!((sum - 1.0).abs() < 0.001);
        // Check proportions are maintained
        assert!((weights.coverage - 0.50).abs() < 0.001);
        assert!((weights.complexity - 0.375).abs() < 0.001);
        assert!((weights.dependency - 0.125).abs() < 0.001);
    }

    #[test]
    fn test_entropy_config_default() {
        let config = EntropyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.weight, 1.0);
        assert_eq!(config.min_tokens, 20);
        assert_eq!(config.pattern_threshold, 0.7);
        assert_eq!(config.entropy_threshold, 0.4);
        assert_eq!(config.branch_threshold, 0.8);
        assert_eq!(config.max_repetition_reduction, 0.20);
        assert_eq!(config.max_entropy_reduction, 0.15);
        assert_eq!(config.max_branch_reduction, 0.25);
        assert_eq!(config.max_combined_reduction, 0.30);
    }

    #[test]
    fn test_error_handling_config_default() {
        let config = ErrorHandlingConfig::default();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
        assert!(config.detect_propagation);
        assert!(config.detect_panic_patterns);
        assert!(config.detect_swallowing);
        assert_eq!(config.custom_patterns.len(), 0);
        assert_eq!(config.severity_overrides.len(), 0);
    }

    #[test]
    fn test_god_object_config_default() {
        let config = GodObjectConfig::default();
        assert!(config.enabled);
        // Test Rust defaults
        assert_eq!(config.rust.max_methods, 20);
        assert_eq!(config.rust.max_fields, 15);
        // Test Python defaults
        assert_eq!(config.python.max_methods, 15);
        assert_eq!(config.python.max_fields, 10);
        // Test JavaScript defaults
        assert_eq!(config.javascript.max_methods, 15);
        assert_eq!(config.javascript.max_fields, 20);
    }

    #[test]
    fn test_context_config_default() {
        let config = ContextConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.rules.len(), 0);
        assert!(config.function_patterns.is_none());
    }

    #[test]
    fn test_language_features_default() {
        let features = LanguageFeatures::default();
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_minimum_debt_score() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let score = get_minimum_debt_score();
        // The default is 1.0 but config might override it to 2.0
        assert!(score >= 1.0);
    }

    #[test]
    fn test_get_minimum_cyclomatic_complexity() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let complexity = get_minimum_cyclomatic_complexity();
        // The default is 2 but config might override it to 3
        assert!(complexity >= 2);
    }

    #[test]
    fn test_get_minimum_cognitive_complexity() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let complexity = get_minimum_cognitive_complexity();
        // The default is 3 but config might override it to 5
        assert!(complexity >= 3);
    }

    #[test]
    fn test_get_minimum_risk_score() {
        // This test will use the config from .debtmap.toml if present, or defaults otherwise
        let score = get_minimum_risk_score();
        // The default is 1.0 but config might override it to 2.0
        assert!(score >= 1.0);
    }

    #[test]
    fn test_validation_thresholds_default() {
        let thresholds = ValidationThresholds::default();

        // Primary quality metrics
        assert_eq!(thresholds.max_average_complexity, 10.0);
        assert_eq!(thresholds.max_debt_density, 50.0);
        assert_eq!(thresholds.max_codebase_risk_score, 7.0);
        assert_eq!(thresholds.min_coverage_percentage, 0.0);

        // Safety net - high ceiling
        assert_eq!(thresholds.max_total_debt_score, 10000);

        // Deprecated metrics should be None by default
        #[allow(deprecated)]
        {
            assert_eq!(thresholds.max_high_complexity_count, None);
            assert_eq!(thresholds.max_debt_items, None);
            assert_eq!(thresholds.max_high_risk_functions, None);
        }
    }

    #[test]
    fn test_get_language_features_rust() {
        use crate::core::Language;
        let features = get_language_features(&Language::Rust);
        assert!(!features.detect_dead_code); // Rust has dead code detection disabled
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_python() {
        use crate::core::Language;
        let features = get_language_features(&Language::Python);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_language_features_unknown() {
        use crate::core::Language;
        let features = get_language_features(&Language::Unknown);
        assert!(features.detect_dead_code);
        assert!(features.detect_complexity);
        assert!(features.detect_duplication);
    }

    #[test]
    fn test_get_entropy_config() {
        let config = get_entropy_config();
        // Config might override these values
        assert!(config.enabled);
        // Weight might be configured to 0.5 in .debtmap.toml
        assert!(config.weight > 0.0);
    }

    #[test]
    fn test_get_role_multipliers() {
        let multipliers = get_role_multipliers();
        assert_eq!(multipliers.pure_logic, 1.2);
        assert_eq!(multipliers.orchestrator, 0.8);
    }

    #[test]
    fn test_get_error_handling_config() {
        let config = get_error_handling_config();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
    }

    #[test]
    fn test_get_scoring_weights() {
        let weights = get_scoring_weights();
        assert_eq!(weights.coverage, 0.50);
        assert_eq!(weights.complexity, 0.35);
        assert_eq!(weights.dependency, 0.15);
    }

    #[test]
    fn test_default_weight_functions() {
        assert_eq!(default_coverage_weight(), 0.50);
        assert_eq!(default_complexity_weight(), 0.35);
        assert_eq!(default_semantic_weight(), 0.00);
        assert_eq!(default_dependency_weight(), 0.15);
        assert_eq!(default_security_weight(), 0.00);
        assert_eq!(default_organization_weight(), 0.00);
    }

    #[test]
    fn test_default_multiplier_functions() {
        assert_eq!(default_pure_logic_multiplier(), 1.2);
        assert_eq!(default_orchestrator_multiplier(), 0.8);
        assert_eq!(default_io_wrapper_multiplier(), 0.7);
        assert_eq!(default_entry_point_multiplier(), 0.9);
        assert_eq!(default_pattern_match_multiplier(), 0.6);
        assert_eq!(default_unknown_multiplier(), 1.0);
    }

    #[test]
    fn test_default_language_feature_functions() {
        use crate::config::languages::*;
        assert!(default_detect_dead_code());
        assert!(default_detect_complexity());
        assert!(default_detect_duplication());
    }

    #[test]
    fn test_default_entropy_functions() {
        use crate::config::languages::*;
        assert!(default_entropy_enabled());
        assert_eq!(default_entropy_weight(), 1.0);
        assert_eq!(default_entropy_min_tokens(), 20);
        assert_eq!(default_entropy_pattern_threshold(), 0.7);
        assert_eq!(default_entropy_threshold(), 0.4);
        assert_eq!(default_branch_threshold(), 0.8);
        assert_eq!(default_max_repetition_reduction(), 0.20);
        assert_eq!(default_max_entropy_reduction(), 0.15);
        assert_eq!(default_max_branch_reduction(), 0.25);
        assert_eq!(default_max_combined_reduction(), 0.30);
    }

    #[test]
    fn test_default_error_handling_functions() {
        // Test through the ErrorHandlingConfig::default() instead of private functions
        let config = ErrorHandlingConfig::default();
        assert!(config.detect_async_errors);
        assert!(config.detect_context_loss);
        assert!(config.detect_propagation);
        assert!(config.detect_panic_patterns);
        assert!(config.detect_swallowing);
    }

    // Tests for extracted pure functions (spec 93)

    #[test]
    fn test_is_valid_weight() {
        // Test valid weights
        assert!(ScoringWeights::is_valid_weight(0.0));
        assert!(ScoringWeights::is_valid_weight(0.5));
        assert!(ScoringWeights::is_valid_weight(1.0));

        // Test invalid weights
        assert!(!ScoringWeights::is_valid_weight(-0.1));
        assert!(!ScoringWeights::is_valid_weight(1.1));
        assert!(!ScoringWeights::is_valid_weight(2.0));
        assert!(!ScoringWeights::is_valid_weight(-10.0));
    }

    #[test]
    fn test_validate_weight() {
        // Test valid weight
        assert!(ScoringWeights::validate_weight(0.5, "Test").is_ok());
        assert!(ScoringWeights::validate_weight(0.0, "Min").is_ok());
        assert!(ScoringWeights::validate_weight(1.0, "Max").is_ok());

        // Test invalid weight
        let result = ScoringWeights::validate_weight(1.5, "Invalid");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Invalid weight must be between 0.0 and 1.0"
        );
    }

    #[test]
    fn test_validate_active_weights_sum() {
        // Test valid sum (exactly 1.0)
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.2).is_ok());

        // Test valid sum (within tolerance)
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.2001).is_ok());
        assert!(ScoringWeights::validate_active_weights_sum(0.5, 0.3, 0.1999).is_ok());

        // Test invalid sum (too high)
        let result = ScoringWeights::validate_active_weights_sum(0.6, 0.5, 0.3);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must sum to 1.0, but sum to 1.400"));

        // Test invalid sum (too low)
        let result = ScoringWeights::validate_active_weights_sum(0.2, 0.2, 0.2);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must sum to 1.0, but sum to 0.600"));
    }

    #[test]
    fn test_collect_weight_validations() {
        // Test with all valid weights
        let weights = ScoringWeights {
            coverage: 0.5,
            complexity: 0.3,
            semantic: 0.0,
            dependency: 0.2,
            security: 0.0,
            organization: 0.0,
        };
        let validations = weights.collect_weight_validations();
        assert_eq!(validations.len(), 6);
        for validation in validations {
            assert!(validation.is_ok());
        }

        // Test with invalid weights
        let weights = ScoringWeights {
            coverage: 1.5,    // Invalid
            complexity: -0.1, // Invalid
            semantic: 0.0,
            dependency: 0.2,
            security: 2.0, // Invalid
            organization: 0.0,
        };
        let validations = weights.collect_weight_validations();
        assert_eq!(validations.len(), 6);
        assert!(validations[0].is_err()); // coverage
        assert!(validations[1].is_err()); // complexity
        assert!(validations[2].is_ok()); // semantic
        assert!(validations[3].is_ok()); // dependency
        assert!(validations[4].is_err()); // security
        assert!(validations[5].is_ok()); // organization
    }

    #[test]
    fn test_read_config_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Write test config
        fs::write(&config_path, "[thresholds]\ncomplexity = 15\n").unwrap();

        // Test reading existing file
        let contents = loader::read_config_file(&config_path).unwrap();
        assert_eq!(contents, "[thresholds]\ncomplexity = 15\n");

        // Test reading non-existent file
        let non_existent = temp_dir.path().join("non_existent.toml");
        assert!(loader::read_config_file(&non_existent).is_err());
    }

    #[test]
    fn test_parse_and_validate_config_impl() {
        // Test valid config
        let valid_toml = r#"
[scoring]
coverage = 0.50
complexity = 0.35
dependency = 0.15
"#;
        let config = loader::parse_and_validate_config_impl(valid_toml).unwrap();
        let scoring = config.scoring.unwrap();
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.dependency, 0.15);

        // Test invalid TOML
        let invalid_toml = "invalid [[ toml";
        assert!(loader::parse_and_validate_config_impl(invalid_toml).is_err());

        // Test config with invalid weights (should be normalized)
        let invalid_weights = r#"
[scoring]
coverage = 0.6
complexity = 0.6
dependency = 0.6
"#;
        let config = loader::parse_and_validate_config_impl(invalid_weights).unwrap();
        // Should use defaults due to invalid sum
        let scoring = config.scoring.unwrap();
        assert_eq!(scoring.coverage, 0.50);
        assert_eq!(scoring.complexity, 0.35);
        assert_eq!(scoring.dependency, 0.15);
    }

    #[test]
    fn test_directory_ancestors_impl() {
        use std::path::PathBuf;

        // Test normal path traversal
        let start = PathBuf::from("/a/b/c/d");
        let ancestors: Vec<PathBuf> = loader::directory_ancestors_impl(start.clone(), 3).collect();
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0], PathBuf::from("/a/b/c/d"));
        assert_eq!(ancestors[1], PathBuf::from("/a/b/c"));
        assert_eq!(ancestors[2], PathBuf::from("/a/b"));

        // Test with depth limit
        let ancestors: Vec<PathBuf> = loader::directory_ancestors_impl(start.clone(), 2).collect();
        assert_eq!(ancestors.len(), 2);

        // Test with root path
        let root = PathBuf::from("/");
        let ancestors: Vec<PathBuf> = loader::directory_ancestors_impl(root, 5).collect();
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0], PathBuf::from("/"));

        // Test with zero depth
        let ancestors: Vec<PathBuf> = loader::directory_ancestors_impl(start, 0).collect();
        assert_eq!(ancestors.len(), 0);
    }

    #[test]
    fn test_handle_read_error() {
        use std::io;
        use std::path::PathBuf;

        let path = PathBuf::from("/test/path.toml");

        // Test NotFound error (should not log warning)
        let not_found = io::Error::new(io::ErrorKind::NotFound, "File not found");
        loader::handle_read_error(&path, &not_found); // Should not panic

        // Test PermissionDenied error (should log warning)
        let permission = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        loader::handle_read_error(&path, &permission); // Should not panic

        // Test other errors
        let other = io::Error::other("Unknown error");
        loader::handle_read_error(&path, &other); // Should not panic
    }
}
