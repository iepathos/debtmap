use debtmap::organization::{
    calculate_god_object_score, determine_confidence, GodObjectConfidence, GodObjectDetector,
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

    assert_eq!(
        analysis.method_count, 25,
        "Should count 5 impl methods + 20 standalone functions"
    );
    assert_eq!(analysis.field_count, 2, "Should count 2 struct fields");
    // With complexity weighting, 25 simple functions (complexity 1 each) should have
    // a low weighted count (~4.8) which is well below the threshold of 20
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
