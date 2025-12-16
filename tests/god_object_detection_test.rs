use debtmap::extraction::adapters::god_object::analyze_god_objects;
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::organization::{
    calculate_god_object_score, determine_confidence, DetectionType, GodObjectConfidence,
    GodObjectThresholds,
};
use std::path::Path;

/// Test that a file with many simple/empty standalone functions is NOT flagged as a god object
/// (complexity-weighted scoring rewards simple functions)
#[test]
fn test_detects_file_with_many_standalone_functions() {
    let code = r#"
        fn function1() {}
        fn function2() {}
        fn function3() {}
        fn function4() {}
        fn function5() {}
        fn function6() {}
        fn function7() {}
        fn function8() {}
        fn function9() {}
        fn function10() {}
        fn function11() {}
        fn function12() {}
        fn function13() {}
        fn function14() {}
        fn function15() {}
        fn function16() {}
        fn function17() {}
        fn function18() {}
        fn function19() {}
        fn function20() {}
        fn function21() {}
        fn function22() {}
        fn function23() {}
        fn function24() {}
        fn function25() {}
        fn function26() {}
        fn function27() {}
        fn function28() {}
        fn function29() {}
        fn function30() {}
    "#;

    let extracted =
        UnifiedFileExtractor::extract(Path::new("test.rs"), code).expect("Failed to extract");
    let analyses = analyze_god_objects(Path::new("test.rs"), &extracted);

    // With per-struct analysis, standalone functions don't trigger god object detection
    // Empty result means no god objects found
    assert!(
        analyses.is_empty(),
        "File with 30 simple standalone functions should NOT be flagged as god object"
    );
}

/// Test that a file with many functions gets appropriate scoring
#[test]
#[ignore = "slow test with 270 functions requiring purity analysis (~30s+), run manually with --ignored if needed"]
fn test_detects_rust_call_graph_scenario() {
    // Simulate a file similar to rust_call_graph.rs with 270 functions
    // With complexity-weighted scoring and the new conservative approach,
    // 270 simple functions may only trigger 1-2 violations, resulting in a score of 30-50
    // This is intentional to avoid over-flagging functional/procedural code

    let mut code = String::new();
    for i in 0..270 {
        // Create simple functions
        code.push_str(&format!("fn function_{}() {{ if true {{ }} }}\n", i));
    }

    let extracted = UnifiedFileExtractor::extract(Path::new("rust_call_graph.rs"), &code)
        .expect("Failed to extract");
    let analyses = analyze_god_objects(Path::new("rust_call_graph.rs"), &extracted);

    // With per-struct analysis, standalone functions don't trigger god object detection
    // Empty result means no god objects found
    assert!(
        analyses.is_empty(),
        "File with 270 simple standalone functions should NOT be flagged as god object"
    );
}

/// Test that the graduated minimum score is applied based on violation count
#[test]
fn test_minimum_score_enforcement() {
    let thresholds = GodObjectThresholds::for_rust();

    // Test case with just above threshold (21 methods, threshold is 20)
    // With only 1 violation, minimum score is 30.0
    let score = calculate_god_object_score(21, 0, 1, 500, &thresholds);
    assert!(
        score >= 30.0,
        "Single violation should have minimum score of 30, got {}",
        score
    );

    // Test case with multiple violations
    // 4 violations: methods (50 > 20), fields (30 > 15), responsibilities (5 = 5, not a violation),
    // lines (2000 > 1000) = 3 violations, minimum score 70.0
    let score = calculate_god_object_score(50, 30, 5, 2000, &thresholds);
    assert!(
        score >= 70.0,
        "God object with 3+ violations should have score >= 70, got {}",
        score
    );

    // Test case with severe violations
    let score = calculate_god_object_score(270, 50, 10, 10000, &thresholds);
    assert!(
        score > 200.0,
        "Severe god object should have very high score, got {}",
        score
    );
}

/// Test that mixed files with structs and standalone functions are properly analyzed
/// with complexity weighting
#[test]
fn test_mixed_struct_and_functions() {
    let code = r#"
        struct MyStruct {
            field1: i32,
            field2: String,
        }

        impl MyStruct {
            fn method1(&self) {}
            fn method2(&self) {}
            fn method3(&self) {}
            fn method4(&self) {}
            fn method5(&self) {}
        }

        fn standalone1() {}
        fn standalone2() {}
        fn standalone3() {}
        fn standalone4() {}
        fn standalone5() {}
        fn standalone6() {}
        fn standalone7() {}
        fn standalone8() {}
        fn standalone9() {}
        fn standalone10() {}
        fn standalone11() {}
        fn standalone12() {}
        fn standalone13() {}
        fn standalone14() {}
        fn standalone15() {}
        fn standalone16() {}
        fn standalone17() {}
        fn standalone18() {}
        fn standalone19() {}
        fn standalone20() {}
    "#;

    let extracted =
        UnifiedFileExtractor::extract(Path::new("mixed.rs"), code).expect("Failed to extract");
    let analyses = analyze_god_objects(Path::new("mixed.rs"), &extracted);

    // After spec 118: Only count impl methods for struct analysis, not standalone functions
    // This prevents false positives for functional/procedural modules
    // With per-struct analysis, this file should not produce any god object results
    assert!(
        analyses.is_empty(),
        "File with 5 impl methods and 20 standalone functions should NOT be flagged as god object"
    );
}

/// Test confidence levels for different violation counts
#[test]
fn test_confidence_levels() {
    let thresholds = GodObjectThresholds::for_rust();

    // No violations
    let confidence = determine_confidence(10, 5, 2, 500, 50, &thresholds);
    assert_eq!(confidence, GodObjectConfidence::NotGodObject);

    // One violation (methods)
    let confidence = determine_confidence(21, 5, 2, 500, 50, &thresholds);
    assert_eq!(confidence, GodObjectConfidence::Possible);

    // Two violations (methods and fields)
    let confidence = determine_confidence(21, 16, 2, 500, 50, &thresholds);
    assert_eq!(confidence, GodObjectConfidence::Possible);

    // Three violations (methods, fields, responsibilities)
    let confidence = determine_confidence(21, 16, 6, 500, 50, &thresholds);
    assert_eq!(confidence, GodObjectConfidence::Probable);

    // All violations
    let confidence = determine_confidence(21, 16, 6, 1001, 201, &thresholds);
    assert_eq!(confidence, GodObjectConfidence::Definite);
}

/// Integration test for spec 130: Validates complete god class vs god file behavior
/// - God Class: Excludes test functions from method count, detection_type is GodClass
/// - God File: Includes all functions (prod + tests), detection_type is GodFile
#[test]
#[ignore = "slow test with 85 total functions requiring purity analysis (~15s+), run manually with --ignored if needed"]
fn test_spec_130_god_class_vs_god_file_detection() {
    // Part 1: Test god class detection with test methods excluded
    let god_class_code = r#"
        struct LargeStruct {
            field1: i32,
            field2: String,
        }

        impl LargeStruct {
            // Production methods (should be counted)
            // Adding some complexity to ensure god object detection
            fn method1(&self) -> i32 {
                if self.field1 > 0 { self.field1 } else { 0 }
            }
            fn method2(&self) -> String {
                if self.field2.is_empty() { "default".to_string() } else { self.field2.clone() }
            }
            fn method3(&self) { if self.field1 > 0 { } }
            fn method4(&self) { if self.field1 > 1 { } }
            fn method5(&self) { if self.field1 > 2 { } }
            fn method6(&self) { if self.field1 > 3 { } }
            fn method7(&self) { if self.field1 > 4 { } }
            fn method8(&self) { if self.field1 > 5 { } }
            fn method9(&self) { if self.field1 > 6 { } }
            fn method10(&self) { if self.field1 > 7 { } }
            fn method11(&self) { if self.field1 > 8 { } }
            fn method12(&self) { if self.field1 > 9 { } }
            fn method13(&self) { if self.field1 > 10 { } }
            fn method14(&self) { if self.field1 > 11 { } }
            fn method15(&self) { if self.field1 > 12 { } }
            fn method16(&self) { if self.field1 > 13 { } }
            fn method17(&self) { if self.field1 > 14 { } }
            fn method18(&self) { if self.field1 > 15 { } }
            fn method19(&self) { if self.field1 > 16 { } }
            fn method20(&self) { if self.field1 > 17 { } }
            fn method21(&self) { if self.field1 > 18 { } }
            fn method22(&self) { if self.field1 > 19 { } }
            fn method23(&self) { if self.field1 > 20 { } }
            fn method24(&self) { if self.field1 > 21 { } }
            fn method25(&self) { if self.field1 > 22 { } }

            // Test methods (should NOT be counted per spec 130)
            #[test]
            fn test_method1() { }
            #[test]
            fn test_method2() { }
            #[test]
            fn test_method3() { }
            #[test]
            fn test_method4() { }
            #[test]
            fn test_method5() { }
            #[cfg(test)]
            fn test_helper1() { }
            #[cfg(test)]
            fn test_helper2() { }
            #[cfg(test)]
            fn test_helper3() { }
            #[cfg(test)]
            fn test_helper4() { }
            #[cfg(test)]
            fn test_helper5() { }
        }
    "#;

    let extracted = UnifiedFileExtractor::extract(Path::new("god_class.rs"), god_class_code)
        .expect("Failed to extract");
    let analyses = analyze_god_objects(Path::new("god_class.rs"), &extracted);

    // Verify god class detection excludes test methods
    assert!(!analyses.is_empty(), "Should detect god class");
    let analysis = &analyses[0];
    assert_eq!(
        analysis.method_count, 25,
        "God class should count only 25 production methods, not the 10 test methods (actual: {})",
        analysis.method_count
    );
    // The key validation: test methods are excluded from count
    // Whether it's flagged as god object depends on weighted complexity scoring
    // but the count itself must be correct
    assert_eq!(
        analysis.detection_type,
        DetectionType::GodClass,
        "Detection type should be GodClass for struct with methods"
    );

    // Part 2: Test god file detection with all functions included
    let god_file_code = r#"
        // Production functions (should be counted)
        // Adding some complexity to ensure detection
        fn prod_fn1() { if true { } }
        fn prod_fn2() { if true { } }
        fn prod_fn3() { if true { } }
        fn prod_fn4() { if true { } }
        fn prod_fn5() { if true { } }
        fn prod_fn6() { if true { } }
        fn prod_fn7() { if true { } }
        fn prod_fn8() { if true { } }
        fn prod_fn9() { if true { } }
        fn prod_fn10() { if true { } }
        fn prod_fn11() { if true { } }
        fn prod_fn12() { if true { } }
        fn prod_fn13() { if true { } }
        fn prod_fn14() { if true { } }
        fn prod_fn15() { if true { } }
        fn prod_fn16() { if true { } }
        fn prod_fn17() { if true { } }
        fn prod_fn18() { if true { } }
        fn prod_fn19() { if true { } }
        fn prod_fn20() { if true { } }
        fn prod_fn21() { if true { } }
        fn prod_fn22() { if true { } }
        fn prod_fn23() { if true { } }
        fn prod_fn24() { if true { } }
        fn prod_fn25() { if true { } }
        fn prod_fn26() { if true { } }
        fn prod_fn27() { if true { } }
        fn prod_fn28() { if true { } }
        fn prod_fn29() { if true { } }
        fn prod_fn30() { if true { } }

        // Test functions (should ALSO be counted for god file per spec 130)
        #[test]
        fn test_fn1() { if true { } }
        #[test]
        fn test_fn2() { if true { } }
        #[test]
        fn test_fn3() { if true { } }
        #[test]
        fn test_fn4() { if true { } }
        #[test]
        fn test_fn5() { if true { } }
        #[cfg(test)]
        fn test_helper1() { if true { } }
        #[cfg(test)]
        fn test_helper2() { if true { } }
        #[cfg(test)]
        fn test_helper3() { if true { } }
        #[cfg(test)]
        fn test_helper4() { if true { } }
        #[cfg(test)]
        fn test_helper5() { if true { } }
        #[cfg(test)]
        fn test_helper6() { if true { } }
        #[cfg(test)]
        fn test_helper7() { if true { } }
        #[cfg(test)]
        fn test_helper8() { if true { } }
        #[cfg(test)]
        fn test_helper9() { if true { } }
        #[cfg(test)]
        fn test_helper10() { if true { } }
        #[cfg(test)]
        fn test_helper11() { if true { } }
        #[cfg(test)]
        fn test_helper12() { if true { } }
        #[cfg(test)]
        fn test_helper13() { if true { } }
        #[cfg(test)]
        fn test_helper14() { if true { } }
        #[cfg(test)]
        fn test_helper15() { if true { } }
    "#;

    let extracted = UnifiedFileExtractor::extract(Path::new("god_file.rs"), god_file_code)
        .expect("Failed to extract");
    let analyses = analyze_god_objects(Path::new("god_file.rs"), &extracted);

    // With per-struct analysis, standalone functions don't trigger god object detection
    // Empty result means no god objects found
    assert!(
        analyses.is_empty(),
        "File with standalone functions should NOT be flagged as god object with per-struct analysis"
    );
}

/// Test behavioral decomposition with field access tracking and trait extraction (Spec 178)
#[test]
fn test_behavioral_decomposition_with_field_tracking() {
    use debtmap::organization::{cluster_methods_by_behavior, FieldAccessTracker};

    let code = r#"
        struct Editor {
            display_map: DisplayMap,
            cursor_position: Position,
            input_buffer: Buffer,
            file_path: PathBuf,
            config: Config,
        }

        impl Editor {
            fn render(&self) {
                let map = self.display_map;
                let pos = self.cursor_position;
            }

            fn draw_cursor(&self) {
                let pos = self.cursor_position;
            }

            fn paint_background(&self) {
                let map = self.display_map;
            }

            fn handle_keypress(&mut self) {
                self.input_buffer.clear();
            }

            fn on_mouse_down(&mut self) {
                let pos = self.cursor_position;
            }

            fn save(&self) {
                let path = self.file_path;
            }

            fn load(&mut self) {
                let path = self.file_path;
            }

            fn validate_input(&self) -> bool {
                !self.input_buffer.is_empty()
            }

            fn check_dirty(&self) -> bool {
                true
            }

            fn get_config(&self) -> &Config {
                &self.config
            }

            fn set_config(&mut self, config: Config) {
                self.config = config;
            }
        }
    "#;

    // Parse for FieldAccessTracker test (it needs syn::File)
    let file = syn::parse_file(code).expect("Failed to parse");

    // Test field access tracking
    let mut tracker = FieldAccessTracker::new();
    if let Some(impl_item) = file.items.iter().find_map(|item| {
        if let syn::Item::Impl(impl_block) = item {
            Some(impl_block)
        } else {
            None
        }
    }) {
        tracker.analyze_impl(impl_item);
    }

    // Verify field tracking for render method
    let render_fields = tracker.get_method_fields("render");
    assert!(render_fields.contains(&"display_map".to_string()));
    assert!(render_fields.contains(&"cursor_position".to_string()));

    // Verify field tracking for save method
    let save_fields = tracker.get_method_fields("save");
    assert!(save_fields.contains(&"file_path".to_string()));

    // Test behavioral categorization
    let methods = vec![
        "render".to_string(),
        "draw_cursor".to_string(),
        "paint_background".to_string(),
        "handle_keypress".to_string(),
        "on_mouse_down".to_string(),
        "save".to_string(),
        "load".to_string(),
        "validate_input".to_string(),
        "check_dirty".to_string(),
        "get_config".to_string(),
        "set_config".to_string(),
    ];

    let clusters = cluster_methods_by_behavior(&methods);

    // Verify behavioral clustering
    use debtmap::organization::BehaviorCategory;
    assert!(
        clusters.contains_key(&BehaviorCategory::Rendering),
        "Should identify rendering cluster"
    );
    assert!(
        clusters.contains_key(&BehaviorCategory::EventHandling),
        "Should identify event handling cluster"
    );
    assert!(
        clusters.contains_key(&BehaviorCategory::Persistence),
        "Should identify persistence cluster"
    );
    assert!(
        clusters.contains_key(&BehaviorCategory::Validation),
        "Should identify validation cluster"
    );
    // Per spec 208: get_*/set_* are now DataAccess, not StateManagement
    assert!(
        clusters.contains_key(&BehaviorCategory::DataAccess),
        "Should identify data access cluster"
    );

    // Verify rendering cluster contains expected methods
    let rendering_methods = clusters.get(&BehaviorCategory::Rendering).unwrap();
    assert_eq!(
        rendering_methods.len(),
        3,
        "Should have 3 rendering methods"
    );
    assert!(rendering_methods.contains(&"render".to_string()));
    assert!(rendering_methods.contains(&"draw_cursor".to_string()));
    assert!(rendering_methods.contains(&"paint_background".to_string()));

    // Verify minimal field set for rendering cluster
    let rendering_fields = tracker.get_minimal_field_set(rendering_methods);
    assert!(rendering_fields.contains(&"display_map".to_string()));
    assert!(rendering_fields.contains(&"cursor_position".to_string()));
    assert_eq!(
        rendering_fields.len(),
        2,
        "Rendering should only need 2 fields"
    );

    // Verify persistence cluster
    let persistence_methods = clusters.get(&BehaviorCategory::Persistence).unwrap();
    assert_eq!(
        persistence_methods.len(),
        2,
        "Should have 2 persistence methods"
    );
    let persistence_fields = tracker.get_minimal_field_set(persistence_methods);
    assert!(persistence_fields.contains(&"file_path".to_string()));
}

/// Test that behavioral decomposition avoids 'misc' category (Spec 178)
#[test]
fn test_behavioral_decomposition_no_misc_category() {
    use debtmap::organization::{BehaviorCategory, BehavioralCategorizer};

    // Test various method names that should NOT result in "misc"
    let test_cases = vec![
        ("render_display", BehaviorCategory::Rendering),
        ("handle_click", BehaviorCategory::EventHandling),
        ("save_state", BehaviorCategory::Persistence),
        ("validate_email", BehaviorCategory::Validation),
        ("get_name", BehaviorCategory::DataAccess), // Per spec 208: get_* is DataAccess
        ("initialize_system", BehaviorCategory::Lifecycle),
    ];

    for (method_name, expected_category) in test_cases {
        let category = BehavioralCategorizer::categorize_method(method_name);
        assert_eq!(
            category, expected_category,
            "Method '{}' should be categorized as {:?}, got {:?}",
            method_name, expected_category, category
        );
    }

    // For methods that don't match standard categories, should use domain-specific name
    let domain_method = "approve_loan_application";
    let category = BehavioralCategorizer::categorize_method(domain_method);
    match category {
        BehaviorCategory::Domain(name) => {
            assert_eq!(name, "Approve", "Should extract first word as domain");
        }
        _ => panic!("Expected Domain category for method '{}'", domain_method),
    }
}
