//! Benchmarks for orchestration score adjustment (Spec 110)
//!
//! Validates that the orchestration adjustment has minimal performance overhead
//! (target: < 5% of total scoring time)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use debtmap::priority::scoring::orchestration_adjustment::{
    adjust_score, extract_composition_metrics, OrchestrationAdjustmentConfig,
};
use debtmap::priority::semantic_classifier::FunctionRole;
use std::hint::black_box;
use std::path::PathBuf;

fn create_test_func(name: &str, cyclomatic: u32, cognitive: u32, length: usize) -> FunctionMetrics {
    FunctionMetrics {
        file: PathBuf::from("test.rs"),
        name: name.to_string(),
        line: 1,
        length,
        cyclomatic,
        cognitive,
        nesting: 0,
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
        language_specific: None,
        purity_reason: None,
        call_dependencies: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
    }
}

fn create_call_graph_with_callees(callee_count: usize) -> (CallGraph, FunctionId, FunctionMetrics) {
    let mut graph = CallGraph::new();

    let orchestrator = FunctionId::new(PathBuf::from("test.rs"), "orchestrator".to_string(), 1);

    graph.add_function(orchestrator.clone(), false, false, 2, 20);

    // Add callees
    for i in 0..callee_count {
        let callee = FunctionId::new(
            PathBuf::from("test.rs"),
            format!("callee_{}", i),
            100 + i * 10,
        );
        graph.add_function(callee.clone(), false, false, 5, 20);
        graph.add_call(FunctionCall {
            caller: orchestrator.clone(),
            callee,
            call_type: CallType::Direct,
        });
    }

    let func = create_test_func("orchestrator", 2, 3, 20);
    (graph, orchestrator, func)
}

fn bench_extract_composition_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("extract_composition_metrics");

    for callee_count in [2, 5, 10, 20, 50].iter() {
        let (graph, func_id, func) = create_call_graph_with_callees(*callee_count);

        group.bench_with_input(
            BenchmarkId::new("callees", callee_count),
            callee_count,
            |b, _| {
                b.iter(|| {
                    extract_composition_metrics(
                        black_box(&func_id),
                        black_box(&func),
                        black_box(&graph),
                    )
                });
            },
        );
    }

    group.finish();
}

fn bench_adjust_score(c: &mut Criterion) {
    let config = OrchestrationAdjustmentConfig::default();
    let (graph, func_id, func) = create_call_graph_with_callees(10);
    let metrics = extract_composition_metrics(&func_id, &func, &graph);

    c.bench_function("adjust_score/orchestrator", |b| {
        b.iter(|| {
            adjust_score(
                black_box(&config),
                black_box(100.0),
                black_box(&FunctionRole::Orchestrator),
                black_box(&metrics),
            )
        });
    });
}

fn bench_adjust_score_disabled(c: &mut Criterion) {
    let config = OrchestrationAdjustmentConfig {
        enabled: false,
        ..Default::default()
    };

    let (graph, func_id, func) = create_call_graph_with_callees(10);
    let metrics = extract_composition_metrics(&func_id, &func, &graph);

    c.bench_function("adjust_score/disabled", |b| {
        b.iter(|| {
            adjust_score(
                black_box(&config),
                black_box(100.0),
                black_box(&FunctionRole::Orchestrator),
                black_box(&metrics),
            )
        });
    });
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_adjustment_pipeline");

    for callee_count in [2, 5, 10, 20].iter() {
        let (graph, func_id, func) = create_call_graph_with_callees(*callee_count);
        let config = OrchestrationAdjustmentConfig::default();

        group.bench_with_input(
            BenchmarkId::new("callees", callee_count),
            callee_count,
            |b, _| {
                b.iter(|| {
                    // Full pipeline: extract metrics + adjust score
                    let metrics = extract_composition_metrics(
                        black_box(&func_id),
                        black_box(&func),
                        black_box(&graph),
                    );
                    adjust_score(
                        black_box(&config),
                        black_box(100.0),
                        black_box(&FunctionRole::Orchestrator),
                        black_box(&metrics),
                    )
                });
            },
        );
    }

    group.finish();
}

fn bench_composition_quality_calculation(c: &mut Criterion) {
    use debtmap::priority::scoring::orchestration_adjustment::CompositionMetrics;

    let config = OrchestrationAdjustmentConfig::default();

    let mut group = c.benchmark_group("composition_quality");

    // Excellent quality
    let excellent = CompositionMetrics {
        callee_count: 8,
        delegation_ratio: 0.6,
        local_complexity: 2,
        ast_composition_quality: None,
    };

    group.bench_function("excellent", |b| {
        b.iter(|| {
            debtmap::priority::scoring::orchestration_adjustment::calculate_composition_quality(
                black_box(&config),
                black_box(&excellent),
            )
        });
    });

    // Poor quality
    let poor = CompositionMetrics {
        callee_count: 2,
        delegation_ratio: 0.1,
        local_complexity: 8,
        ast_composition_quality: None,
    };

    group.bench_function("poor", |b| {
        b.iter(|| {
            debtmap::priority::scoring::orchestration_adjustment::calculate_composition_quality(
                black_box(&config),
                black_box(&poor),
            )
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_extract_composition_metrics,
    bench_adjust_score,
    bench_adjust_score_disabled,
    bench_full_pipeline,
    bench_composition_quality_calculation
);
criterion_main!(benches);
