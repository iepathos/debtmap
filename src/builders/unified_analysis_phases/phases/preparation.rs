//! Shared preparation for sequential and parallel debt scoring.

use crate::core::FunctionMetrics;
use crate::data_flow::{DataFlowGraph, PurityInfo};
use crate::extraction::ExtractedFileData;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::HashMap;
use std::path::PathBuf;

/// Build one data-flow graph for either execution mode.
///
/// Pre-extracted facts are used when callers provide them. Otherwise the graph
/// is built from the call graph and metric purity without hidden file I/O.
pub fn build_data_flow_graph(
    metrics: &[FunctionMetrics],
    call_graph: &CallGraph,
    extracted_data: Option<&HashMap<PathBuf, ExtractedFileData>>,
) -> DataFlowGraph {
    let mut graph = DataFlowGraph::from_call_graph(call_graph.clone());
    populate_extracted_facts(&mut graph, extracted_data);
    populate_metric_purity(&mut graph, metrics);
    graph
}

fn populate_extracted_facts(
    graph: &mut DataFlowGraph,
    extracted_data: Option<&HashMap<PathBuf, ExtractedFileData>>,
) {
    if let Some(extracted) = extracted_data {
        crate::extraction::adapters::data_flow::populate_data_flow(graph, extracted);
    }
}

fn populate_metric_purity(graph: &mut DataFlowGraph, metrics: &[FunctionMetrics]) {
    for metric in metrics {
        let function = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);
        graph.set_purity_info(function, purity_from_metric(metric));
    }
}

fn purity_from_metric(metric: &FunctionMetrics) -> PurityInfo {
    let is_pure = metric.is_pure.unwrap_or(false);
    PurityInfo {
        is_pure,
        confidence: metric.purity_confidence.unwrap_or(0.0),
        impurity_reasons: (!is_pure)
            .then(|| "Function may have side effects".to_string())
            .into_iter()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_purity_is_available_without_source_extraction() {
        let file = PathBuf::from("missing.rs");
        let mut metric = FunctionMetrics::new("pure_function".to_string(), file.clone(), 7);
        metric.is_pure = Some(true);
        metric.purity_confidence = Some(0.9);
        let function = FunctionId::new(file, metric.name.clone(), metric.line);

        let graph = build_data_flow_graph(&[metric], &CallGraph::new(), None);

        let purity = graph.get_purity_info(&function).expect("purity fact");
        assert!(purity.is_pure);
        assert_eq!(purity.confidence, 0.9);
        assert!(purity.impurity_reasons.is_empty());
    }
}
