use super::{definition, graphs};
use debtmap::analysis::call_graph::RustCallGraphBuilder;
use debtmap::priority::coverage_propagation::calculate_indirect_coverage;
use debtmap::risk::lcov::{FunctionCoverage, LcovData, NormalizedFunctionName};
use std::path::Path;

const SOURCE: &str = r#"
struct Foo;
impl Foo { fn run(&self) {} } // impossible target
trait Work { fn run(&self); }
impl Work for u32 { fn run(&self) {} } // admissible target
#[test]
fn root() { let value: u32 = 1; value.run(); } // covered root
"#;

#[test]
fn impossible_receivers_gain_no_liveness_or_coverage_after_enhancement_and_merges() {
    for graph in graphs(SOURCE) {
        let impossible = definition(&graph, SOURCE, "impossible target");
        let possible = definition(&graph, SOURCE, "admissible target");
        let root = definition(&graph, SOURCE, "covered root");
        assert_eq!(
            graph.get_possible_callers(&possible),
            std::slice::from_ref(&root),
            "before enhancement: {:?}",
            graph.uncertain_calls().collect::<Vec<_>>()
        );
        let mut builder = RustCallGraphBuilder::from_base_graph(graph);
        builder
            .analyze_trait_dispatch(Path::new("src/lib.rs"), &syn::parse_file(SOURCE).unwrap())
            .unwrap();
        builder.finalize_trait_analysis().unwrap();
        let mut enhanced = builder.build();
        let expected_nodes = enhanced.base_graph.node_count();
        for _ in 0..2 {
            enhanced.base_graph.merge(enhanced.base_graph.clone());
        }
        assert_eq!(enhanced.base_graph.node_count(), expected_nodes);
        assert!(enhanced.base_graph.get_all_calls().is_empty());
        assert_eq!(enhanced.base_graph.uncertain_calls().count(), 1);
        assert!(
            enhanced
                .base_graph
                .get_possible_callers(&impossible)
                .is_empty()
        );
        assert_eq!(
            enhanced.base_graph.get_possible_callers(&possible),
            std::slice::from_ref(&root),
            "after enhancement: {:?}",
            enhanced.base_graph.uncertain_calls().collect::<Vec<_>>()
        );
        assert_eq!(enhanced.base_graph.get_dependency_count(&possible), 0);
        let dead = enhanced.get_potential_dead_code();
        assert!(dead.contains(&impossible));
        assert!(!dead.contains(&possible));
        assert!(!enhanced.get_live_functions().contains(&possible));
        let mut coverage = LcovData::default();
        coverage.functions.insert(
            root.file.clone(),
            [&root, &possible, &impossible]
                .into_iter()
                .map(|id| FunctionCoverage {
                    name: id.name.clone(),
                    start_line: id.line,
                    execution_count: u64::from(id == &root),
                    coverage_percentage: if id == &root { 100.0 } else { 0.0 },
                    uncovered_lines: vec![],
                    normalized: NormalizedFunctionName::simple(&id.name),
                })
                .collect(),
        );
        coverage.build_index();
        assert_eq!(
            calculate_indirect_coverage(&root, &enhanced.base_graph, &coverage).direct_coverage,
            1.0
        );
        for target in [&possible, &impossible] {
            let observed = calculate_indirect_coverage(target, &enhanced.base_graph, &coverage);
            assert_eq!(observed.effective_coverage, 0.0);
            assert!(observed.coverage_sources.is_empty());
        }
    }
}
