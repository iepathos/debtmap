//! Labeled, deliberately bounded corpus: reported precision/recall apply only to these cases.
#[path = "rust_method_resolution_support/additional_cases.rs"]
mod additional_cases;
#[path = "rust_method_resolution_support/mod.rs"]
mod support;

use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::builders::call_graph::process_rust_files_for_call_graph_with_files;
use debtmap::builders::parallel_call_graph::build_call_graph_parallel_with_files;
use debtmap::priority::call_graph::CallGraph;
use support::*;

#[test]
fn labeled_propagation_has_exact_edges_and_possible_targets() {
    let graph = graph("propagation.rs");
    let mut failures = Vec::new();
    for case in cases() {
        let actual = callees(&graph, &case.caller);
        if actual != case.resolved {
            failures.push(format!(
                "{}: expected {:?}, got {actual:?}",
                case.caller, case.resolved
            ));
        }
        for (query, expected) in case.possible.unwrap_or_default() {
            let actual = possible(&graph, &case.caller, &query);
            if actual != expected {
                failures.push(format!(
                    "{} possible {query}: expected {expected:?}, got {actual:?}",
                    case.caller
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn receiver_constraints_exclude_impossible_targets() {
    let graph = graph("constraints.rs");
    assert_eq!(
        callees(&graph, "suffix_collision"),
        ["Timeline::new".into()].into()
    );
    assert!(possible(&graph, "suffix_collision", "bar").is_empty());
    assert!(callees(&graph, "dotted_associated").is_empty());
    assert!(possible(&graph, "dotted_associated", "bar").is_empty());
    assert!(callees(&graph, "unknown_singleton").is_empty());
    assert_eq!(
        possible(&graph, "unknown_singleton", "unique"),
        ["Singleton::unique".into()].into()
    );
    let zero = graph
        .uncertain_calls()
        .filter(|call| call.caller.name == "zero_candidates")
        .collect::<Vec<_>>();
    assert_eq!(
        zero.len(),
        1,
        "Zero-candidate calls must survive once per site"
    );
    assert!(zero[0].candidates.is_empty());
}

#[test]
fn same_short_type_names_resolve_by_declaration_location() {
    let graph = graph("constraints.rs");
    let callers: Vec<_> = graph
        .get_all_functions()
        .filter(|id| id.name.ends_with("caller"))
        .collect();
    assert_eq!(callers.len(), 2);
    for caller in callers {
        let targets = graph.get_callees(caller);
        assert_eq!(targets.len(), 1, "{caller:?}");
        assert_eq!(
            targets[0].line + 1,
            caller.line,
            "Must choose the declaration in the same module"
        );
    }
    let alias = graph.get_callees(&function(&graph, "alias_import"));
    assert_eq!(alias.len(), 1);
    assert_eq!(alias[0].line, 16, "Import alias must select left::Same");
}

#[test]
fn unsupported_trait_calls_remain_uncertain() {
    let graph = graph("traits.rs");
    let qualified = graph.get_callees(&function(&graph, "qualified"));
    assert_eq!(qualified.len(), 1);
    assert_eq!(
        qualified[0].line, 4,
        "Explicit First qualification selects its implementation body"
    );
    for caller in [
        "competing",
        "dynamic",
        "generic_trait",
        "unsupported_blanket",
        "recursive_alias",
    ] {
        assert!(
            callees(&graph, caller).is_empty(),
            "{caller} must not guess a concrete implementation"
        );
        assert!(
            graph
                .uncertain_calls()
                .any(|call| call.caller.name == caller),
            "{caller} must retain diagnostics"
        );
    }
}

#[test]
fn complete_workspace_is_file_order_independent() {
    let mut files: Vec<_> = fixture_paths()
        .into_iter()
        .map(|path| {
            (
                syn::parse_file(&std::fs::read_to_string(&path).unwrap()).unwrap(),
                path,
            )
        })
        .collect();
    let forward = extract_call_graph_multi_file(&files);
    files.reverse();
    assert_eq!(
        normalized(&forward),
        normalized(&extract_call_graph_multi_file(&files))
    );
}

#[test]
fn final_sequential_and_parallel_graphs_agree() {
    let paths = fixture_paths();
    let mut sequential = CallGraph::new();
    process_rust_files_for_call_graph_with_files(
        &fixture_dir(),
        &mut sequential,
        false,
        false,
        Some(&paths),
        |_| {},
    )
    .unwrap();
    let (parallel, _, _) = build_call_graph_parallel_with_files(
        &fixture_dir(),
        CallGraph::new(),
        Some(2),
        Some(&paths),
        |_| {},
    )
    .unwrap();
    assert_eq!(normalized(&sequential), normalized(&parallel));
    for graph in [&sequential, &parallel] {
        assert_eq!(
            callees(graph, "suffix_collision"),
            ["Timeline::new".into()].into()
        );
        assert!(possible(graph, "suffix_collision", "bar").is_empty());
        assert!(callees(graph, "dotted_associated").is_empty());
    }
}

#[test]
fn report_labeled_corpus_metrics() {
    support::report_labeled_corpus_metrics();
}
