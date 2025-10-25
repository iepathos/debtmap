use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::builders::unified_analysis;
use debtmap::core::Language;
use debtmap::utils::analyze_project;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();
    let src_dir = project_path.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Create multiple files with various complexity levels
    for i in 1..=10 {
        let mut content = String::from("use std::collections::HashMap;\n\n");

        // Add functions with varying complexity
        for j in 1..=5 {
            content.push_str(&format!(
                r#"
fn func_{}_{}(data: Vec<i32>) -> i32 {{
    let mut sum = 0;
    for k in 0..data.len() {{
        if data[k] > {} {{
            for l in 0..{} {{
                sum += data[k] * l;
                if sum > {} {{
                    break;
                }}
            }}
        }}
    }}
    sum
}}
"#,
                i,
                j,
                i * j,
                i + j,
                i * j * 10
            ));
        }

        std::fs::write(src_dir.join(format!("file_{}.rs", i)), content).unwrap();
    }

    let path_buf = project_path.to_path_buf();
    (temp_dir, path_buf)
}

fn benchmark_analysis_without_aggregation(c: &mut Criterion) {
    let (_temp_dir, project_path) = create_test_project();

    let languages = vec![Language::Rust];
    let results = analyze_project(
        project_path.clone(),
        languages,
        2,  // complexity threshold
        50, // duplication threshold
    )
    .unwrap();

    c.bench_function("analysis_without_aggregation", |b| {
        b.iter(|| {
            let _ = unified_analysis::perform_unified_analysis_with_options(
                unified_analysis::UnifiedAnalysisOptions {
                    results: &results,
                    coverage_file: None,
                    semantic_off: false,
                    project_path: &project_path,
                    verbose_macro_warnings: false,
                    show_macro_stats: false,
                    parallel: false,
                    jobs: 0,
                    use_cache: false,
                    multi_pass: false,
                    show_attribution: false,
                    aggregate_only: false,
                    no_aggregation: true, // Disable aggregation
                    aggregation_method: None,
                    min_problematic: None,
                    no_god_object: false,
                    _formatting_config: Default::default(),
                },
            );
        });
    });
}

fn benchmark_analysis_with_aggregation(c: &mut Criterion) {
    let (_temp_dir, project_path) = create_test_project();

    let languages = vec![Language::Rust];
    let results = analyze_project(
        project_path.clone(),
        languages,
        2,  // complexity threshold
        50, // duplication threshold
    )
    .unwrap();

    c.bench_function("analysis_with_aggregation", |b| {
        b.iter(|| {
            let _ = unified_analysis::perform_unified_analysis_with_options(
                unified_analysis::UnifiedAnalysisOptions {
                    results: &results,
                    coverage_file: None,
                    semantic_off: false,
                    project_path: &project_path,
                    verbose_macro_warnings: false,
                    show_macro_stats: false,
                    parallel: false,
                    jobs: 0,
                    use_cache: false,
                    multi_pass: false,
                    show_attribution: false,
                    aggregate_only: false,
                    no_aggregation: false, // Enable aggregation
                    aggregation_method: Some("weighted_sum".to_string()),
                    min_problematic: None,
                    no_god_object: false,
                    _formatting_config: Default::default(),
                },
            );
        });
    });
}

fn benchmark_aggregation_methods(c: &mut Criterion) {
    let (_temp_dir, project_path) = create_test_project();

    let languages = vec![Language::Rust];
    let results = analyze_project(
        project_path.clone(),
        languages,
        2,  // complexity threshold
        50, // duplication threshold
    )
    .unwrap();

    let methods = vec!["sum", "weighted_sum", "logarithmic_sum", "max_plus_average"];

    for method in methods {
        c.bench_function(&format!("aggregation_{}", method), |b| {
            b.iter(|| {
                let _ = unified_analysis::perform_unified_analysis_with_options(
                    unified_analysis::UnifiedAnalysisOptions {
                        results: &results,
                        coverage_file: None,
                        semantic_off: false,
                        project_path: &project_path,
                        verbose_macro_warnings: false,
                        show_macro_stats: false,
                        parallel: false,
                        jobs: 0,
                        use_cache: false,
                        multi_pass: false,
                        show_attribution: false,
                        aggregate_only: false,
                        no_aggregation: false,
                        aggregation_method: Some(method.to_string()),
                        min_problematic: None,
                        no_god_object: false,
                        _formatting_config: Default::default(),
                    },
                );
            });
        });
    }
}

criterion_group!(
    benches,
    benchmark_analysis_without_aggregation,
    benchmark_analysis_with_aggregation,
    benchmark_aggregation_methods
);
criterion_main!(benches);
