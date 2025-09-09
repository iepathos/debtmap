use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::risk::lcov::{parse_lcov_file, FunctionCoverage, LcovData};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn create_large_lcov_file() -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();
    
    // Create LCOV content with many functions to test parallel processing
    for file_idx in 0..50 {
        let file_path = format!("/path/to/file_{}.rs", file_idx);
        writeln!(temp_file, "TN:").unwrap();
        writeln!(temp_file, "SF:{}", file_path).unwrap();
        
        // Add many functions per file
        for func_idx in 0..20 {
            let line_start = func_idx * 10 + 10;
            let func_name = format!("function_{}_{}", file_idx, func_idx);
            
            writeln!(temp_file, "FN:{},{}", line_start, func_name).unwrap();
            writeln!(temp_file, "FNDA:5,{}", func_name).unwrap();
            
            // Add line coverage data
            for line_offset in 0..8 {
                let line_num = line_start + line_offset;
                let count = if line_offset < 6 { 5 } else { 0 };
                writeln!(temp_file, "DA:{},{}", line_num, count).unwrap();
            }
        }
        
        writeln!(temp_file, "LF:160").unwrap(); // 20 functions * 8 lines
        writeln!(temp_file, "LH:120").unwrap(); // 20 functions * 6 covered lines
        writeln!(temp_file, "end_of_record").unwrap();
    }
    
    temp_file
}

fn benchmark_lcov_parsing(c: &mut Criterion) {
    let temp_file = create_large_lcov_file();
    
    c.bench_function("parse_large_lcov_file", |b| {
        b.iter(|| {
            let data = parse_lcov_file(black_box(temp_file.path())).unwrap();
            black_box(data);
        })
    });
    
    // Benchmark coverage queries
    let data = parse_lcov_file(temp_file.path()).unwrap();
    let test_paths: Vec<PathBuf> = (0..50)
        .map(|i| PathBuf::from(format!("/path/to/file_{}.rs", i)))
        .collect();
    
    c.bench_function("get_all_file_coverages", |b| {
        b.iter(|| {
            let coverages = data.get_all_file_coverages();
            black_box(coverages);
        })
    });
    
    // Benchmark batch queries
    let queries: Vec<(PathBuf, String, usize)> = (0..50)
        .flat_map(|file_idx| {
            (0..20).map(move |func_idx| {
                (
                    PathBuf::from(format!("/path/to/file_{}.rs", file_idx)),
                    format!("function_{}_{}", file_idx, func_idx),
                    func_idx * 10 + 10,
                )
            })
        })
        .collect();
    
    c.bench_function("batch_get_function_coverage", |b| {
        b.iter(|| {
            let results = data.batch_get_function_coverage(black_box(&queries));
            black_box(results);
        })
    });
}

criterion_group!(benches, benchmark_lcov_parsing);
criterion_main!(benches);