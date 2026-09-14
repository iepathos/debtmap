//! Equivalent receiver expressions must preserve exclusions and positive targets.
#[path = "rust_resolution_constraint_matrix/cases.rs"]
mod cases;
#[path = "rust_resolution_matrix_support/mod.rs"]
mod support;

#[test]
fn receiver_forms_and_qualification_matrix() {
    check_group("receiver", |name| {
        name.starts_with("dynamic/")
            || name.starts_with("nominal/")
            || name.starts_with("primitive/")
    });
}

#[test]
fn generic_owner_and_trait_argument_matrix() {
    check_group("generic", |name| {
        name.starts_with("generic-owner/") || name.starts_with("trait-argument/")
    });
}

#[test]
fn return_field_and_binding_propagation_matrix() {
    check_group("propagation", |name| {
        name.starts_with("returned-bound/") || name.starts_with("shadow-propagation/")
    });
}

fn check_group(group: &str, include: impl Fn(&str) -> bool) {
    let rows: Vec<_> = cases::all()
        .into_iter()
        .filter(|row| include(&row.name))
        .collect();
    let mut failures = Vec::new();
    for row in &rows {
        match support::verify(&row.name, &row.source, &row.expected) {
            Ok(()) => eprintln!("constraint {}: PASS", row.name),
            Err(error) => {
                eprintln!("constraint {}: FAIL", row.name);
                failures.push(error);
            }
        }
    }
    eprintln!(
        "constraint {group}: {} passed, {} failed, {} rows",
        rows.len() - failures.len(),
        failures.len(),
        rows.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
