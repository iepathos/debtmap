//! Desired namespace behavior, including deliberately ambiguous source inputs.
#[path = "rust_resolution_namespace_matrix/cases.rs"]
mod cases;
#[path = "rust_resolution_matrix_support/mod.rs"]
mod support;

use cases::{Case, SourceKind};
use support::verify;

#[test]
fn valid_namespace_matrix() {
    check_cases(SourceKind::Valid);
}

#[test]
fn deliberately_ambiguous_namespace_matrix() {
    check_cases(SourceKind::Ambiguous);
}

fn check_cases(kind: SourceKind) {
    let rows: Vec<_> = cases::all()
        .into_iter()
        .filter(|row| row.kind == kind)
        .collect();
    let failures: Vec<_> = rows.iter().filter_map(check_case).collect();
    eprintln!(
        "namespace {kind:?}: {} passed, {} failed, {} rows",
        rows.len() - failures.len(),
        failures.len(),
        rows.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

fn check_case(case: &Case) -> Option<String> {
    match verify(&case.name, &case.source, &case.expected) {
        Ok(()) => {
            eprintln!("namespace {:?}/{}: PASS", case.kind, case.name);
            None
        }
        Err(error) => {
            eprintln!("namespace {:?}/{}: FAIL", case.kind, case.name);
            Some(error)
        }
    }
}

#[test]
fn ambiguous_constructor_arguments_record_each_nested_call_once() {
    use support::Expectation as E;
    let source = "mod left { pub struct Item(pub u32); impl Item { /*@left_hit*/ pub fn hit(&self) {} } }
        mod right { pub struct Item(pub u32); impl Item { /*@right_hit*/ pub fn hit(&self) {} } }
        struct Other; impl Other { /*@other_hit*/ fn hit(&self) {} }
        use left::*; use right::*;
        /*@argument*/ fn argument() -> u32 { 1 }
        /*@caller*/ fn caller() { let x = /*#constructor*/ Item(/*#argument_call*/ argument()); /*#method*/ x.hit(); }";
    verify(
        "ambiguous_constructor_argument",
        source,
        &[
            E::absent("constructor", "Item("),
            E::resolved("argument_call", "argument(", &["argument"]),
            E::uncertain("method", "hit(", &["left_hit", "right_hit"]),
        ],
    )
    .expect("constructor arguments visit once without a constructor body diagnostic");
}

#[test]
fn mixed_function_constructor_results_retain_every_admissible_owner() {
    use support::Expectation as E;
    let source = "mod left { pub struct Item(pub u32); impl Item { /*@left_hit*/ pub fn hit(&self) {} } }
        struct Other; impl Other { /*@other_hit*/ fn hit(&self) {} }
        struct Noise; impl Noise { /*@noise_hit*/ fn hit(&self) {} }
        mod right { /*@function*/ pub fn Item(_: u32) -> crate::Other { crate::Other } }
        use left::*; use right::*;
        /*@argument*/ fn argument() -> u32 { 1 }
        /*@caller*/ fn caller() { let x = /*#constructor*/ Item(/*#argument_call*/ argument()); /*#method*/ x.hit(); }";
    verify(
        "mixed_invocation_results",
        source,
        &[
            E::uncertain("constructor", "Item(", &["function"]),
            E::resolved("argument_call", "argument(", &["argument"]),
            E::uncertain("method", "hit(", &["left_hit", "other_hit"]),
        ],
    )
    .expect("all invocation alternatives constrain the propagated result");
}
