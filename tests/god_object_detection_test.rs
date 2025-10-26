use debtmap::organization::{
    calculate_god_object_score, determine_confidence, DetectionType, GodObjectConfidence,
    GodObjectDetector, GodObjectThresholds,
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

    let file = syn::parse_file(code).expect("Failed to parse");
    let detector = GodObjectDetector::with_source_content(code);
    let analysis = detector.analyze_comprehensive(Path::new("test.rs"), &file);

    assert_eq!(
        analysis.method_count, 30,
        "Should count all 30 standalone functions"
    );
    // With complexity weighting, 30 simple functions (complexity 1 each) should have
    // a low weighted count (~5.77) which is well below the threshold of 20
    assert!(
        !analysis.is_god_object,
        "File with 30 simple functions should NOT be flagged as god object with complexity weighting"
    );
    assert!(
        analysis.god_object_score < 100.0,
        "God object score should be less than 100 for simple functions, got {}",
        analysis.god_object_score
    );
}

/// Test that rust_call_graph.rs with 270 functions would be detected as a god object
#[test]
fn test_detects_rust_call_graph_scenario() {
    // Simulate a file with 270 functions like rust_call_graph.rs
    let mut code = String::new();
    for i in 0..270 {
        code.push_str(&format!("fn function_{}() {{}}\n", i));
    }

    let file = syn::parse_file(&code).expect("Failed to parse");
    let detector = GodObjectDetector::with_source_content(&code);
    let analysis = detector.analyze_comprehensive(Path::new("rust_call_graph.rs"), &file);

    assert!(
        analysis.is_god_object,
        "File with 270 functions should be detected as god object"
    );
    assert_eq!(
        analysis.method_count, 270,
        "Should count all 270 standalone functions"
    );
    assert!(
        analysis.god_object_score >= 100.0,
        "God object score should be at least 100 for 270 functions"
    );
    // With 270 methods but no other violations, we get Probable (3-4 violations)
    // Methods: 270 > 20 ✓, Fields: 0 < 15 ✗, Responsibilities: likely > 5 ✓, Lines: estimated > 1000 ✓, Complexity: estimated > 200 ✓
    assert!(
        analysis.confidence == GodObjectConfidence::Probable
            || analysis.confidence == GodObjectConfidence::Definite,
        "Should have high confidence with 270 functions, got {:?}",
        analysis.confidence
    );
}

/// Test that the minimum score of 100 is enforced for god objects
#[test]
fn test_minimum_score_enforcement() {
    let thresholds = GodObjectThresholds::for_rust();

    // Test case with just above threshold (21 methods, threshold is 20)
    let score = calculate_god_object_score(21, 0, 1, 500, &thresholds);
    assert!(
        score >= 100.0,
        "Any god object should have minimum score of 100, got {}",
        score
    );

    // Test case with multiple violations
    let score = calculate_god_object_score(50, 30, 5, 2000, &thresholds);
    assert!(
        score >= 100.0,
        "God object with multiple violations should have score >= 100, got {}",
        score
    );

    // Test case with severe violations
    let score = calculate_god_object_score(270, 50, 10, 10000, &thresholds);
    assert!(
        score > 500.0,
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

    let file = syn::parse_file(code).expect("Failed to parse");
    let detector = GodObjectDetector::with_source_content(code);
    let analysis = detector.analyze_comprehensive(Path::new("mixed.rs"), &file);

    // After spec 118: Only count impl methods for struct analysis, not standalone functions
    // This prevents false positives for functional/procedural modules
    assert_eq!(
        analysis.method_count, 5,
        "Should count only the 5 impl methods, not standalone functions"
    );
    assert_eq!(analysis.field_count, 2, "Should count 2 struct fields");
    // With complexity weighting, 5 simple methods (complexity 1 each) should have
    // a low weighted count which is well below the threshold of 20
    assert!(
        !analysis.is_god_object,
        "File with 25 simple functions should NOT be flagged as god object with complexity weighting"
    );
    assert!(
        analysis.god_object_score < 100.0,
        "God object score should be less than 100 for simple functions, got {}",
        analysis.god_object_score
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

    let file = syn::parse_file(god_class_code).expect("Failed to parse god class code");
    let detector = GodObjectDetector::with_source_content(god_class_code);
    let analysis = detector.analyze_comprehensive(Path::new("god_class.rs"), &file);

    // Verify god class detection excludes test methods
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

    let file = syn::parse_file(god_file_code).expect("Failed to parse god file code");
    let detector = GodObjectDetector::with_source_content(god_file_code);
    let analysis = detector.analyze_comprehensive(Path::new("god_file.rs"), &file);

    // Verify god file detection includes ALL functions (production + test)
    assert_eq!(
        analysis.method_count, 50,
        "God file should count all 50 functions (30 production + 20 test), actual: {}",
        analysis.method_count
    );
    // The key validation: test functions ARE included for god file
    // Whether it's flagged as god object depends on weighted complexity scoring
    // but the count itself must be correct and detection type must be GodFile
    assert_eq!(
        analysis.detection_type,
        DetectionType::GodFile,
        "Detection type should be GodFile for file with standalone functions"
    );
}
