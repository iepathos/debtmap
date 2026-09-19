//! Immediate dependency metrics computed from definition identities before formatting.
use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{HashMap, HashSet};

// Pure function: Extract dependency metrics (spec 205: public for FunctionScoringContext)
// Spec 267: Now includes production/test caller separation
#[derive(Clone)]
pub(crate) struct DependencyMetrics {
    pub(super) upstream_count: usize,
    pub(super) downstream_count: usize,
    pub(super) upstream_names: Vec<String>,
    pub(super) downstream_names: Vec<String>,
    // Spec 267: Separated production and test callers
    pub(super) production_upstream_names: Vec<String>,
    pub(super) test_upstream_names: Vec<String>,
    pub(super) production_blast_radius: usize,
    pub(super) immediate_neighbor_count: Option<usize>,
}

pub(super) fn extract_dependency_metrics(
    func: &FunctionMetrics,
    func_id: &FunctionId,
    call_graph: &CallGraph,
) -> DependencyMetrics {
    if matches_definition(call_graph, func_id) {
        return graph_dependency_metrics(func, call_graph, func_id);
    }
    use crate::priority::caller_classification::{ClassifiedCallers, classify_callers};

    // Use pre-populated call graph data from FunctionMetrics if available
    let (upstream_names, downstream_names) =
        if func.upstream_callers.is_some() || func.downstream_callees.is_some() {
            (
                func.upstream_callers.clone().unwrap_or_default(),
                func.downstream_callees.clone().unwrap_or_default(),
            )
        } else {
            // Fallback: query call graph directly
            let upstream = call_graph.get_callers(func_id);
            let downstream = call_graph.get_callees(func_id);
            (
                upstream.iter().map(|f| f.name.clone()).collect(),
                downstream.iter().map(|f| f.name.clone()).collect(),
            )
        };

    // Spec 267: Classify callers into production and test
    let classified: ClassifiedCallers = classify_callers(upstream_names.iter(), Some(call_graph));

    // Legacy records have only display labels, so exact identity overlap is unavailable.
    let production_blast_radius = classified
        .production
        .iter()
        .chain(&downstream_names)
        .collect::<HashSet<_>>()
        .len();

    DependencyMetrics {
        upstream_count: upstream_names.len(),
        downstream_count: downstream_names.len(),
        upstream_names,
        downstream_names,
        // Spec 267: Separated callers
        production_upstream_names: classified.production,
        test_upstream_names: classified.test,
        production_blast_radius,
        immediate_neighbor_count: None,
    }
}

fn matches_definition(graph: &CallGraph, query: &FunctionId) -> bool {
    graph.find_function(query).is_some_and(|id| {
        id.file == query.file
            && id.name == query.name
            && id.line == query.line
            && (query.column.is_none() || id.column.is_none() || query.column == id.column)
    })
}

fn graph_dependency_metrics(
    func: &FunctionMetrics,
    graph: &CallGraph,
    id: &FunctionId,
) -> DependencyMetrics {
    let callers = graph.external_callers(id);
    let callees = graph.external_callees(id);
    let neighbors: HashSet<_> = callers.iter().chain(&callees).collect();
    let (test, production): (Vec<_>, Vec<_>) =
        callers.iter().partition(|id| graph.is_test_dependency(id));
    let production_blast_radius = production
        .iter()
        .copied()
        .chain(callees.iter())
        .collect::<HashSet<_>>()
        .len();
    let include_file = func.upstream_callers.is_some() || func.downstream_callees.is_some();
    let names = dependency_display_names(&neighbors, include_file);
    DependencyMetrics {
        upstream_count: callers.len(),
        downstream_count: callees.len(),
        upstream_names: callers.iter().map(|id| names[id].clone()).collect(),
        downstream_names: callees.iter().map(|id| names[id].clone()).collect(),
        production_upstream_names: production.iter().map(|id| names[id].clone()).collect(),
        test_upstream_names: test.iter().map(|id| names[id].clone()).collect(),
        production_blast_radius,
        immediate_neighbor_count: Some(neighbors.len()),
    }
}

fn dependency_display_names<'a>(
    ids: &HashSet<&'a FunctionId>,
    include_file: bool,
) -> HashMap<&'a FunctionId, String> {
    let labels: HashMap<_, _> = ids
        .iter()
        .map(|id| (*id, display_name(id, include_file)))
        .collect();
    let mut counts = HashMap::new();
    for label in labels.values() {
        *counts.entry(label).or_insert(0) += 1;
    }
    ids.iter()
        .map(|id| {
            let label = if counts[&labels[id]] > 1 {
                format!(
                    "{}:{}:{}:{}:{}",
                    id.file.display(),
                    id.name,
                    id.line,
                    id.column
                        .map(|column| column.to_string())
                        .unwrap_or_else(|| "?".into()),
                    id.module_path
                )
            } else {
                labels[id].clone()
            };
            (*id, label)
        })
        .collect()
}

fn display_name(id: &FunctionId, include_file: bool) -> String {
    if include_file {
        let file = id
            .file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");
        format!("{file}:{}", id.name)
    } else {
        id.name.clone()
    }
}
