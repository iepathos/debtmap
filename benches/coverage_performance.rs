use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::risk::lcov::parse_lcov_file;
use std::hint::black_box;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Create a realistic LCOV file with specified number of files and functions
fn create_lcov_file(num_files: usize, funcs_per_file: usize) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();

    for file_idx in 0..num_files {
        let file_path = format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx);
        writeln!(temp_file, "TN:").unwrap();
        writeln!(temp_file, "SF:{}", file_path).unwrap();

        for func_idx in 0..funcs_per_file {
            let line_start = func_idx * 15 + 10;
            let func_name = format!("function_{}_{}", file_idx, func_idx);

            writeln!(temp_file, "FN:{},{}", line_start, func_name).unwrap();
            writeln!(temp_file, "FNDA:5,{}", func_name).unwrap();

            // Add realistic line coverage data (10 lines per function)
            for line_offset in 0..10 {
                let line_num = line_start + line_offset;
                let count = if line_offset < 7 { 5 } else { 0 };
                writeln!(temp_file, "DA:{},{}", line_num, count).unwrap();
            }
        }

        writeln!(temp_file, "LF:{}", funcs_per_file * 10).unwrap();
        writeln!(temp_file, "LH:{}", funcs_per_file * 7).unwrap();
        writeln!(temp_file, "end_of_record").unwrap();
    }

    temp_file
}

/// Benchmark coverage index build time
fn benchmark_index_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("coverage_index_build");

    for size in [10, 50, 100, 200].iter() {
        let temp_file = create_lcov_file(*size, 20);
        let data = parse_lcov_file(temp_file.path()).unwrap();

        group.bench_with_input(
            BenchmarkId::new("files", size),
            &data,
            |b, _| {
                b.iter(|| {
                    // The index is built during parse_lcov_file, so we measure full parse
                    let data = parse_lcov_file(black_box(temp_file.path())).unwrap();
                    black_box(data);
                })
            },
        );
    }

    group.finish();
}

/// Benchmark indexed lookup vs linear search performance
fn benchmark_lookup_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("coverage_lookup");

    // Create a large dataset
    let temp_file = create_lcov_file(100, 20);
    let data = parse_lcov_file(temp_file.path()).unwrap();

    // Test indexed lookup (O(1))
    group.bench_function("indexed_lookup_by_name", |b| {
        b.iter(|| {
            for file_idx in 0..100 {
                for func_idx in 0..20 {
                    let file = PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
                    let func_name = format!("function_{}_{}", file_idx, func_idx);
                    let coverage = data.get_function_coverage(black_box(&file), black_box(&func_name));
                    black_box(coverage);
                }
            }
        })
    });

    // Test indexed lookup with line fallback (O(log n))
    group.bench_function("indexed_lookup_with_line", |b| {
        b.iter(|| {
            for file_idx in 0..100 {
                for func_idx in 0..20 {
                    let file = PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
                    let func_name = format!("unknown_name_{}", func_idx);
                    let line = func_idx * 15 + 10;
                    let coverage = data.get_function_coverage_with_line(
                        black_box(&file),
                        black_box(&func_name),
                        black_box(line),
                    );
                    black_box(coverage);
                }
            }
        })
    });

    // Test batch queries (parallel)
    let queries: Vec<(PathBuf, String, usize)> = (0..100)
        .flat_map(|file_idx| {
            (0..20).map(move |func_idx| {
                (
                    PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx)),
                    format!("function_{}_{}", file_idx, func_idx),
                    func_idx * 15 + 10,
                )
            })
        })
        .collect();

    group.bench_function("batch_parallel_lookup", |b| {
        b.iter(|| {
            let results = data.batch_get_function_coverage(black_box(&queries));
            black_box(results);
        })
    });

    group.finish();
}

/// Benchmark file analysis with coverage overhead
/// This measures the target: ≤3x overhead (≤160ms for baseline ~53ms)
fn benchmark_analysis_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("coverage_analysis_overhead");
    group.sample_size(50); // Reduce sample size for longer-running benchmarks

    // Create realistic coverage data
    let temp_file = create_lcov_file(100, 20);
    let data = parse_lcov_file(temp_file.path()).unwrap();

    // Simulate baseline analysis (without coverage lookups)
    group.bench_function("baseline_no_coverage", |b| {
        b.iter(|| {
            // Simulate processing 100 files with 20 functions each
            for file_idx in 0..100 {
                for func_idx in 0..20 {
                    // Simulate some analysis work
                    let complexity = file_idx * func_idx + 42;
                    black_box(complexity);
                }
            }
        })
    });

    // Simulate analysis with indexed coverage lookups
    group.bench_function("with_indexed_coverage", |b| {
        b.iter(|| {
            for file_idx in 0..100 {
                for func_idx in 0..20 {
                    // Simulate analysis work
                    let complexity = file_idx * func_idx + 42;
                    black_box(complexity);

                    // Add coverage lookup (indexed O(1))
                    let file = PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
                    let func_name = format!("function_{}_{}", file_idx, func_idx);
                    let coverage = data.get_function_coverage(&file, &func_name);
                    black_box(coverage);
                }
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_index_build,
    benchmark_lookup_comparison,
    benchmark_analysis_overhead
);
criterion_main!(benches);
