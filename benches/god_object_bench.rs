//! Performance benchmarks for god object detection critical paths
//!
//! These benchmarks establish baseline performance metrics before refactoring
//! the god object detection module (Spec 181a). They measure:
//! - God object score calculation
//! - Classification determination
//! - Method grouping by responsibility
//! - Module split recommendation generation
//! - Full analysis pipeline orchestration

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::organization::{
    calculate_god_object_score, determine_confidence, group_methods_by_responsibility,
    recommend_module_splits, GodObjectDetector, GodObjectThresholds,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Benchmark god object score calculation (pure function)
fn bench_calculate_score(c: &mut Criterion) {
    let thresholds = GodObjectThresholds::for_rust();

    c.bench_function("calculate_god_object_score", |b| {
        b.iter(|| {
            calculate_god_object_score(
                black_box(25),  // method_count
                black_box(15),  // field_count
                black_box(6),   // responsibility_count
                black_box(500), // lines_of_code
                black_box(&thresholds),
            )
        })
    });
}

/// Benchmark confidence determination (pure function)
fn bench_determine_confidence(c: &mut Criterion) {
    let thresholds = GodObjectThresholds::for_rust();

    c.bench_function("determine_confidence", |b| {
        b.iter(|| {
            determine_confidence(
                black_box(25),  // method_count
                black_box(15),  // field_count
                black_box(6),   // responsibility_count
                black_box(500), // lines_of_code
                black_box(150), // complexity_sum
                black_box(&thresholds),
            )
        })
    });
}

/// Benchmark method grouping by responsibility
fn bench_group_methods(c: &mut Criterion) {
    let methods = vec![
        "get_value".to_string(),
        "set_value".to_string(),
        "validate_input".to_string(),
        "save_to_database".to_string(),
        "load_from_database".to_string(),
        "render_output".to_string(),
        "format_display".to_string(),
        "parse_config".to_string(),
        "write_config".to_string(),
        "handle_error".to_string(),
    ];

    c.bench_function("group_methods_by_responsibility", |b| {
        b.iter(|| group_methods_by_responsibility(black_box(&methods)))
    });
}

/// Benchmark module split recommendation generation
fn bench_recommend_splits(c: &mut Criterion) {
    let methods = vec![
        "get_value".to_string(),
        "set_value".to_string(),
        "validate_input".to_string(),
        "save_to_database".to_string(),
        "load_from_database".to_string(),
        "render_output".to_string(),
        "format_display".to_string(),
        "parse_config".to_string(),
        "write_config".to_string(),
        "handle_error".to_string(),
    ];

    let mut responsibility_groups = HashMap::new();
    responsibility_groups.insert("data_access".to_string(), methods.clone());

    c.bench_function("recommend_module_splits", |b| {
        b.iter(|| {
            recommend_module_splits(
                black_box("TestStruct"),
                black_box(&methods),
                black_box(&responsibility_groups),
            )
        })
    });
}

/// Benchmark full analysis pipeline on config.rs
fn bench_full_analysis_pipeline(c: &mut Criterion) {
    let config_path = Path::new("src/config.rs");
    if !config_path.exists() {
        eprintln!("Warning: src/config.rs not found, skipping full pipeline benchmark");
        return;
    }

    let source_content = fs::read_to_string(config_path).expect("Failed to read config.rs");
    let file = syn::parse_file(&source_content).expect("Failed to parse config.rs");

    c.bench_function("full_analysis_pipeline", |b| {
        b.iter(|| {
            let detector = GodObjectDetector::with_source_content(&source_content);
            black_box(detector.analyze_comprehensive(config_path, &file))
        })
    });
}

/// Benchmark enhanced analysis with classification
fn bench_enhanced_analysis(c: &mut Criterion) {
    let config_path = Path::new("src/config.rs");
    if !config_path.exists() {
        eprintln!("Warning: src/config.rs not found, skipping enhanced analysis benchmark");
        return;
    }

    let source_content = fs::read_to_string(config_path).expect("Failed to read config.rs");
    let file = syn::parse_file(&source_content).expect("Failed to parse config.rs");

    c.bench_function("enhanced_analysis_pipeline", |b| {
        b.iter(|| {
            let detector = GodObjectDetector::with_source_content(&source_content);
            black_box(detector.analyze_enhanced(config_path, &file))
        })
    });
}

criterion_group!(
    benches,
    bench_calculate_score,
    bench_determine_confidence,
    bench_group_methods,
    bench_recommend_splits,
    bench_full_analysis_pipeline,
    bench_enhanced_analysis
);
criterion_main!(benches);
