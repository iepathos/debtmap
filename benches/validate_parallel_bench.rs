use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::builders::{call_graph, parallel_call_graph};
use debtmap::core::*;
use debtmap::priority::call_graph::CallGraph;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test project with multiple Rust files
fn create_test_project(num_files: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    for i in 0..num_files {
        let file_content = format!(
            r#"
            pub fn function_{}_a(x: i32) -> i32 {{
                if x > 0 {{
                    if x > 10 {{
                        x * 2
                    }} else {{
                        x + 1
                    }}
                }} else {{
                    0
                }}
            }}

            pub fn function_{}_b(a: i32, b: i32) -> i32 {{
                let result = a + b;
                if result > 0 {{
                    result * 2
                }} else {{
                    result / 2
                }}
            }}

            pub fn function_{}_c() -> i32 {{
                let mut sum = 0;
                for i in 0..10 {{
                    if i % 2 == 0 {{
                        sum += i;
                    }} else {{
                        sum -= i;
                    }}
                }}
                sum
            }}

            pub fn function_{}_d(values: &[i32]) -> i32 {{
                values.iter().filter(|&&x| x > 0).sum()
            }}
            "#,
            i, i, i, i
        );

        std::fs::write(
            temp_dir.path().join(format!("module_{}.rs", i)),
            file_content,
        )
        .unwrap();
    }

    temp_dir
}

/// Benchmark sequential call graph construction
fn bench_validate_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_sequential");

    for num_files in [10, 25, 50].iter() {
        let temp_dir = create_test_project(*num_files);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", num_files)),
            num_files,
            |b, _| {
                b.iter(|| {
                    let mut call_graph = CallGraph::new();
                    let project_path = temp_dir.path();

                    // Sequential call graph construction
                    let result = call_graph::process_rust_files_for_call_graph(
                        black_box(project_path),
                        &mut call_graph,
                        false,
                        false,
                    );

                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel call graph construction
fn bench_validate_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_parallel");

    for num_files in [10, 25, 50].iter() {
        let temp_dir = create_test_project(*num_files);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_files", num_files)),
            num_files,
            |b, _| {
                b.iter(|| {
                    let base_graph = CallGraph::new();
                    let project_path = temp_dir.path();

                    // Parallel call graph construction
                    let result = parallel_call_graph::build_call_graph_parallel(
                        black_box(project_path),
                        base_graph,
                        None,  // Use all cores
                        false, // No progress output
                    );

                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel vs sequential with different thread counts
fn bench_validate_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_thread_scaling");
    let num_files = 50;
    let temp_dir = create_test_project(num_files);

    // Benchmark sequential
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let mut call_graph = CallGraph::new();
            let result = call_graph::process_rust_files_for_call_graph(
                black_box(temp_dir.path()),
                &mut call_graph,
                false,
                false,
            );
            black_box(result)
        });
    });

    // Benchmark parallel with different thread counts
    for threads in [2, 4, 8, 0].iter() {
        let thread_label = if *threads == 0 {
            "all_cores".to_string()
        } else {
            format!("{}_threads", threads)
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(&thread_label),
            threads,
            |b, &num_threads| {
                b.iter(|| {
                    let base_graph = CallGraph::new();
                    let thread_count = if num_threads == 0 {
                        None
                    } else {
                        Some(num_threads)
                    };

                    let result = parallel_call_graph::build_call_graph_parallel(
                        black_box(temp_dir.path()),
                        base_graph,
                        thread_count,
                        false,
                    );
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark to verify parallel speedup on large projects
fn bench_validate_performance_target(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_performance_target");
    group.sample_size(10); // Reduce sample size for large benchmarks

    let num_files = 100;
    let temp_dir = create_test_project(num_files);

    // Sequential baseline
    group.bench_function("sequential_100_files", |b| {
        b.iter(|| {
            let mut call_graph = CallGraph::new();
            let result = call_graph::process_rust_files_for_call_graph(
                black_box(temp_dir.path()),
                &mut call_graph,
                false,
                false,
            );
            black_box(result)
        });
    });

    // Parallel target (should be 70-90% faster)
    group.bench_function("parallel_100_files", |b| {
        b.iter(|| {
            let base_graph = CallGraph::new();
            let result = parallel_call_graph::build_call_graph_parallel(
                black_box(temp_dir.path()),
                base_graph,
                None,
                false,
            );
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_validate_sequential,
    bench_validate_parallel,
    bench_validate_thread_scaling,
    bench_validate_performance_target
);
criterion_main!(benches);
