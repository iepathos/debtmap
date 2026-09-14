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
