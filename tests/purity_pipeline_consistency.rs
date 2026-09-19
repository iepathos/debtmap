//! Purity propagation must give reporting, scoring and data flow the same facts.
use debtmap::analysis::workflow::{
    actions::analyze_purity,
    env::ProgressReporter,
    state::{AnalysisConfig, AnalysisPhase, AnalysisState},
};
use debtmap::analyzers::{Analyzer, rust::RustAnalyzer};
use debtmap::builders::unified_analysis_phases::{
    orchestration::run_purity_propagation, phases::preparation::build_data_flow_graph,
};
use debtmap::builders::{
    call_graph::build_initial_call_graph, parallel_call_graph::build_call_graph_from_extracted,
};
use debtmap::core::{FunctionMetrics, PurityLevel};
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::priority::{
    call_graph::{CallGraph, FunctionId},
    scoring::trace::explanation_lines,
    semantic_classifier::{FunctionRole, classify_function_role},
    unified_scorer::calculate_unified_priority_with_role,
};
use std::collections::HashMap;
use std::path::PathBuf;

const CACHE_FIXTURE: &str = r#"
struct FrameworkDetector { regex_cache: Cache }
impl FrameworkDetector {
    fn matches_pattern(&self, pattern: &str, text: &str) -> bool {
        if text.is_empty() { return false; }
        self.regex_match(pattern, text)
    }
    fn regex_match(&self, pattern: &str, text: &str) -> bool {
        if !self.regex_cache.contains_key(pattern) {
            match Regex::new(pattern) {
                Ok(regex) => { self.regex_cache.insert(pattern.to_string(), regex); }
                Err(error) => { eprintln!("Invalid pattern: {}", error); return false; }
            }
        }
        match self.regex_cache.get(pattern) {
            Some(regex) => regex.is_match(text),
            None => false,
        }
    }
}
"#;

struct SilentProgress;
impl ProgressReporter for SilentProgress {
    fn phase_starting(&mut self, _: &str) {}
    fn phase_progress(&mut self, _: f64) {}
    fn phase_complete(&mut self) {}
    fn warn(&mut self, _: &str) {}
    fn info(&mut self, _: &str) {}
}

fn extract(source: &str, path: &str) -> (Vec<FunctionMetrics>, CallGraph) {
    let path = PathBuf::from(path);
    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(source, path.clone()).expect("Rust source");
    let metrics = analyzer.analyze(&ast).complexity.functions;
    drop(ast);
    let extracted = UnifiedFileExtractor::extract(&path, source).expect("extracted source");
    let sources = HashMap::from([(path, extracted)]);
    let (graph, _, _) =
        build_call_graph_from_extracted(build_initial_call_graph(&metrics), &sources);
    (metrics, graph)
}

fn definition(metric: &FunctionMetrics) -> FunctionId {
    FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
        .with_column(metric.column)
}

fn method<'a>(metrics: &'a [FunctionMetrics], name: &str) -> &'a FunctionMetrics {
    metrics
        .iter()
        .find(|metric| metric.name == format!("FrameworkDetector::{name}"))
        .expect("detector method")
}

fn workflow(metrics: &[FunctionMetrics], graph: &CallGraph) -> Vec<FunctionMetrics> {
    let mut state = AnalysisState::with_metrics(AnalysisConfig::default(), metrics.to_vec());
    state.results.call_graph = Some(graph.clone());
    state.phase = AnalysisPhase::CoverageComplete;
    analyze_purity(&mut state, &mut SilentProgress).expect("workflow purity analysis");
    assert_eq!(state.phase, AnalysisPhase::PurityComplete);
    state.results.enriched_metrics.expect("propagated metrics")
}

fn assert_impure_scoring(metric: &FunctionMetrics, graph: &CallGraph) {
    let score = calculate_unified_priority_with_role(
        metric,
        &definition(metric),
        graph,
        None,
        None,
        false,
        FunctionRole::Unknown,
    );
    let explanation = explanation_lines(&score).join("\n");
    let expected = format!(
        "Cyclomatic input (purity): trunc({0} × 1.0000) = {0}",
        metric.cyclomatic
    );
    assert!(explanation.contains(&expected), "{explanation}");
}

fn assert_pipeline_consistency(metrics: &[FunctionMetrics], graph: &CallGraph) {
    let data_flow = build_data_flow_graph(metrics, graph, None);
    for name in ["matches_pattern", "regex_match"] {
        let metric = method(metrics, name);
        assert_eq!(metric.is_pure, Some(false), "{name}");
        assert_eq!(metric.purity_level, Some(PurityLevel::Impure), "{name}");
        assert_ne!(
            classify_function_role(metric, &definition(metric), graph),
            FunctionRole::PureLogic
        );
        let purity = data_flow
            .get_purity_info(&definition(metric))
            .expect("data-flow purity");
        assert!(!purity.is_pure, "{name}");
        assert_impure_scoring(metric, graph);
    }
}

fn check_source(source: &str, path: &str, use_workflow: bool) {
    let (metrics, graph) = extract(source, path);
    let caller = method(&metrics, "matches_pattern");
    let callee = method(&metrics, "regex_match");
    assert_eq!(caller.purity_level, Some(PurityLevel::StrictlyPure));
    assert_eq!(callee.purity_level, Some(PurityLevel::Impure));
    assert!(caller.column.is_some());
    assert!(
        graph
            .get_callees(&definition(caller))
            .contains(&definition(callee)),
        "caller {:?}, callee {:?}, nodes {:?}",
        definition(caller),
        definition(callee),
        graph.get_all_functions().collect::<Vec<_>>()
    );
    let propagated = if use_workflow {
        workflow(&metrics, &graph)
    } else {
        run_purity_propagation(&metrics, &graph)
    };
    assert_pipeline_consistency(&propagated, &graph);
}

#[test]
fn orchestration_propagates_cache_side_effects_to_reporting_and_scoring() {
    check_source(CACHE_FIXTURE, "cache_fixture.rs", false);
}

#[test]
fn workflow_propagates_cache_side_effects_to_reporting_and_scoring() {
    check_source(CACHE_FIXTURE, "cache_fixture.rs", true);
}

#[test]
fn actual_framework_detector_has_coherent_purity_in_both_paths() {
    let source = include_str!("../src/analysis/framework_patterns_multi/detector.rs");
    for use_workflow in [false, true] {
        check_source(
            source,
            "src/analysis/framework_patterns_multi/detector.rs",
            use_workflow,
        );
    }
}
