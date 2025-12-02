//! Performance benchmarks for framework pattern detection
//!
//! Measures the overhead of framework pattern detection to verify < 5% impact requirement

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::analysis::framework_patterns_multi::{
    detector::{FileContext, FunctionAst, Parameter},
    patterns::Language,
    FrameworkDetector,
};
use std::hint::black_box;
use std::path::Path;

/// Generate test functions with varying framework signatures
fn generate_test_functions(count: usize) -> Vec<FunctionAst> {
    let mut functions = Vec::new();

    for i in 0..count {
        // Mix of framework and non-framework functions
        if i % 3 == 0 {
            // Axum handler
            let mut func = FunctionAst::new(format!("handler_{}", i));
            func.is_async = true;
            func.parameters.push(Parameter {
                name: "path".to_string(),
                type_annotation: "Path<u32>".to_string(),
            });
            func.return_type = Some("Json<Response>".to_string());
            functions.push(func);
        } else if i % 3 == 1 {
            // Test function
            let mut func = FunctionAst::new(format!("test_case_{}", i));
            func.attributes.push(
                debtmap::analysis::framework_patterns_multi::detector::Attribute {
                    text: "#[test]".to_string(),
                },
            );
            functions.push(func);
        } else {
            // Regular function
            let func = FunctionAst::new(format!("regular_function_{}", i));
            functions.push(func);
        }
    }

    functions
}

/// Generate file context with framework imports
fn generate_file_context(language: Language) -> FileContext {
    let mut context = FileContext::new(language, "test_file.rs".into());

    match language {
        Language::Rust => {
            context.add_import("use axum::extract::Path;".to_string());
            context.add_import("use axum::response::Json;".to_string());
        }
        Language::Python => {
            context.add_import("from fastapi import FastAPI".to_string());
            context.add_import("import pytest".to_string());
        }
    }

    context
}

/// Benchmark framework detection with configuration loading
fn bench_framework_detection_with_config(c: &mut Criterion) {
    let config_path = Path::new("framework_patterns.toml");

    // Skip if config doesn't exist
    if !config_path.exists() {
        eprintln!("Skipping benchmark: framework_patterns.toml not found");
        return;
    }

    let detector =
        FrameworkDetector::from_config(config_path).expect("Failed to load framework config");

    let functions = generate_test_functions(100);
    let file_context = generate_file_context(Language::Rust);

    c.bench_function("framework_detection_100_functions", |b| {
        b.iter(|| {
            for func in &functions {
                black_box(detector.detect_framework_patterns(func, &file_context));
            }
        })
    });
}

/// Benchmark baseline: processing functions without framework detection
fn bench_baseline_function_processing(c: &mut Criterion) {
    let functions = generate_test_functions(100);

    c.bench_function("baseline_function_iteration", |b| {
        b.iter(|| {
            for func in &functions {
                // Simulate basic function processing without framework detection
                black_box(&func.name);
                black_box(func.parameters.len());
                black_box(func.is_async);
            }
        })
    });
}

/// Benchmark overhead: measure impact of framework detection
fn bench_overhead_comparison(c: &mut Criterion) {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping overhead benchmark: framework_patterns.toml not found");
        return;
    }

    let detector =
        FrameworkDetector::from_config(config_path).expect("Failed to load framework config");

    let mut group = c.benchmark_group("framework_detection_overhead");

    for size in [10, 50, 100, 200].iter() {
        let functions = generate_test_functions(*size);
        let file_context = generate_file_context(Language::Rust);

        group.bench_with_input(BenchmarkId::new("without_detection", size), size, |b, _| {
            b.iter(|| {
                for func in &functions {
                    black_box(&func.name);
                    black_box(func.parameters.len());
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("with_detection", size), size, |b, _| {
            b.iter(|| {
                for func in &functions {
                    black_box(detector.detect_framework_patterns(func, &file_context));
                }
            })
        });
    }

    group.finish();
}

/// Benchmark regex caching efficiency
fn bench_regex_caching(c: &mut Criterion) {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping regex caching benchmark: framework_patterns.toml not found");
        return;
    }

    let detector =
        FrameworkDetector::from_config(config_path).expect("Failed to load framework config");

    let func = FunctionAst::new("test_example".to_string());
    let file_context = generate_file_context(Language::Rust);

    c.bench_function("regex_cache_warm", |b| {
        b.iter(|| {
            // Run detection multiple times to test cache effectiveness
            for _ in 0..100 {
                black_box(detector.detect_framework_patterns(&func, &file_context));
            }
        })
    });
}

/// Benchmark pattern matching for different languages
fn bench_multi_language_detection(c: &mut Criterion) {
    let config_path = Path::new("framework_patterns.toml");

    if !config_path.exists() {
        eprintln!("Skipping multi-language benchmark: framework_patterns.toml not found");
        return;
    }

    let detector =
        FrameworkDetector::from_config(config_path).expect("Failed to load framework config");

    let mut group = c.benchmark_group("multi_language_detection");

    for lang in [Language::Rust, Language::Python].iter() {
        let functions = generate_test_functions(50);
        let file_context = generate_file_context(*lang);

        group.bench_with_input(
            BenchmarkId::new("detection", format!("{:?}", lang)),
            lang,
            |b, _| {
                b.iter(|| {
                    for func in &functions {
                        black_box(detector.detect_framework_patterns(func, &file_context));
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_framework_detection_with_config,
    bench_baseline_function_processing,
    bench_overhead_comparison,
    bench_regex_caching,
    bench_multi_language_detection,
);
criterion_main!(benches);
