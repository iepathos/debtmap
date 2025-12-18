use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::builders::unified_analysis::{
    perform_unified_analysis_with_options, UnifiedAnalysisOptions,
};
use debtmap::core::Language;
use debtmap::utils::analysis_helpers::analyze_project;
use std::hint::black_box;
use tempfile::TempDir;

fn create_test_project(num_files: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    for i in 0..num_files {
        let file_path = temp_dir.path().join(format!("file_{}.rs", i));
        let content = format!(
            r#"
            pub fn function_{}(x: i32, y: i32) -> i32 {{
                if x > 0 {{
                    if y > 0 {{
                        x + y
                    }} else {{
                        x - y
                    }}
                }} else {{
                    if y > 0 {{
                        y - x
                    }} else {{
                        -x - y
                    }}
                }}
            }}
            
            pub fn complex_function_{}(data: Vec<i32>) -> i32 {{
                let mut result = 0;
                for item in data.iter() {{
                    if *item > 0 {{
                        result += item;
                    }} else {{
                        result -= item;
                    }}
                }}
                result
            }}
            
            #[test]
            fn test_function_{}() {{
                assert_eq!(function_{}(1, 2), 3);
                assert_eq!(function_{}(-1, 2), 3);
            }}
            "#,
            i, i, i, i, i
        );
        std::fs::write(file_path, content).unwrap();
    }

    temp_dir
}

fn benchmark_sequential_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_analysis");

    for size in [10, 50, 100, 250].iter() {
        let temp_dir = create_test_project(*size);
        let path = temp_dir.path().to_path_buf();

        // Analyze files first to get results
        let results = analyze_project(path.clone(), vec![Language::Rust], 15, 50).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(&results, &path),
            |b, (results, path)| {
                b.iter(|| {
                    let options = UnifiedAnalysisOptions {
                        results,
                        coverage_file: None,
                        semantic_off: false,
                        project_path: path,
                        verbose_macro_warnings: false,
                        show_macro_stats: false,
                        parallel: false,
                        jobs: 0,
                        multi_pass: false,
                        show_attribution: false,
                        aggregate_only: false,
                        no_aggregation: false,
                        aggregation_method: None,
                        min_problematic: None,
                        no_god_object: false,
                        suppress_coverage_tip: false,
                        _formatting_config: Default::default(),
                        enable_context: false,
                        context_providers: None,
                        disable_context: None,
                        rust_files: None,
                        extracted_data: None,
                    };
                    let analysis = perform_unified_analysis_with_options(options).unwrap();
                    black_box(analysis);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_parallel_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_analysis");

    for size in [10, 50, 100, 250].iter() {
        let temp_dir = create_test_project(*size);
        let path = temp_dir.path().to_path_buf();

        // Analyze files first to get results
        let results = analyze_project(path.clone(), vec![Language::Rust], 15, 50).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(&results, &path),
            |b, (results, path)| {
                b.iter(|| {
                    let options = UnifiedAnalysisOptions {
                        results,
                        coverage_file: None,
                        semantic_off: false,
                        project_path: path,
                        verbose_macro_warnings: false,
                        show_macro_stats: false,
                        parallel: true,
                        jobs: 4,
                        multi_pass: false,
                        show_attribution: false,
                        aggregate_only: false,
                        no_aggregation: false,
                        aggregation_method: None,
                        min_problematic: None,
                        no_god_object: false,
                        suppress_coverage_tip: false,
                        _formatting_config: Default::default(),
                        enable_context: false,
                        context_providers: None,
                        disable_context: None,
                        rust_files: None,
                        extracted_data: None,
                    };
                    let analysis = perform_unified_analysis_with_options(options).unwrap();
                    black_box(analysis);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_parallel_with_job_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_jobs_comparison");
    let temp_dir = create_test_project(100);
    let path = temp_dir.path().to_path_buf();

    // Analyze files first to get results
    let results = analyze_project(path.clone(), vec![Language::Rust], 15, 50).unwrap();

    for jobs in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_jobs", jobs)),
            &(*jobs, &path, &results),
            |b, (jobs, path, results)| {
                b.iter(|| {
                    let options = UnifiedAnalysisOptions {
                        results,
                        coverage_file: None,
                        semantic_off: false,
                        project_path: path,
                        verbose_macro_warnings: false,
                        show_macro_stats: false,
                        parallel: true,
                        jobs: *jobs,
                        multi_pass: false,
                        show_attribution: false,
                        aggregate_only: false,
                        no_aggregation: false,
                        aggregation_method: None,
                        min_problematic: None,
                        no_god_object: false,
                        suppress_coverage_tip: false,
                        _formatting_config: Default::default(),
                        enable_context: false,
                        context_providers: None,
                        disable_context: None,
                        rust_files: None,
                        extracted_data: None,
                    };
                    let analysis = perform_unified_analysis_with_options(options).unwrap();
                    black_box(analysis);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_file_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_count_scaling");

    for size in [10, 50, 100, 200, 500].iter() {
        let temp_dir = create_test_project(*size);
        let path = temp_dir.path().to_path_buf();

        // Analyze files first to get results
        let results = analyze_project(path.clone(), vec![Language::Rust], 15, 50).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", size)),
            &(&results, &path),
            |b, (results, path)| {
                b.iter(|| {
                    let options = UnifiedAnalysisOptions {
                        results,
                        coverage_file: None,
                        semantic_off: false,
                        project_path: path,
                        verbose_macro_warnings: false,
                        show_macro_stats: false,
                        parallel: true,
                        jobs: 4,
                        multi_pass: false,
                        show_attribution: false,
                        aggregate_only: false,
                        no_aggregation: false,
                        aggregation_method: None,
                        min_problematic: None,
                        no_god_object: false,
                        suppress_coverage_tip: false,
                        _formatting_config: Default::default(),
                        enable_context: false,
                        context_providers: None,
                        disable_context: None,
                        rust_files: None,
                        extracted_data: None,
                    };
                    let analysis = perform_unified_analysis_with_options(options).unwrap();
                    black_box(analysis);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_sequential_analysis,
    benchmark_parallel_analysis,
    benchmark_parallel_with_job_counts,
    benchmark_file_counts
);
criterion_main!(benches);
