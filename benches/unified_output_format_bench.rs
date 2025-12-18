//! Benchmarks for unified output format performance characteristics.
//!
//! # Performance Acceptance Criteria
//!
//! The unified format provides:
//! - Consistent field structure across all debt item types (File and Function)
//! - Simplified filtering: `item.location` works uniformly for all items
//! - Simplified sorting: `item.priority` works consistently
//! - Rich metadata with format version and summary statistics
//!
//! Note: Legacy format was removed in spec 202 - unified format is now the only JSON format.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::builders::unified_analysis;
use debtmap::core::Language;
use debtmap::output::json::output_json_with_format;
use debtmap::utils::analyze_project;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test project with varying complexity to benchmark serialization
fn create_test_project(num_files: usize, functions_per_file: usize) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    let src_dir = project_path.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    for i in 1..=num_files {
        let mut content = String::from("use std::collections::HashMap;\n\n");

        for j in 1..=functions_per_file {
            content.push_str(&format!(
                r#"
fn func_{}_{}(data: Vec<i32>, threshold: i32) -> i32 {{
    let mut sum = 0;
    for k in 0..data.len() {{
        if data[k] > threshold {{
            for l in 0..{} {{
                sum += data[k] * l;
                if sum > {} {{
                    break;
                }}
            }}
        }} else {{
            match data[k] {{
                0 => sum += 1,
                1..=10 => sum += data[k],
                _ => sum -= 1,
            }}
        }}
    }}
    sum
}}
"#,
                i,
                j,
                i + j,
                i * j * 10
            ));
        }

        std::fs::write(src_dir.join(format!("file_{}.rs", i)), content).unwrap();
    }

    let path_buf = project_path.to_path_buf();
    (temp_dir, path_buf)
}

/// Benchmark unified format serialization
fn benchmark_unified_format_serialization(c: &mut Criterion) {
    let sizes = vec![
        (5, 5),   // Small: 5 files, 5 functions each = 25 functions
        (10, 10), // Medium: 10 files, 10 functions each = 100 functions
        (20, 10), // Large: 20 files, 10 functions each = 200 functions
    ];

    let mut group = c.benchmark_group("unified_format_serialization");

    for (num_files, functions_per_file) in sizes {
        let total_functions = num_files * functions_per_file;
        let (_temp_dir, project_path) = create_test_project(num_files, functions_per_file);

        let languages = vec![Language::Rust];
        let results = analyze_project(
            project_path.clone(),
            languages,
            5,  // complexity threshold
            50, // duplication threshold
        )
        .unwrap();

        let analysis_results = unified_analysis::perform_unified_analysis_with_options(
            unified_analysis::UnifiedAnalysisOptions {
                results: &results,
                coverage_file: None,
                semantic_off: false,
                project_path: &project_path,
                verbose_macro_warnings: false,
                show_macro_stats: false,
                parallel: false,
                jobs: 0,
                multi_pass: false,
                show_attribution: false,
                aggregate_only: false,
                no_aggregation: false,
                aggregation_method: Some("weighted_sum".to_string()),
                min_problematic: None,
                no_god_object: false,
                suppress_coverage_tip: false,
                _formatting_config: Default::default(),
                enable_context: false,
                context_providers: None,
                disable_context: None,
                rust_files: None,
                extracted_data: None,
            },
        )
        .unwrap();

        group.bench_with_input(
            BenchmarkId::new("unified", total_functions),
            &analysis_results,
            |b, results| {
                b.iter(|| {
                    let temp_output = tempfile::NamedTempFile::new().unwrap();
                    output_json_with_format(
                        results,
                        None,
                        None,
                        Some(temp_output.path().to_path_buf()),
                        false,
                    )
                    .expect("Serialization should succeed");
                });
            },
        );
    }

    group.finish();
}

/// Benchmark serialization with different debt item counts
fn benchmark_scaling_by_debt_items(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_by_debt_items");

    // Test with different complexity thresholds to vary debt item count
    let thresholds = vec![
        (2, "high_debt"),   // Lower threshold = more debt items
        (5, "medium_debt"), // Medium threshold
        (10, "low_debt"),   // Higher threshold = fewer debt items
    ];

    let (_temp_dir, project_path) = create_test_project(15, 10);
    let languages = vec![Language::Rust];

    for (threshold, label) in thresholds {
        let results =
            analyze_project(project_path.clone(), languages.clone(), threshold, 50).unwrap();

        let analysis_results = unified_analysis::perform_unified_analysis_with_options(
            unified_analysis::UnifiedAnalysisOptions {
                results: &results,
                coverage_file: None,
                semantic_off: false,
                project_path: &project_path,
                verbose_macro_warnings: false,
                show_macro_stats: false,
                parallel: false,
                jobs: 0,
                multi_pass: false,
                show_attribution: false,
                aggregate_only: false,
                no_aggregation: false,
                aggregation_method: Some("weighted_sum".to_string()),
                min_problematic: None,
                no_god_object: false,
                suppress_coverage_tip: false,
                _formatting_config: Default::default(),
                enable_context: false,
                context_providers: None,
                disable_context: None,
                rust_files: None,
                extracted_data: None,
            },
        )
        .unwrap();

        group.bench_with_input(
            BenchmarkId::new("unified", label),
            &analysis_results,
            |b, results| {
                b.iter(|| {
                    let temp_output = tempfile::NamedTempFile::new().unwrap();
                    output_json_with_format(
                        results,
                        None,
                        None,
                        Some(temp_output.path().to_path_buf()),
                        false,
                    )
                    .expect("Serialization should succeed");
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_unified_format_serialization,
    benchmark_scaling_by_debt_items
);
criterion_main!(benches);
