use std::path::{Path, PathBuf};

fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn architecture_references_existing_repository_paths() {
    let architecture = include_str!("../ARCHITECTURE.md");
    let paths = architecture
        .split('`')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, value)| value)
        .filter(|value| {
            value.starts_with("src/")
                || value.starts_with("tests/")
                || value.starts_with("schemas/")
                || value.starts_with("benches/")
        });

    for path in paths {
        assert!(
            repository_path(path).exists(),
            "stale architecture path: {path}"
        );
    }
}

#[test]
fn public_docs_describe_current_language_and_output_contracts() {
    let architecture = include_str!("../ARCHITECTURE.md");
    let getting_started = include_str!("../book/src/getting-started.md");

    for language in [
        "Rust",
        "Python",
        "JavaScript",
        "TypeScript",
        "Go",
        "Solidity",
    ] {
        assert!(architecture.contains(language), "missing {language}");
    }
    assert!(architecture.contains("incomplete, deprecated, and fail closed"));
    assert!(architecture.contains("canonical CLI JSON contract is v4"));
    assert!(getting_started.contains("Solidity `.sol`"));
    assert!(!architecture.contains("focused exclusively on Rust"));
}

#[test]
fn pre_commit_hooks_manifest_exposes_validate_hooks() {
    let manifest = include_str!("../.pre-commit-hooks.yaml");
    assert!(manifest.contains("id: debtmap\n"));
    assert!(manifest.contains("id: debtmap-system\n"));
    assert!(manifest.contains("language: rust"));
    assert!(manifest.contains("language: system"));
    assert!(manifest.contains("entry: debtmap"));
    assert!(manifest.contains("pass_filenames: false"));
    assert!(manifest.contains("validate"));
}

#[test]
fn public_docs_describe_pre_commit_consumption() {
    let readme = include_str!("../README.md");
    let validation_gates = include_str!("../book/src/validation-gates.md");
    assert!(readme.contains("repo: https://github.com/iepathos/debtmap"));
    assert!(readme.contains("id: debtmap"));
    assert!(readme.contains("id: debtmap-system"));
    assert!(validation_gates.contains(".pre-commit-config.yaml"));
    assert!(validation_gates.contains("id: debtmap"));
}
