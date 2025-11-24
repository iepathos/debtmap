//! Benchmark for score-based ranking and sorting (Spec 171)
//!
//! Validates that score-based sorting has acceptable performance
//! (target: within 5% of previous tier-based implementation).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::data_flow::DataFlowGraph;
use debtmap::priority::call_graph::CallGraph;
use debtmap::priority::scoring::scaling::{calculate_final_score, ScalingConfig};
use debtmap::priority::semantic_classifier::FunctionRole;
use debtmap::priority::unified_analysis_queries::UnifiedAnalysisQueries;
use debtmap::priority::unified_scorer::{Location, UnifiedScore};
use debtmap::priority::{
    ActionableRecommendation, DebtType, ImpactMetrics, UnifiedAnalysis, UnifiedDebtItem,
};
use std::hint::black_box;
use std::path::PathBuf;

fn create_test_item(
    base_score: f64,
    debt_type: DebtType,
    upstream_deps: usize,
    downstream_deps: usize,
    role: FunctionRole,
    cyclomatic: u32,
) -> UnifiedDebtItem {
    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("test.rs"),
            function: "test_func".to_string(),
            line: 1,
        },
        debt_type,
        unified_score: UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 5.0,
            dependency_factor: 5.0,
            role_multiplier: 1.0,
            final_score: base_score,
            base_score: Some(base_score),
            exponential_factor: Some(1.0),
            risk_boost: Some(1.0),
            pre_adjustment_score: None,
            adjustment_applied: None,
        },
        function_role: role,
        recommendation: ActionableRecommendation {
            primary_action: "Test".to_string(),
            rationale: "Test".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        transitive_coverage: None,
        upstream_dependencies: upstream_deps,
        downstream_dependencies: downstream_deps,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: 1,
        function_length: 10,
        cyclomatic_complexity: cyclomatic,
        cognitive_complexity: cyclomatic,
        entropy_details: None,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
        file_context: None,
    }
}

fn bench_score_calculation(c: &mut Criterion) {
    let config = ScalingConfig::default();
    let item = create_test_item(
        30.0,
        DebtType::GodObject {
            methods: 50,
            fields: 20,
            responsibilities: 10,
            god_object_score: 85.0,
        },
        10,
        10,
        FunctionRole::EntryPoint,
        35,
    );

    c.bench_function("score_calculation_single", |b| {
        b.iter(|| {
            calculate_final_score(
                black_box(30.0),
                black_box(&item.debt_type),
                black_box(&item),
                black_box(&config),
            )
        })
    });
}

fn bench_sorting_various_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("score_based_sorting");

    for size in [100, 500, 1000, 5000].iter() {
        // Generate test items with varying scores
        let items: im::Vector<_> = (0..*size)
            .map(|i| {
                let base_score = (i as f64 % 100.0) + 1.0;
                let debt_type = match i % 3 {
                    0 => DebtType::GodObject {
                        methods: 50,
                        fields: 20,
                        responsibilities: 10,
                        god_object_score: 85.0,
                    },
                    1 => DebtType::ComplexityHotspot {
                        cyclomatic: 35,
                        cognitive: 40,
                        adjusted_cyclomatic: None,
                    },
                    _ => DebtType::TestingGap {
                        coverage: 0.0,
                        cyclomatic: 15,
                        cognitive: 20,
                    },
                };

                create_test_item(base_score, debt_type, 5, 5, FunctionRole::PureLogic, 20)
            })
            .collect();

        let analysis = UnifiedAnalysis {
            items: items.clone(),
            file_items: im::Vector::new(),
            total_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 0.0,
            debt_density: 0.0,
            total_lines_of_code: 0,
            call_graph: CallGraph::new(),
            data_flow_graph: DataFlowGraph::new(),
            overall_coverage: None,
            has_coverage_data: false,
            timings: None,
        };

        group.bench_with_input(
            BenchmarkId::new("sort_and_get_top_100", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(analysis.get_top_mixed_priorities(100));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sort_and_get_top_50", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(analysis.get_top_mixed_priorities(50));
                })
            },
        );
    }

    group.finish();
}

fn bench_worst_case_sorting(c: &mut Criterion) {
    // Worst case: all items have very similar scores (lots of comparisons)
    let items: im::Vector<_> = (0..1000)
        .map(|i| {
            let base_score = 50.0 + (i as f64 % 10.0) / 10.0; // Scores clustered around 50
            create_test_item(
                base_score,
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 15,
                    cognitive: 20,
                },
                5,
                5,
                FunctionRole::PureLogic,
                20,
            )
        })
        .collect();

    let analysis = UnifiedAnalysis {
        items: items.clone(),
        file_items: im::Vector::new(),
        total_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        total_debt_score: 0.0,
        debt_density: 0.0,
        total_lines_of_code: 0,
        call_graph: CallGraph::new(),
        data_flow_graph: DataFlowGraph::new(),
        overall_coverage: None,
        has_coverage_data: false,
        timings: None,
    };

    c.bench_function("worst_case_similar_scores_1000", |b| {
        b.iter(|| {
            black_box(analysis.get_top_mixed_priorities(100));
        })
    });
}

fn bench_mixed_debt_types(c: &mut Criterion) {
    // Mix of different debt types with exponential scaling
    let items: im::Vector<_> = (0..1000)
        .map(|i| {
            let base_score = ((i * 7) % 100) as f64 + 1.0;
            let debt_type = match i % 5 {
                0 => DebtType::GodObject {
                    methods: 50,
                    fields: 20,
                    responsibilities: 10,
                    god_object_score: 85.0,
                },
                1 => DebtType::GodModule {
                    functions: 100,
                    lines: 1000,
                    responsibilities: 15,
                },
                2 => DebtType::ComplexityHotspot {
                    cyclomatic: 35,
                    cognitive: 40,
                    adjusted_cyclomatic: None,
                },
                3 => DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 25,
                    cognitive: 30,
                },
                _ => DebtType::Risk {
                    risk_score: 0.8,
                    factors: vec!["High risk".to_string()],
                },
            };

            create_test_item(base_score, debt_type, 5, 5, FunctionRole::PureLogic, 20)
        })
        .collect();

    let analysis = UnifiedAnalysis {
        items: items.clone(),
        file_items: im::Vector::new(),
        total_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        total_debt_score: 0.0,
        debt_density: 0.0,
        total_lines_of_code: 0,
        call_graph: CallGraph::new(),
        data_flow_graph: DataFlowGraph::new(),
        overall_coverage: None,
        has_coverage_data: false,
        timings: None,
    };

    c.bench_function("mixed_debt_types_1000", |b| {
        b.iter(|| {
            black_box(analysis.get_top_mixed_priorities(100));
        })
    });
}

fn bench_with_risk_boosts(c: &mut Criterion) {
    // Items with varying dependency counts to trigger risk boosts
    let items: im::Vector<_> = (0..1000)
        .map(|i| {
            let base_score = ((i * 3) % 100) as f64 + 1.0;
            let upstream = (i % 30) as usize;
            let downstream = ((i * 2) % 30) as usize;
            let role = if i % 5 == 0 {
                FunctionRole::EntryPoint
            } else {
                FunctionRole::PureLogic
            };

            create_test_item(
                base_score,
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 25,
                    cognitive: 30,
                },
                upstream,
                downstream,
                role,
                25,
            )
        })
        .collect();

    let analysis = UnifiedAnalysis {
        items: items.clone(),
        file_items: im::Vector::new(),
        total_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        total_debt_score: 0.0,
        debt_density: 0.0,
        total_lines_of_code: 0,
        call_graph: CallGraph::new(),
        data_flow_graph: DataFlowGraph::new(),
        overall_coverage: None,
        has_coverage_data: false,
        timings: None,
    };

    c.bench_function("with_risk_boosts_1000", |b| {
        b.iter(|| {
            black_box(analysis.get_top_mixed_priorities(100));
        })
    });
}

criterion_group!(
    benches,
    bench_score_calculation,
    bench_sorting_various_sizes,
    bench_worst_case_sorting,
    bench_mixed_debt_types,
    bench_with_risk_boosts
);
criterion_main!(benches);
