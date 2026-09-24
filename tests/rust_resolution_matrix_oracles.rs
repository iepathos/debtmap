//! Independently typecheck the exact matrix sources, without analyzer expectations.
#[path = "rust_resolution_constraint_matrix/cases.rs"]
mod constraint;
#[path = "rust_resolution_namespace_matrix/cases.rs"]
mod namespace;
#[path = "rust_resolution_matrix_support/mod.rs"]
mod support;

use std::path::Path;
use std::process::{Command, Output};

#[derive(Clone, Copy, Debug)]
enum Classification {
    Compiles,
    Ambiguous,
}

struct Fixture {
    name: String,
    source: String,
    classification: Classification,
}

#[test]
fn namespace_sources_match_their_compiler_classifications() {
    let fixtures = namespace::all().into_iter().map(|case| {
        // Graph expectations are deliberately excluded from this compiler oracle.
        drop(case.expected);
        Fixture {
            name: case.name,
            source: case.source,
            classification: match case.kind {
                namespace::SourceKind::Valid => Classification::Compiles,
                namespace::SourceKind::Ambiguous => Classification::Ambiguous,
            },
        }
    });
    check_fixtures("namespace", fixtures);
}

#[test]
fn constraint_sources_compile() {
    let fixtures = constraint::all().into_iter().map(|case| {
        drop(case.expected);
        Fixture {
            name: case.name,
            source: case.source,
            classification: Classification::Compiles,
        }
    });
    check_fixtures("constraint", fixtures);
}

fn check_fixtures(group: &str, fixtures: impl Iterator<Item = Fixture>) {
    let directory = tempfile::tempdir().expect("compiler oracle temporary directory");
    let mut failures = Vec::new();
    let mut count = 0;
    for fixture in fixtures {
        count += 1;
        match check_fixture(&fixture, directory.path()) {
            Ok(()) => eprintln!(
                "compiler {group}/{}: PASS ({:?})",
                fixture.name, fixture.classification
            ),
            Err(error) => {
                eprintln!("compiler {group}/{}: FAIL", fixture.name);
                failures.push(error);
            }
        }
    }
    eprintln!(
        "compiler {group}: {} passed, {} failed, {count} rows",
        count - failures.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

fn check_fixture(fixture: &Fixture, directory: &Path) -> Result<(), String> {
    let output = compile_source(&fixture.source, directory).map_err(|error| {
        format!(
            "CASE {}: {error}\nSOURCE:\n{}",
            fixture.name, fixture.source
        )
    })?;
    if matches_classification(&output, fixture.classification) {
        return Ok(());
    }
    Err(format!(
        "CASE {} expected {:?}, compiler status {}\nSTDOUT:\n{}\nSTDERR:\n{}\nSOURCE:\n{}",
        fixture.name,
        fixture.classification,
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        fixture.source,
    ))
}

fn compile_source(source: &str, directory: &Path) -> Result<Output, String> {
    let input = directory.join("fixture.rs");
    std::fs::write(&input, source).map_err(|error| format!("write fixture: {error}"))?;
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    Command::new(&compiler)
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--emit=metadata",
            "--cap-lints=allow",
            "--error-format=json",
        ])
        .arg(&input)
        .arg("-o")
        .arg(directory.join("fixture.rmeta"))
        .output()
        .map_err(|error| format!("run {compiler:?}: {error}"))
}

fn matches_classification(output: &Output, classification: Classification) -> bool {
    match classification {
        Classification::Compiles => output.status.success(),
        Classification::Ambiguous => {
            !output.status.success() && has_ambiguity_diagnostic(&output.stderr)
        }
    }
}

fn has_ambiguity_diagnostic(stderr: &[u8]) -> bool {
    String::from_utf8_lossy(stderr).lines().any(|line| {
        serde_json::from_str::<serde_json::Value>(line).is_ok_and(|diagnostic| {
            diagnostic["level"] == "error" && diagnostic["code"]["code"] == "E0659"
        })
    })
}
