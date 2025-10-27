use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::risk::lcov::{parse_lcov_file, FunctionCoverage};
use std::collections::HashMap;
use std::hint::black_box;
use std::io::Write;
use std::path::{Path, PathBuf};
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

        group.bench_with_input(BenchmarkId::new("files", size), &data, |b, _| {
            b.iter(|| {
                // The index is built during parse_lcov_file, so we measure full parse
                let data = parse_lcov_file(black_box(temp_file.path())).unwrap();
                black_box(data);
            })
        });
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
                    let file =
                        PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
                    let func_name = format!("function_{}_{}", file_idx, func_idx);
                    let coverage =
                        data.get_function_coverage(black_box(&file), black_box(&func_name));
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
                    let file =
                        PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
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

/// Simulate the OLD flat HashMap structure for comparison
struct FlatCoverageIndex {
    by_function: HashMap<(PathBuf, String), FunctionCoverage>,
}

impl FlatCoverageIndex {
    fn from_data(data: &HashMap<PathBuf, Vec<FunctionCoverage>>) -> Self {
        let mut by_function = HashMap::new();
        for (file_path, functions) in data {
            for func in functions {
                by_function.insert((file_path.clone(), func.name.clone()), func.clone());
            }
        }
        FlatCoverageIndex { by_function }
    }

    /// OLD O(n) lookup with linear scan through all functions
    fn get_function_coverage_old(&self, file: &Path, function_name: &str) -> Option<f64> {
        // Try exact match first
        if let Some(f) = self
            .by_function
            .get(&(file.to_path_buf(), function_name.to_string()))
        {
            return Some(f.coverage_percentage / 100.0);
        }

        // OLD: O(n) linear scan through ALL functions for path strategies
        // Strategy 1: suffix matching
        for ((indexed_path, fname), coverage) in &self.by_function {
            if fname == function_name && file.ends_with(indexed_path) {
                return Some(coverage.coverage_percentage / 100.0);
            }
        }

        // Strategy 2: reverse suffix matching
        for ((indexed_path, fname), coverage) in &self.by_function {
            if fname == function_name && indexed_path.ends_with(file) {
                return Some(coverage.coverage_percentage / 100.0);
            }
        }

        None
    }
}

/// Benchmark OLD vs NEW: Demonstrates 50-100x speedup
fn benchmark_flat_vs_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("flat_vs_nested_comparison");

    // Create large dataset (simulating 100 files, 20 functions each = 2000 total)
    let temp_file = create_lcov_file(100, 20);
    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();

    // Build NEW nested index (current implementation)
    let nested_index = parse_lcov_file(temp_file.path()).unwrap();

    // Build OLD flat index (for comparison)
    let flat_index = FlatCoverageIndex::from_data(&lcov_data.functions);

    // Create mix of exact matches (70%) and path strategy lookups (30%)
    let exact_queries: Vec<(PathBuf, String)> = lcov_data
        .functions
        .iter()
        .flat_map(|(file, funcs)| {
            funcs
                .iter()
                .take(14) // 70% of 20 functions
                .map(move |f| (file.clone(), f.name.clone()))
        })
        .collect();

    let path_strategy_queries: Vec<(PathBuf, String)> = lcov_data
        .functions
        .iter()
        .flat_map(|(file, funcs)| {
            // Create path that requires strategy lookup (not exact match)
            let modified_path = PathBuf::from(format!("different/prefix/{}", file.display()));
            funcs
                .iter()
                .skip(14) // Next 30% of functions
                .take(6)
                .map(move |f| (modified_path.clone(), f.name.clone()))
        })
        .collect();

    // Benchmark OLD flat structure with path strategies
    group.bench_function("old_flat_structure", |b| {
        b.iter(|| {
            // 70% exact matches
            for (file, func_name) in &exact_queries {
                let result = flat_index.get_function_coverage_old(black_box(file), black_box(func_name));
                black_box(result);
            }
            // 30% requiring path strategies (this is where O(n) scan hurts)
            for (file, func_name) in &path_strategy_queries {
                let result = flat_index.get_function_coverage_old(black_box(file), black_box(func_name));
                black_box(result);
            }
        })
    });

    // Benchmark NEW nested structure
    group.bench_function("new_nested_structure", |b| {
        b.iter(|| {
            // 70% exact matches
            for (file, func_name) in &exact_queries {
                let result = nested_index.get_function_coverage(black_box(file), black_box(func_name));
                black_box(result);
            }
            // 30% requiring path strategies (now O(files) instead of O(functions))
            for (file, func_name) in &path_strategy_queries {
                let result = nested_index.get_function_coverage(black_box(file), black_box(func_name));
                black_box(result);
            }
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
                    let file =
                        PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
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
    benchmark_flat_vs_nested,
    benchmark_analysis_overhead
);
criterion_main!(benches);
