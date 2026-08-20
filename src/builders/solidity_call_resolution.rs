//! Conservative Solidity call resolution for the shared priority graph.

use crate::analyzers::solidity::call_graph::{SolidityBatchSnapshot, compute_call_edges};
use crate::core::{FunctionMetrics, Language};
use crate::priority::CallGraph;
use crate::priority::call_graph::{CallEdgeProvenance, CallType, FunctionId, ResolutionOutcome};
use std::collections::HashMap;
use std::path::PathBuf;

pub(super) fn add_resolved_calls(graph: &mut CallGraph, metrics: &[FunctionMetrics]) {
    let functions = production_functions(metrics);
    let edges = compute_call_edges(&snapshots(&functions));
    let targets = target_index(&functions);

    for ((caller_file, caller_name), callees) in sorted_edges(edges) {
        let Some(caller) = function_id_for(&functions, &caller_file, &caller_name) else {
            continue;
        };
        for callee in callees {
            let candidates = targets.get(&callee).cloned().unwrap_or_default();
            graph.add_resolution(
                caller.clone(),
                CallType::Direct,
                resolution(candidates, &callee),
            );
        }
    }
}

fn production_functions(metrics: &[FunctionMetrics]) -> Vec<&FunctionMetrics> {
    metrics
        .iter()
        .filter(|metric| Language::from_path(&metric.file) == Language::Solidity)
        .filter(|metric| crate::analysis::role_policy::roles_for_metric(metric).is_production())
        .collect()
}

fn snapshots(functions: &[&FunctionMetrics]) -> Vec<SolidityBatchSnapshot> {
    let mut by_file: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for function in functions {
        by_file
            .entry(function.file.clone())
            .or_default()
            .push(function.name.clone());
    }
    let mut snapshots: Vec<_> = by_file
        .into_iter()
        .map(|(path, mut functions)| {
            functions.sort();
            functions.dedup();
            SolidityBatchSnapshot {
                path,
                language: Language::Solidity,
                functions,
            }
        })
        .collect();
    snapshots.sort_by(|left, right| left.path.cmp(&right.path));
    snapshots
}

fn target_index(functions: &[&FunctionMetrics]) -> HashMap<String, Vec<FunctionId>> {
    functions.iter().fold(HashMap::new(), |mut index, metric| {
        let candidates = index.entry(metric.name.clone()).or_default();
        candidates.push(function_id(metric));
        candidates.sort();
        candidates.dedup();
        index
    })
}

fn sorted_edges(
    edges: HashMap<(PathBuf, String), Vec<String>>,
) -> Vec<((PathBuf, String), Vec<String>)> {
    let mut edges: Vec<_> = edges.into_iter().collect();
    edges.sort_by(|left, right| left.0.cmp(&right.0));
    edges
}

fn function_id_for(
    functions: &[&FunctionMetrics],
    file: &PathBuf,
    name: &str,
) -> Option<FunctionId> {
    let candidates: Vec<_> = functions
        .iter()
        .filter(|metric| metric.file == *file && metric.name == name)
        .map(|metric| function_id(metric))
        .collect();
    match candidates.as_slice() {
        [candidate] => Some(candidate.clone()),
        _ => None,
    }
}

fn resolution(mut candidates: Vec<FunctionId>, query: &str) -> ResolutionOutcome {
    candidates.sort();
    match candidates.as_slice() {
        [target] => ResolutionOutcome::Resolved {
            target: target.clone(),
            provenance: CallEdgeProvenance::TypeResolution,
            confidence: 95,
            call_site: None,
        },
        [] => ResolutionOutcome::Unresolved {
            query: query.to_string(),
        },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

fn function_id(metric: &FunctionMetrics) -> FunctionId {
    FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
}
