use super::{definition, graphs, possible};

const PRIMITIVES: &str = r#"
struct Foo;
impl Foo { fn run(&self) {} } // forbidden nominal
trait Work { fn run(&self); }
impl Work for u32 { fn run(&self) {} } // admissible primitive
impl Work for u64 { fn run(&self) {} } // incompatible primitive
mod empty {}
use empty::*;
type Number = u32;
fn primitive(value: u32) { value.run(); }
fn reference(value: &&u32) { value.run(); }
fn alias(value: Number) { value.run(); }
fn qualified(value: u32) { <u32 as Work>::run(&value); }
fn literal() { 1u32.run(); }
mod shadowed {
    pub struct u32;
    impl u32 { pub fn run(&self) {} } // shadow implementation
    pub fn caller(value: u32) { value.run(); }
}
"#;

#[test]
fn primitives_exclude_nominal_owners_and_preserve_qualified_local_trait_impls() {
    for graph in graphs(PRIMITIVES) {
        let primitive = definition(&graph, PRIMITIVES, "admissible primitive");
        for caller in ["primitive", "reference", "alias", "literal"] {
            assert_eq!(
                possible(&graph, caller),
                [primitive.clone()].into(),
                "{caller}"
            );
            let caller_id = graph
                .get_all_functions()
                .find(|id| id.name == caller)
                .unwrap();
            assert!(
                graph.get_callees(caller_id).is_empty(),
                "dotted precedence is uncertain"
            );
        }
        let caller = graph
            .get_all_functions()
            .find(|id| id.name == "qualified")
            .unwrap();
        assert_eq!(graph.get_callees(caller), [primitive]);
        let caller = graph
            .get_all_functions()
            .find(|id| id.name == "shadowed::caller")
            .unwrap();
        assert_eq!(
            graph.get_callees(caller),
            [definition(&graph, PRIMITIVES, "shadow implementation")]
        );
    }
}

#[test]
fn unavailable_explicit_primitive_name_import_stays_unavailable() {
    let source = r#"
use missing::u32;
trait Work { fn run(&self); }
impl Work for u32 { fn run(&self) {} }
fn caller(value: u32) { value.run(); }
"#;
    for graph in graphs(source) {
        assert!(possible(&graph, "caller").is_empty());
    }
}
