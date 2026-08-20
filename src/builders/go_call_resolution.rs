//! Conservative same-package Go call resolution for the shared priority graph.

use crate::core::{FunctionMetrics, Language};
use crate::priority::CallGraph;
use crate::priority::call_graph::{CallEdgeProvenance, CallType, FunctionId, ResolutionOutcome};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type PackageSymbol = (PathBuf, String);

pub(super) fn add_resolved_calls(graph: &mut CallGraph, metrics: &[FunctionMetrics]) {
    let index = symbol_index(metrics);
    let mut callers: Vec<_> = metrics
        .iter()
        .filter(|metric| is_production_go(metric))
        .collect();
    callers.sort_by_key(|metric| function_id(metric));

    for caller in callers {
        add_caller_edges(graph, caller, &index);
    }
}

fn symbol_index(metrics: &[FunctionMetrics]) -> HashMap<PackageSymbol, Vec<FunctionId>> {
    metrics
        .iter()
        .filter(|metric| is_production_go(metric))
        .fold(HashMap::new(), |mut index, metric| {
            let candidates = index
                .entry(symbol_key(&metric.file, &metric.name))
                .or_default();
            candidates.push(function_id(metric));
            candidates.sort();
            candidates.dedup();
            index
        })
}

fn add_caller_edges(
    graph: &mut CallGraph,
    caller: &FunctionMetrics,
    index: &HashMap<PackageSymbol, Vec<FunctionId>>,
) {
    let mut calls = caller.call_dependencies.clone().unwrap_or_default();
    calls.sort();
    calls.dedup();
    for call in calls {
        let outcome = resolve_call(caller, &call, index);
        graph.add_resolution(function_id(caller), CallType::Direct, outcome);
    }
}

fn resolve_call(
    caller: &FunctionMetrics,
    call: &str,
    index: &HashMap<PackageSymbol, Vec<FunctionId>>,
) -> ResolutionOutcome {
    let candidates = index
        .get(&symbol_key(&caller.file, call))
        .cloned()
        .unwrap_or_default();
    let (provenance, confidence) = if call.contains('.') {
        (CallEdgeProvenance::TypeResolution, 95)
    } else {
        (CallEdgeProvenance::AstDirect, 100)
    };
    outcome(candidates, call, provenance, confidence)
}

fn outcome(
    mut candidates: Vec<FunctionId>,
    call: &str,
    provenance: CallEdgeProvenance,
    confidence: u8,
) -> ResolutionOutcome {
    candidates.sort();
    match candidates.as_slice() {
        [target] => ResolutionOutcome::Resolved {
            target: target.clone(),
            provenance,
            confidence,
            call_site: None,
        },
        [] => ResolutionOutcome::Unresolved {
            query: call.to_string(),
        },
        _ => ResolutionOutcome::Ambiguous { candidates },
    }
}

fn symbol_key(file: &Path, name: &str) -> PackageSymbol {
    (
        file.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
        name.to_string(),
    )
}

fn function_id(metric: &FunctionMetrics) -> FunctionId {
    FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
}

fn is_production_go(metric: &FunctionMetrics) -> bool {
    Language::from_path(&metric.file) == Language::Go
        && crate::analysis::role_policy::roles_for_metric(metric).is_production()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_unique_same_package_calls_with_evidence() {
        let mut caller = metric("pkg/app.go", "run", 1);
        caller.call_dependencies = Some(vec!["helper".into(), "Worker.Validate".into()]);
        let metrics = vec![
            caller.clone(),
            metric("pkg/helpers.go", "helper", 10),
            metric("pkg/worker.go", "Worker.Validate", 20),
            metric("other/helpers.go", "helper", 30),
        ];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);

        add_resolved_calls(&mut graph, &metrics);

        let caller_id = function_id(&caller);
        let callees = graph.get_callees_exact(&caller_id);
        assert_eq!(callees.len(), 2);
        let evidence: Vec<_> = graph.edge_evidence().collect();
        assert_eq!(evidence.len(), 2);
        assert!(evidence.iter().all(|edge| edge.confidence >= 95));
        assert!(evidence.iter().all(|edge| edge.call_site.is_none()));
    }

    #[test]
    fn does_not_guess_cross_package_targets() {
        let mut caller = metric("pkg/app.go", "run", 1);
        caller.call_dependencies = Some(vec!["helper".into()]);
        let metrics = vec![caller.clone(), metric("other/helpers.go", "helper", 30)];
        let mut graph = crate::builders::call_graph::build_initial_call_graph(&metrics);

        add_resolved_calls(&mut graph, &metrics);

        assert!(graph.get_callees_exact(&function_id(&caller)).is_empty());
        assert_eq!(graph.edge_evidence().count(), 0);
    }

    fn metric(file: &str, name: &str, line: usize) -> FunctionMetrics {
        FunctionMetrics::new(name.to_string(), PathBuf::from(file), line)
    }
}
