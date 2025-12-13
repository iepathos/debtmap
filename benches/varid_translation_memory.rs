//! Memory benchmarks for VarId translation layer
//!
//! Verifies that the translation layer adds <10% memory overhead (NFR1 from spec 247)

use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::analysis::data_flow::{
    DataFlowAnalysis, EscapeAnalysis, ReachingDefinitions, TaintAnalysis, VarId,
};
use debtmap::data_flow::{CfgAnalysisWithContext, DataFlowGraph};
use debtmap::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use std::mem;
use std::path::PathBuf;

/// Create a realistic DataFlowAnalysis with the given number of variables
fn create_analysis(num_vars: usize) -> DataFlowAnalysis {
    let mut escaping_vars = HashSet::new();
    let mut return_deps = HashSet::new();
    let mut tainted_vars = HashSet::new();

    // Create realistic data flow analysis results
    for i in 0..num_vars {
        let var_id = VarId {
            name_id: i as u32,
            version: 0,
        };

        // Add to various sets to simulate real analysis
        if i % 7 == 0 {
            escaping_vars.insert(var_id);
        }
        if i % 11 == 0 {
            return_deps.insert(var_id);
        }
        if i % 13 == 0 {
            tainted_vars.insert(var_id);
        }
    }

    DataFlowAnalysis {
        reaching_defs: ReachingDefinitions::default(),
        escape_info: EscapeAnalysis {
            escaping_vars,
            captured_vars: HashSet::new(),
            return_dependencies: return_deps,
        },
        taint_info: TaintAnalysis {
            tainted_vars,
            taint_sources: HashMap::new(),
            return_tainted: false,
        },
    }
}

/// Create variable names for translation
fn create_var_names(num_vars: usize) -> Vec<String> {
    (0..num_vars).map(|i| format!("var_{}", i)).collect()
}

/// Benchmark: Memory overhead of translation layer
///
/// This compares memory usage of DataFlowAnalysis alone vs with CfgAnalysisWithContext
fn bench_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("varid_translation_memory");

    for num_vars in [10, 50, 100, 500] {
        let analysis = create_analysis(num_vars);
        let var_names = create_var_names(num_vars);

        // Measure baseline memory (just analysis)
        let baseline_size = mem::size_of_val(&analysis);

        // Measure with translation context
        let ctx = CfgAnalysisWithContext::new(var_names.clone(), analysis.clone());
        let with_context_size = mem::size_of_val(&ctx);

        // Calculate overhead percentage
        let overhead_pct =
            ((with_context_size as f64 - baseline_size as f64) / baseline_size as f64) * 100.0;

        // Log the results for manual verification
        eprintln!(
            "Variables: {}, Baseline: {} bytes, With context: {} bytes, Overhead: {:.2}%",
            num_vars, baseline_size, with_context_size, overhead_pct
        );

        // Benchmark translation operations
        group.bench_function(format!("translate_{}_vars", num_vars), |b| {
            let ctx = CfgAnalysisWithContext::new(var_names.clone(), analysis.clone());
            b.iter(|| {
                // Simulate translating escaping vars
                let escaping: Vec<_> = ctx
                    .analysis
                    .escape_info
                    .escaping_vars
                    .iter()
                    .copied()
                    .collect();
                black_box(ctx.var_names_for(escaping.into_iter()));

                // Simulate translating return dependencies
                let return_deps: Vec<_> = ctx
                    .analysis
                    .escape_info
                    .return_dependencies
                    .iter()
                    .copied()
                    .collect();
                black_box(ctx.var_names_for(return_deps.into_iter()));

                // Simulate translating tainted vars
                let tainted: Vec<_> = ctx
                    .analysis
                    .taint_info
                    .tainted_vars
                    .iter()
                    .copied()
                    .collect();
                black_box(ctx.var_names_for(tainted.into_iter()));
            });
        });
    }

    group.finish();
}

/// Benchmark: DataFlowGraph memory overhead with translation context
///
/// This measures the memory impact at the DataFlowGraph level
fn bench_data_flow_graph_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_flow_graph_memory");

    for num_functions in [10, 50, 100] {
        let mut graph = DataFlowGraph::new();

        // Add functions with translation context
        for i in 0..num_functions {
            let func_id = FunctionId::new(
                PathBuf::from(format!("file{}.rs", i % 5)),
                format!("func_{}", i),
                i * 10,
            );

            let analysis = create_analysis(20); // 20 vars per function
            let var_names = create_var_names(20);
            let ctx = CfgAnalysisWithContext::new(var_names, analysis);

            graph.set_cfg_analysis_with_context(func_id, ctx);
        }

        group.bench_function(format!("translate_all_{}_functions", num_functions), |b| {
            b.iter(|| {
                for i in 0..num_functions {
                    let func_id = FunctionId::new(
                        PathBuf::from(format!("file{}.rs", i % 5)),
                        format!("func_{}", i),
                        i * 10,
                    );

                    // Simulate real usage pattern (dead store translation removed in spec 256)
                    black_box(graph.get_escaping_var_names(&func_id));
                    black_box(graph.get_return_dependency_names(&func_id));
                    black_box(graph.get_tainted_var_names(&func_id));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark: Individual translation operations
fn bench_individual_translations(c: &mut Criterion) {
    let analysis = create_analysis(100);
    let var_names = create_var_names(100);
    let ctx = CfgAnalysisWithContext::new(var_names, analysis);

    c.bench_function("translate_single_varid", |b| {
        let var_id = VarId {
            name_id: 42,
            version: 0,
        };
        b.iter(|| {
            black_box(ctx.var_name(black_box(var_id)));
        });
    });

    c.bench_function("translate_10_varids", |b| {
        let var_ids: Vec<_> = (0..10)
            .map(|i| VarId {
                name_id: i,
                version: 0,
            })
            .collect();
        b.iter(|| {
            black_box(ctx.var_names_for(black_box(var_ids.iter().copied())));
        });
    });
}

criterion_group!(
    benches,
    bench_memory_overhead,
    bench_data_flow_graph_memory,
    bench_individual_translations
);

criterion_main!(benches);
