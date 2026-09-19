//! Coverage provenance must survive LCOV, scoring, conversion and rendering.
use debtmap::core::FunctionMetrics;
use debtmap::io::writers::llm_markdown::format;
use debtmap::output::unified::FunctionDebtItemOutput;
use debtmap::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use debtmap::priority::scoring::construction::create_unified_debt_item_enhanced;
use debtmap::priority::scoring::trace::ScoreOperation;
use debtmap::risk::lcov::{LcovData, parse_lcov_file};
use std::io::Write;

fn metric(name: &str, line: usize) -> FunctionMetrics {
    let mut metric = FunctionMetrics::new(name.into(), "src/resolver.rs".into(), line);
    metric.length = 3;
    metric.cyclomatic = 14;
    metric.cognitive = 32;
    metric.nesting = 2;
    metric
}

fn id(metric: &FunctionMetrics) -> FunctionId {
    FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line)
        .with_column(metric.column)
}

fn fixture(
    caller_hits: [u64; 3],
    covered_helpers: usize,
) -> (FunctionMetrics, CallGraph, LcovData) {
    let caller = metric("resolve_call_outcome", 1);
    let helpers: Vec<_> = (0..6)
        .map(|index| metric(&format!("helper_{index}"), 10 + index * 10))
        .collect();
    let mut graph = CallGraph::new();
    graph.add_function(id(&caller), false, false, 14, 3);
    for helper in &helpers {
        graph.add_function(id(helper), false, false, 14, 3);
        graph.add_call(FunctionCall {
            caller: id(&caller),
            callee: id(helper),
            call_type: CallType::Direct,
        });
    }
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "SF:src/resolver.rs\nFN:1,resolve_call_outcome").unwrap();
    for (offset, hits) in caller_hits.into_iter().enumerate() {
        writeln!(file, "DA:{},{hits}", 1 + offset).unwrap();
    }
    for (index, helper) in helpers.iter().enumerate() {
        writeln!(file, "FN:{},{}", helper.line, helper.name).unwrap();
        for line in helper.line..helper.line + helper.length {
            writeln!(file, "DA:{line},{}", u64::from(index < covered_helpers)).unwrap();
        }
    }
    writeln!(file, "end_of_record").unwrap();
    let metrics: Vec<_> = std::iter::once(caller.clone()).chain(helpers).collect();
    let coverage = parse_lcov_file(file.path())
        .unwrap()
        .with_function_bounds(&metrics);
    (caller, graph, coverage)
}

#[test]
fn covered_helpers_do_not_become_direct_caller_coverage() {
    let (caller, graph, coverage) = fixture([0, 0, 0], 5);
    let item = create_unified_debt_item_enhanced(&caller, &graph, None, Some(&coverage)).unwrap();
    let evidence = item.transitive_coverage.as_ref().unwrap();
    assert_eq!(evidence.direct, 0.0);
    assert_eq!(evidence.transitive, 5.0 / 6.0);
    assert_eq!(evidence.propagated_from.len(), 5);
    let coverage_step = item
        .unified_score
        .score_trace
        .iter()
        .find(|step| step.label == "Coverage (role-adjusted uncovered fraction)")
        .unwrap();
    assert_eq!(coverage_step.operation, ScoreOperation::Multiply(1.0));

    let output = FunctionDebtItemOutput::from_function_item(&item, true);
    let json = serde_json::to_value(&output).unwrap();
    assert_eq!(json["metrics"]["coverage"], 0.0);
    assert_eq!(json["metrics"]["transitive_coverage"], 0.8333);
    let restored: FunctionDebtItemOutput = serde_json::from_value(json).unwrap();
    let rendered = format::coverage(&restored.metrics).unwrap();
    assert!(rendered.contains("- Direct Coverage: 0%"), "{rendered}");
    assert!(
        rendered.contains("- Transitive Coverage: 83%"),
        "{rendered}"
    );
}

#[test]
fn measured_coverage_and_absent_data_keep_their_meaning() {
    for (hits, expected) in [([0, 0, 0], 0.0), ([1, 1, 0], 0.6667), ([1, 1, 1], 1.0)] {
        let (caller, graph, coverage) = fixture(hits, 0);
        let item =
            create_unified_debt_item_enhanced(&caller, &graph, None, Some(&coverage)).unwrap();
        let output = FunctionDebtItemOutput::from_function_item(&item, true);
        assert_eq!(output.metrics.coverage, Some(expected));
        assert_eq!(output.metrics.transitive_coverage, Some(expected));
    }
    let (caller, graph, _) = fixture([0, 0, 0], 5);
    let item = create_unified_debt_item_enhanced(&caller, &graph, None, None).unwrap();
    let output = FunctionDebtItemOutput::from_function_item(&item, true);
    let json = serde_json::to_value(&output).unwrap();
    assert!(json["metrics"].get("coverage").is_none());
    assert!(json["metrics"].get("transitive_coverage").is_none());
    assert!(format::coverage(&output.metrics).is_none());
}
