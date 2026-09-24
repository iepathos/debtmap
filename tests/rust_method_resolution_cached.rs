//! The standard CLI consumes cached extraction data, not the legacy AST builders.
use debtmap::analyzers::rust_call_graph::extract_call_graph;
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::{ExtractedFileData, UnifiedFileExtractor};
use debtmap::priority::call_graph::CallGraph;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::process::Command;

const DECLARATIONS: &str = "struct A; impl A { fn bar(&self) {} } fn bar() {} fn make() -> A { A } struct Timeline; impl Timeline { fn new() -> Self { Timeline } } struct PyTimeline; impl PyTimeline { fn bar(&self) {} }\n";
const CALLERS: &str = "fn borrowed(a: &A) { a.bar(); } fn factory() { make().bar(); } fn suffix() { Timeline::new().bar(); }";

fn source() -> String {
    format!("{DECLARATIONS}{CALLERS}")
}

fn extracted(source: &str) -> ExtractedFileData {
    UnifiedFileExtractor::extract(Path::new("src/lib.rs"), source).unwrap()
}

fn build(data: ExtractedFileData) -> CallGraph {
    build_call_graph_from_extracted(
        CallGraph::new(),
        &HashMap::from([(data.path.clone(), data)]),
    )
    .0
}

fn callees(graph: &CallGraph, caller: &str) -> BTreeSet<String> {
    let id = graph
        .get_all_functions()
        .find(|id| id.name == caller)
        .unwrap();
    graph
        .get_callees(id)
        .into_iter()
        .map(|id| id.name)
        .collect()
}

#[test]
fn cached_source_uses_the_same_receiver_constraints_as_ast_extraction() {
    let source = source();
    let data = extracted(&source);
    assert_eq!(serde_json::to_value(&data).unwrap()["rust_source"], source);
    let graph = build(data);
    let ast_graph = extract_call_graph(&syn::parse_file(&source).unwrap(), Path::new("src/lib.rs"));
    for (caller, expected) in [
        ("borrowed", vec!["A::bar"]),
        ("factory", vec!["A::bar", "make"]),
        ("suffix", vec!["Timeline::new"]),
    ] {
        assert_eq!(
            callees(&graph, caller),
            expected.into_iter().map(String::from).collect()
        );
        assert_eq!(callees(&graph, caller), callees(&ast_graph, caller));
    }
    assert!(
        graph
            .uncertain_calls()
            .filter(|call| call.caller.name == "suffix")
            .all(|call| call.candidates.is_empty())
    );
}

#[test]
fn adapter_graph_and_counts_preserve_workspace_outcomes() {
    let data = extracted(&source());
    let graph = build(data.clone());
    let files = HashMap::from([(data.path.clone(), data)]);
    let adapter = debtmap::extraction::adapters::call_graph::build_call_graph(&files);
    for caller in ["borrowed", "factory", "suffix"] {
        assert_eq!(callees(&adapter, caller), callees(&graph, caller));
    }
    assert_eq!(
        debtmap::extraction::adapters::call_graph::count_resolvable_calls(&files),
        (
            graph.edge_evidence().count(),
            graph.uncertain_calls().count()
        )
    );
    assert_eq!(
        adapter.uncertain_calls().collect::<Vec<_>>(),
        graph.uncertain_calls().collect::<Vec<_>>()
    );
}

#[test]
fn cached_source_round_trip_keeps_declaration_propagation() {
    let encoded = serde_json::to_string(&extracted(&source())).unwrap();
    let restored: ExtractedFileData = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        callees(&build(restored), "factory"),
        ["A::bar".into(), "make".into()].into()
    );
}

#[test]
fn legacy_cache_without_source_retains_method_uncertainty() {
    let mut encoded = serde_json::to_value(extracted(&source())).unwrap();
    encoded.as_object_mut().unwrap().remove("rust_source");
    let legacy: ExtractedFileData = serde_json::from_value(encoded).unwrap();
    let graph = build(legacy);
    for caller in ["borrowed", "factory", "suffix"] {
        let targets = callees(&graph, caller);
        assert!(
            !targets.contains("bar")
                && !targets.contains("A::bar")
                && !targets.contains("PyTimeline::bar")
        );
        assert!(
            graph
                .uncertain_calls()
                .any(|call| call.caller.name == caller),
            "Missing legacy uncertainty for {caller}"
        );
    }
}

fn cli_source() -> String {
    let branches: String = (0..12)
        .map(|index| format!("    if flags[{index}] {{ result += {index}; }}\n"))
        .collect();
    let callers: String = [
        ("borrowed", "a: &A, ", "a.bar();"),
        ("factory", "", "make().bar();"),
        ("suffix", "", "Timeline::new().bar();"),
    ].into_iter().map(|(name, parameter, call)| format!(
        "pub fn {name}({parameter}flags: [bool; 12]) -> u32 {{\n    {call}\n    let mut result = 0;\n{branches}    result\n}}\n"
    )).collect();
    format!("{DECLARATIONS}{callers}")
}

fn run_cli(root: &Path, parallel: bool) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_debtmap"));
    command
        .current_dir(root)
        .env_remove("DEBTMAP_CONFIG")
        .args([
            "analyze",
            ".",
            "--format",
            "json",
            "--quiet",
            "--no-tui",
            "--no-context-aware",
            "--no-aggregation",
            "--threshold-complexity",
            "1",
            "--min-score",
            "0",
            "-vv",
        ]);
    if !parallel {
        command.arg("--no-parallel");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn reported_callees(report: &Value, caller: &str) -> BTreeSet<String> {
    let item = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["location"]["function"] == caller)
        .unwrap_or_else(|| panic!("Missing CLI function item {caller}: {report}"));
    item["dependencies"]["downstream_callees"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|target| {
            target
                .as_str()
                .unwrap()
                .strip_prefix("lib.rs:")
                .expect("CLI dependencies retain file identity")
                .to_string()
        })
        .collect()
}

#[test]
fn standard_analyze_cli_recovers_methods_and_rejects_false_edges() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("lib.rs"), cli_source()).unwrap();
    for parallel in [false, true] {
        let report = run_cli(directory.path(), parallel);
        assert_eq!(report["receipt"]["scope"]["analyzed_files"], 1);
        for (caller, expected) in [
            ("borrowed", vec!["A::bar"]),
            ("factory", vec!["A::bar", "make"]),
            ("suffix", vec!["Timeline::new"]),
        ] {
            assert_eq!(
                reported_callees(&report, caller),
                expected.into_iter().map(String::from).collect(),
                "CLI mode parallel={parallel}, caller={caller}"
            );
        }
    }
}

#[test]
fn standard_analyze_cli_preserves_inline_module_identities_on_one_line() {
    let branches: String = (0..12)
        .map(|index| format!("if flags[{index}] {{ result += {index}; }} "))
        .collect();
    let source: String = ["left", "right"]
        .into_iter()
        .map(|module| format!(
            "pub mod {module} {{ pub struct A; impl A {{ pub fn bar(&self) {{}} }} pub fn caller(a: &A, flags: [bool; 12]) -> u32 {{ a.bar(); let mut result = 0; {branches} result }} }} "
        ))
        .collect();
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("lib.rs"), source).unwrap();
    for parallel in [false, true] {
        let report = run_cli(directory.path(), parallel);
        assert_eq!(
            report["receipt"]["scope"]["code_breakdown"]["production_functions"],
            4
        );
        for module in ["left", "right"] {
            assert_eq!(
                reported_callees(&report, &format!("{module}::caller")),
                [format!("{module}::A::bar")].into(),
                "CLI mode parallel={parallel}, module={module}"
            );
        }
    }
}
