/// Performance benchmarks for concise recommendation generation (spec 138a)
///
/// Verifies that the recommendation generation has minimal overhead (<5ms)
/// as required by spec 138a for real-time analysis workflow integration.
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::core::FunctionMetrics;
use debtmap::priority::scoring::concise_recommendation::generate_concise_recommendation;
use debtmap::priority::semantic_classifier::FunctionRole;
use debtmap::priority::{DebtType, FunctionVisibility, TransitiveCoverage};
use std::hint::black_box;
use std::path::PathBuf;

fn create_test_metrics(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from("test.rs"),
        line: 10,
        cyclomatic,
        cognitive,
        nesting: 2,
        length: 50,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
    }
}

fn bench_testing_gap_recommendation(c: &mut Criterion) {
    let mut group = c.benchmark_group("testing_gap_recommendation");

    // Benchmark different complexity levels for testing gap recommendations
    let scenarios = [
        ("simple", 5, 8, 50.0),
        ("moderate", 15, 20, 30.0),
        ("complex", 30, 40, 20.0),
        ("high_complexity", 50, 60, 10.0),
    ];

    for (name, cyclomatic, cognitive, coverage_pct) in scenarios.iter() {
        group.bench_with_input(
            BenchmarkId::new("single", name),
            &(cyclomatic, cognitive, coverage_pct),
            |b, &(cyc, cog, cov)| {
                let metrics = create_test_metrics("test_func", *cyc, *cog);
                let debt = DebtType::TestingGap {
                    coverage: *cov / 100.0,
                    cyclomatic: *cyc,
                    cognitive: *cog,
                };
                b.iter(|| {
                    generate_concise_recommendation(
                        black_box(&debt),
                        black_box(&metrics),
                        black_box(FunctionRole::PureLogic),
                        black_box(&None),
                    )
                })
            },
        );
    }

    group.finish();
}

fn bench_complexity_hotspot_recommendation(c: &mut Criterion) {
    let mut group = c.benchmark_group("complexity_hotspot_recommendation");

    let scenarios = [
        ("moderate", 16, 22),
        ("high", 25, 35),
        ("extreme", 40, 55),
    ];

    for (name, cyclomatic, cognitive) in scenarios.iter() {
        group.bench_with_input(
            BenchmarkId::new("single", name),
            &(cyclomatic, cognitive),
            |b, &(cyc, cog)| {
                let metrics = create_test_metrics("complex_func", *cyc, *cog);
                let debt = DebtType::ComplexityHotspot {
                    cyclomatic: *cyc,
                    cognitive: *cog,
                };
                b.iter(|| {
                    generate_concise_recommendation(
                        black_box(&debt),
                        black_box(&metrics),
                        black_box(FunctionRole::Orchestrator),
                        black_box(&None),
                    )
                })
            },
        );
    }

    group.finish();
}

fn bench_dead_code_recommendation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dead_code_recommendation");

    let visibilities = [
        ("public", FunctionVisibility::Public),
        ("private", FunctionVisibility::Private),
        ("crate", FunctionVisibility::Crate),
    ];

    for (name, visibility) in visibilities.iter() {
        group.bench_with_input(BenchmarkId::new("single", name), visibility, |b, vis| {
            let metrics = create_test_metrics("unused_func", 10, 12);
            let debt = DebtType::DeadCode {
                visibility: vis.clone(),
                cyclomatic: 10,
                cognitive: 12,
                usage_hints: vec![],
            };
            b.iter(|| {
                generate_concise_recommendation(
                    black_box(&debt),
                    black_box(&metrics),
                    black_box(FunctionRole::PureLogic),
                    black_box(&None),
                )
            })
        });
    }

    group.finish();
}

fn bench_batch_recommendation_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_recommendation");

    // Simulate analyzing a file with multiple debt items
    let batch_sizes = [10, 50, 100];

    for batch_size in batch_sizes.iter() {
        group.bench_with_input(
            BenchmarkId::new("mixed_debt_types", batch_size),
            batch_size,
            |b, &size| {
                // Create a mix of debt items
                let debt_items: Vec<_> = (0..size)
                    .map(|i| {
                        let cyclomatic = 10 + (i % 30);
                        let cognitive = 15 + (i % 40);
                        let coverage_pct = 20.0 + (i as f64 % 80.0);

                        match i % 3 {
                            0 => DebtType::TestingGap {
                                coverage: coverage_pct / 100.0,
                                cyclomatic,
                                cognitive,
                            },
                            1 => DebtType::ComplexityHotspot {
                                cyclomatic,
                                cognitive,
                            },
                            _ => DebtType::DeadCode {
                                visibility: FunctionVisibility::Private,
                                cyclomatic,
                                cognitive,
                                usage_hints: vec![],
                            },
                        }
                    })
                    .collect();

                b.iter(|| {
                    for (i, debt) in debt_items.iter().enumerate() {
                        let metrics =
                            create_test_metrics(&format!("func_{}", i), 15 + i as u32, 20 + i as u32);
                        black_box(generate_concise_recommendation(
                            debt,
                            &metrics,
                            FunctionRole::PureLogic,
                            &None,
                        ));
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_with_transitive_coverage(c: &mut Criterion) {
    let mut group = c.benchmark_group("with_transitive_coverage");

    let coverage = Some(TransitiveCoverage {
        direct: 0.5,
        transitive: 0.7,
        propagated_from: vec![],
        uncovered_lines: vec![],
    });

    group.bench_function("testing_gap_with_coverage", |b| {
        let metrics = create_test_metrics("tested_func", 15, 20);
        let debt = DebtType::TestingGap {
            coverage: 0.5,
            cyclomatic: 15,
            cognitive: 20,
        };
        b.iter(|| {
            generate_concise_recommendation(
                black_box(&debt),
                black_box(&metrics),
                black_box(FunctionRole::Orchestrator),
                black_box(&coverage),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_testing_gap_recommendation,
    bench_complexity_hotspot_recommendation,
    bench_dead_code_recommendation,
    bench_batch_recommendation_generation,
    bench_with_transitive_coverage,
);
criterion_main!(benches);
