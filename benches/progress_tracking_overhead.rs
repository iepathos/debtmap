use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::utils::analysis_helpers::{detect_duplications, detect_duplications_with_progress};
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_files(num_files: usize) -> (TempDir, Vec<PathBuf>) {
    let temp_dir = TempDir::new().unwrap();
    let mut files = Vec::new();

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
            "#,
            i, i
        );
        std::fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }

    (temp_dir, files)
}

fn bench_duplication_detection_without_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("duplication_detection_no_progress");

    for num_files in [50, 100, 200] {
        let (_temp_dir, files) = create_test_files(num_files);

        group.bench_with_input(
            BenchmarkId::from_parameter(num_files),
            &files,
            |b, files| {
                b.iter(|| {
                    let result = detect_duplications(black_box(files), 50);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn bench_duplication_detection_with_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("duplication_detection_with_progress");

    for num_files in [50, 100, 200] {
        let (_temp_dir, files) = create_test_files(num_files);

        group.bench_with_input(
            BenchmarkId::from_parameter(num_files),
            &files,
            |b, files| {
                b.iter(|| {
                    let result = detect_duplications_with_progress(
                        black_box(files),
                        50,
                        |current, total| {
                            black_box((current, total));
                        },
                    );
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn bench_duplication_detection_with_noop_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("duplication_detection_noop_progress");

    for num_files in [50, 100, 200] {
        let (_temp_dir, files) = create_test_files(num_files);

        group.bench_with_input(
            BenchmarkId::from_parameter(num_files),
            &files,
            |b, files| {
                b.iter(|| {
                    let result = detect_duplications_with_progress(black_box(files), 50, |_, _| {});
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn bench_progress_callback_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("progress_callback_overhead");

    for num_files in [50, 100, 200] {
        let (_temp_dir, files) = create_test_files(num_files);

        group.bench_with_input(
            BenchmarkId::new("minimal_callback", num_files),
            &files,
            |b, files| {
                b.iter(|| {
                    let result = detect_duplications_with_progress(
                        black_box(files),
                        50,
                        |current, total| {
                            // Minimal callback - just access the values
                            let _ = black_box((current, total));
                        },
                    );
                    black_box(result)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("atomic_callback", num_files),
            &files,
            |b, files| {
                b.iter(|| {
                    use std::sync::atomic::{AtomicUsize, Ordering};
                    let counter = AtomicUsize::new(0);
                    let result = detect_duplications_with_progress(
                        black_box(files),
                        50,
                        |current, _total| {
                            counter.store(current, Ordering::Relaxed);
                        },
                    );
                    black_box((result, counter.load(Ordering::Relaxed)))
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("mutex_callback", num_files),
            &files,
            |b, files| {
                b.iter(|| {
                    use std::sync::{Arc, Mutex};
                    let progress = Arc::new(Mutex::new(0));
                    let progress_clone = progress.clone();
                    let result = detect_duplications_with_progress(
                        black_box(files),
                        50,
                        move |current, _total| {
                            if let Ok(mut p) = progress_clone.try_lock() {
                                *p = current;
                            }
                        },
                    );
                    black_box((result, *progress.lock().unwrap()))
                });
            },
        );
    }

    group.finish();
}

fn bench_throttling_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("throttling_effectiveness");

    for num_files in [50, 100, 200] {
        let (_temp_dir, files) = create_test_files(num_files);

        group.bench_with_input(
            BenchmarkId::new("every_file", num_files),
            &files,
            |b, files| {
                b.iter(|| {
                    use std::sync::atomic::{AtomicUsize, Ordering};
                    let call_count = AtomicUsize::new(0);
                    let result = detect_duplications_with_progress(
                        black_box(files),
                        50,
                        |_current, _total| {
                            call_count.fetch_add(1, Ordering::Relaxed);
                        },
                    );
                    black_box((result, call_count.load(Ordering::Relaxed)))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_duplication_detection_without_progress,
    bench_duplication_detection_with_progress,
    bench_duplication_detection_with_noop_progress,
    bench_progress_callback_overhead,
    bench_throttling_effectiveness
);
criterion_main!(benches);
