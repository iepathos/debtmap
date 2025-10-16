/// False Positive Validation Tests for Enhanced Python Dead Code Detection
///
/// These tests validate that the analyzer meets the <10% false positive rate
/// requirement (Spec 107) by testing against real-world Python patterns that
/// are commonly incorrectly flagged as dead code.

use debtmap::analysis::python_dead_code_enhanced::{
    DeadCodeConfidence, EnhancedDeadCodeAnalyzer,
};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;

/// Test result tracking for false positive validation
struct ValidationResult {
    function_name: String,
    expected_live: bool,
    actual_dead: bool,
    confidence: f32,
    is_false_positive: bool,
}

impl ValidationResult {
    fn new(
        function_name: String,
        expected_live: bool,
        actual_dead: bool,
        confidence: f32,
    ) -> Self {
        // False positive: Expected to be live but marked as dead with high confidence
        let is_false_positive = expected_live && actual_dead && confidence >= 0.8;

        Self {
            function_name,
            expected_live,
            actual_dead,
            confidence,
            is_false_positive,
        }
    }
}

/// Calculate false positive rate from validation results
fn calculate_false_positive_rate(results: &[ValidationResult]) -> f32 {
    let total_live_functions = results.iter().filter(|r| r.expected_live).count();
    let false_positives = results.iter().filter(|r| r.is_false_positive).count();

    if total_live_functions == 0 {
        0.0
    } else {
        (false_positives as f32) / (total_live_functions as f32)
    }
}

#[test]
fn test_framework_patterns_false_positive_rate() {
    // Test common framework patterns that should NOT be flagged as dead code
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let test_cases = vec![
        // Flask routes
        ("index", "app.py", true),
        ("api_users", "api.py", true),
        ("handle_error", "app.py", true),

        // Django views
        ("user_list", "views.py", true),
        ("save", "models.py", true),
        ("clean", "forms.py", true),

        // Click commands
        ("cli", "cli.py", true),
        ("run", "main.py", true),

        // Event handlers
        ("on_click", "gui.py", true),
        ("on_paint", "panel.py", true),

        // Actually dead code (for comparison)
        ("_old_implementation", "legacy.py", false),
        ("_unused_helper", "utils.py", false),
    ];

    let mut results = Vec::new();

    for (func_name, file_name, expected_live) in test_cases {
        let func = FunctionMetrics::new(
            func_name.to_string(),
            PathBuf::from(file_name),
            10,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        results.push(ValidationResult::new(
            func_name.to_string(),
            expected_live,
            result.is_dead,
            result.confidence.score(),
        ));
    }

    let fp_rate = calculate_false_positive_rate(&results);

    println!("Framework patterns false positive rate: {:.1}%", fp_rate * 100.0);

    // Report any false positives
    for result in &results {
        if result.is_false_positive {
            println!(
                "FALSE POSITIVE: {} (confidence: {:.2})",
                result.function_name, result.confidence
            );
        }
    }

    assert!(
        fp_rate < 0.10,
        "False positive rate {:.1}% exceeds 10% threshold",
        fp_rate * 100.0
    );
}

#[test]
fn test_magic_methods_false_positive_rate() {
    // Magic methods should NEVER be flagged as dead code
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let magic_methods = vec![
        "Cls.__init__",
        "Cls.__str__",
        "Cls.__repr__",
        "Cls.__eq__",
        "Cls.__hash__",
        "Cls.__getitem__",
        "Cls.__setitem__",
        "Cls.__len__",
        "Cls.__call__",
        "Cls.__enter__",
        "Cls.__exit__",
        "Cls.__iter__",
        "Cls.__next__",
        "Cls.__contains__",
        "Cls.__add__",
    ];

    let mut results = Vec::new();

    for method_name in magic_methods {
        let func = FunctionMetrics::new(
            method_name.to_string(),
            PathBuf::from("model.py"),
            10,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        results.push(ValidationResult::new(
            method_name.to_string(),
            true, // All magic methods should be live
            result.is_dead,
            result.confidence.score(),
        ));
    }

    let fp_rate = calculate_false_positive_rate(&results);

    println!("Magic methods false positive rate: {:.1}%", fp_rate * 100.0);

    // Magic methods should have 0% false positive rate
    assert_eq!(
        fp_rate, 0.0,
        "Magic methods should NEVER be false positives, but got {:.1}%",
        fp_rate * 100.0
    );
}

#[test]
fn test_callback_patterns_false_positive_rate() {
    // Test callback patterns with proper tracking
    let mut analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    // Simulate callback registration
    use debtmap::analysis::python_call_graph::callback_tracker::{
        CallbackContext, CallbackType, Location, PendingCallback,
    };

    let mut tracker = debtmap::analysis::python_call_graph::callback_tracker::CallbackTracker::new();

    // Register some callbacks
    tracker.track_decorator("app.route".to_string(), "index".to_string());
    tracker.track_decorator("click.command".to_string(), "cli".to_string());

    // Add callback registrations
    let callback1 = PendingCallback {
        callback_expr: "on_button_click".to_string(),
        registration_point: Location {
            file: PathBuf::from("gui.py"),
            line: 50,
            caller_function: Some("setup_ui".to_string()),
        },
        registration_type: CallbackType::EventBinding,
        context: CallbackContext {
            current_class: Some("MainWindow".to_string()),
            current_function: Some("setup_ui".to_string()),
            scope_variables: std::collections::HashMap::new(),
        },
        target_hint: None,
    };
    tracker.track_callback(callback1);

    analyzer.register_callback_tracker(tracker);

    let test_cases = vec![
        ("index", "app.py", true),           // Decorator target
        ("cli", "cli.py", true),             // Decorator target
        ("on_button_click", "gui.py", true), // Callback target
        ("_unrelated", "other.py", false),   // Not a callback
    ];

    let mut results = Vec::new();

    for (func_name, file_name, expected_live) in test_cases {
        let func = FunctionMetrics::new(
            func_name.to_string(),
            PathBuf::from(file_name),
            10,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        results.push(ValidationResult::new(
            func_name.to_string(),
            expected_live,
            result.is_dead,
            result.confidence.score(),
        ));
    }

    let fp_rate = calculate_false_positive_rate(&results);

    println!("Callback patterns false positive rate: {:.1}%", fp_rate * 100.0);

    assert!(
        fp_rate < 0.10,
        "Callback false positive rate {:.1}% exceeds 10% threshold",
        fp_rate * 100.0
    );
}

#[test]
fn test_comprehensive_validation_suite() {
    // Comprehensive test with a realistic mix of live and dead code
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let mut call_graph = CallGraph::new();

    // Setup some call relationships
    let main_id = FunctionId {
        name: "main".to_string(),
        file: PathBuf::from("app.py"),
        line: 10,
    };
    let helper_id = FunctionId {
        name: "process_data".to_string(),
        file: PathBuf::from("app.py"),
        line: 20,
    };
    call_graph.add_call(FunctionCall {
        caller: main_id.clone(),
        callee: helper_id.clone(),
        call_type: CallType::Direct,
    });

    // Test cases: (name, file, line, expected_live, description)
    let test_cases = vec![
        // LIVE CODE - Should NOT be flagged as dead
        ("main", "app.py", 10, true, "Entry point"),
        ("cli", "cli.py", 5, true, "CLI entry point"),
        ("run", "runner.py", 8, true, "Run entry point"),
        ("Cls.__init__", "model.py", 15, true, "Magic method"),
        ("Cls.__str__", "model.py", 25, true, "Magic method"),
        ("test_something", "test_app.py", 10, true, "Test function"),
        ("TestCase.setUp", "test_base.py", 5, true, "Test setup"),
        ("process_data", "app.py", 20, true, "Has caller (main)"),
        ("index", "routes.py", 30, true, "Framework route"),
        ("on_click", "ui.py", 40, true, "Event handler"),

        // DEAD CODE - Should be flagged as dead
        ("_old_helper", "utils.py", 50, false, "Unused private function"),
        ("_legacy_impl", "legacy.py", 60, false, "Old implementation"),
        ("_unused_calc", "math.py", 70, false, "Unused calculation"),
        ("deprecated_func", "old_api.py", 80, false, "Unused public (edge case)"),
    ];

    let mut results = Vec::new();
    let mut total_live = 0;
    let mut false_positives = 0;
    let mut false_negatives = 0;

    for (func_name, file_name, line, expected_live, description) in test_cases {
        let func = FunctionMetrics::new(
            func_name.to_string(),
            PathBuf::from(file_name),
            line,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        let validation = ValidationResult::new(
            func_name.to_string(),
            expected_live,
            result.is_dead,
            result.confidence.score(),
        );

        if expected_live {
            total_live += 1;
            if validation.is_false_positive {
                false_positives += 1;
                println!(
                    "FALSE POSITIVE: {} - {} (confidence: {:.2})",
                    func_name, description, result.confidence.score()
                );
            }
        } else {
            // Check for false negatives (dead code marked as live)
            if !result.is_dead {
                false_negatives += 1;
                println!(
                    "FALSE NEGATIVE: {} - {} (confidence: {:.2})",
                    func_name, description, result.confidence.score()
                );
            }
        }

        results.push(validation);
    }

    let fp_rate = if total_live > 0 {
        (false_positives as f32) / (total_live as f32)
    } else {
        0.0
    };

    println!("\n=== Comprehensive Validation Results ===");
    println!("Total live functions tested: {}", total_live);
    println!("False positives: {}", false_positives);
    println!("False positive rate: {:.1}%", fp_rate * 100.0);
    println!("False negatives: {}", false_negatives);
    println!("========================================\n");

    // Primary requirement: < 10% false positive rate
    assert!(
        fp_rate < 0.10,
        "False positive rate {:.1}% exceeds 10% threshold (Spec 107 requirement)",
        fp_rate * 100.0
    );

    // Secondary check: false negatives should be low too
    assert!(
        false_negatives <= 2,
        "Too many false negatives: {} (may indicate overly conservative detection)",
        false_negatives
    );
}

#[test]
fn test_test_file_detection() {
    // Test files should have lower false positive rates
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let test_functions = vec![
        ("test_addition", "test_math.py"),
        ("test_subtraction", "test_math.py"),
        ("helper", "test_utils.py"), // Helper in test file
        ("create_fixture", "test_fixtures.py"),
        ("setup_database", "test_db.py"),
    ];

    let mut results = Vec::new();

    for (func_name, file_name) in test_functions {
        let func = FunctionMetrics::new(
            func_name.to_string(),
            PathBuf::from(file_name),
            10,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        results.push(ValidationResult::new(
            func_name.to_string(),
            true, // All test-related functions should be live
            result.is_dead,
            result.confidence.score(),
        ));
    }

    let fp_rate = calculate_false_positive_rate(&results);

    println!("Test file functions false positive rate: {:.1}%", fp_rate * 100.0);

    // Test files should have very low false positive rate
    assert!(
        fp_rate < 0.10,
        "Test file false positive rate {:.1}% exceeds 10% threshold",
        fp_rate * 100.0
    );
}

#[test]
fn test_public_api_detection() {
    // Public API functions require careful handling
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    let api_functions = vec![
        ("calculate", "api.py", true),
        ("process", "api.py", true),
        ("_internal", "api.py", false), // Private, likely safe to flag
    ];

    let mut results = Vec::new();

    for (func_name, file_name, expected_live) in api_functions {
        let func = FunctionMetrics::new(
            func_name.to_string(),
            PathBuf::from(file_name),
            10,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        // Public API should either:
        // 1. Not be marked as dead, OR
        // 2. Have medium/low confidence (not high)
        let is_safe = !result.is_dead || result.confidence.score() < 0.8;

        if !is_safe && expected_live {
            println!(
                "WARNING: Public API {} marked dead with high confidence {:.2}",
                func_name,
                result.confidence.score()
            );
        }

        results.push(ValidationResult::new(
            func_name.to_string(),
            expected_live,
            result.is_dead,
            result.confidence.score(),
        ));
    }

    let fp_rate = calculate_false_positive_rate(&results);

    println!("Public API false positive rate: {:.1}%", fp_rate * 100.0);

    assert!(
        fp_rate < 0.10,
        "Public API false positive rate {:.1}% exceeds 10% threshold",
        fp_rate * 100.0
    );
}

#[test]
fn test_property_decorators() {
    // Property decorators should not be flagged as dead
    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let call_graph = CallGraph::new();

    // Note: Property detection requires source file reading
    // For this test, we verify that the detection doesn't incorrectly
    // flag properties with high confidence when they're public

    let property_like = vec![
        ("full_name", "model.py", true),
        ("email", "user.py", true),
        ("computed_value", "calculator.py", true),
    ];

    let mut results = Vec::new();

    for (func_name, file_name, expected_live) in property_like {
        let func = FunctionMetrics::new(
            func_name.to_string(),
            PathBuf::from(file_name),
            10,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        results.push(ValidationResult::new(
            func_name.to_string(),
            expected_live,
            result.is_dead,
            result.confidence.score(),
        ));
    }

    let fp_rate = calculate_false_positive_rate(&results);

    println!("Property-like functions false positive rate: {:.1}%", fp_rate * 100.0);

    // Should be conservative with property-like functions
    assert!(
        fp_rate < 0.10,
        "Property-like functions false positive rate {:.1}% exceeds 10% threshold",
        fp_rate * 100.0
    );
}

#[test]
fn test_spec_107_requirement() {
    // Final validation test that directly addresses Spec 107 requirement:
    // "False positive rate should be < 10%"

    println!("\n=== Spec 107 Validation Test ===");
    println!("Requirement: False positive rate < 10% for Python dead code detection");
    println!("Testing against diverse real-world Python patterns...\n");

    let analyzer = EnhancedDeadCodeAnalyzer::new();
    let mut call_graph = CallGraph::new();

    // Setup realistic call graph
    let main_id = FunctionId {
        name: "main".to_string(),
        file: PathBuf::from("app.py"),
        line: 1,
    };
    let util_id = FunctionId {
        name: "utility".to_string(),
        file: PathBuf::from("utils.py"),
        line: 10,
    };
    call_graph.add_call(FunctionCall {
        caller: main_id,
        callee: util_id,
        call_type: CallType::Direct,
    });

    // 50 test cases covering various patterns
    let test_cases: Vec<(&str, &str, usize, bool)> = vec![
        // Entry points (10)
        ("main", "app.py", 1, true),
        ("cli", "cli.py", 1, true),
        ("run", "runner.py", 1, true),
        ("app.main", "app.py", 5, true),
        ("script.run", "script.py", 1, true),
        ("entrypoint", "main.py", 1, true),
        ("start", "server.py", 1, true),
        ("execute", "executor.py", 1, true),
        ("launch", "launcher.py", 1, true),
        ("bootstrap", "bootstrap.py", 1, true),

        // Magic methods (10)
        ("C.__init__", "m.py", 5, true),
        ("C.__str__", "m.py", 10, true),
        ("C.__repr__", "m.py", 15, true),
        ("C.__eq__", "m.py", 20, true),
        ("C.__hash__", "m.py", 25, true),
        ("C.__getitem__", "m.py", 30, true),
        ("C.__len__", "m.py", 35, true),
        ("C.__enter__", "m.py", 40, true),
        ("C.__exit__", "m.py", 45, true),
        ("C.__call__", "m.py", 50, true),

        // Framework patterns (10)
        ("index", "routes.py", 10, true),
        ("api_handler", "api.py", 20, true),
        ("on_click", "gui.py", 30, true),
        ("on_paint", "panel.py", 40, true),
        ("handle_request", "server.py", 50, true),
        ("process_event", "events.py", 60, true),
        ("setup_view", "views.py", 70, true),
        ("configure", "config.py", 80, true),
        ("initialize", "init.py", 90, true),
        ("finalize", "cleanup.py", 100, true),

        // Test functions (5)
        ("test_func", "test_a.py", 10, true),
        ("test_case", "test_b.py", 20, true),
        ("TestC.test_m", "test_c.py", 30, true),
        ("TestD.setUp", "test_d.py", 40, true),
        ("fixture", "test_fixtures.py", 50, true),

        // Called functions (5)
        ("utility", "utils.py", 10, true),
        ("helper", "helpers.py", 20, true),
        ("process", "processor.py", 30, true),
        ("validate", "validator.py", 40, true),
        ("transform", "transformer.py", 50, true),

        // Dead code (10)
        ("_old1", "old.py", 10, false),
        ("_old2", "old.py", 20, false),
        ("_unused1", "unused.py", 30, false),
        ("_unused2", "unused.py", 40, false),
        ("_legacy1", "legacy.py", 50, false),
        ("_legacy2", "legacy.py", 60, false),
        ("_deprecated1", "dep.py", 70, false),
        ("_deprecated2", "dep.py", 80, false),
        ("_temp1", "temp.py", 90, false),
        ("_temp2", "temp.py", 100, false),
    ];

    let mut live_count = 0;
    let mut false_positives = 0;

    for (name, file, line, expected_live) in test_cases {
        if expected_live {
            live_count += 1;
        }

        let func = FunctionMetrics::new(
            name.to_string(),
            PathBuf::from(file),
            line,
        );

        let result = analyzer.analyze_function(&func, &call_graph);

        // False positive: expected live but marked dead with high confidence
        if expected_live && result.is_dead && result.confidence.score() >= 0.8 {
            false_positives += 1;
            println!("FALSE POSITIVE: {} (confidence: {:.2})", name, result.confidence.score());
        }
    }

    let fp_rate = (false_positives as f32) / (live_count as f32);

    println!("\n=== Results ===");
    println!("Total live functions: {}", live_count);
    println!("False positives: {}", false_positives);
    println!("False positive rate: {:.1}%", fp_rate * 100.0);
    println!("=================\n");

    if fp_rate < 0.10 {
        println!("✅ PASS: Meets Spec 107 requirement (<10% false positive rate)");
    } else {
        println!("❌ FAIL: Does not meet Spec 107 requirement");
    }

    assert!(
        fp_rate < 0.10,
        "SPEC 107 FAILURE: False positive rate {:.1}% exceeds 10% requirement",
        fp_rate * 100.0
    );
}
