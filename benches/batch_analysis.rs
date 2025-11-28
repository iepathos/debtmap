//! Benchmarks for batch file analysis comparing sequential vs parallel performance.
//!
//! These benchmarks measure the performance difference between:
//! - Sequential file analysis (config.enabled = false)
//! - Parallel file analysis (config.enabled = true)
//!
//! This helps validate that the traverse pattern batch analysis provides
//! meaningful performance improvements for multi-file workloads.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use debtmap::analyzers::batch::{analyze_files_effect, validate_files};
use debtmap::config::{BatchAnalysisConfig, DebtmapConfig};
use debtmap::effects::run_validation;
use debtmap::{analyzers::get_analyzer, core::Language, run_effect};
use rayon::prelude::*;
use std::hint::black_box;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test project with the specified number of Rust files.
///
/// Each file contains moderately complex functions to ensure
/// meaningful analysis work.
fn create_test_files(num_files: usize) -> (TempDir, Vec<PathBuf>) {
    let temp_dir = TempDir::new().unwrap();
    let mut paths = Vec::with_capacity(num_files);

    for i in 0..num_files {
        let file_path = temp_dir.path().join(format!("file_{}.rs", i));
        let content = format!(
            r#"
/// A function with moderate complexity for benchmarking.
pub fn calculate_{i}(x: i32, y: i32, z: i32) -> i32 {{
    if x > 0 {{
        if y > 0 {{
            if z > 0 {{
                x + y + z
            }} else {{
                x + y - z
            }}
        }} else {{
            x - y + z.abs()
        }}
    }} else {{
        (-x).saturating_add(y).saturating_sub(z)
    }}
}}

/// Another function to increase file complexity.
pub fn process_{i}(data: &[i32]) -> Vec<i32> {{
    data.iter()
        .filter(|&&n| n > 0)
        .map(|&n| n * 2)
        .collect()
}}

/// A function with a loop for additional complexity metrics.
pub fn aggregate_{i}(values: &[i32]) -> i32 {{
    let mut sum = 0;
    for value in values {{
        if *value > 0 {{
            sum += value;
        }} else {{
            sum -= value;
        }}
    }}
    sum
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_calculate_{i}() {{
        assert_eq!(calculate_{i}(1, 2, 3), 6);
    }}
}}
"#,
            i = i
        );
        std::fs::write(&file_path, content).unwrap();
        paths.push(file_path);
    }

    (temp_dir, paths)
}

/// Benchmark sequential file analysis using the batch module.
fn benchmark_batch_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_analysis_sequential");

    for &size in &[5, 10, 25, 50] {
        let (_temp_dir, paths) = create_test_files(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &paths, |b, paths| {
            b.iter(|| {
                // Sequential processing - analyze each file one by one
                let results: Vec<_> = paths
                    .iter()
                    .map(|path| {
                        let content = std::fs::read_to_string(path).unwrap();
                        let language = Language::from_path(path);
                        let analyzer = get_analyzer(language);
                        let ast = analyzer.parse(&content, path.clone()).unwrap();
                        analyzer.analyze(&ast)
                    })
                    .collect();
                black_box(results)
            });
        });
    }

    group.finish();
}

/// Benchmark parallel file analysis using the batch module.
fn benchmark_batch_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_analysis_parallel");

    for &size in &[5, 10, 25, 50] {
        let (_temp_dir, paths) = create_test_files(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &paths, |b, paths| {
            b.iter(|| {
                // Parallel processing using rayon
                let results: Vec<_> = paths
                    .par_iter()
                    .map(|path| {
                        let content = std::fs::read_to_string(path).unwrap();
                        let language = Language::from_path(path);
                        let analyzer = get_analyzer(language);
                        let ast = analyzer.parse(&content, path.clone()).unwrap();
                        analyzer.analyze(&ast)
                    })
                    .collect();
                black_box(results)
            });
        });
    }

    group.finish();
}

/// Compare sequential vs parallel directly for the same file counts.
fn benchmark_sequential_vs_parallel_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_seq_vs_parallel");

    // Test with a fixed moderate size to show the difference clearly
    let size = 25;
    let (_temp_dir, paths) = create_test_files(size);

    group.throughput(Throughput::Elements(size as u64));

    // Sequential benchmark
    group.bench_with_input(
        BenchmarkId::new("sequential", size),
        &paths,
        |b, paths| {
            b.iter(|| {
                let results: Vec<_> = paths
                    .iter()
                    .map(|path| {
                        let content = std::fs::read_to_string(path).unwrap();
                        let language = Language::from_path(path);
                        let analyzer = get_analyzer(language);
                        let ast = analyzer.parse(&content, path.clone()).unwrap();
                        analyzer.analyze(&ast)
                    })
                    .collect();
                black_box(results)
            });
        },
    );

    // Parallel benchmark
    group.bench_with_input(BenchmarkId::new("parallel", size), &paths, |b, paths| {
        b.iter(|| {
            let results: Vec<_> = paths
                .par_iter()
                .map(|path| {
                    let content = std::fs::read_to_string(path).unwrap();
                    let language = Language::from_path(path);
                    let analyzer = get_analyzer(language);
                    let ast = analyzer.parse(&content, path.clone()).unwrap();
                    analyzer.analyze(&ast)
                })
                .collect();
            black_box(results)
        });
    });

    group.finish();
}

/// Benchmark validation with error accumulation using the traverse pattern.
fn benchmark_validation_accumulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_validation");

    for &size in &[5, 10, 25] {
        let (_temp_dir, paths) = create_test_files(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &paths, |b, paths| {
            b.iter(|| {
                let validation = validate_files(paths);
                let result = run_validation(validation);
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark the effect-based analysis with timing collection.
fn benchmark_effect_with_timing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_effect_timing");

    for &size in &[5, 10, 25] {
        let (_temp_dir, paths) = create_test_files(size);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &paths, |b, paths| {
            b.iter(|| {
                // Create a config with timing enabled
                let mut config = DebtmapConfig::default();
                config.batch_analysis = Some(BatchAnalysisConfig::default().with_timing());

                let effect = analyze_files_effect(paths.clone());
                let result = run_effect(effect, config);
                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_batch_sequential,
    benchmark_batch_parallel,
    benchmark_sequential_vs_parallel_comparison,
    benchmark_validation_accumulation,
    benchmark_effect_with_timing
);
criterion_main!(benches);
