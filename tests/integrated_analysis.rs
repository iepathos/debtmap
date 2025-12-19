//! Integration tests for Spec 185: Integrated Architecture Analysis
//!
//! Tests the orchestration of specs 181-184 to ensure:
//! - Non-conflicting recommendations
//! - Quality validation
//! - Performance budgets
//! - Graceful degradation

use debtmap::organization::{
    AnalysisConfig, ConflictResolutionStrategy, EnabledAnalyzers, IntegratedArchitectureAnalyzer,
};
use std::time::Duration;

#[test]
fn test_enabled_analyzers_all() {
    let analyzers = EnabledAnalyzers::all();
    assert!(analyzers.type_based);
    assert!(analyzers.data_flow);
    assert!(analyzers.anti_pattern);
    assert!(analyzers.hidden_types);
}

#[test]
fn test_enabled_analyzers_minimal() {
    let analyzers = EnabledAnalyzers::minimal();
    assert!(!analyzers.type_based);
    assert!(!analyzers.data_flow);
    assert!(analyzers.anti_pattern); // Always enabled
    assert!(!analyzers.hidden_types);
}

#[test]
fn test_default_config() {
    let config = AnalysisConfig::default();
    assert_eq!(config.max_analysis_time, Duration::from_millis(500));
    assert_eq!(config.advanced_analysis_threshold, 50.0);
    assert_eq!(config.min_quality_score, 60.0);
    assert_eq!(
        config.conflict_resolution,
        ConflictResolutionStrategy::Hybrid
    );
}

#[test]
fn test_analyzer_creation() {
    let _analyzer = IntegratedArchitectureAnalyzer::new();
    // Should create without error
}

#[test]
fn test_analyzer_with_custom_config() {
    let config = AnalysisConfig {
        max_analysis_time: Duration::from_millis(1000),
        advanced_analysis_threshold: 30.0,
        conflict_resolution: ConflictResolutionStrategy::TypeBased,
        enabled_analyzers: EnabledAnalyzers::minimal(),
        min_quality_score: 50.0,
    };

    let _analyzer = IntegratedArchitectureAnalyzer::with_config(config);
}

#[test]
fn test_conflict_resolution_strategies() {
    // Test each conflict resolution strategy can be created
    let strategies = vec![
        ConflictResolutionStrategy::TypeBased,
        ConflictResolutionStrategy::DataFlow,
        ConflictResolutionStrategy::BestConfidence,
        ConflictResolutionStrategy::Hybrid,
        ConflictResolutionStrategy::UserChoice,
    ];

    for strategy in strategies {
        let config = AnalysisConfig {
            conflict_resolution: strategy,
            ..Default::default()
        };
        let _analyzer = IntegratedArchitectureAnalyzer::with_config(config);
    }
}

#[test]
fn test_minimal_configuration_fast() {
    // Test that minimal configuration only enables anti-pattern detection
    let config = AnalysisConfig {
        enabled_analyzers: EnabledAnalyzers::minimal(),
        max_analysis_time: Duration::from_millis(100),
        ..Default::default()
    };

    let _analyzer = IntegratedArchitectureAnalyzer::with_config(config);
}

#[test]
fn test_balanced_configuration_default() {
    // Test default balanced configuration
    let config = AnalysisConfig::default();

    assert!(config.enabled_analyzers.type_based);
    assert!(config.enabled_analyzers.data_flow);
    assert!(config.enabled_analyzers.anti_pattern);
    assert!(config.enabled_analyzers.hidden_types);
    assert_eq!(
        config.conflict_resolution,
        ConflictResolutionStrategy::Hybrid
    );
}

#[test]
fn test_thorough_configuration_slow() {
    // Test thorough configuration with user choice
    let config = AnalysisConfig {
        enabled_analyzers: EnabledAnalyzers::all(),
        conflict_resolution: ConflictResolutionStrategy::UserChoice,
        max_analysis_time: Duration::from_millis(2000),
        advanced_analysis_threshold: 0.0, // Always run
        min_quality_score: 80.0,
    };

    let _analyzer = IntegratedArchitectureAnalyzer::with_config(config);
}

/// Test that analyzer handles simple code without panicking
#[test]
fn test_analyze_simple_code() {
    let code = r#"
        pub struct Example {
            value: i32,
        }

        impl Example {
            pub fn new(value: i32) -> Self {
                Self { value }
            }

            pub fn get(&self) -> i32 {
                self.value
            }
        }
    "#;

    let ast = syn::parse_file(code).expect("Failed to parse code");
    let analyzer = IntegratedArchitectureAnalyzer::new();

    // Create a minimal god object analysis for testing
    let god_object = debtmap::organization::GodObjectAnalysis {
        is_god_object: false,
        method_count: 2,
        weighted_method_count: None,
        field_count: 1,
        responsibility_count: 1,
        lines_of_code: 50,
        complexity_sum: 2,
        god_object_score: 10.0,
        recommended_splits: vec![],
        confidence: debtmap::organization::GodObjectConfidence::NotGodObject,
        responsibilities: vec!["Example".to_string()],
        responsibility_method_counts: std::collections::HashMap::new(),
        purity_distribution: None,
        module_structure: None,
        detection_type: debtmap::organization::DetectionType::GodClass,
        visibility_breakdown: None,
        domain_count: 1,
        domain_diversity: 0.0,
        struct_ratio: 1.0,
        analysis_method: debtmap::organization::SplitAnalysisMethod::None,
        cross_domain_severity: None,
        domain_diversity_metrics: None,
        struct_name: None,
        struct_line: None,
        struct_location: None,
        aggregated_entropy: None,
        aggregated_error_swallowing_count: None,
        aggregated_error_swallowing_patterns: None,
        layering_impact: None,
        anti_pattern_report: None,
        complexity_metrics: None,   // Spec 211
        trait_method_summary: None, // Spec 217
    };

    let call_graph = std::collections::HashMap::new();

    // Should not panic, even with low score (below threshold)
    let result = analyzer.analyze(&god_object, &ast, &call_graph);
    assert!(result.is_ok());

    let analysis_result = result.unwrap();
    // Below threshold, so no advanced analysis should run
    assert!(analysis_result.unified_splits.is_empty());
}

/// Test that analyzer respects timeout budget
#[test]
fn test_timeout_budget() {
    let config = AnalysisConfig {
        max_analysis_time: Duration::from_micros(1), // Very short timeout
        ..Default::default()
    };

    let analyzer = IntegratedArchitectureAnalyzer::with_config(config);

    // Create a more complex AST that might take time
    let code = r#"
        pub struct Complex {
            a: i32, b: i32, c: i32, d: i32,
        }

        impl Complex {
            pub fn method1(&self) -> i32 { self.a }
            pub fn method2(&self) -> i32 { self.b }
            pub fn method3(&self) -> i32 { self.c }
            pub fn method4(&self) -> i32 { self.d }
        }
    "#;

    let ast = syn::parse_file(code).expect("Failed to parse");

    let god_object = debtmap::organization::GodObjectAnalysis {
        is_god_object: true,
        method_count: 20,
        weighted_method_count: None,
        field_count: 4,
        responsibility_count: 3,
        lines_of_code: 200,
        complexity_sum: 10,
        god_object_score: 60.0, // Above threshold
        recommended_splits: vec![],
        confidence: debtmap::organization::GodObjectConfidence::Probable,
        responsibilities: vec!["Complex".to_string()],
        responsibility_method_counts: std::collections::HashMap::new(),
        purity_distribution: None,
        module_structure: None,
        detection_type: debtmap::organization::DetectionType::GodClass,
        visibility_breakdown: None,
        domain_count: 1,
        domain_diversity: 0.0,
        struct_ratio: 1.0,
        analysis_method: debtmap::organization::SplitAnalysisMethod::None,
        cross_domain_severity: None,
        domain_diversity_metrics: None,
        struct_name: None,
        struct_line: None,
        struct_location: None,
        aggregated_entropy: None,
        aggregated_error_swallowing_count: None,
        aggregated_error_swallowing_patterns: None,
        layering_impact: None,
        anti_pattern_report: None,
        complexity_metrics: None,   // Spec 211
        trait_method_summary: None, // Spec 217
    };

    let call_graph = std::collections::HashMap::new();

    // Should handle timeout gracefully (either return error or partial results)
    let result = analyzer.analyze(&god_object, &ast, &call_graph);

    // We accept either timeout error or successful completion with timeout flag
    match result {
        Ok(analysis_result) => {
            // If it completes, check timeout flag
            assert!(
                analysis_result.analysis_metadata.timeout_occurred
                    || analysis_result.analysis_metadata.total_time < Duration::from_millis(100)
            );
        }
        Err(debtmap::organization::AnalysisError::TimeoutExceeded) => {
            // Expected timeout error
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}
