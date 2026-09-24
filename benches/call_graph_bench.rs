//! Performance benchmarks for call graph operations
//!
//! This ensures that the refactoring hasn't introduced performance regressions

#[path = "support/rust_workspace.rs"]
mod rust_workspace;

use criterion::{Criterion, criterion_group, criterion_main};
use debtmap::analysis::call_graph::RustCallGraph;
use debtmap::analysis::effect_evidence::{
    EffectAssessment, EffectDependency, EffectProvenance, ObservedEffect, ObservedEffectKind,
};
use debtmap::analysis::purity_analysis::PurityAnalyzer;
use debtmap::analysis::purity_propagation::{PurityCallGraphAdapter, PurityPropagator};
use debtmap::analyzers::call_graph::debug::{CallGraphDebugger, DebugConfig, DebugFormat};
use debtmap::analyzers::call_graph::validation::CallGraphValidator;
use debtmap::config::DataFlowScoringConfig;
use debtmap::core::FunctionMetrics;
use debtmap::data_flow::{DataFlowGraph, PurityInfo};
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use debtmap::priority::unified_scorer::calculate_unified_priority_with_data_flow;
use std::hint::black_box;
use std::path::PathBuf;

fn create_test_graph(size: usize) -> CallGraph {
    let mut graph = CallGraph::new();

    // Create functions
    for i in 0..size {
        let func_id = FunctionId::new(
            PathBuf::from(format!("file{}.rs", i % 10)),
            format!("func_{}", i),
            i * 10,
        );
        graph.add_function(func_id, i == 0, false, (i % 10) as u32, i * 5);
    }

    // Create call relationships
    for i in 0..size - 1 {
        let caller = FunctionId::new(
            PathBuf::from(format!("file{}.rs", i % 10)),
            format!("func_{}", i),
            i * 10,
        );
        let callee = FunctionId::new(
            PathBuf::from(format!("file{}.rs", (i + 1) % 10)),
            format!("func_{}", i + 1),
            (i + 1) * 10,
        );
        graph.add_call(FunctionCall {
            caller,
            callee,
            call_type: CallType::Direct,
        });
    }

    graph
}

fn bench_add_function(c: &mut Criterion) {
    c.bench_function("add_function", |b| {
        b.iter(|| {
            let mut graph = CallGraph::new();
            for i in 0..100 {
                let func_id =
                    FunctionId::new(PathBuf::from("test.rs"), format!("func_{}", i), i * 10);
                graph.add_function(black_box(func_id), false, false, 5, 50);
            }
        });
    });
}

fn bench_add_call(c: &mut Criterion) {
    c.bench_function("add_call", |b| {
        let mut graph = create_test_graph(100);
        b.iter(|| {
            let call = FunctionCall {
                caller: FunctionId::new(PathBuf::from("test.rs"), "caller".to_string(), 10),
                callee: FunctionId::new(PathBuf::from("test.rs"), "callee".to_string(), 20),
                call_type: CallType::Direct,
            };
            graph.add_call(black_box(call));
        });
    });
}

fn bench_get_callees(c: &mut Criterion) {
    let graph = create_test_graph(1000);
    let func_id = FunctionId::new(PathBuf::from("file0.rs"), "func_0".to_string(), 0);

    c.bench_function("get_callees", |b| {
        b.iter(|| {
            graph.get_callees(black_box(&func_id));
        });
    });
}

fn bench_transitive_callees(c: &mut Criterion) {
    let graph = create_test_graph(100);
    let func_id = FunctionId::new(PathBuf::from("file0.rs"), "func_0".to_string(), 0);

    c.bench_function("get_transitive_callees_depth_3", |b| {
        b.iter(|| {
            graph.get_transitive_callees(black_box(&func_id), 3);
        });
    });

    c.bench_function("get_transitive_callees_depth_10", |b| {
        b.iter(|| {
            graph.get_transitive_callees(black_box(&func_id), 10);
        });
    });
}

fn bench_criticality_calculation(c: &mut Criterion) {
    let graph = create_test_graph(500);
    let func_id = FunctionId::new(PathBuf::from("file5.rs"), "func_50".to_string(), 500);

    c.bench_function("calculate_criticality", |b| {
        b.iter(|| {
            graph.calculate_criticality(black_box(&func_id));
        });
    });
}

fn bench_delegation_detection(c: &mut Criterion) {
    let graph = create_test_graph(200);
    let func_id = FunctionId::new(PathBuf::from("file2.rs"), "func_20".to_string(), 200);

    c.bench_function("detect_delegation_pattern", |b| {
        b.iter(|| {
            graph.detect_delegation_pattern(black_box(&func_id));
        });
    });
}

fn bench_cross_file_resolution(c: &mut Criterion) {
    c.bench_function("resolve_cross_file_calls", |b| {
        b.iter(|| {
            let mut graph = create_test_graph(500);
            // Add some unresolved calls
            for i in 0..50 {
                let call = FunctionCall {
                    caller: FunctionId {
                        column: None,
                        file: PathBuf::from(format!("file{}.rs", i % 10)),
                        name: format!("caller_{}", i),
                        line: i * 10,
                        module_path: String::new(),
                    },
                    callee: FunctionId {
                        column: None,
                        file: PathBuf::from("unknown.rs"),
                        name: format!("unresolved_{}", i),
                        line: 0, // Line 0 indicates unresolved
                        module_path: String::new(),
                    },
                    call_type: CallType::Direct,
                };
                graph.add_call(call);
            }
            graph.resolve_cross_file_calls();
        });
    });
}

/// Benchmark debug mode overhead - verifies <20% overhead requirement
fn bench_debug_mode_overhead(c: &mut Criterion) {
    let graph = create_test_graph(500);

    // Baseline: validation without debug mode
    c.bench_function("validation_baseline", |b| {
        b.iter(|| {
            CallGraphValidator::validate(black_box(&graph));
        });
    });

    // With debug mode enabled
    c.bench_function("validation_with_debug", |b| {
        b.iter(|| {
            let debug_config = DebugConfig {
                show_successes: false,
                show_timing: true,
                max_candidates_shown: 5,
                format: DebugFormat::Text,
                filter_functions: None,
            };
            let mut debugger = CallGraphDebugger::new(debug_config);
            debugger.finalize_statistics();

            CallGraphValidator::validate(black_box(&graph));
        });
    });

    // Debug report generation
    c.bench_function("debug_report_generation_text", |b| {
        b.iter(|| {
            let debug_config = DebugConfig {
                show_successes: false,
                show_timing: true,
                max_candidates_shown: 5,
                format: DebugFormat::Text,
                filter_functions: None,
            };
            let mut debugger = CallGraphDebugger::new(debug_config);
            debugger.finalize_statistics();

            let mut output = Vec::new();
            let _ = debugger.write_report(&mut output);
        });
    });

    c.bench_function("debug_report_generation_json", |b| {
        b.iter(|| {
            let debug_config = DebugConfig {
                show_successes: false,
                show_timing: true,
                max_candidates_shown: 5,
                format: DebugFormat::Json,
                filter_functions: None,
            };
            let mut debugger = CallGraphDebugger::new(debug_config);
            debugger.finalize_statistics();

            let mut output = Vec::new();
            let _ = debugger.write_report(&mut output);
        });
    });
}

/// Benchmark validation operations
fn bench_validation_operations(c: &mut Criterion) {
    let graph = create_test_graph(1000);

    c.bench_function("validate_call_graph_1000_functions", |b| {
        b.iter(|| {
            CallGraphValidator::validate(black_box(&graph));
        });
    });

    // Test with smaller graph
    let small_graph = create_test_graph(100);
    c.bench_function("validate_call_graph_100_functions", |b| {
        b.iter(|| {
            CallGraphValidator::validate(black_box(&small_graph));
        });
    });
}

fn evidence_fixture(size: usize) -> EffectAssessment {
    let owner = FunctionId::new(PathBuf::from("src/effects.rs"), "collect".into(), 1);
    (0..size).fold(EffectAssessment::complete(), |assessment, index| {
        assessment.with_effect(ObservedEffect {
            kind: ObservedEffectKind::LocalMutation,
            detail: format!("local_{index}"),
            provenance: EffectProvenance::source(owner.clone(), index + 1, Some(index)),
        })
    })
}

fn bench_effect_collection(c: &mut Criterion) {
    c.bench_function("effect_evidence_collection_100", |b| {
        b.iter(|| black_box(evidence_fixture(100)))
    });
    let source = (0..100)
        .map(|index| format!("fn function_{index}(value: i32) -> i32 {{ let mut local = value; local += 1; local }}\n"))
        .collect::<String>();
    let ast = syn::parse_file(&source).unwrap();
    c.bench_function("effect_resolver_collection_100", |b| {
        b.iter(|| {
            black_box(
                debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file(&[(
                    ast.clone(),
                    PathBuf::from("src/effect_bench.rs"),
                )]),
            )
        })
    });
}

fn propagation_fixture(size: usize) -> (RustCallGraph, Vec<FunctionMetrics>) {
    let ids: Vec<_> = (0..size)
        .map(|index| {
            FunctionId::new(
                PathBuf::from("src/propagation.rs"),
                format!("function_{index}"),
                index + 1,
            )
        })
        .collect();
    let mut graph = RustCallGraph::new();
    for (index, id) in ids.iter().enumerate() {
        let assessment = ids
            .get(index + 1)
            .map(|target| {
                EffectAssessment::complete().with_dependency(EffectDependency {
                    target: target.clone(),
                    provenance: EffectProvenance::source(id.clone(), id.line, id.column),
                })
            })
            .unwrap_or_else(|| {
                EffectAssessment::complete().with_effect(ObservedEffect {
                    kind: ObservedEffectKind::Io,
                    detail: "leaf console output".into(),
                    provenance: EffectProvenance::source(id.clone(), id.line, id.column),
                })
            });
        graph
            .base_graph
            .record_effect_assessment(id.clone(), assessment);
    }
    let metrics = ids
        .into_iter()
        .map(|id| FunctionMetrics::new(id.name, id.file, id.line))
        .collect();
    (graph, metrics)
}

fn bench_effect_propagation(c: &mut Criterion) {
    let (graph, metrics) = propagation_fixture(100);
    c.bench_function("effect_assessment_propagation_100", |b| {
        b.iter(|| {
            let adapter = PurityCallGraphAdapter::from_rust_graph(graph.clone());
            let mut propagator = PurityPropagator::new(adapter, PurityAnalyzer::new());
            propagator.propagate(black_box(&metrics)).unwrap();
            black_box(propagator)
        })
    });
}

fn bench_effect_aware_scoring(c: &mut Criterion) {
    let mut metric =
        FunctionMetrics::new("score_effects".into(), PathBuf::from("src/scoring.rs"), 10);
    metric.cyclomatic = 12;
    metric.cognitive = 18;
    metric.length = 80;
    let id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);
    let assessment = evidence_fixture(4);
    let mut graph = CallGraph::new();
    graph.add_function(id.clone(), false, false, metric.cyclomatic, metric.length);
    graph.record_effect_assessment(id.clone(), assessment.clone());
    let mut data_flow = DataFlowGraph::from_call_graph(graph.clone());
    data_flow.set_purity_info(
        id,
        PurityInfo {
            assessment: Some(assessment),
            is_pure: false,
            confidence: 1.0,
            impurity_reasons: Vec::new(),
        },
    );
    let config = DataFlowScoringConfig::default();
    c.bench_function("effect_aware_scoring", |b| {
        b.iter(|| {
            calculate_unified_priority_with_data_flow(
                black_box(&metric),
                black_box(&graph),
                black_box(&data_flow),
                None,
                None,
                None,
                black_box(&config),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_add_function,
    bench_add_call,
    bench_get_callees,
    bench_transitive_callees,
    bench_criticality_calculation,
    bench_delegation_detection,
    bench_cross_file_resolution,
    bench_debug_mode_overhead,
    bench_validation_operations,
    bench_effect_collection,
    bench_effect_propagation,
    bench_effect_aware_scoring,
    rust_workspace::bench_workspace_resolution
);

criterion_main!(benches);
