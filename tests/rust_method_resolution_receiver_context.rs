//! Receiver constraints retain their declaring context and exclude incompatible owners.
#[path = "rust_method_resolution_support/receiver_consumers.rs"]
mod consumers;
#[path = "rust_method_resolution_support/receiver_modules.rs"]
mod modules;
#[path = "rust_method_resolution_support/receiver_primitives.rs"]
mod primitives;
#[path = "rust_method_resolution_support/source_builders.rs"]
mod source_builders;
use debtmap::analyzers::rust_call_graph::extract_call_graph_multi_file;
use debtmap::builders::parallel_call_graph::build_call_graph_from_extracted;
use debtmap::extraction::UnifiedFileExtractor;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn graphs(source: &str) -> Vec<CallGraph> {
    graphs_files(&[("src/lib.rs", source)])
}

fn graphs_files(files: &[(&str, &str)]) -> Vec<CallGraph> {
    let ast = files
        .iter()
        .map(|(path, source)| (syn::parse_file(source).unwrap(), PathBuf::from(path)))
        .collect::<Vec<_>>();
    let direct = extract_call_graph_multi_file(&ast);
    drop(ast);
    let cached = files
        .iter()
        .map(|(path, source)| {
            let path = PathBuf::from(path);
            let data = UnifiedFileExtractor::extract(&path, source).unwrap();
            (path, data)
        })
        .collect();
    [
        direct,
        build_call_graph_from_extracted(CallGraph::new(), &cached).0,
    ]
    .into_iter()
    .chain(source_builders::graphs(files))
    .collect()
}

fn definition(graph: &CallGraph, source: &str, marker: &str) -> FunctionId {
    let line = source
        .lines()
        .position(|line| line.contains(marker))
        .unwrap()
        + 1;
    let candidates: Vec<_> = graph
        .get_all_functions()
        .filter(|id| id.file == PathBuf::from("src/lib.rs") && id.line == line)
        .collect();
    assert_eq!(candidates.len(), 1, "unique definition at {marker}");
    candidates[0].clone()
}

fn possible(graph: &CallGraph, caller: &str) -> BTreeSet<FunctionId> {
    graph
        .uncertain_calls()
        .filter(|call| call.caller.name == caller && call.query.ends_with("run"))
        .flat_map(|call| call.candidates.iter().cloned())
        .collect()
}

const DYNAMIC: &str = r#"
mod declared {
    pub trait T { fn run(&self); }
    pub struct A;
    pub struct B;
    impl T for A { fn run(&self) {} } // first implementation
    impl T for B { fn run(&self) {} } // second implementation
    pub type Dynamic = dyn T;
    pub struct Holder { pub value: &'static Dynamic }
    pub struct Wrapper<G> { pub value: G }
    pub fn make() -> &'static dyn T { todo!() }
    pub fn aliased() -> &'static Dynamic { todo!() }
    pub fn holder() -> Holder { todo!() }
    pub fn wrapped() -> Wrapper<&'static dyn T> { todo!() }
    pub fn generic<G: T>() -> G { todo!() }
}
trait T { fn run(&self); }
struct Other;
impl T for Other { fn run(&self) {} } // forbidden implementation
fn returned() { declared::make().run(); }
fn alias() { declared::aliased().run(); }
fn field() { declared::holder().value.run(); }
fn substitution() { declared::wrapped().value.run(); }
fn generic_return() { declared::generic().run(); }
"#;

#[test]
fn returned_dynamic_and_generic_bounds_preserve_declaring_trait_identity() {
    for graph in graphs(DYNAMIC) {
        let expected: BTreeSet<_> = ["first implementation", "second implementation"]
            .into_iter()
            .map(|marker| definition(&graph, DYNAMIC, marker))
            .collect();
        let forbidden = definition(&graph, DYNAMIC, "forbidden implementation");
        for caller in [
            "returned",
            "alias",
            "field",
            "substitution",
            "generic_return",
        ] {
            assert_eq!(possible(&graph, caller), expected, "{caller}");
            let caller_id = graph
                .get_all_functions()
                .find(|id| id.name == caller)
                .unwrap();
            let resolved = graph.get_callees(caller_id);
            assert!(!resolved.contains(&forbidden));
            assert!(
                expected
                    .iter()
                    .all(|candidate| !resolved.contains(candidate))
            );
        }
    }
}

#[test]
fn missing_dynamic_bound_does_not_acquire_a_caller_scope_trait() {
    let source = r#"
mod declared { pub fn make() -> &'static dyn Missing { todo!() } }
trait Missing { fn run(&self); }
struct Other;
impl Missing for Other { fn run(&self) {} }
fn caller() { declared::make().run(); }
"#;
    for graph in graphs(source) {
        assert!(possible(&graph, "caller").is_empty());
        let call = graph
            .uncertain_calls()
            .find(|call| call.caller.name == "caller")
            .unwrap();
        assert!(format!("{:?}", call.receiver).contains("UnavailableDefinition"));
    }
}
