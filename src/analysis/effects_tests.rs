//! Integration tests for effect-based analysis (Spec 207).
//!
//! These tests verify that effect-based analysis functions work correctly
//! with the effect system and Reader pattern for configuration access.
//!
//! # Note on Environment Types
//!
//! The `AnalysisEffect` type is currently bound to `RealEnv`. Tests use
//! `RealEnv` with custom configurations to verify effect behavior with
//! different settings.

#[cfg(test)]
mod tests {
    use crate::analysis::attribution::{AttributedComplexity, ComplexityAttribution};
    use crate::analysis::diagnostics::effects::{
        generate_report_effect, generate_summary_effect, get_detail_level_effect,
    };
    use crate::analysis::diagnostics::DetailLevel;
    use crate::analysis::effects::{
        analyze_with_config, get_complexity_threshold, lift_pure, run_analysis_effect,
        sequence_effects, traverse_effect,
    };
    use crate::analysis::multi_pass::{AnalysisType, ComplexityResult, MultiPassResult};
    use crate::config::{DebtmapConfig, OutputConfig, ThresholdsConfig};
    use crate::env::RealEnv;
    use stillwater::Effect;

    fn create_test_result() -> MultiPassResult {
        MultiPassResult {
            raw_complexity: ComplexityResult {
                total_complexity: 20,
                cognitive_complexity: 15,
                functions: vec![],
                analysis_type: AnalysisType::Raw,
            },
            normalized_complexity: ComplexityResult {
                total_complexity: 15,
                cognitive_complexity: 12,
                functions: vec![],
                analysis_type: AnalysisType::Normalized,
            },
            attribution: ComplexityAttribution {
                logical_complexity: AttributedComplexity {
                    total: 12,
                    breakdown: vec![],
                    confidence: 0.9,
                },
                formatting_artifacts: AttributedComplexity {
                    total: 5,
                    breakdown: vec![],
                    confidence: 0.8,
                },
                pattern_complexity: AttributedComplexity {
                    total: 3,
                    breakdown: vec![],
                    confidence: 0.7,
                },
                source_mappings: vec![],
            },
            insights: vec![],
            recommendations: vec![],
            performance_metrics: None,
        }
    }

    // =========================================================================
    // Basic Effect Operations
    // =========================================================================

    #[tokio::test]
    async fn test_lift_pure() {
        let env = RealEnv::default();
        let effect = lift_pure(42);
        let result = effect.run(&env).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_sequence_effects() {
        let env = RealEnv::default();
        let effects = vec![lift_pure(1), lift_pure(2), lift_pure(3)];
        let effect = sequence_effects(effects);
        let result = effect.run(&env).await;
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_traverse_effect() {
        let env = RealEnv::default();
        let items = vec![1, 2, 3, 4, 5];
        let effect = traverse_effect(items, |n| lift_pure(n * 2));
        let result = effect.run(&env).await;
        assert_eq!(result.unwrap(), vec![2, 4, 6, 8, 10]);
    }

    // =========================================================================
    // Config Access via Reader Pattern
    // =========================================================================

    #[tokio::test]
    async fn test_get_complexity_threshold_with_default() {
        let env = RealEnv::default();
        let effect = get_complexity_threshold();
        let result = effect.run(&env).await;
        // Default is 10
        assert_eq!(result.unwrap(), 10);
    }

    #[tokio::test]
    async fn test_get_complexity_threshold_with_custom_config() {
        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(30),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = get_complexity_threshold();
        let result = effect.run(&env).await;
        assert_eq!(result.unwrap(), 30);
    }

    #[tokio::test]
    async fn test_analyze_with_config_closure() {
        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(5),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = analyze_with_config(|config| {
            let threshold = config
                .thresholds
                .as_ref()
                .and_then(|t| t.complexity)
                .unwrap_or(10);
            Ok(threshold * 3)
        });
        let result = effect.run(&env).await;
        assert_eq!(result.unwrap(), 15); // 5 * 3
    }

    // =========================================================================
    // Diagnostics Effects
    // =========================================================================

    #[tokio::test]
    async fn test_generate_summary_effect() {
        let env = RealEnv::default();
        let result = create_test_result();

        let effect = generate_summary_effect(result);
        let summary = effect.run(&env).await.unwrap();

        assert_eq!(summary.raw_complexity, 20);
        assert_eq!(summary.normalized_complexity, 15);
    }

    #[tokio::test]
    async fn test_generate_report_effect() {
        let env = RealEnv::default();
        let result = create_test_result();

        let effect = generate_report_effect(result);
        let report = effect.run(&env).await.unwrap();

        assert_eq!(report.summary.raw_complexity, 20);
    }

    #[tokio::test]
    async fn test_get_detail_level_effect_default() {
        let env = RealEnv::default();

        let effect = get_detail_level_effect();
        let level = effect.run(&env).await.unwrap();
        // Default is Standard
        assert!(matches!(level, DetailLevel::Standard));
    }

    #[tokio::test]
    async fn test_get_detail_level_effect_with_custom_config() {
        let config = DebtmapConfig {
            output: Some(OutputConfig {
                detail_level: Some("debug".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = get_detail_level_effect();
        let level = effect.run(&env).await.unwrap();
        assert!(matches!(level, DetailLevel::Debug));
    }

    #[tokio::test]
    async fn test_get_detail_level_comprehensive() {
        let config = DebtmapConfig {
            output: Some(OutputConfig {
                detail_level: Some("comprehensive".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = get_detail_level_effect();
        let level = effect.run(&env).await.unwrap();
        assert!(matches!(level, DetailLevel::Comprehensive));
    }

    // =========================================================================
    // Different Config Values
    // =========================================================================

    #[tokio::test]
    async fn test_effect_with_different_configs() {
        // Create effect factory
        let make_effect = || {
            analyze_with_config(|config| {
                Ok(config
                    .thresholds
                    .as_ref()
                    .and_then(|t| t.complexity)
                    .unwrap_or(0))
            })
        };

        // Test with config having complexity = 10
        let config1 = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(10),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env1 = RealEnv::new(config1);
        let result1 = make_effect().run(&env1).await.unwrap();
        assert_eq!(result1, 10);

        // Test with config having complexity = 20
        let config2 = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(20),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env2 = RealEnv::new(config2);
        let result2 = make_effect().run(&env2).await.unwrap();
        assert_eq!(result2, 20);
    }

    // =========================================================================
    // Backwards Compatibility
    // =========================================================================

    #[test]
    fn test_run_analysis_effect_sync_wrapper() {
        let config = DebtmapConfig::default();
        let effect = lift_pure(42);
        let result = run_analysis_effect(effect, config);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_run_analysis_effect_with_config() {
        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(15),
                ..Default::default()
            }),
            ..Default::default()
        };
        let effect = analyze_with_config(|c| {
            Ok(c.thresholds
                .as_ref()
                .and_then(|t| t.complexity)
                .unwrap_or(0))
        });
        let result = run_analysis_effect(effect, config);
        assert_eq!(result.unwrap(), 15);
    }
}
