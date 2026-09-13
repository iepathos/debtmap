//! Bounded Rust resolution from declarations and lexical source facts.
//!
//! This module deliberately keeps possible targets outside resolved graph edges.
mod bindings;
mod body;
pub(crate) mod cached;
mod expressions;
mod flow;
pub(crate) mod index;
pub(crate) mod types;
mod visitor;
pub(crate) mod workspace;

use crate::priority::call_graph::CallGraph;
use index::WorkspaceIndex;
use std::path::PathBuf;

pub(crate) fn extract(files: &[(PathBuf, syn::File)]) -> CallGraph {
    let index = WorkspaceIndex::build(files);
    let mut graph = workspace::definitions(&index);
    workspace::analyze(&index, files, &mut graph);
    report_counts(&graph);
    graph
}

fn report_counts(graph: &CallGraph) {
    let (ambiguous, unresolved) = graph.uncertain_calls().fold((0, 0), |(a, u), call| {
        if call.candidates.len() > 1 {
            (a + 1, u)
        } else {
            (a, u + 1)
        }
    });
    log::debug!(
        "Rust resolution: {} definitions, {} resolved sites, {} ambiguous sites, {} unresolved sites",
        graph.node_count(),
        graph.edge_evidence().count(),
        ambiguous,
        unresolved
    );
}
