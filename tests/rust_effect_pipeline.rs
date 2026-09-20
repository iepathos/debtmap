//! Actual resolver effects must agree across extraction, propagation and scoring.
use debtmap::analysis::effect_evidence::{EffectClassification, ObservedEffectKind};
use debtmap::analyzers::{
    Analyzer, rust::RustAnalyzer, rust_call_graph::extract_call_graph_multi_file,
};
use debtmap::builders::unified_analysis_phases::{
    orchestration::run_purity_propagation, phases::preparation::build_data_flow_graph,
};
use debtmap::builders::{
    call_graph::build_initial_call_graph, parallel_call_graph::build_call_graph_from_extracted,
};
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::priority::{
    call_graph::FunctionId, unified_scorer::calculate_unified_priority_with_data_flow,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn actual_call_resolver_has_no_fabricated_external_effects_or_discount() {
    let source = include_str!("../src/analyzers/call_graph/call_resolution.rs");
    let file = PathBuf::from("src/analyzers/call_graph/call_resolution.rs");
    let ast = syn::parse_file(source).unwrap();
    let direct_graph = extract_call_graph_multi_file(&[(ast, file.clone())]);
    let analyzer = RustAnalyzer::new();
    let metrics = analyzer
        .analyze(&analyzer.parse(source, file.clone()).unwrap())
        .complexity
        .functions;
    let extracted = UnifiedFileExtractor::extract(&file, source).unwrap();
    let (graph, _, _) = build_call_graph_from_extracted(
        build_initial_call_graph(&metrics),
        &HashMap::from([(file, extracted)]),
    );
    let metrics = run_purity_propagation(&metrics, &graph);
    let data_flow = build_data_flow_graph(&metrics, &graph, None);
    for name in [
        "CallResolver::resolve_call_outcome",
        "CallResolver::select_best_candidate",
    ] {
        let direct_id = direct_graph
            .get_all_functions()
            .find(|id| id.name == name)
            .unwrap();
        let direct = direct_graph.effect_assessment(direct_id).unwrap();
        assert!(
            direct
                .observed()
                .all(|effect| effect.kind == ObservedEffectKind::LocalMutation),
            "{name}: {direct:?}"
        );
        let metric = metrics.iter().find(|metric| metric.name == name).unwrap();
        let id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
            .with_column(metric.column);
        let assessment = data_flow
            .get_purity_info(&id)
            .unwrap()
            .assessment
            .as_ref()
            .unwrap();
        assert!(
            assessment.observed().all(|effect| !matches!(
                effect.kind,
                ObservedEffectKind::ExternalRead
                    | ObservedEffectKind::ExternalWrite
                    | ObservedEffectKind::Io
                    | ObservedEffectKind::Nondeterminism
            )),
            "{name}: {assessment:?}"
        );
        assert_eq!(
            assessment.classification(),
            EffectClassification::Unknown,
            "{name}"
        );
        let score = calculate_unified_priority_with_data_flow(
            metric,
            &graph,
            &data_flow,
            None,
            None,
            None,
            &debtmap::config::DataFlowScoringConfig::default(),
        );
        assert_eq!(score.purity_factor, Some(1.0), "{name}");
    }
}
