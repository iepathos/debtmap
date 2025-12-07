// Performance benchmark for multi-debt type accumulation (spec 228)
// Benchmarks debt classification performance with multi-debt accumulation

use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use debtmap::priority::scoring::classification::classify_debt_type_with_exclusions;
use debtmap::priority::TransitiveCoverage;
use std::collections::HashSet;
use std::hint::black_box;
use std::path::PathBuf;

/// Create a test function with specified metrics
fn create_test_function(
    name: &str,
    file: &str,
    cyclomatic: u32,
    cognitive: u32,
    length: usize,
) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from(file),
        line: 10,
        cyclomatic,
        cognitive,
        nesting: 2,
        length,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
    }
}

fn benchmark_debt_classification(c: &mut Criterion) {
    let func = create_test_function("benchmark_func", "src/lib.rs", 12, 18, 80);
    let call_graph = CallGraph::new();
    let func_id = FunctionId::new(
        PathBuf::from("src/lib.rs"),
        "benchmark_func".to_string(),
        10,
    );
    let framework_exclusions = HashSet::new();
    let coverage = Some(TransitiveCoverage {
        direct: 0.15,
        transitive: 0.3,
        propagated_from: vec![],
        uncovered_lines: vec![15, 16, 17],
    });

    c.bench_function("debt_classification", |b| {
        b.iter(|| {
            classify_debt_type_with_exclusions(
                black_box(&func),
                black_box(&call_graph),
                black_box(&func_id),
                black_box(&framework_exclusions),
                black_box(None),
                black_box(coverage.as_ref()),
            )
        })
    });
}

fn benchmark_batch_debt_classification(c: &mut Criterion) {
    // Create 1000 test functions
    let functions: Vec<_> = (0..1000)
        .map(|i| {
            (
                create_test_function(&format!("func_{}", i), "src/lib.rs", 12, 18, 80),
                FunctionId::new(PathBuf::from("src/lib.rs"), format!("func_{}", i), 10 + i),
            )
        })
        .collect();

    let call_graph = CallGraph::new();
    let framework_exclusions = HashSet::new();
    let coverage = Some(TransitiveCoverage {
        direct: 0.15,
        transitive: 0.3,
        propagated_from: vec![],
        uncovered_lines: vec![15, 16, 17],
    });

    c.bench_function("batch_1000_debt_classification", |b| {
        b.iter(|| {
            for (func, func_id) in &functions {
                black_box(classify_debt_type_with_exclusions(
                    black_box(func),
                    black_box(&call_graph),
                    black_box(func_id),
                    black_box(&framework_exclusions),
                    black_box(None),
                    black_box(coverage.as_ref()),
                ));
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_debt_classification,
    benchmark_batch_debt_classification
);
criterion_main!(benches);
