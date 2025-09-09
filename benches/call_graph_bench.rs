//! Performance benchmarks for call graph operations
//!
//! This ensures that the refactoring hasn't introduced performance regressions

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::path::PathBuf;

fn create_test_graph(size: usize) -> CallGraph {
    let mut graph = CallGraph::new();

    // Create functions
    for i in 0..size {
        let func_id = FunctionId {
            file: PathBuf::from(format!("file{}.rs", i % 10)),
            name: format!("func_{}", i),
            line: i * 10,
        };
        graph.add_function(func_id, i == 0, false, (i % 10) as u32, i * 5);
    }

    // Create call relationships
    for i in 0..size - 1 {
        let caller = FunctionId {
            file: PathBuf::from(format!("file{}.rs", i % 10)),
            name: format!("func_{}", i),
            line: i * 10,
        };
        let callee = FunctionId {
            file: PathBuf::from(format!("file{}.rs", (i + 1) % 10)),
            name: format!("func_{}", i + 1),
            line: (i + 1) * 10,
        };
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
                let func_id = FunctionId {
                    file: PathBuf::from("test.rs"),
                    name: format!("func_{}", i),
                    line: i * 10,
                };
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
                caller: FunctionId {
                    file: PathBuf::from("test.rs"),
                    name: "caller".to_string(),
                    line: 10,
                },
                callee: FunctionId {
                    file: PathBuf::from("test.rs"),
                    name: "callee".to_string(),
                    line: 20,
                },
                call_type: CallType::Direct,
            };
            graph.add_call(black_box(call));
        });
    });
}

fn bench_get_callees(c: &mut Criterion) {
    let graph = create_test_graph(1000);
    let func_id = FunctionId {
        file: PathBuf::from("file0.rs"),
        name: "func_0".to_string(),
        line: 0,
    };

    c.bench_function("get_callees", |b| {
        b.iter(|| {
            graph.get_callees(black_box(&func_id));
        });
    });
}

fn bench_transitive_callees(c: &mut Criterion) {
    let graph = create_test_graph(100);
    let func_id = FunctionId {
        file: PathBuf::from("file0.rs"),
        name: "func_0".to_string(),
        line: 0,
    };

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
    let func_id = FunctionId {
        file: PathBuf::from("file5.rs"),
        name: "func_50".to_string(),
        line: 500,
    };

    c.bench_function("calculate_criticality", |b| {
        b.iter(|| {
            graph.calculate_criticality(black_box(&func_id));
        });
    });
}

fn bench_delegation_detection(c: &mut Criterion) {
    let graph = create_test_graph(200);
    let func_id = FunctionId {
        file: PathBuf::from("file2.rs"),
        name: "func_20".to_string(),
        line: 200,
    };

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
                        file: PathBuf::from(format!("file{}.rs", i % 10)),
                        name: format!("caller_{}", i),
                        line: i * 10,
                    },
                    callee: FunctionId {
                        file: PathBuf::from("unknown.rs"),
                        name: format!("unresolved_{}", i),
                        line: 0, // Line 0 indicates unresolved
                    },
                    call_type: CallType::Direct,
                };
                graph.add_call(call);
            }
            graph.resolve_cross_file_calls();
        });
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
    bench_cross_file_resolution
);

criterion_main!(benches);
