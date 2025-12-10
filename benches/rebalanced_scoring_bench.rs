//! Benchmark for rebalanced scoring algorithm (Spec 136)
//!
//! Validates that the rebalanced scoring has minimal performance impact
//! compared to legacy scoring.

use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::core::FunctionMetrics;
use debtmap::priority::scoring::rebalanced::{DebtScore, ScoreWeights};
use debtmap::priority::DebtType;
use std::hint::black_box;
use std::path::PathBuf;

fn create_test_function(
    name: &str,
    cyclomatic: u32,
    cognitive: u32,
    length: usize,
) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from("src/test.rs"),
        line: 1,
        cyclomatic,
        cognitive,
        nesting: (cognitive / 10).min(5),
        length,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.5),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_reason: None,
        call_dependencies: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

fn bench_single_score_calculation(c: &mut Criterion) {
    let func = create_test_function("test_function", 25, 40, 150);
    let debt_type = DebtType::TestingGap {
        coverage: 0.3,
        cyclomatic: 25,
        cognitive: 40,
    };
    let weights = ScoreWeights::default();

    c.bench_function("rebalanced_single_score", |b| {
        b.iter(|| {
            DebtScore::calculate(black_box(&func), black_box(&debt_type), black_box(&weights))
        })
    });
}

fn bench_batch_score_calculation(c: &mut Criterion) {
    // Create a batch of 100 functions with varying complexity
    let functions: Vec<_> = (0..100)
        .map(|i| {
            create_test_function(
                &format!("func_{}", i),
                (i % 50) as u32,
                (i % 80) as u32,
                100 + (i * 2),
            )
        })
        .collect();

    let debt_types: Vec<_> = functions
        .iter()
        .enumerate()
        .map(|(i, f)| {
            if i % 3 == 0 {
                DebtType::TestingGap {
                    coverage: 0.3,
                    cyclomatic: f.cyclomatic,
                    cognitive: f.cognitive,
                }
            } else if i % 3 == 1 {
                DebtType::ComplexityHotspot {
                    cyclomatic: f.cyclomatic,
                    cognitive: f.cognitive,
                }
            } else {
                DebtType::Risk {
                    risk_score: 0.5,
                    factors: vec!["Test factor".to_string()],
                }
            }
        })
        .collect();

    let weights = ScoreWeights::default();

    c.bench_function("rebalanced_batch_100_scores", |b| {
        b.iter(|| {
            functions
                .iter()
                .zip(debt_types.iter())
                .map(|(func, debt_type)| {
                    DebtScore::calculate(black_box(func), black_box(debt_type), black_box(&weights))
                })
                .collect::<Vec<_>>()
        })
    });
}

fn bench_preset_weights(c: &mut Criterion) {
    let func = create_test_function("test_function", 25, 40, 150);
    let debt_type = DebtType::TestingGap {
        coverage: 0.3,
        cyclomatic: 25,
        cognitive: 40,
    };

    let mut group = c.benchmark_group("preset_weights");

    group.bench_function("balanced", |b| {
        let weights = ScoreWeights::balanced();
        b.iter(|| {
            DebtScore::calculate(black_box(&func), black_box(&debt_type), black_box(&weights))
        })
    });

    group.bench_function("quality_focused", |b| {
        let weights = ScoreWeights::quality_focused();
        b.iter(|| {
            DebtScore::calculate(black_box(&func), black_box(&debt_type), black_box(&weights))
        })
    });

    group.bench_function("test_coverage_focused", |b| {
        let weights = ScoreWeights::test_coverage_focused();
        b.iter(|| {
            DebtScore::calculate(black_box(&func), black_box(&debt_type), black_box(&weights))
        })
    });

    group.bench_function("size_focused", |b| {
        let weights = ScoreWeights::size_focused();
        b.iter(|| {
            DebtScore::calculate(black_box(&func), black_box(&debt_type), black_box(&weights))
        })
    });

    group.finish();
}

fn bench_generated_code_detection(c: &mut Criterion) {
    let mut generated_func = create_test_function("generated", 10, 15, 500);
    generated_func.file = PathBuf::from("src/proto/api.pb.rs");

    let mut normal_func = create_test_function("normal", 10, 15, 500);
    normal_func.file = PathBuf::from("src/processor.rs");

    let debt_type = DebtType::Risk {
        risk_score: 0.5,
        factors: vec!["Long function".to_string()],
    };
    let weights = ScoreWeights::default();

    let mut group = c.benchmark_group("generated_code_detection");

    group.bench_function("generated_file", |b| {
        b.iter(|| {
            DebtScore::calculate(
                black_box(&generated_func),
                black_box(&debt_type),
                black_box(&weights),
            )
        })
    });

    group.bench_function("normal_file", |b| {
        b.iter(|| {
            DebtScore::calculate(
                black_box(&normal_func),
                black_box(&debt_type),
                black_box(&weights),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_score_calculation,
    bench_batch_score_calculation,
    bench_preset_weights,
    bench_generated_code_detection
);
criterion_main!(benches);
