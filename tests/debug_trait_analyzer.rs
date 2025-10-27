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

    // Debug: manually calculate confidence
    println!("=== MANUAL CONFIDENCE CALCULATION ===");
    let mut score = 0.0;

    // Signal 1: impl count
    if metrics.impl_block_count >= detector.min_impl_blocks {
        let normalized = (metrics.impl_block_count as f64 / 100.0).min(1.0);
        let contribution = 30.0 * normalized;
        println!(
            "Signal 1 (impl count >= {}): +{:.2} (normalized: {:.2})",
            detector.min_impl_blocks, contribution, normalized
        );
        score += contribution;
    } else {
        println!(
            "Signal 1 FAILED: {} < {}",
            metrics.impl_block_count, detector.min_impl_blocks
        );
    }

    // Signal 2: uniformity
    if metrics.method_uniformity >= detector.method_uniformity_threshold {
        let contribution = 25.0 * metrics.method_uniformity;
        println!(
            "Signal 2 (uniformity >= {:.2}): +{:.2}",
            detector.method_uniformity_threshold, contribution
        );
        score += contribution;
    } else {
        println!(
            "Signal 2 FAILED: {:.2} < {:.2}",
            metrics.method_uniformity, detector.method_uniformity_threshold
        );
    }

    // Signal 3: complexity
    if metrics.avg_method_complexity < detector.max_avg_complexity {
        let inverse_complexity =
            1.0 - (metrics.avg_method_complexity / detector.max_avg_complexity);
        let contribution = 20.0 * inverse_complexity;
        println!(
            "Signal 3 (complexity < {:.2}): +{:.2} (inverse: {:.2})",
            detector.max_avg_complexity, contribution, inverse_complexity
        );
        score += contribution;
    } else {
        println!(
            "Signal 3 FAILED: {:.2} >= {:.2}",
            metrics.avg_method_complexity, detector.max_avg_complexity
        );
    }

    // Signal 4: variance
    if metrics.complexity_variance < 2.0 {
        let normalized = 1.0 - (metrics.complexity_variance / 10.0).min(1.0);
        let contribution = 15.0 * normalized;
        println!(
            "Signal 4 (variance < 2.0): +{:.2} (normalized: {:.2})",
            contribution, normalized
        );
        score += contribution;
    } else {
        println!("Signal 4 FAILED: {:.2} >= 2.0", metrics.complexity_variance);
    }

    // Signal 5: dominant trait
    if let Some((_, count)) = &metrics.most_common_trait {
        let ratio = *count as f64 / metrics.impl_block_count as f64;
        if ratio > 0.8 {
            let contribution = 10.0 * ratio;
            println!(
                "Signal 5 (ratio > 0.8): +{:.2} (ratio: {:.2})",
                contribution, ratio
            );
            score += contribution;
        } else {
            println!("Signal 5 FAILED: ratio {:.2} <= 0.8", ratio);
        }
    } else {
        println!("Signal 5 FAILED: no most_common_trait");
    }

    let final_confidence = (score / 100.0).min(1.0);
    println!(
        "Total score: {:.2}/100.0 = {:.2}% confidence",
        score,
        final_confidence * 100.0
    );
    println!("======================================\n");

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
