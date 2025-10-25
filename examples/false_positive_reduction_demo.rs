use debtmap::complexity::threshold_manager::{ComplexityThresholds, FunctionRole, ThresholdPreset};
use debtmap::core::FunctionMetrics;
use std::path::PathBuf;

fn main() {
    println!("=== False Positive Reduction Demo ===\n");

    // Example functions that would previously be flagged but shouldn't be
    let trivial_functions = vec![
        create_getter(),
        create_setter(),
        create_simple_validator(),
        create_small_util(),
        create_test_helper(),
    ];

    // Test with different threshold presets
    let presets = vec![
        ("Strict", ThresholdPreset::Strict),
        ("Balanced", ThresholdPreset::Balanced),
        ("Lenient", ThresholdPreset::Lenient),
    ];

    for (preset_name, preset) in presets {
        let thresholds = ComplexityThresholds::from_preset(preset);
        println!("\n## Using {} Thresholds", preset_name);
        println!(
            "Minimum cyclomatic: {}",
            thresholds.minimum_cyclomatic_complexity
        );
        println!(
            "Minimum cognitive: {}",
            thresholds.minimum_cognitive_complexity
        );
        println!(
            "Minimum function length: {}\n",
            thresholds.minimum_function_length
        );

        let mut flagged_count = 0;
        let mut not_flagged_count = 0;

        for func in &trivial_functions {
            let role = determine_role(&func.name);
            let should_flag = thresholds.should_flag_function(func, role.clone());

            if should_flag {
                flagged_count += 1;
                println!(
                    "  ❌ FLAGGED: {} (cyclo: {}, cog: {}, lines: {})",
                    func.name, func.cyclomatic, func.cognitive, func.length
                );
            } else {
                not_flagged_count += 1;
                println!(
                    "  ✅ NOT FLAGGED: {} (cyclo: {}, cog: {}, lines: {})",
                    func.name, func.cyclomatic, func.cognitive, func.length
                );
            }
        }

        println!(
            "\n  Summary: {} flagged, {} not flagged",
            flagged_count, not_flagged_count
        );

        // Calculate false positive reduction
        let reduction_percentage =
            (not_flagged_count as f64 / trivial_functions.len() as f64) * 100.0;
        println!("  False Positive Reduction: {:.1}%", reduction_percentage);
    }

    println!("\n=== Complex Functions (Should Always Be Flagged) ===\n");

    let complex_functions = vec![
        create_complex_handler(),
        create_nested_logic(),
        create_long_switch(),
    ];

    let balanced_thresholds = ComplexityThresholds::from_preset(ThresholdPreset::Balanced);

    for func in &complex_functions {
        let role = determine_role(&func.name);
        let should_flag = balanced_thresholds.should_flag_function(func, role.clone());

        if should_flag {
            println!(
                "  ✅ CORRECTLY FLAGGED: {} (cyclo: {}, cog: {}, lines: {})",
                func.name, func.cyclomatic, func.cognitive, func.length
            );
        } else {
            println!(
                "  ❌ MISSED: {} (cyclo: {}, cog: {}, lines: {})",
                func.name, func.cyclomatic, func.cognitive, func.length
            );
        }
    }

    println!("\n=== Role-Based Adjustment Demo ===\n");

    let moderate_func = create_moderate_complexity();
    let roles = vec![
        FunctionRole::Test,
        FunctionRole::EntryPoint,
        FunctionRole::CoreLogic,
        FunctionRole::Utility,
    ];

    for role in roles {
        let should_flag = balanced_thresholds.should_flag_function(&moderate_func, role.clone());
        let multiplier = balanced_thresholds.get_role_multiplier(role.clone());

        println!("  Function '{}' as {:?}:", moderate_func.name, role);
        println!("    Multiplier: {:.1}x", multiplier);
        println!("    Flagged: {}", if should_flag { "Yes" } else { "No" });
    }
}

fn determine_role(name: &str) -> FunctionRole {
    FunctionRole::from_name(name)
}

fn create_getter() -> FunctionMetrics {
    FunctionMetrics {
        name: "get_value".to_string(),
        file: PathBuf::from("lib.rs"),
        line: 10,
        cyclomatic: 1,
        cognitive: 0,
        nesting: 0,
        length: 3,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(true),
        purity_confidence: Some(1.0),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_setter() -> FunctionMetrics {
    FunctionMetrics {
        name: "set_value".to_string(),
        file: PathBuf::from("lib.rs"),
        line: 20,
        cyclomatic: 1,
        cognitive: 1,
        nesting: 0,
        length: 4,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.9),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_simple_validator() -> FunctionMetrics {
    FunctionMetrics {
        name: "is_valid".to_string(),
        file: PathBuf::from("validators.rs"),
        line: 5,
        cyclomatic: 3,
        cognitive: 2,
        nesting: 1,
        length: 8,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(true),
        purity_confidence: Some(0.95),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_small_util() -> FunctionMetrics {
    FunctionMetrics {
        name: "format_string".to_string(),
        file: PathBuf::from("utils.rs"),
        line: 15,
        cyclomatic: 2,
        cognitive: 3,
        nesting: 1,
        length: 10,
        is_test: false,
        visibility: Some("pub(crate)".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(true),
        purity_confidence: Some(0.85),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_test_helper() -> FunctionMetrics {
    FunctionMetrics {
        name: "test_setup".to_string(),
        file: PathBuf::from("tests/helpers.rs"),
        line: 30,
        cyclomatic: 4,
        cognitive: 5,
        nesting: 2,
        length: 15,
        is_test: true,
        visibility: None,
        is_trait_method: false,
        in_test_module: true,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.5),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_complex_handler() -> FunctionMetrics {
    FunctionMetrics {
        name: "handle_request".to_string(),
        file: PathBuf::from("handlers.rs"),
        line: 100,
        cyclomatic: 15,
        cognitive: 25,
        nesting: 4,
        length: 120,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.3),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_nested_logic() -> FunctionMetrics {
    FunctionMetrics {
        name: "process_data".to_string(),
        file: PathBuf::from("processor.rs"),
        line: 50,
        cyclomatic: 12,
        cognitive: 20,
        nesting: 5,
        length: 80,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.4),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_long_switch() -> FunctionMetrics {
    FunctionMetrics {
        name: "categorize_items".to_string(),
        file: PathBuf::from("categorizer.rs"),
        line: 200,
        cyclomatic: 20,
        cognitive: 15,
        nesting: 2,
        length: 150,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(true),
        purity_confidence: Some(0.8),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}

fn create_moderate_complexity() -> FunctionMetrics {
    FunctionMetrics {
        name: "calculate_score".to_string(),
        file: PathBuf::from("scoring.rs"),
        line: 75,
        cyclomatic: 6,
        cognitive: 8,
        nesting: 2,
        length: 25,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(true),
        purity_confidence: Some(0.9),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
    }
}
