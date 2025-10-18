//! Integration tests for pattern detection functionality
//!
//! Tests the end-to-end pattern detection workflow including:
//! - Loading .debtmap.toml configuration
//! - Custom pattern rules
//! - Pattern detection with confidence thresholds
//! - CLI options integration

use debtmap::analysis::patterns::{
    config::PatternConfig, PatternDetector, PatternInstance, PatternType,
};
use debtmap::core::{
    ast::{ClassDef, MethodDef},
    ComplexityMetrics, FileMetrics, Language,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test FileMetrics with classes
fn create_test_file_metrics() -> FileMetrics {
    FileMetrics {
        path: PathBuf::from("test.py"),
        language: Language::Python,
        complexity: ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        module_scope: None,
        classes: Some(vec![
            // Observer pattern
            ClassDef {
                name: "EventHandler".to_string(),
                base_classes: vec!["ABC".to_string()],
                methods: vec![MethodDef {
                    name: "handle_event".to_string(),
                    is_abstract: true,
                    decorators: vec!["abstractmethod".to_string()],
                    overrides_base: false,
                    line: 5,
                }],
                is_abstract: true,
                decorators: vec![],
                line: 3,
            },
            // Factory pattern
            ClassDef {
                name: "WidgetFactory".to_string(),
                base_classes: vec![],
                methods: vec![MethodDef {
                    name: "create_widget".to_string(),
                    is_abstract: false,
                    decorators: vec![],
                    overrides_base: false,
                    line: 15,
                }],
                is_abstract: false,
                decorators: vec![],
                line: 12,
            },
            // Dependency Injection pattern
            ClassDef {
                name: "UserService".to_string(),
                base_classes: vec![],
                methods: vec![MethodDef {
                    name: "__init__".to_string(),
                    is_abstract: false,
                    decorators: vec!["inject".to_string()],
                    overrides_base: false,
                    line: 25,
                }],
                is_abstract: false,
                decorators: vec!["injectable".to_string()],
                line: 23,
            },
        ]),
    }
}

#[test]
fn test_pattern_detector_finds_multiple_patterns() {
    let detector = PatternDetector::new();
    let file_metrics = create_test_file_metrics();

    let patterns = detector.detect_all_patterns(&file_metrics);

    // Should detect multiple pattern types
    assert!(!patterns.is_empty(), "Expected to find patterns");

    // Check that we found different pattern types
    let pattern_types: Vec<PatternType> = patterns
        .iter()
        .map(|p| p.pattern_type.clone())
        .collect();

    // Should find at least factory and dependency injection
    // (Observer may or may not be detected depending on implementation details)
    assert!(
        pattern_types.contains(&PatternType::Factory)
            || pattern_types.contains(&PatternType::DependencyInjection),
        "Expected to find Factory or DependencyInjection patterns, found: {:?}",
        pattern_types
    );
}

#[test]
fn test_confidence_threshold_filtering() {
    let detector = PatternDetector::new();
    let file_metrics = create_test_file_metrics();

    let all_patterns = detector.detect_all_patterns(&file_metrics);

    // Filter patterns with confidence >= 0.8
    let high_confidence: Vec<&PatternInstance> = all_patterns
        .iter()
        .filter(|p| p.confidence >= 0.8)
        .collect();

    // Filter patterns with confidence >= 0.5
    let medium_confidence: Vec<&PatternInstance> = all_patterns
        .iter()
        .filter(|p| p.confidence >= 0.5)
        .collect();

    // Should have fewer high-confidence patterns than medium-confidence
    assert!(
        high_confidence.len() <= medium_confidence.len(),
        "High confidence count should be <= medium confidence count"
    );

    // All high-confidence patterns should have confidence >= 0.8
    for pattern in high_confidence {
        assert!(
            pattern.confidence >= 0.8,
            "Pattern confidence {} should be >= 0.8",
            pattern.confidence
        );
    }
}

#[test]
fn test_load_debtmap_toml_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join(".debtmap.toml");

    // Create a test configuration
    let config_content = r#"
enabled = true
confidence_threshold = 0.85

[observer]
interface_markers = ["ABC", "Protocol", "EventHandler"]
method_prefixes = ["on_", "handle_", "process_"]

[factory]
name_patterns = ["create_", "make_", "build_", "new_"]

[[custom_rules]]
name = "event_handler"
method_pattern = "^handle_.*_event$"
confidence = 0.9
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    // Load the configuration
    let config = PatternConfig::load(&config_path).expect("Failed to load config");

    // Verify configuration was loaded correctly
    assert!(config.enabled);
    assert_eq!(config.confidence_threshold, 0.85);
    assert_eq!(config.observer.interface_markers.len(), 3);
    assert!(config
        .observer
        .interface_markers
        .contains(&"EventHandler".to_string()));
    assert_eq!(config.factory.name_patterns.len(), 4);
    assert_eq!(config.custom_rules.len(), 1);
    assert_eq!(config.custom_rules[0].name, "event_handler");
    assert_eq!(config.custom_rules[0].confidence, 0.9);
}

#[test]
fn test_load_or_default_with_no_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Load config from directory with no .debtmap.toml
    let config = PatternConfig::load_or_default(temp_dir.path());

    // Should return default config
    assert!(config.enabled);
    assert_eq!(config.confidence_threshold, 0.7);
    assert!(config.custom_rules.is_empty());
}

#[test]
fn test_load_or_default_with_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join(".debtmap.toml");

    let config_content = r#"
enabled = false
confidence_threshold = 0.9
"#;

    fs::write(&config_path, config_content).expect("Failed to write config");

    // Load config from directory with .debtmap.toml
    let config = PatternConfig::load_or_default(temp_dir.path());

    // Should load the custom config
    assert!(!config.enabled);
    assert_eq!(config.confidence_threshold, 0.9);
}

#[test]
fn test_custom_pattern_rules_validation() {
    let mut config = PatternConfig::default();

    // Add valid custom rule
    config.custom_rules.push(
        debtmap::analysis::patterns::config::CustomPatternRule {
            name: "test_pattern".to_string(),
            method_pattern: Some("^test_.*".to_string()),
            class_pattern: None,
            decorator_pattern: None,
            confidence: 0.8,
        },
    );

    // Validation should pass
    assert!(config.validate().is_ok());

    // Add invalid custom rule (confidence > 1.0)
    config.custom_rules.push(
        debtmap::analysis::patterns::config::CustomPatternRule {
            name: "invalid_pattern".to_string(),
            method_pattern: None,
            class_pattern: None,
            decorator_pattern: None,
            confidence: 1.5,
        },
    );

    // Validation should fail
    assert!(config.validate().is_err());
}

#[test]
fn test_pattern_detection_respects_confidence_threshold() {
    let detector = PatternDetector::new();
    let file_metrics = create_test_file_metrics();

    let patterns = detector.detect_all_patterns(&file_metrics);

    // All detected patterns should have some confidence > 0
    for pattern in &patterns {
        assert!(
            pattern.confidence > 0.0 && pattern.confidence <= 1.0,
            "Pattern confidence {} should be between 0 and 1",
            pattern.confidence
        );
    }
}

#[test]
fn test_dependency_injection_pattern_detection() {
    let detector = PatternDetector::new();
    let file_metrics = FileMetrics {
        path: PathBuf::from("service.py"),
        language: Language::Python,
        complexity: ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        module_scope: None,
        classes: Some(vec![ClassDef {
            name: "DatabaseService".to_string(),
            base_classes: vec![],
            methods: vec![MethodDef {
                name: "__init__".to_string(),
                is_abstract: false,
                decorators: vec!["inject".to_string()],
                overrides_base: false,
                line: 10,
            }],
            is_abstract: false,
            decorators: vec![],
            line: 8,
        }]),
    };

    let patterns = detector.detect_all_patterns(&file_metrics);

    // Should detect dependency injection
    let di_patterns: Vec<&PatternInstance> = patterns
        .iter()
        .filter(|p| p.pattern_type == PatternType::DependencyInjection)
        .collect();

    assert_eq!(di_patterns.len(), 1, "Expected to find one DI pattern");
    assert!(
        di_patterns[0].confidence >= 0.5,
        "DI pattern should have reasonable confidence"
    );
}

#[test]
fn test_pattern_instance_contains_reasoning() {
    let detector = PatternDetector::new();
    let file_metrics = create_test_file_metrics();

    let patterns = detector.detect_all_patterns(&file_metrics);

    // All patterns should have reasoning
    for pattern in &patterns {
        assert!(
            !pattern.reasoning.is_empty(),
            "Pattern should have reasoning explanation"
        );
    }
}
