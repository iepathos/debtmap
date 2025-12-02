//! Performance benchmarks for state field detection (Spec 202)
//!
//! Measures the overhead of enhanced state field detection with multi-strategy analysis.
//! Target: < 5ms per-function overhead for state detection.

use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::analyzers::state_field_detector::{StateDetectionConfig, StateFieldDetector};
use std::hint::black_box as hint_black_box;
use syn::{parse_quote, ExprField};

/// Generate field access expressions for benchmarking
fn generate_field_accesses() -> Vec<ExprField> {
    vec![
        // State-related fields (should be detected)
        parse_quote! { self.state },
        parse_quote! { self.current_state },
        parse_quote! { self.status },
        parse_quote! { self.phase },
        parse_quote! { self.mode },
        parse_quote! { self.connection_state },
        parse_quote! { self.task_status },
        parse_quote! { self.lifecycle_phase },
        // Non-state fields (should not be detected)
        parse_quote! { self.data },
        parse_quote! { self.config },
        parse_quote! { self.buffer },
        parse_quote! { self.counter },
        parse_quote! { self.name },
        parse_quote! { self.id },
        parse_quote! { self.timestamp },
        parse_quote! { self.value },
        // Edge cases
        parse_quote! { self.is_active },
        parse_quote! { self.has_state },
        parse_quote! { self.state_machine },
        parse_quote! { self.current_mode },
    ]
}

/// Generate a larger corpus of field accesses for throughput testing
fn generate_large_corpus(size: usize) -> Vec<ExprField> {
    let base_fields = generate_field_accesses();
    let mut corpus = Vec::with_capacity(size);

    for i in 0..size {
        let field = &base_fields[i % base_fields.len()];
        corpus.push(field.clone());
    }

    corpus
}

/// Benchmark baseline state field detection (keyword-only)
fn bench_baseline_detection(c: &mut Criterion) {
    let config = StateDetectionConfig {
        use_type_analysis: false,
        use_frequency_analysis: false,
        use_pattern_recognition: false,
        min_enum_variants: 3,
        custom_keywords: vec![],
        custom_patterns: vec![],
    };

    let detector = StateFieldDetector::new(config);
    let fields = generate_field_accesses();

    c.bench_function("baseline_keyword_detection", |b| {
        b.iter(|| {
            for field in &fields {
                hint_black_box(detector.detect_state_field(field));
            }
        })
    });
}

/// Benchmark enhanced state field detection (all strategies)
fn bench_enhanced_detection(c: &mut Criterion) {
    let config = StateDetectionConfig::default();
    let detector = StateFieldDetector::new(config);
    let fields = generate_field_accesses();

    c.bench_function("enhanced_multi_strategy_detection", |b| {
        b.iter(|| {
            for field in &fields {
                hint_black_box(detector.detect_state_field(field));
            }
        })
    });
}

/// Benchmark per-field detection overhead
fn bench_per_field_overhead(c: &mut Criterion) {
    let config = StateDetectionConfig::default();
    let detector = StateFieldDetector::new(config);
    let field: ExprField = parse_quote! { self.current_state };

    c.bench_function("single_field_detection", |b| {
        b.iter(|| {
            hint_black_box(detector.detect_state_field(&field));
        })
    });
}

/// Benchmark detection with custom keywords
fn bench_custom_keywords(c: &mut Criterion) {
    let config = StateDetectionConfig {
        use_type_analysis: true,
        use_frequency_analysis: true,
        use_pattern_recognition: true,
        min_enum_variants: 3,
        custom_keywords: vec![
            "workflow".to_string(),
            "step".to_string(),
            "stage".to_string(),
        ],
        custom_patterns: vec![
            "current_workflow".to_string(),
            "active_step".to_string(),
        ],
    };

    let detector = StateFieldDetector::new(config);
    let fields = vec![
        parse_quote! { self.workflow },
        parse_quote! { self.current_workflow },
        parse_quote! { self.step },
        parse_quote! { self.stage },
    ];

    c.bench_function("detection_with_custom_keywords", |b| {
        b.iter(|| {
            for field in &fields {
                hint_black_box(detector.detect_state_field(field));
            }
        })
    });
}

/// Benchmark throughput (fields analyzed per second)
fn bench_throughput(c: &mut Criterion) {
    let config = StateDetectionConfig::default();
    let detector = StateFieldDetector::new(config);
    let fields = generate_large_corpus(1000);

    c.bench_function("throughput_1000_fields", |b| {
        b.iter(|| {
            for field in &fields {
                hint_black_box(detector.detect_state_field(field));
            }
        })
    });
}

/// Benchmark comparison: baseline vs enhanced
fn bench_overhead_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("detection_overhead_comparison");

    // Baseline (keyword-only)
    let baseline_config = StateDetectionConfig {
        use_type_analysis: false,
        use_frequency_analysis: false,
        use_pattern_recognition: false,
        min_enum_variants: 3,
        custom_keywords: vec![],
        custom_patterns: vec![],
    };
    let baseline_detector = StateFieldDetector::new(baseline_config);
    let fields = generate_field_accesses();

    group.bench_function("baseline", |b| {
        b.iter(|| {
            for field in &fields {
                hint_black_box(baseline_detector.detect_state_field(field));
            }
        })
    });

    // Enhanced (all strategies)
    let enhanced_config = StateDetectionConfig::default();
    let enhanced_detector = StateFieldDetector::new(enhanced_config);

    group.bench_function("enhanced", |b| {
        b.iter(|| {
            for field in &fields {
                hint_black_box(enhanced_detector.detect_state_field(field));
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_baseline_detection,
    bench_enhanced_detection,
    bench_per_field_overhead,
    bench_custom_keywords,
    bench_throughput,
    bench_overhead_comparison,
);
criterion_main!(benches);
