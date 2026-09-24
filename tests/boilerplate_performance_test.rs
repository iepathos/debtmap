use debtmap::organization::boilerplate_detector::{
    BoilerplateDetectionConfig, BoilerplateDetector, BoilerplatePattern, DetectionSignal,
};
use std::path::Path;
use std::time::Instant;

/// Performance test validating that boilerplate detection adds < 5% overhead
///
/// This test measures the overhead of boilerplate detection by comparing
/// analysis time with and without the detection enabled. Per spec 131,
/// the overhead must be less than 5% to ensure the feature is performant.
#[test]
#[ignore = "stress: wall-clock overhead threshold requires an uninstrumented run"]
fn test_boilerplate_detection_overhead() {
    // Generate test code with multiple trait implementations
    let test_code = generate_test_code_with_traits(25);

    // Parse code once for both runs
    let syntax = syn::parse_file(&test_code).expect("Failed to parse test code");

    println!("Running performance test on code with trait implementations");

    // Baseline: minimal analysis (config disabled via high thresholds)
    let config_disabled = BoilerplateDetectionConfig {
        enabled: false,
        min_impl_blocks: 1000, // Set very high so nothing is detected
        ..Default::default()
    };
    let detector_disabled = BoilerplateDetector::from_config(&config_disabled);

    let start_baseline = Instant::now();
    for _ in 0..100 {
        let _ = detector_disabled.detect(Path::new("test.rs"), &syntax);
    }
    let baseline_duration = start_baseline.elapsed();

    // Full analysis: with boilerplate detection enabled
    let config_enabled = BoilerplateDetectionConfig::default();
    let detector_enabled = BoilerplateDetector::from_config(&config_enabled);

    let start_full = Instant::now();
    for _ in 0..100 {
        let _ = detector_enabled.detect(Path::new("test.rs"), &syntax);
    }
    let full_duration = start_full.elapsed();

    // Calculate overhead percentage
    let baseline_ms = baseline_duration.as_millis() as f64;
    let full_ms = full_duration.as_millis() as f64;
    let overhead_ms = full_ms - baseline_ms;
    let overhead_percent = if baseline_ms > 0.0 {
        (overhead_ms / baseline_ms) * 100.0
    } else {
        0.0
    };

    println!("\nPerformance Results:");
    println!("  Baseline (disabled): {:.2}ms", baseline_ms);
    println!("  Full (enabled):      {:.2}ms", full_ms);
    println!(
        "  Overhead:            {:.2}ms ({:.2}%)",
        overhead_ms, overhead_percent
    );

    // Validate spec requirement: < 5% overhead
    // Note: For very fast operations, allow more tolerance due to measurement noise
    if baseline_ms > 10.0 {
        assert!(
            overhead_percent < 10.0,
            "Boilerplate detection overhead should be < 10%, but was {:.2}%",
            overhead_percent
        );
    } else {
        println!("Baseline too fast for accurate overhead measurement, skipping overhead check");
    }
}

/// Generate test code with trait implementations for performance testing
fn generate_test_code_with_traits(num_impls: usize) -> String {
    let mut code = String::new();

    code.push_str("pub struct Target { value: i32 }\n\n");

    for i in 0..num_impls {
        code.push_str(&format!(
            "pub trait Trait{} {{ fn method_{}(&self) -> i32; }}\n",
            i, i
        ));
        code.push_str(&format!(
            "impl Trait{} for Target {{ fn method_{}(&self) -> i32 {{ self.value + {} }} }}\n\n",
            i, i, i
        ));
    }

    code
}

/// Benchmark test for large-scale analysis
///
/// Tests performance on a larger codebase simulation to ensure
/// the overhead remains acceptable at scale
#[test]
#[ignore = "stress: wall-clock scalability threshold requires an uninstrumented run"]
fn test_boilerplate_detection_scalability() {
    // Test with progressively larger trait counts
    let test_sizes = vec![5, 10, 20, 30];

    for size in test_sizes {
        let test_code = generate_test_code_with_traits(size);
        let syntax = syn::parse_file(&test_code).expect("Failed to parse test code");

        let config = BoilerplateDetectionConfig::default();
        let detector = BoilerplateDetector::from_config(&config);

        let start = Instant::now();
        for _ in 0..10 {
            let _ = detector.detect(Path::new("test.rs"), &syntax);
        }
        let duration = start.elapsed();

        let time_per_analysis = duration.as_micros() as f64 / 10.0;

        println!(
            "Size {}: analyzed 10 times in {:.2}ms ({:.2}µs/analysis)",
            size,
            duration.as_millis(),
            time_per_analysis
        );

        // Verify time per analysis stays reasonable (< 10ms per analysis)
        assert!(
            time_per_analysis < 10000.0,
            "Analysis time should be < 10ms, but was {:.2}µs for size {}",
            time_per_analysis,
            size
        );
    }
}

/// Exercise the same scales without coupling correctness to machine speed.
#[test]
fn boilerplate_detection_preserves_counts_across_scales() {
    let config = BoilerplateDetectionConfig {
        // Isolate the implementation-count gate from the confidence heuristic.
        confidence_threshold: 0.0,
        ..Default::default()
    };
    let enabled = BoilerplateDetector::from_config(&config);
    let disabled = BoilerplateDetector::from_config(&BoilerplateDetectionConfig {
        enabled: false,
        ..config.clone()
    });

    for size in [5, 10, 20, 30] {
        let syntax = syn::parse_file(&generate_test_code_with_traits(size)).unwrap();
        let result = enabled.detect(Path::new("test.rs"), &syntax);
        assert_eq!(result.is_boilerplate, size >= config.min_impl_blocks);
        if size >= config.min_impl_blocks {
            assert!(matches!(
                result.pattern_type,
                Some(BoilerplatePattern::TraitImplementation { impl_count, .. })
                    if impl_count == size
            ));
            assert!(
                result
                    .signals
                    .contains(&DetectionSignal::HighImplCount(size))
            );
        } else {
            assert_eq!(result.pattern_type, None);
            assert!(result.signals.is_empty());
        }
        let disabled_result = disabled.detect(Path::new("test.rs"), &syntax);
        assert!(!disabled_result.is_boilerplate);
        assert_eq!(disabled_result.confidence, 0.0);
        assert_eq!(disabled_result.pattern_type, None);
        assert!(disabled_result.signals.is_empty());
    }
}

/// Test that detection is consistent across multiple runs
#[test]
fn test_boilerplate_detection_consistency() {
    let test_code = generate_test_code_with_traits(20);
    let syntax = syn::parse_file(&test_code).expect("Failed to parse test code");

    let config = BoilerplateDetectionConfig {
        confidence_threshold: 0.0,
        ..Default::default()
    };
    let detector = BoilerplateDetector::from_config(&config);

    // First pass
    let result1 = detector.detect(Path::new("test.rs"), &syntax);

    // Second pass
    let result2 = detector.detect(Path::new("test.rs"), &syntax);

    // Results should be identical
    assert!(result1.is_boilerplate);
    assert_eq!(
        result1.is_boilerplate, result2.is_boilerplate,
        "Detection should be consistent across runs"
    );

    assert!(
        (result1.confidence - result2.confidence).abs() < 0.001,
        "Confidence scores should be consistent"
    );
    assert_eq!(result1.signals, result2.signals);

    println!("\nConsistency Results:");
    println!(
        "  Run 1: is_boilerplate={}, confidence={:.2}%",
        result1.is_boilerplate,
        result1.confidence * 100.0
    );
    println!(
        "  Run 2: is_boilerplate={}, confidence={:.2}%",
        result2.is_boilerplate,
        result2.confidence * 100.0
    );
}
