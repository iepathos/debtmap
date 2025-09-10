use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::builders::parallel_unified_analysis::{
    ParallelUnifiedAnalysis, ParallelUnifiedAnalysisOptions,
};
use debtmap::builders::unified_analysis::UnifiedAnalysisBuilder;
use debtmap::priority::UnifiedAnalysis;
use std::path::PathBuf;
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
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &path,
            |b, path| {
                b.iter(|| {
                    let builder = UnifiedAnalysisBuilder::new();
                    let analysis = builder
                        .with_path(path.clone())
                        .build()
                        .unwrap();
                    black_box(analysis.run().unwrap());
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
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &path,
            |b, path| {
                b.iter(|| {
                    let options = ParallelUnifiedAnalysisOptions {
                        parallel: true,
                        jobs: None,
                        batch_size: 100,
                        progress: false,
                    };
                    
                    let analysis = ParallelUnifiedAnalysis::new(
                        path.clone(),
                        None,
                        None,
                        options,
                    );
                    black_box(analysis.run().unwrap());
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
    
    for jobs in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_jobs", jobs)),
            &(*jobs, &path),
            |b, (jobs, path)| {
                b.iter(|| {
                    let options = ParallelUnifiedAnalysisOptions {
                        parallel: true,
                        jobs: Some(*jobs),
                        batch_size: 100,
                        progress: false,
                    };
                    
                    let analysis = ParallelUnifiedAnalysis::new(
                        path.clone(),
                        None,
                        None,
                        options,
                    );
                    black_box(analysis.run().unwrap());
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_size_comparison");
    let temp_dir = create_test_project(200);
    let path = temp_dir.path().to_path_buf();
    
    for batch_size in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{}", batch_size)),
            &(*batch_size, &path),
            |b, (batch_size, path)| {
                b.iter(|| {
                    let options = ParallelUnifiedAnalysisOptions {
                        parallel: true,
                        jobs: None,
                        batch_size: *batch_size,
                        progress: false,
                    };
                    
                    let analysis = ParallelUnifiedAnalysis::new(
                        path.clone(),
                        None,
                        None,
                        options,
                    );
                    black_box(analysis.run().unwrap());
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
    benchmark_batch_sizes
);
criterion_main!(benches);