use super::{LcovData, parse_lcov_file};
use std::{io::Write, path::Path};

fn parse(records: &str) -> LcovData {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(records.as_bytes()).unwrap();
    parse_lcov_file(file.path()).unwrap()
}

const CLOSURE: &str = "SF:/project/src/sample.rs\nFN:10,Body::infer\nFN:12,Body::infer::{closure#0}\nFN:20,next\nFNDA:1,Body::infer\nDA:10,1\nDA:11,0\nDA:12,0\nDA:14,1\nDA:18,1\nDA:20,0\nend_of_record\n";

#[test]
fn registered_callee_cannot_borrow_coverage_outside_its_ast_body() {
    use crate::core::FunctionMetrics;
    use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
    use crate::priority::coverage_propagation::calculate_transitive_coverage_with_bounds;
    let data = parse(
        "SF:test.rs\nFN:1,caller\nFN:10,callee\nFN:100,next\nDA:1,0\nDA:10,0\nDA:20,1\nDA:21,1\nDA:22,1\nDA:23,1\nDA:24,1\nDA:25,1\nDA:26,1\nDA:27,1\nDA:28,1\nend_of_record\n",
    );
    let mut metric = FunctionMetrics::new("callee".into(), "test.rs".into(), 10);
    metric.length = 1;
    assert!(
        data.get_function_coverage_with_line(Path::new("test.rs"), "callee", 10)
            .unwrap()
            > 0.8
    );
    let bounded = data.with_function_bounds(&[metric]);
    let caller = FunctionId::new("test.rs".into(), "caller".into(), 1);
    let callee = FunctionId::new("test.rs".into(), "callee".into(), 10);
    let mut graph = CallGraph::new();
    graph.add_function(caller.clone(), false, false, 2, 1);
    graph.add_function(callee.clone(), false, false, 2, 1);
    graph.add_call(FunctionCall {
        caller: caller.clone(),
        callee,
        call_type: CallType::Delegate,
    });
    let coverage = calculate_transitive_coverage_with_bounds(&caller, 1, &graph, &bounded);
    assert_eq!(coverage.transitive, 0.0);
    assert!(coverage.propagated_from.is_empty());
    assert_eq!(
        bounded.get_function_uncovered_lines(Path::new("./test.rs"), "callee", 10),
        Some(vec![10])
    );
}

#[test]
fn registered_same_line_columns_remain_ambiguous_but_duplicates_do_not() {
    use crate::core::FunctionMetrics;
    let data = parse("SF:test.rs\nFN:10,same\nDA:10,1\nend_of_record\n");
    let mut first = FunctionMetrics::new("same".into(), "test.rs".into(), 10);
    first.length = 1;
    first.column = Some(1);
    let duplicate = data.with_function_bounds(&[first.clone(), first.clone()]);
    assert_eq!(
        duplicate.get_function_coverage_with_line(Path::new("test.rs"), "same", 10),
        Some(1.0)
    );
    let mut second = first.clone();
    second.column = Some(30);
    let ambiguous = data.with_function_bounds(&[first, second]);
    assert_eq!(
        ambiguous.get_function_coverage_with_line(Path::new("test.rs"), "same", 10),
        None
    );
    assert_eq!(
        ambiguous.get_function_coverage_with_bounds(Path::new("test.rs"), "same", 10, 10),
        None
    );
    assert_eq!(
        ambiguous.get_function_uncovered_lines(Path::new("test.rs"), "same", 10),
        None
    );
}

#[test]
fn registered_bounds_retain_legacy_uncovered_lines_without_da_records() {
    use crate::core::FunctionMetrics;
    let mut data = parse("SF:test.rs\nFN:10,legacy\nFNDA:0,legacy\nend_of_record\n");
    data.functions.get_mut(Path::new("test.rs")).unwrap()[0].uncovered_lines = vec![10, 11];
    data.build_index();
    let mut metric = FunctionMetrics::new("legacy".into(), "test.rs".into(), 10);
    metric.length = 2;
    let bound = data.with_function_bounds(&[metric]);
    assert_eq!(
        bound.get_function_uncovered_lines(Path::new("test.rs"), "legacy", 10),
        Some(vec![10, 11])
    );
}

#[test]
fn ast_bounds_include_closure_and_tail_but_exclude_neighbor() {
    let data = parse(CLOSURE);
    let coverage =
        data.get_function_coverage_with_bounds(Path::new("src/sample.rs"), "Body::infer", 10, 18);
    assert_eq!(coverage, Some(3.0 / 5.0));
}

#[test]
fn inferred_parent_boundary_skips_nested_closure() {
    let data = parse(CLOSURE);
    assert_eq!(
        data.get_function_coverage_with_line(
            Path::new("/project/src/sample.rs"),
            "Body::infer",
            10,
        ),
        Some(3.0 / 5.0)
    );
}

#[test]
fn public_function_map_helper_uses_the_same_nested_boundaries() {
    use super::{
        NormalizedFunctionName, coverage::process_function_coverage_parallel,
        handlers::create_function_coverage,
    };
    let mut functions = [(10, "parent"), (12, "parent::{closure#0}"), (20, "next")]
        .into_iter()
        .map(|(line, name)| {
            (
                name.into(),
                create_function_coverage(NormalizedFunctionName::simple(name), line),
            )
        })
        .collect();
    let lines = [(10, 1), (11, 0), (12, 0), (14, 1), (18, 1), (20, 0)]
        .into_iter()
        .collect();
    process_function_coverage_parallel(&mut functions, &lines);
    assert_eq!(functions["parent"].coverage_percentage, 60.0);
    assert_eq!(functions["parent"].uncovered_lines, [11, 12]);
}

#[test]
fn repeated_records_union_executed_lines_in_either_order() {
    let first = "SF:/project/src/sample.rs\nFN:10,f\nDA:10,1\nDA:11,0\nend_of_record\n";
    let second = "SF:/project/src/sample.rs\nFN:10,f\nDA:10,0\nDA:11,1\nend_of_record\n";
    for records in [format!("{first}{second}"), format!("{second}{first}")] {
        let data = parse(&records);
        assert_eq!(data.get_overall_coverage(), 100.0);
        assert_eq!(
            data.get_file_coverage(Path::new("src/sample.rs")),
            Some(1.0)
        );
        assert_eq!(
            data.get_function_coverage_with_bounds(Path::new("src/sample.rs"), "f", 10, 11,),
            Some(1.0)
        );
        assert_eq!(
            data.get_function_coverage_with_line(Path::new("/project/src/sample.rs"), "f", 10,),
            Some(1.0)
        );
    }
}

#[test]
fn absolute_ast_path_matches_relative_lcov_source() {
    let data = parse(&CLOSURE.replace("/project/src/", "src/"));
    assert_eq!(
        data.get_function_coverage_with_bounds(
            Path::new("/project/src/sample.rs"),
            "Body::infer",
            10,
            18,
        ),
        Some(0.6)
    );
}

#[test]
fn bounded_score_and_report_use_same_lines() {
    use crate::{
        core::FunctionMetrics,
        priority::{
            call_graph::CallGraph, scoring::construction::create_unified_debt_item_enhanced,
        },
    };
    let data = parse(CLOSURE);
    let mut function =
        FunctionMetrics::new("Body::infer".into(), "/project/src/sample.rs".into(), 10);
    function.length = 5; // Exact AST end 14, earlier than inferred next symbol.
    function.cyclomatic = 17;
    function.cognitive = 6;
    let item =
        create_unified_debt_item_enhanced(&function, &CallGraph::new(), None, Some(&data)).unwrap();
    let coverage = item.transitive_coverage.unwrap();
    assert_eq!(coverage.direct, 0.5);
    assert_eq!(coverage.transitive, 0.5);
    assert_eq!(coverage.uncovered_lines, [11, 12]);
    assert_eq!(item.unified_score.coverage_factor, 5.0);
}

#[test]
fn single_line_and_record_transition_preserve_observations() {
    let data = parse("SF:first.rs\nFN:1,f\nDA:1,0\nDA:1,1\nSF:second.rs\nDA:1,0\nend_of_record\n");
    assert_eq!(
        data.get_function_coverage_with_bounds(Path::new("first.rs"), "f", 1, 1),
        Some(1.0)
    );
    assert_eq!(
        data.get_function_coverage_with_bounds(Path::new("second.rs"), "f", 1, 1),
        Some(0.0)
    );
    assert_eq!(data.get_overall_coverage(), 50.0);
}

#[test]
fn function_only_coverage_retains_legacy_fallback() {
    let data = parse(&format!(
        "{CLOSURE}SF:other.rs\nFN:1,g\nFNDA:1,g\nend_of_record\n"
    ));
    assert_eq!(
        data.get_function_coverage_with_bounds(Path::new("other.rs"), "g", 1, 2),
        Some(1.0)
    );
}

#[test]
fn ast_bounds_work_without_function_symbols_or_end_marker() {
    let data = parse("SF:/project/src/sample.rs\nDA:10,1\nDA:11,0\nDA:20,1\n");
    assert_eq!(
        data.get_function_coverage_with_bounds(Path::new("src/sample.rs"), "f", 10, 11,),
        Some(0.5)
    );
}

#[test]
fn invalid_bounds_and_ambiguous_file_suffix_do_not_borrow_coverage() {
    let data = parse(&format!(
        "{CLOSURE}{}",
        CLOSURE.replace("/project/", "/other/")
    ));
    assert_eq!(
        data.get_function_coverage_with_bounds(Path::new("src/sample.rs"), "Body::infer", 10, 18,),
        None
    );
    assert_eq!(
        data.get_function_coverage_with_bounds(
            Path::new("/project/src/sample.rs"),
            "Body::infer",
            18,
            10,
        ),
        None
    );
}
