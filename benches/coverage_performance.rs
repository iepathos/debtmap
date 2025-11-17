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
                let result =
                    flat_index.get_function_coverage_old(black_box(file), black_box(func_name));
                black_box(result);
            }
            // 30% requiring path strategies (this is where O(n) scan hurts)
            for (file, func_name) in &path_strategy_queries {
                let result =
                    flat_index.get_function_coverage_old(black_box(file), black_box(func_name));
                black_box(result);
            }
        })
    });

    // Benchmark NEW nested structure
    group.bench_function("new_nested_structure", |b| {
        b.iter(|| {
            // 70% exact matches
            for (file, func_name) in &exact_queries {
                let result =
                    nested_index.get_function_coverage(black_box(file), black_box(func_name));
                black_box(result);
            }
            // 30% requiring path strategies (now O(files) instead of O(functions))
            for (file, func_name) in &path_strategy_queries {
                let result =
                    nested_index.get_function_coverage(black_box(file), black_box(func_name));
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

/// Benchmark trait method coverage lookup with name variants (Spec 181)
///
/// Tests the performance impact of trying multiple name variants for trait methods:
/// 1. Full qualified name (e.g., "RecursiveMatchDetector::visit_expr")
/// 2. Method name only (e.g., "visit_expr")
/// 3. Trait-qualified name (e.g., "Visit::visit_expr")
///
/// Target: <5% performance impact compared to single-name lookup
fn benchmark_trait_method_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("trait_method_coverage_variants");

    // Create LCOV file with trait methods stored by method name only
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "TN:").unwrap();
    writeln!(temp_file, "SF:src/complexity/recursive_detector.rs").unwrap();

    // Add trait method with method name only (as LCOV demangles it)
    writeln!(temp_file, "FN:177,visit_expr").unwrap();
    writeln!(temp_file, "FNDA:3507,visit_expr").unwrap();
    for line in 177..200 {
        writeln!(temp_file, "DA:{},{}", line, if line < 195 { 5 } else { 0 }).unwrap();
    }

    // Add more trait methods
    writeln!(temp_file, "FN:250,visit_stmt").unwrap();
    writeln!(temp_file, "FNDA:2100,visit_stmt").unwrap();
    writeln!(temp_file, "FN:300,visit_item").unwrap();
    writeln!(temp_file, "FNDA:1500,visit_item").unwrap();

    writeln!(temp_file, "LF:50").unwrap();
    writeln!(temp_file, "LH:43").unwrap();
    writeln!(temp_file, "end_of_record").unwrap();

    let data = parse_lcov_file(temp_file.path()).unwrap();
    let file = PathBuf::from("src/complexity/recursive_detector.rs");

    // Benchmark 1: Single name lookup (baseline - what regular functions do)
    group.bench_function("baseline_single_name", |b| {
        b.iter(|| {
            // Try exact match with method name
            let coverage = data.get_function_coverage(
                black_box(&file),
                black_box("visit_expr"),
            );
            black_box(coverage);
        })
    });

    // Benchmark 2: Lookup with name variants (Spec 181 implementation)
    // This simulates trying multiple variants until one matches
    group.bench_function("with_name_variants", |b| {
        b.iter(|| {
            let full_name = "RecursiveMatchDetector::visit_expr";
            let method_name = "visit_expr";
            let trait_name = "Visit::visit_expr";

            // Try full name first (won't match in this case)
            let mut coverage = data.get_function_coverage(
                black_box(&file),
                black_box(full_name),
            );

            // Try method name (will match)
            if coverage.is_none() {
                coverage = data.get_function_coverage(
                    black_box(&file),
                    black_box(method_name),
                );
            }

            // Try trait-qualified name (won't be needed)
            if coverage.is_none() {
                coverage = data.get_function_coverage(
                    black_box(&file),
                    black_box(trait_name),
                );
            }

            black_box(coverage);
        })
    });

    // Benchmark 3: Batch lookup with variants for multiple trait methods
    group.bench_function("batch_variant_lookup", |b| {
        let trait_methods = vec![
            ("RecursiveMatchDetector::visit_expr", "visit_expr", "Visit::visit_expr"),
            ("RecursiveMatchDetector::visit_stmt", "visit_stmt", "Visit::visit_stmt"),
            ("RecursiveMatchDetector::visit_item", "visit_item", "Visit::visit_item"),
        ];

        b.iter(|| {
            for (full_name, method_name, trait_name) in &trait_methods {
                let mut coverage = data.get_function_coverage(black_box(&file), black_box(full_name));
                if coverage.is_none() {
                    coverage = data.get_function_coverage(black_box(&file), black_box(method_name));
                }
                if coverage.is_none() {
                    coverage = data.get_function_coverage(black_box(&file), black_box(trait_name));
                }
                black_box(coverage);
            }
        })
    });

    // Benchmark 4: Worst case - all 3 variants miss, fallback to line-based lookup
    group.bench_function("worst_case_line_fallback", |b| {
        b.iter(|| {
            let full_name = "SomeType::unknown_method";
            let method_name = "unknown_method";
            let trait_name = "SomeTrait::unknown_method";

            // All variants fail
            let mut coverage = data.get_function_coverage(black_box(&file), black_box(full_name));
            if coverage.is_none() {
                coverage = data.get_function_coverage(black_box(&file), black_box(method_name));
            }
            if coverage.is_none() {
                coverage = data.get_function_coverage(black_box(&file), black_box(trait_name));
            }
            // Fallback to line-based lookup
            if coverage.is_none() {
                coverage = data.get_function_coverage_with_line(
                    black_box(&file),
                    black_box(full_name),
                    black_box(177),
                );
            }
            black_box(coverage);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_index_build,
    benchmark_lookup_comparison,
    benchmark_flat_vs_nested,
    benchmark_analysis_overhead,
    benchmark_trait_method_variants
);
criterion_main!(benches);
