use debtmap::core::FunctionMetrics;
use debtmap::priority::external_api_detector::{
    generate_enhanced_dead_code_hints_with_config, is_likely_external_api_with_config,
    ExternalApiConfig,
};
use debtmap::priority::FunctionVisibility;
use std::path::PathBuf;

fn create_function(name: &str, path: &str, visibility: Option<String>) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from(path),
        line: 1,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 2,
        length: 20,
        is_test: false,
        visibility,
        is_trait_method: false,
        in_test_module: false,
    }
}

// Create a test config with detection enabled (for test isolation)
fn test_config() -> ExternalApiConfig {
    ExternalApiConfig {
        detect_external_api: true,
        api_functions: vec![],
        api_files: vec![],
    }
}

#[test]
fn test_lib_rs_functions_are_apis() {
    let test_cases = vec![
        ("new", "src/lib.rs"),
        ("from_string", "src/lib.rs"),
        ("process", "crate/lib.rs"),
        ("analyze", "my_crate/src/lib.rs"),
    ];

    for (name, path) in test_cases {
        let func = create_function(name, path, Some("pub".to_string()));
        let visibility = FunctionVisibility::Public;
        let (is_api, indicators) =
            is_likely_external_api_with_config(&func, &visibility, &test_config());

        assert!(
            is_api,
            "Function '{name}' in '{path}' should be detected as API"
        );
        assert!(
            indicators.iter().any(|i| i.contains("lib.rs")),
            "Should have lib.rs indicator for '{name}' in '{path}'"
        );
    }
}

#[test]
fn test_mod_rs_functions_are_likely_apis() {
    let test_cases = vec![
        ("init", "src/mod.rs"),
        ("create_instance", "src/parser/mod.rs"),
        ("configure", "src/config/mod.rs"),
    ];

    for (name, path) in test_cases {
        let func = create_function(name, path, Some("pub".to_string()));
        let visibility = FunctionVisibility::Public;
        let (_is_api, indicators) =
            is_likely_external_api_with_config(&func, &visibility, &test_config());

        assert!(
            indicators.iter().any(|i| i.contains("mod.rs")),
            "Should have mod.rs indicator for '{name}' in '{path}'"
        );
    }
}

#[test]
fn test_common_constructors_are_apis() {
    let constructors = vec![
        "new",
        "new_with_config",
        "default",
        "from_str",
        "from_path",
        "from_config",
        "init",
        "init_with_options",
        "builder",
        "build",
    ];

    for name in constructors {
        let func = create_function(name, "src/any/path.rs", Some("pub".to_string()));
        let visibility = FunctionVisibility::Public;
        let (is_api, indicators) =
            is_likely_external_api_with_config(&func, &visibility, &test_config());

        assert!(is_api, "Constructor '{name}' should be detected as API");
        assert!(
            indicators.iter().any(|i| i.contains("Constructor")
                || i.contains("Builder")
                || i.contains("trait method")),
            "Should have constructor/builder indicator for '{name}'"
        );
    }
}

#[test]
fn test_api_pattern_prefixes() {
    let api_functions = vec![
        ("get_value", "getter"),
        ("set_value", "setter"),
        ("with_option", "builder pattern"),
        ("try_parse", "fallible operation"),
        ("is_valid", "predicate"),
        ("has_feature", "predicate"),
        ("create_instance", "factory"),
        ("parse_input", "parser"),
        ("to_string", "conversion"),
        ("into_iter", "conversion"),
        ("as_ref", "conversion"),
    ];

    for (name, expected_pattern) in api_functions {
        let func = create_function(name, "src/module.rs", Some("pub".to_string()));
        let visibility = FunctionVisibility::Public;
        let (_is_api, indicators) =
            is_likely_external_api_with_config(&func, &visibility, &test_config());

        assert!(
            indicators.iter().any(|i| i.contains("API pattern")),
            "Function '{name}' should have API pattern indicator ({expected_pattern})"
        );
    }
}

#[test]
fn test_internal_paths_not_apis() {
    let internal_paths = vec![
        "src/internal/helper.rs",
        "src/private/utils.rs",
        "src/impl/detail.rs",
        "src/detail/internal.rs",
        "src/tests/fixtures.rs",
        "src/benches/utils.rs",
        "src/examples/demo.rs",
    ];

    for path in internal_paths {
        let func = create_function("public_func", path, Some("pub".to_string()));
        let visibility = FunctionVisibility::Public;
        let (is_api, indicators) =
            is_likely_external_api_with_config(&func, &visibility, &test_config());

        assert!(
            !is_api,
            "Function in '{path}' should NOT be detected as API"
        );

        // Should still get some indicators but not enough to be classified as API
        assert!(
            !indicators.is_empty() || path.contains("internal") || path.contains("private"),
            "Should have some analysis for '{path}'"
        );
    }
}

#[test]
fn test_deep_vs_shallow_paths() {
    let shallow_api = create_function("process", "src/processor.rs", Some("pub".to_string()));
    let deep_internal = create_function(
        "process",
        "src/core/internal/impl/detail/utils/processor.rs",
        Some("pub".to_string()),
    );

    let visibility = FunctionVisibility::Public;

    let (shallow_is_api, shallow_indicators) =
        is_likely_external_api_with_config(&shallow_api, &visibility, &test_config());
    let (deep_is_api, deep_indicators) =
        is_likely_external_api_with_config(&deep_internal, &visibility, &test_config());

    assert!(
        shallow_indicators
            .iter()
            .any(|i| i.contains("Shallow module path")),
        "Shallow path should be noted"
    );

    assert!(
        deep_indicators
            .iter()
            .any(|i| i.contains("Deep module path")),
        "Deep path should be noted"
    );

    // Shallow paths are more likely to be APIs
    assert!(
        shallow_is_api || !deep_is_api,
        "Shallow paths should be more likely APIs than deep paths"
    );
}

#[test]
fn test_non_public_functions_never_apis() {
    let visibilities = vec![
        (FunctionVisibility::Private, None),
        (FunctionVisibility::Crate, Some("pub(crate)".to_string())),
        (FunctionVisibility::Private, Some("pub(super)".to_string())),
    ];

    for (vis_enum, vis_string) in visibilities {
        let func = create_function("new", "src/lib.rs", vis_string);
        let (is_api, indicators) =
            is_likely_external_api_with_config(&func, &vis_enum, &test_config());

        assert!(
            !is_api,
            "Non-public functions should never be external APIs"
        );
        assert!(
            indicators.is_empty(),
            "Non-public functions should have no API indicators"
        );
    }
}

#[test]
fn test_specific_api_prefixes() {
    let api_prefixed = vec!["public_interface", "api_endpoint", "export_data"];

    for name in api_prefixed {
        let func = create_function(name, "src/some/path.rs", Some("pub".to_string()));
        let visibility = FunctionVisibility::Public;
        let (_is_api, indicators) =
            is_likely_external_api_with_config(&func, &visibility, &test_config());

        assert!(
            indicators.iter().any(|i| i.contains("Public API prefix")),
            "Function '{name}' should have public API prefix indicator"
        );
    }
}

#[test]
fn test_enhanced_hints_generation() {
    // Test that enhanced hints include API detection info
    let api_func = create_function("new", "src/lib.rs", Some("pub".to_string()));
    let visibility = FunctionVisibility::Public;

    let hints =
        generate_enhanced_dead_code_hints_with_config(&api_func, &visibility, &test_config());

    assert!(
        hints.iter().any(|h| h.contains("Likely external API")),
        "Hints should indicate likely external API"
    );
    assert!(
        hints.iter().any(|h| h.contains("lib.rs")),
        "Hints should mention specific indicators"
    );
}

#[test]
fn test_test_helper_detection() {
    let test_helpers = vec![
        "test_helper",
        "create_test_fixture",
        "mock_util",
        "setup_test_fixture",
    ];

    for name in test_helpers {
        let func = create_function(name, "src/utils.rs", Some("pub".to_string()));
        let visibility = FunctionVisibility::Public;

        let hints =
            generate_enhanced_dead_code_hints_with_config(&func, &visibility, &test_config());

        assert!(
            hints.iter().any(|h| h.contains("test helper")),
            "Function '{name}' should be identified as potential test helper"
        );
    }
}

#[test]
fn test_complexity_impact_hints() {
    // Low complexity function
    let mut simple_func = create_function("simple", "src/utils.rs", Some("pub".to_string()));
    simple_func.cyclomatic = 2;
    simple_func.cognitive = 3;

    let visibility = FunctionVisibility::Public;
    let hints =
        generate_enhanced_dead_code_hints_with_config(&simple_func, &visibility, &test_config());

    assert!(
        hints.iter().any(|h| h.contains("Low complexity")),
        "Low complexity functions should be noted"
    );

    // High complexity function
    let mut complex_func = create_function("complex", "src/utils.rs", Some("pub".to_string()));
    complex_func.cyclomatic = 15;
    complex_func.cognitive = 20;

    let hints_complex =
        generate_enhanced_dead_code_hints_with_config(&complex_func, &visibility, &test_config());

    assert!(
        hints_complex.iter().any(|h| h.contains("High complexity")),
        "High complexity functions should be noted"
    );
}

#[test]
fn test_score_threshold_for_api_detection() {
    // This test verifies the scoring threshold logic
    // A function needs enough indicators to be considered an API

    // Case 1: Single indicator - not enough
    let func1 = create_function(
        "random_name",
        "src/deep/nested/path.rs",
        Some("pub".to_string()),
    );
    let visibility = FunctionVisibility::Public;
    let (is_api1, _) = is_likely_external_api_with_config(&func1, &visibility, &test_config());
    assert!(
        !is_api1,
        "Single indicator shouldn't be enough for API classification"
    );

    // Case 2: Multiple strong indicators - should be API
    let func2 = create_function("new", "src/lib.rs", Some("pub".to_string()));
    let (is_api2, indicators2) =
        is_likely_external_api_with_config(&func2, &visibility, &test_config());
    assert!(is_api2, "Multiple strong indicators should classify as API");
    assert!(indicators2.len() >= 3, "Should have multiple indicators");

    // Case 3: Medium indicators - should be API
    let func3 = create_function("get_config", "src/mod.rs", Some("pub".to_string()));
    let (is_api3, indicators3) =
        is_likely_external_api_with_config(&func3, &visibility, &test_config());
    assert!(is_api3, "Medium strength indicators should be enough");
    assert!(indicators3.len() >= 2, "Should have at least 2 indicators");
}

#[test]
fn test_real_world_patterns() {
    // Test against patterns we'd see in real Rust libraries

    // Pattern from serde
    let deserialize = create_function("deserialize", "src/de/mod.rs", Some("pub".to_string()));
    let visibility = FunctionVisibility::Public;
    let (is_api, _) = is_likely_external_api_with_config(&deserialize, &visibility, &test_config());
    assert!(is_api, "Common trait methods in mod.rs should be APIs");

    // Pattern from tokio
    let spawn = create_function("spawn", "src/runtime/mod.rs", Some("pub".to_string()));
    let (is_api2, _) = is_likely_external_api_with_config(&spawn, &visibility, &test_config());
    assert!(is_api2, "Runtime spawn functions should be APIs");

    // Internal implementation detail
    let internal = create_function(
        "poll_next",
        "src/runtime/task/core/detail.rs",
        Some("pub".to_string()),
    );
    let (is_api3, _) = is_likely_external_api_with_config(&internal, &visibility, &test_config());
    assert!(!is_api3, "Deep internal implementation should not be API");
}
