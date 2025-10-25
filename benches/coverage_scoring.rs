/// Performance benchmarks for role-based coverage scoring (spec 119)
///
/// Verifies that the role-based coverage expectations and scoring have
/// minimal performance impact (<3% of analysis time) as required by spec.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::priority::scoring::{calculate_coverage_score, CoverageExpectations};

fn bench_coverage_score_calculation(c: &mut Criterion) {
    let expectations = CoverageExpectations::default();
    let roles = [
        "Pure",
        "BusinessLogic",
        "StateManagement",
        "IoOperations",
        "Validation",
        "ErrorHandling",
        "Configuration",
        "Initialization",
        "Orchestration",
        "Utilities",
        "Debug",
        "Performance",
    ];

    let mut group = c.benchmark_group("coverage_score_calculation");

    // Benchmark individual score calculation
    group.bench_function("single_score", |b| {
        b.iter(|| {
            calculate_coverage_score(black_box(65.0), black_box("BusinessLogic"), &expectations)
        })
    });

    // Benchmark batch calculation (simulating 1000 functions)
    for batch_size in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            batch_size,
            |b, &size| {
                b.iter(|| {
                    for i in 0..size {
                        let coverage = (i as f64 % 100.0); // Vary coverage
                        let role = roles[i % roles.len()];
                        black_box(calculate_coverage_score(coverage, role, &expectations));
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_gap_severity_classification(c: &mut Criterion) {
    use debtmap::priority::scoring::{CoverageGap, CoverageRange};

    let mut group = c.benchmark_group("gap_severity");

    let range = CoverageRange::new(80.0, 90.0, 100.0);

    // Benchmark gap calculation for different severities
    for (name, coverage) in [
        ("critical", 30.0),
        ("moderate", 50.0),
        ("minor", 85.0),
        ("none", 95.0),
    ]
    .iter()
    {
        group.bench_with_input(BenchmarkId::new("calculate", name), coverage, |b, &cov| {
            b.iter(|| CoverageGap::calculate(black_box(cov), &range))
        });
    }

    group.finish();
}

fn bench_role_expectations_lookup(c: &mut Criterion) {
    let expectations = CoverageExpectations::default();
    let roles = [
        "Pure",
        "BusinessLogic",
        "Debug",
        "Validation",
        "IoOperations",
    ];

    let mut group = c.benchmark_group("role_expectations_lookup");

    // Benchmark role lookup performance
    group.bench_function("single_lookup", |b| {
        b.iter(|| expectations.for_role(black_box("BusinessLogic")))
    });

    // Benchmark batch lookups
    group.bench_function("batch_lookup_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let role = roles[i % roles.len()];
                black_box(expectations.for_role(role));
            }
        })
    });

    group.finish();
}

fn bench_coverage_expectations_default(c: &mut Criterion) {
    c.bench_function("coverage_expectations_default", |b| {
        b.iter(|| CoverageExpectations::default())
    });
}

criterion_group!(
    benches,
    bench_coverage_score_calculation,
    bench_gap_severity_classification,
    bench_role_expectations_lookup,
    bench_coverage_expectations_default,
);
criterion_main!(benches);
