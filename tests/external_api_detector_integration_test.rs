/// Integration tests for external API detection in the dead code analysis workflow
use debtmap::core::FunctionMetrics;
use debtmap::priority::{
    external_api_detector::{generate_enhanced_dead_code_hints, is_likely_external_api},
    FunctionVisibility,
};
use std::path::PathBuf;

fn create_test_function(name: &str, path: &str, visibility: Option<String>) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from(path),
        line: 1,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility,
    }
}

#[test]
fn test_workflow_public_api_in_lib_rs() {
    // Simulate a public function in lib.rs
    let func = create_test_function("new", "src/lib.rs", Some("pub".to_string()));
    let visibility = FunctionVisibility::Public;

    // Check if it's detected as external API
    let (is_api, indicators) = is_likely_external_api(&func, &visibility);
    assert!(is_api, "Constructor in lib.rs should be external API");
    assert!(!indicators.is_empty(), "Should have indicators");

    // Verify enhanced hints
    let hints = generate_enhanced_dead_code_hints(&func, &visibility);
    assert!(
        hints.iter().any(|h| h.contains("Likely external API")),
        "Hints should indicate external API: {hints:?}"
    );
}

#[test]
fn test_workflow_internal_public_function() {
    // Simulate a public function in an internal module
    let func = create_test_function(
        "helper",
        "src/internal/impl/utils.rs",
        Some("pub".to_string()),
    );
    let visibility = FunctionVisibility::Public;

    // Check it's NOT detected as external API
    let (is_api, _) = is_likely_external_api(&func, &visibility);
    assert!(
        !is_api,
        "Internal module function should not be external API"
    );

    // Verify hints don't suggest it's an API
    let hints = generate_enhanced_dead_code_hints(&func, &visibility);

    // Should still get analyzed but not flagged as likely API
    let has_api_warning = hints.iter().any(|h| h.contains("⚠️ Likely external API"));
    assert!(
        !has_api_warning,
        "Should not have API warning for internal function"
    );
}

#[test]
fn test_workflow_private_function() {
    // Private functions should never be external APIs
    let func = create_test_function("internal_helper", "src/utils.rs", None);
    let visibility = FunctionVisibility::Private;

    let (is_api, indicators) = is_likely_external_api(&func, &visibility);
    assert!(!is_api, "Private functions cannot be external APIs");
    assert!(
        indicators.is_empty(),
        "Private functions should have no API indicators"
    );

    let hints = generate_enhanced_dead_code_hints(&func, &visibility);
    assert!(
        !hints.iter().any(|h| h.contains("external API")),
        "Private functions should not mention external API"
    );
}

#[test]
fn test_workflow_mod_rs_with_api_pattern() {
    // Function in mod.rs with API pattern - likely external API
    let func = create_test_function(
        "get_configuration",
        "src/config/mod.rs",
        Some("pub".to_string()),
    );
    let visibility = FunctionVisibility::Public;

    let (is_api, indicators) = is_likely_external_api(&func, &visibility);
    assert!(is_api, "get_* function in mod.rs should be detected as API");

    // Should have multiple indicators
    assert!(
        indicators.len() >= 2,
        "Should have multiple indicators: {indicators:?}"
    );

    // Check specific indicators
    assert!(
        indicators.iter().any(|i| i.contains("mod.rs")),
        "Should mention mod.rs"
    );
    assert!(
        indicators.iter().any(|i| i.contains("API pattern")),
        "Should mention API pattern"
    );
}

#[test]
fn test_action_recommendations_based_on_api_detection() {
    // Test that the action recommendations differ based on API detection

    // API function
    let api_func = create_test_function("builder", "src/lib.rs", Some("pub".to_string()));
    let pub_vis = FunctionVisibility::Public;

    let (is_api, _) = is_likely_external_api(&api_func, &pub_vis);
    assert!(is_api);

    // The action for API functions should be to verify before removal
    // This would be used in generate_dead_code_action
    let expected_action = if is_api {
        "Verify external usage before removal"
    } else {
        "Remove unused public function"
    };
    assert_eq!(expected_action, "Verify external usage before removal");

    // Non-API public function
    let non_api_func = create_test_function(
        "internal_helper",
        "src/internal/utils.rs",
        Some("pub".to_string()),
    );

    let (is_api2, _) = is_likely_external_api(&non_api_func, &pub_vis);
    assert!(!is_api2);

    let expected_action2 = if is_api2 {
        "Verify external usage before removal"
    } else {
        "Remove unused public function"
    };
    assert_eq!(expected_action2, "Remove unused public function");
}

#[test]
fn test_complexity_hints_integration() {
    // Test that complexity hints are integrated with API detection
    let mut func = create_test_function("process", "src/lib.rs", Some("pub".to_string()));

    // Low complexity
    func.cyclomatic = 2;
    func.cognitive = 3;
    let visibility = FunctionVisibility::Public;

    let hints = generate_enhanced_dead_code_hints(&func, &visibility);
    assert!(
        hints.iter().any(|h| h.contains("Low complexity")),
        "Should indicate low complexity"
    );

    // High complexity
    func.cyclomatic = 15;
    func.cognitive = 20;

    let hints_high = generate_enhanced_dead_code_hints(&func, &visibility);
    assert!(
        hints_high.iter().any(|h| h.contains("High complexity")),
        "Should indicate high complexity"
    );

    // Both should still show API detection
    assert!(
        hints.iter().any(|h| h.contains("Likely external API")),
        "Low complexity API should still be flagged"
    );
    assert!(
        hints_high.iter().any(|h| h.contains("Likely external API")),
        "High complexity API should still be flagged"
    );
}
