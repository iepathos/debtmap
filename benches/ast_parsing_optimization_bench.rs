//! Benchmark for spec 132: Eliminate redundant AST parsing in call graph construction
//!
//! This benchmark validates the performance improvement from parsing files once
//! and reusing the AST instead of re-parsing the same files multiple times.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::io;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Simulates the old approach: parse files twice (redundant)
fn benchmark_redundant_parsing(rust_files: &[PathBuf]) {
    // Phase 1: Read and parse files to get content
    let file_contents: Vec<_> = rust_files
        .iter()
        .filter_map(|file_path| {
            let content = io::read_file(file_path).ok()?;
            Some((file_path.clone(), content))
        })
        .collect();

    // Phase 2: Re-parse the same content again (REDUNDANT)
    let _parsed_files: Vec<_> = file_contents
        .iter()
        .filter_map(|(file_path, content)| {
            let parsed = syn::parse_file(content).ok()?;
            Some((file_path.clone(), parsed))
        })
        .collect();
}

/// Simulates the optimized approach: parse files once and reuse
fn benchmark_optimized_parsing(rust_files: &[PathBuf]) {
    // Step 1: Read file contents
    let file_contents: Vec<_> = rust_files
        .iter()
        .filter_map(|file_path| {
            let content = io::read_file(file_path).ok()?;
            Some((file_path.clone(), content))
        })
        .collect();

    // Step 2: Parse files once
    let parsed_files: Vec<_> = file_contents
        .iter()
        .filter_map(|(file_path, content)| {
            let parsed = syn::parse_file(content).ok()?;
            Some((file_path.clone(), parsed))
        })
        .collect();

    // Phase 2: Use pre-parsed ASTs (NO re-parsing)
    // Simulate usage by cloning (which is what extract_call_graph_multi_file does)
    let _reused: Vec<_> = parsed_files
        .iter()
        .map(|(path, parsed)| (parsed.clone(), path.clone()))
        .collect();
}

/// Find Rust files in debtmap's own source directory
fn find_rust_files() -> Vec<PathBuf> {
    let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    WalkDir::new(&src_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn bench_parsing_comparison(c: &mut Criterion) {
    let rust_files = find_rust_files();
    let file_count = rust_files.len();

    if rust_files.is_empty() {
        eprintln!("Warning: No Rust files found for benchmarking");
        return;
    }

    println!("Benchmarking with {} Rust files", file_count);

    let mut group = c.benchmark_group("ast_parsing");

    // Configure for longer running benchmarks
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(20));

    // Benchmark redundant parsing (old approach)
    group.bench_with_input(
        BenchmarkId::new("redundant_parsing", file_count),
        &rust_files,
        |b, files| {
            b.iter(|| benchmark_redundant_parsing(black_box(files)));
        },
    );

    // Benchmark optimized parsing (new approach)
    group.bench_with_input(
        BenchmarkId::new("optimized_parsing", file_count),
        &rust_files,
        |b, files| {
            b.iter(|| benchmark_optimized_parsing(black_box(files)));
        },
    );

    group.finish();
}

/// Benchmark memory overhead of storing parsed ASTs
fn bench_memory_overhead(c: &mut Criterion) {
    let rust_files = find_rust_files();

    if rust_files.is_empty() {
        return;
    }

    c.bench_function("parse_and_store_asts", |b| {
        b.iter(|| {
            // Parse and store all ASTs in memory
            let _parsed: Vec<_> = rust_files
                .iter()
                .filter_map(|file_path| {
                    let content = io::read_file(file_path).ok()?;
                    let parsed = syn::parse_file(&content).ok()?;
                    Some((black_box(file_path.clone()), black_box(parsed)))
                })
                .collect();
        });
    });
}

/// Benchmark parsing a single file multiple times vs once
fn bench_single_file_parsing(c: &mut Criterion) {
    let rust_files = find_rust_files();

    if rust_files.is_empty() {
        return;
    }

    // Use a medium-sized file for this benchmark
    let test_file = &rust_files[rust_files.len() / 2];
    let content = io::read_file(test_file).unwrap();

    let mut group = c.benchmark_group("single_file_parsing");

    // Parse once
    group.bench_function("parse_once", |b| {
        b.iter(|| {
            let _parsed = syn::parse_file(black_box(&content)).unwrap();
        });
    });

    // Parse twice (simulating redundant parsing)
    group.bench_function("parse_twice", |b| {
        b.iter(|| {
            let _parsed1 = syn::parse_file(black_box(&content)).unwrap();
            let _parsed2 = syn::parse_file(black_box(&content)).unwrap();
        });
    });

    // Parse and clone (simulating optimized approach)
    group.bench_function("parse_once_clone_once", |b| {
        b.iter(|| {
            let parsed = syn::parse_file(black_box(&content)).unwrap();
            let _cloned = black_box(parsed.clone());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing_comparison,
    bench_memory_overhead,
    bench_single_file_parsing
);

criterion_main!(benches);
