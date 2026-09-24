use super::{graphs_files, possible};
use std::path::PathBuf;

#[test]
fn nested_out_of_line_modules_rebase_bound_context_before_resolution() {
    let files = [
        (
            "src/lib.rs",
            r#"
mod outer;
trait T { fn run(&self); }
struct Other;
impl T for Other { fn run(&self) {} }
fn generic_return() { outer::declared::generic().run(); }
fn dynamic_return() { outer::declared::dynamic().run(); }
"#,
        ),
        ("src/outer.rs", "pub mod declared;"),
        (
            "src/outer/declared.rs",
            r#"
pub trait T { fn run(&self); }
pub struct A;
impl T for A { fn run(&self) {} }
pub fn generic<G: T>() -> G { todo!() }
pub fn dynamic() -> &'static dyn T { todo!() }
"#,
        ),
    ];
    for graph in graphs_files(&files) {
        let expected: Vec<_> = graph
            .get_all_functions()
            .filter(|id| id.file == PathBuf::from("src/outer/declared.rs") && id.line == 4)
            .cloned()
            .collect();
        assert_eq!(expected.len(), 1);
        for caller in ["generic_return", "dynamic_return"] {
            assert_eq!(possible(&graph, caller), expected.iter().cloned().collect());
            let caller_id = graph
                .get_all_functions()
                .find(|id| id.name == caller)
                .unwrap();
            assert!(
                graph
                    .get_callees(caller_id)
                    .iter()
                    .all(|id| id.file != PathBuf::from("src/lib.rs"))
            );
        }
    }
}
