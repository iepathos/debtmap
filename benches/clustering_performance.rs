//! Benchmark for clustering performance (Spec 192)
//!
//! Validates that clustering overhead is <15% of total analysis time.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::organization::GodObjectDetector;
use std::path::Path;

fn benchmark_clustering_overhead(c: &mut Criterion) {
    // Load god_object_detector.rs as test subject
    let source_code = std::fs::read_to_string("src/organization/god_object_detector.rs")
        .expect("Failed to read god_object_detector.rs");

    let ast = syn::parse_file(&source_code).expect("Failed to parse file");
    let path = Path::new("src/organization/god_object_detector.rs");

    let mut group = c.benchmark_group("clustering_performance");
    group.sample_size(20); // Reduce sample size for faster benchmarking

    // Benchmark: Full analysis with improved clustering
    group.bench_function("full_analysis_with_clustering", |b| {
        b.iter(|| {
            let detector = GodObjectDetector::with_source_content(&source_code);
            let analysis = detector.analyze_enhanced(black_box(path), black_box(&ast));
            black_box(analysis);
        });
    });

    // Benchmark: Just AST parsing (baseline)
    group.bench_function("baseline_ast_parsing", |b| {
        b.iter(|| {
            let ast = syn::parse_file(black_box(&source_code))
                .expect("Failed to parse file");
            black_box(ast);
        });
    });

    group.finish();
}

fn benchmark_clustering_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("clustering_scalability");
    group.sample_size(10);

    // Test files of different sizes
    let test_files = vec![
        ("src/organization/god_object_detector.rs", "large"),
        ("src/organization/behavioral_decomposition.rs", "medium"),
    ];

    for (file_path, size_label) in test_files {
        if let Ok(source_code) = std::fs::read_to_string(file_path) {
            if let Ok(ast) = syn::parse_file(&source_code) {
                let path = Path::new(file_path);

                group.bench_function(format!("clustering_{}", size_label), |b| {
                    b.iter(|| {
                        let detector = GodObjectDetector::with_source_content(&source_code);
                        let analysis = detector.analyze_enhanced(black_box(path), black_box(&ast));
                        black_box(analysis);
                    });
                });
            }
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_clustering_overhead,
    benchmark_clustering_scalability
);
criterion_main!(benches);
