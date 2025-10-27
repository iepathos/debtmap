use debtmap::organization::boilerplate_detector::BoilerplateDetector;
/// Debug test to understand what the trait pattern analyzer sees
use debtmap::organization::trait_pattern_analyzer::TraitPatternAnalyzer;
use std::path::Path;

#[test]
fn debug_trait_pattern_detection() {
    let code = r#"
        pub trait Flag {
            fn name(&self) -> &str;
        }

        pub struct Flag1;
        impl Flag for Flag1 {
            fn name(&self) -> &str { "flag1" }
        }

        pub struct Flag2;
        impl Flag for Flag2 {
            fn name(&self) -> &str { "flag2" }
        }

        pub struct Flag3;
        impl Flag for Flag3 {
            fn name(&self) -> &str { "flag3" }
        }
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse");
    let metrics = TraitPatternAnalyzer::analyze_file(&syntax);

    println!("\n=== TRAIT PATTERN ANALYSIS (3 impls) ===");
    println!("Impl block count: {}", metrics.impl_block_count);
    println!("Unique traits: {:?}", metrics.unique_traits);
    println!("Most common trait: {:?}", metrics.most_common_trait);
    println!("Method uniformity: {:.2}", metrics.method_uniformity);
    println!("Shared methods: {:?}", metrics.shared_methods);
    println!(
        "Avg method complexity: {:.2}",
        metrics.avg_method_complexity
    );
    println!("Complexity variance: {:.2}", metrics.complexity_variance);
    println!("Avg method lines: {:.2}", metrics.avg_method_lines);
    println!("========================================\n");

    // With 3 impl blocks of Flag trait, we should see:
    assert_eq!(metrics.impl_block_count, 3);
    assert!(metrics.unique_traits.contains("Flag"));
    assert!(metrics.method_uniformity > 0.9); // All 3 impls have "name" method
}

#[test]
fn debug_25_flag_structs() {
    // Copy the exact code from our failing test
    let code = include_str!("../tests/test_data/25_flag_structs.txt");

    let syntax = syn::parse_file(code).expect("Failed to parse 25 flag structs");

    // First check what the trait analyzer sees
    let metrics = TraitPatternAnalyzer::analyze_file(&syntax);

    println!("\n=== TRAIT PATTERN ANALYSIS (25 Flag impls) ===");
    println!("Impl block count: {}", metrics.impl_block_count);
    println!("Unique traits: {:?}", metrics.unique_traits);
    println!("Most common trait: {:?}", metrics.most_common_trait);
    println!("Method uniformity: {:.2}", metrics.method_uniformity);
    println!(
        "Shared methods (first 10): {:?}",
        &metrics.shared_methods[..metrics.shared_methods.len().min(10)]
    );
    println!(
        "Avg method complexity: {:.2}",
        metrics.avg_method_complexity
    );
    println!("Complexity variance: {:.2}", metrics.complexity_variance);
    println!("Avg method lines: {:.2}", metrics.avg_method_lines);
    println!("=============================================\n");

    // Now check what the boilerplate detector sees
    let detector = BoilerplateDetector::default();
    let result = detector.detect(Path::new("test_25_flags.rs"), &syntax);

    println!("=== BOILERPLATE DETECTION ===");
    println!("Is boilerplate: {}", result.is_boilerplate);
    println!("Confidence: {:.1}%", result.confidence * 100.0);
    println!("Signals: {:?}", result.signals);
    println!("=============================\n");

    // Debug: what are the thresholds?
    println!("=== DETECTOR CONFIG ===");
    println!("Min impl blocks: {}", detector.min_impl_blocks);
    println!(
        "Method uniformity threshold: {:.2}",
        detector.method_uniformity_threshold
    );
    println!("Max avg complexity: {:.2}", detector.max_avg_complexity);
    println!("Confidence threshold: {:.2}", detector.confidence_threshold);
    println!("=======================\n");

    // Should have 25 impl blocks
    assert_eq!(metrics.impl_block_count, 25, "Should detect 25 impl blocks");

    // Should detect Flag trait
    assert!(
        metrics.unique_traits.contains("Flag"),
        "Should detect Flag trait"
    );

    // Should have high method uniformity (all impl the same 5 methods)
    assert!(
        metrics.method_uniformity >= 0.9,
        "Method uniformity should be >= 0.9, got {:.2}",
        metrics.method_uniformity
    );
}
