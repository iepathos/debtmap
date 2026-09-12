use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Case {
    pub caller: String,
    pub resolved: BTreeSet<String>,
    pub possible: Option<BTreeMap<String, BTreeSet<String>>>,
}

pub fn fixture_paths() -> Vec<PathBuf> {
    ["propagation.rs", "constraints.rs", "traits.rs"]
        .map(|file| fixture_dir().join(file))
        .to_vec()
}

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/rust_method_resolution")
}

pub fn graph(file: &str) -> CallGraph {
    let path = fixture_dir().join(file);
    let source = std::fs::read_to_string(&path).unwrap();
    extract_call_graph_multi_file(&[(syn::parse_file(&source).unwrap(), path)])
}

pub fn cases() -> Vec<Case> {
    serde_json::from_str(include_str!(
        "../data/rust_method_resolution/expectations.json"
    ))
    .unwrap()
}

/// Erase display-only generic arguments; module and source identities are tested separately.
pub fn display_name(id: &FunctionId) -> String {
    let mut depth = 0;
    id.name
        .chars()
        .filter(|ch| match ch {
            '<' => {
                depth += 1;
                false
            }
            '>' => {
                depth -= 1;
                false
            }
            _ => depth == 0,
        })
        .collect()
}

pub fn function(graph: &CallGraph, name: &str) -> FunctionId {
    graph
        .get_all_functions()
        .find(|id| display_name(id) == name)
        .unwrap_or_else(|| panic!("Missing function {name}"))
        .clone()
}

pub fn callees(graph: &CallGraph, caller: &str) -> BTreeSet<String> {
    graph
        .get_callees(&function(graph, caller))
        .iter()
        .map(display_name)
        .collect()
}

pub fn possible(graph: &CallGraph, caller: &str, query: &str) -> BTreeSet<String> {
    graph
        .uncertain_calls()
        .filter(|call| {
            display_name(&call.caller) == caller && call.query.split("::").last() == Some(query)
        })
        .flat_map(|call| call.candidates.iter().map(display_name))
        .collect()
}

pub fn normalized(graph: &CallGraph) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let sorted = |mut values: Vec<String>| {
        values.sort();
        values
    };
    (
        sorted(
            graph
                .get_all_functions()
                .map(|id| {
                    format!(
                        "{id:?} {:?} {:?} {:?}",
                        graph.get_function_info(id),
                        graph.get_roles(id),
                        graph.get_role_evidence(id)
                    )
                })
                .collect(),
        ),
        sorted(
            graph
                .get_all_calls()
                .iter()
                .map(|call| format!("{call:?}"))
                .collect(),
        ),
        sorted(
            graph
                .edge_evidence()
                .map(|evidence| format!("{evidence:?}"))
                .collect(),
        ),
        sorted(
            graph
                .uncertain_calls()
                .map(|call| format!("{call:?}"))
                .collect(),
        ),
    )
}

pub fn report_labeled_corpus_metrics() {
    let graph = graph("propagation.rs");
    let (correct, incorrect, missing) = cases().iter().fold((0, 0, 0), |totals, case| {
        let actual = callees(&graph, &case.caller);
        (
            totals.0 + actual.intersection(&case.resolved).count(),
            totals.1 + actual.difference(&case.resolved).count(),
            totals.2 + case.resolved.difference(&actual).count(),
        )
    });
    let uncertain: Vec<_> = graph.uncertain_calls().collect();
    let ambiguous = uncertain
        .iter()
        .filter(|call| call.candidates.len() > 1)
        .count();
    let unresolved = uncertain
        .iter()
        .filter(|call| call.candidates.is_empty())
        .count();
    let singleton = uncertain
        .iter()
        .filter(|call| call.candidates.len() == 1)
        .count();
    let sites: std::collections::BTreeSet<_> = uncertain
        .iter()
        .map(|call| (&call.caller, &call.call_site))
        .collect();
    assert_eq!(
        sites.len(),
        uncertain.len(),
        "Count outcomes once per source call"
    );
    println!(
        "Labeled propagation corpus: correct={correct}, incorrect={incorrect}, missing={missing}, precision={:.4}, recall={:.4}, ambiguous={ambiguous}, unresolved={unresolved}, singleton_possible={singleton}, uncertain={}",
        correct as f64 / (correct + incorrect).max(1) as f64,
        correct as f64 / (correct + missing).max(1) as f64,
        uncertain.len()
    );
    for call in uncertain {
        println!(
            "Possible {}:{} {} -> {:?}",
            call.caller.name,
            call.call_site.line,
            call.query,
            call.candidates.iter().map(display_name).collect::<Vec<_>>()
        );
    }
}
