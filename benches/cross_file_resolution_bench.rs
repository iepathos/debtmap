//! Performance benchmarks for parallel vs sequential cross-file call resolution
//!
//! Validates spec 133 requirement: 10-15% performance improvement on multi-core systems

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::hint::black_box;
use std::path::PathBuf;

/// Create a test graph with many unresolved cross-file calls
/// This simulates a large codebase (392+ files) with complex call patterns
fn create_large_graph_with_unresolved_calls(num_files: usize, calls_per_file: usize) -> CallGraph {
    let mut graph = CallGraph::new();

    // Create functions across many files
    for file_idx in 0..num_files {
        for func_idx in 0..10 {
            let func_id = FunctionId::new(
                PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx)),
                format!("function_{}_{}", file_idx, func_idx),
                func_idx * 10,
            );
            graph.add_function(
                func_id,
                func_idx == 0,
                false,
                (func_idx % 5) as u32,
                func_idx * 5,
            );
        }
    }

    // Create unresolved cross-file calls
    for file_idx in 0..num_files {
        for call_idx in 0..calls_per_file {
            let caller = FunctionId::new(
                PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx)),
                format!("function_{}_{}", file_idx, call_idx % 10),
                (call_idx % 10) * 10,
            );

            // Target function in a different file
            let target_file = (file_idx + 1 + call_idx) % num_files;
            let target_func = call_idx % 10;

            let callee = FunctionId {
                file: PathBuf::from("unknown.rs"),
                name: format!("function_{}_{}", target_file, target_func),
                line: 0, // Line 0 indicates unresolved
                module_path: String::new(),
            };

            graph.add_call(FunctionCall {
                caller,
                callee,
                call_type: CallType::Direct,
            });
        }
    }

    graph
}

/// Benchmark parallel cross-file resolution
fn bench_parallel_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_file_resolution_parallel");

    for num_files in [100, 200, 392, 500].iter() {
        let calls_per_file = 5;
        group.bench_with_input(
            BenchmarkId::new("parallel", num_files),
            num_files,
            |b, &num_files| {
                b.iter(|| {
                    let mut graph = create_large_graph_with_unresolved_calls(num_files, calls_per_file);
                    black_box(&mut graph).resolve_cross_file_calls();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sequential cross-file resolution for comparison
fn bench_sequential_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_file_resolution_sequential");

    for num_files in [100, 200, 392, 500].iter() {
        let calls_per_file = 5;
        group.bench_with_input(
            BenchmarkId::new("sequential", num_files),
            num_files,
            |b, &num_files| {
                b.iter(|| {
                    let mut graph = create_large_graph_with_unresolved_calls(num_files, calls_per_file);
                    black_box(&mut graph).resolve_cross_file_calls_sequential();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark comparing parallel vs sequential on a realistic large codebase
fn bench_parallel_vs_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_vs_sequential_comparison");

    // Simulate a large codebase (392 files, 5 unresolved calls per file = 1960 calls)
    let num_files = 392;
    let calls_per_file = 5;

    group.bench_function("parallel_large_codebase", |b| {
        b.iter(|| {
            let mut graph = create_large_graph_with_unresolved_calls(num_files, calls_per_file);
            black_box(&mut graph).resolve_cross_file_calls();
        });
    });

    group.bench_function("sequential_large_codebase", |b| {
        b.iter(|| {
            let mut graph = create_large_graph_with_unresolved_calls(num_files, calls_per_file);
            black_box(&mut graph).resolve_cross_file_calls_sequential();
        });
    });

    group.finish();
}

/// Benchmark with varying numbers of unresolved calls
fn bench_varying_call_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("varying_unresolved_calls");
    let num_files = 200;

    for calls_per_file in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("parallel", calls_per_file),
            calls_per_file,
            |b, &calls_per_file| {
                b.iter(|| {
                    let mut graph = create_large_graph_with_unresolved_calls(num_files, calls_per_file);
                    black_box(&mut graph).resolve_cross_file_calls();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parallel_resolution,
    bench_sequential_resolution,
    bench_parallel_vs_sequential,
    bench_varying_call_counts
);

criterion_main!(benches);
