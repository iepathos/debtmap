//! Benchmark for pure tier classification functions (Spec 224)
//!
//! Validates that tier classification has acceptable performance
//! and ensures no regression after refactoring to smaller functions.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::priority::tiers::pure::classify_tier;
use debtmap::priority::{
    ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, TierConfig,
    UnifiedDebtItem, UnifiedScore,
};
use std::hint::black_box;
use std::path::PathBuf;

fn create_test_item(
    debt_type: DebtType,
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    deps: usize,
    final_score: f64,
    complexity_factor: f64,
) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("test.rs"),
            function: "test_fn".to_string(),
            line: 1,
        },
        debt_type,
        unified_score: UnifiedScore {
            complexity_factor,
            coverage_factor: 0.0,
            dependency_factor: 0.0,
            role_multiplier: 1.0,
            final_score: debtmap::priority::score_types::Score0To100::new(final_score),
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: ActionableRecommendation {
            primary_action: "Test".into(),
            rationale: "Test".into(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            risk_reduction: 0.0,
            complexity_reduction: 0.0,
            coverage_improvement: 0.0,
            lines_reduction: 0,
        },
        transitive_coverage: None,
        file_context: None,
        upstream_dependencies: deps,
        downstream_dependencies: deps,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: nesting,
        function_length: 10,
        cyclomatic_complexity: cyclomatic,
        cognitive_complexity: cognitive,
        entropy_details: None,
        entropy_adjusted_cyclomatic: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        is_pure: Some(false),
        purity_confidence: Some(0.0),
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: None,
        contextual_risk: None,
        file_line_count: None,
            responsibility_category: None,
    }
}

fn bench_classify_god_object(c: &mut Criterion) {
    let config = TierConfig::default();
    let item = create_test_item(
        DebtType::GodObject {
            methods: 100,
            fields: 50,
            responsibilities: 5,
            god_object_score: debtmap::priority::score_types::Score0To100::new(95.0),
        },
        10,
        10,
        2,
        5,
        5.0,
        2.0,
    );

    c.bench_function("classify_god_object", |b| {
        b.iter(|| classify_tier(black_box(&item), black_box(&config)))
    });
}

fn bench_classify_error_handling(c: &mut Criterion) {
    let config = TierConfig::default();
    let item = create_test_item(
        DebtType::ErrorSwallowing {
            pattern: "unwrap".into(),
            context: None,
        },
        10,
        10,
        2,
        5,
        5.0,
        2.0,
    );

    c.bench_function("classify_error_handling", |b| {
        b.iter(|| classify_tier(black_box(&item), black_box(&config)))
    });
}

fn bench_classify_complexity_hotspot(c: &mut Criterion) {
    let config = TierConfig::default();
    let item = create_test_item(
        DebtType::ComplexityHotspot {
            adjusted_cyclomatic: Some(30),
            cyclomatic: 35,
            cognitive: 25,
        },
        35,
        25,
        3,
        5,
        6.0,
        3.0,
    );

    c.bench_function("classify_complexity_hotspot", |b| {
        b.iter(|| classify_tier(black_box(&item), black_box(&config)))
    });
}

fn bench_classify_testing_gap(c: &mut Criterion) {
    let config = TierConfig::default();
    let item = create_test_item(
        DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 15,
            cognitive: 12,
        },
        15,
        12,
        2,
        5,
        3.0,
        1.5,
    );

    c.bench_function("classify_testing_gap", |b| {
        b.iter(|| classify_tier(black_box(&item), black_box(&config)))
    });
}

fn bench_classify_extreme_scores(c: &mut Criterion) {
    let config = TierConfig::default();

    let mut group = c.benchmark_group("classify_extreme_scores");

    // Extreme final score
    let item_extreme_score = create_test_item(
        DebtType::ComplexityHotspot {
            adjusted_cyclomatic: None,
            cyclomatic: 30,
            cognitive: 25,
        },
        30,
        25,
        3,
        5,
        11.0, // Extreme score
        3.0,
    );

    group.bench_function("extreme_final_score", |b| {
        b.iter(|| classify_tier(black_box(&item_extreme_score), black_box(&config)))
    });

    // Extreme cyclomatic
    let item_extreme_cyclomatic = create_test_item(
        DebtType::ComplexityHotspot {
            adjusted_cyclomatic: Some(60),
            cyclomatic: 70,
            cognitive: 15,
        },
        70,
        15,
        2,
        5,
        5.0,
        2.0,
    );

    group.bench_function("extreme_cyclomatic", |b| {
        b.iter(|| classify_tier(black_box(&item_extreme_cyclomatic), black_box(&config)))
    });

    // Extreme cognitive
    let item_extreme_cognitive = create_test_item(
        DebtType::ComplexityHotspot {
            adjusted_cyclomatic: None,
            cyclomatic: 15,
            cognitive: 25,
        },
        15,
        25,
        2,
        5,
        5.0,
        2.0,
    );

    group.bench_function("extreme_cognitive", |b| {
        b.iter(|| classify_tier(black_box(&item_extreme_cognitive), black_box(&config)))
    });

    // Deep nesting
    let item_deep_nesting = create_test_item(
        DebtType::ComplexityHotspot {
            adjusted_cyclomatic: None,
            cyclomatic: 15,
            cognitive: 10,
        },
        15,
        10,
        5, // Deep nesting
        5,
        5.0,
        2.0,
    );

    group.bench_function("deep_nesting", |b| {
        b.iter(|| classify_tier(black_box(&item_deep_nesting), black_box(&config)))
    });

    group.finish();
}

fn bench_classify_batch(c: &mut Criterion) {
    let config = TierConfig::default();
    let mut group = c.benchmark_group("classify_batch");

    for size in [100, 500, 1000].iter() {
        let items: Vec<_> = (0..*size)
            .map(|i| {
                let debt_type = match i % 4 {
                    0 => DebtType::GodObject {
                        methods: 50 + (i % 50),
                        fields: 20 + (i % 30),
                        responsibilities: 5,
                        god_object_score: debtmap::priority::score_types::Score0To100::new(85.0),
                    },
                    1 => DebtType::ComplexityHotspot {
                        adjusted_cyclomatic: Some(20 + (i % 30)),
                        cyclomatic: 25 + (i % 40),
                        cognitive: 15 + (i % 20),
                    },
                    2 => DebtType::TestingGap {
                        coverage: 0.0,
                        cyclomatic: 10 + (i % 15),
                        cognitive: 10 + (i % 15),
                    },
                    _ => DebtType::ErrorSwallowing {
                        pattern: "unwrap".into(),
                        context: None,
                    },
                };

                create_test_item(
                    debt_type,
                    20 + (i % 30),
                    15 + (i % 20),
                    2 + (i % 3),
                    5 + (i % 10) as usize,
                    5.0 + (i as f64 % 5.0),
                    2.0 + (i as f64 % 3.0),
                )
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("classify_items", size), size, |b, _| {
            b.iter(|| {
                for item in &items {
                    black_box(classify_tier(black_box(item), black_box(&config)));
                }
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_classify_god_object,
    bench_classify_error_handling,
    bench_classify_complexity_hotspot,
    bench_classify_testing_gap,
    bench_classify_extreme_scores,
    bench_classify_batch
);
criterion_main!(benches);
