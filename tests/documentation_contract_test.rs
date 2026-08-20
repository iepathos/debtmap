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
                || *value == "IMPLEMENTATION_PLAN.md"
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
