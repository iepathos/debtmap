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
    let assessments =
        crate::analysis::purity_propagation::propagate_graph_assessments(graph.call_graph());
    for metric in metrics {
        let function = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
            .with_column(metric.column);
        let assessment = assessments.get(&function).cloned();
        graph.set_purity_info(function, purity_from_metric(metric, assessment));
    }
}

fn purity_from_metric(
    metric: &FunctionMetrics,
    assessment: Option<crate::analysis::effect_evidence::EffectAssessment>,
) -> PurityInfo {
    let is_pure = assessment
        .as_ref()
        .map(|evidence| {
            evidence.classification()
                == crate::analysis::effect_evidence::EffectClassification::StrictlyPure
        })
        .unwrap_or_else(|| metric.is_pure.unwrap_or(false));
    PurityInfo {
        impurity_reasons: assessment
            .as_ref()
            .map(assessment_reasons)
            .unwrap_or_default(),
        assessment,
        is_pure,
        confidence: metric.purity_confidence.unwrap_or(0.0),
    }
}

fn assessment_reasons(
    assessment: &crate::analysis::effect_evidence::EffectAssessment,
) -> Vec<String> {
    assessment
        .observed()
        .map(|effect| effect.detail.clone())
        .chain(
            assessment
                .unresolved()
                .map(|behavior| format!("Unresolved: {}", behavior.detail)),
        )
        .collect()
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
        let function =
            FunctionId::new(file, metric.name.clone(), metric.line).with_column(metric.column);

        let graph = build_data_flow_graph(&[metric], &CallGraph::new(), None);

        let purity = graph.get_purity_info(&function).expect("purity fact");
        assert!(purity.is_pure);
        assert_eq!(purity.confidence, 0.9);
        assert!(purity.impurity_reasons.is_empty());
    }

    #[test]
    fn source_assessment_is_authoritative_in_data_flow() {
        let file = PathBuf::from("source.rs");
        let metric = FunctionMetrics::new("known".to_string(), file.clone(), 4);
        let function = FunctionId::new(file, metric.name.clone(), metric.line);
        let mut calls = CallGraph::new();
        calls.add_function(function.clone(), false, false, 1, 1);
        calls.record_effect_assessment(
            function.clone(),
            crate::analysis::effect_evidence::EffectAssessment::complete(),
        );

        let graph = build_data_flow_graph(&[metric], &calls, None);

        assert_eq!(
            graph
                .get_purity_info(&function)
                .and_then(|purity| purity.assessment.as_ref())
                .map(|assessment| assessment.classification()),
            Some(crate::analysis::effect_evidence::EffectClassification::StrictlyPure)
        );
    }

    #[test]
    fn data_flow_uses_propagated_callee_assessment() {
        use crate::analysis::effect_evidence::{
            EffectAssessment, EffectDependency, EffectProvenance, ObservedEffect,
            ObservedEffectKind,
        };

        let file = PathBuf::from("source.rs");
        let caller = FunctionId::new(file.clone(), "caller".into(), 4);
        let callee = FunctionId::new(file.clone(), "callee".into(), 8);
        let mut calls = CallGraph::new();
        calls.add_function(caller.clone(), false, false, 1, 1);
        calls.add_function(callee.clone(), false, false, 1, 1);
        calls.record_effect_assessment(
            caller.clone(),
            EffectAssessment::complete().with_dependency(EffectDependency {
                target: callee.clone(),
                provenance: EffectProvenance::source(caller.clone(), 5, Some(4)),
            }),
        );
        calls.record_effect_assessment(
            callee.clone(),
            EffectAssessment::complete().with_effect(ObservedEffect {
                kind: ObservedEffectKind::Io,
                detail: "file read".into(),
                provenance: EffectProvenance::source(callee.clone(), 9, Some(4)),
            }),
        );
        let metrics = [
            FunctionMetrics::new("caller".into(), file.clone(), 4),
            FunctionMetrics::new("callee".into(), file, 8),
        ];

        let graph = build_data_flow_graph(&metrics, &calls, None);

        assert_eq!(
            graph
                .get_purity_info(&caller)
                .and_then(|purity| purity.assessment.as_ref())
                .map(|assessment| assessment.classification()),
            Some(crate::analysis::effect_evidence::EffectClassification::Impure)
        );
    }
}
