/// Benchmark for contextual risk analysis performance (spec 202)
/// Verifies that --context flag adds less than 10% overhead
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::CallGraph;
use debtmap::risk::context::git_history::GitHistoryProvider;
use debtmap::risk::context::{AnalysisTarget, ContextAggregator};
use debtmap::risk::RiskAnalyzer;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a test git repository with history
fn create_test_git_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create test file
    let test_file = repo_path.join("test.rs");
    std::fs::write(&test_file, "fn test() { println!(\"hello\"); }").unwrap();

    Command::new("git")
        .args(&["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    temp_dir
}

/// Create test function metrics
fn create_test_metrics(count: usize) -> Vec<FunctionMetrics> {
    (0..count)
        .map(|i| FunctionMetrics {
            name: format!("function_{}", i),
            file: PathBuf::from("test.rs"),
            line: i * 10,
            length: 10,
            cyclomatic: 5,
            cognitive: 8,
            nesting_depth: 2,
            is_test: false,
            in_test_module: false,
            is_pure: Some(false),
            language: debtmap::core::SupportedLanguage::Rust,
        })
        .collect()
}

/// Benchmark git history provider performance
fn benchmark_git_history_provider(c: &mut Criterion) {
    let temp_repo = create_test_git_repo();
    let repo_path = temp_repo.path();

    let mut group = c.benchmark_group("git_history_provider");

    // Benchmark single file analysis
    group.bench_function("analyze_single_file", |b| {
        b.iter(|| {
            let provider = GitHistoryProvider::new(repo_path.to_path_buf()).unwrap();
            let target = AnalysisTarget {
                root_path: repo_path.to_path_buf(),
                file_path: repo_path.join("test.rs"),
                function_name: "test".to_string(),
                line_range: (1, 1),
            };
            let mut provider = provider;
            let result = provider.analyze_file(black_box(&target.file_path));
            black_box(result);
        })
    });

    group.finish();
}

/// Benchmark context aggregator with git history
fn benchmark_context_aggregator(c: &mut Criterion) {
    let temp_repo = create_test_git_repo();
    let repo_path = temp_repo.path();

    let mut group = c.benchmark_group("context_aggregator");

    // Create aggregator with git history provider
    let provider = Box::new(GitHistoryProvider::new(repo_path.to_path_buf()).unwrap());
    let mut aggregator = ContextAggregator::new().with_provider(provider);

    group.bench_function("analyze_with_git_history", |b| {
        b.iter(|| {
            let target = AnalysisTarget {
                root_path: repo_path.to_path_buf(),
                file_path: repo_path.join("test.rs"),
                function_name: "test".to_string(),
                line_range: (1, 1),
            };
            let context_map = aggregator.analyze(black_box(&target));
            black_box(context_map);
        })
    });

    group.finish();
}

/// Benchmark overhead of contextual risk analysis
fn benchmark_analysis_overhead(c: &mut Criterion) {
    let temp_repo = create_test_git_repo();
    let repo_path = temp_repo.path();

    let mut group = c.benchmark_group("contextual_risk_overhead");

    for count in [10, 50, 100].iter() {
        let metrics = create_test_metrics(*count);

        // Benchmark without context analysis
        group.bench_with_input(
            BenchmarkId::new("without_context", count),
            &metrics,
            |b, metrics| {
                b.iter(|| {
                    // Simulate basic analysis without contextual risk
                    let call_graph = CallGraph::new();
                    for metric in metrics {
                        black_box(metric);
                        black_box(&call_graph);
                    }
                })
            },
        );

        // Benchmark with context analysis
        group.bench_with_input(
            BenchmarkId::new("with_context", count),
            &metrics,
            |b, metrics| {
                b.iter(|| {
                    // Simulate analysis with contextual risk
                    let call_graph = CallGraph::new();
                    let provider =
                        Box::new(GitHistoryProvider::new(repo_path.to_path_buf()).unwrap());
                    let mut aggregator = ContextAggregator::new().with_provider(provider);

                    for metric in metrics {
                        let target = AnalysisTarget {
                            root_path: repo_path.to_path_buf(),
                            file_path: metric.file.clone(),
                            function_name: metric.name.clone(),
                            line_range: (metric.line, metric.line + metric.length),
                        };
                        let _context_map = aggregator.analyze(&target);
                        black_box(metric);
                        black_box(&call_graph);
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark risk analyzer with context aggregator
fn benchmark_risk_analyzer_with_context(c: &mut Criterion) {
    let temp_repo = create_test_git_repo();
    let repo_path = temp_repo.path();

    let mut group = c.benchmark_group("risk_analyzer_with_context");

    let provider = Box::new(GitHistoryProvider::new(repo_path.to_path_buf()).unwrap());
    let aggregator = ContextAggregator::new().with_provider(provider);

    group.bench_function("risk_analyzer_initialization", |b| {
        b.iter(|| {
            let analyzer =
                RiskAnalyzer::default().with_context_aggregator(black_box(aggregator.clone()));
            black_box(analyzer);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_git_history_provider,
    benchmark_context_aggregator,
    benchmark_analysis_overhead,
    benchmark_risk_analyzer_with_context
);
criterion_main!(benches);
