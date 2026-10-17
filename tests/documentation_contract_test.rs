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

fn pre_commit_hooks() -> Vec<serde_yaml::Value> {
    serde_yaml::from_str(include_str!("../.pre-commit-hooks.yaml"))
        .expect(".pre-commit-hooks.yaml must be a YAML list of hooks")
}

fn hook_field<'a>(hook: &'a serde_yaml::Value, key: &str) -> &'a serde_yaml::Value {
    hook.get(key)
        .unwrap_or_else(|| panic!("hook {:?} missing `{key}`", hook.get("id")))
}

#[test]
fn pre_commit_hooks_manifest_exposes_validate_hooks() {
    let hooks = pre_commit_hooks();
    let ids: Vec<_> = hooks
        .iter()
        .map(|hook| hook_field(hook, "id").as_str().unwrap())
        .collect();
    assert_eq!(ids, ["debtmap", "debtmap-system"]);

    for (hook, language) in hooks.iter().zip(["rust", "system"]) {
        let args: Vec<_> = hook_field(hook, "args")
            .as_sequence()
            .unwrap()
            .iter()
            .map(|arg| arg.as_str().unwrap())
            .collect();
        assert_eq!(hook_field(hook, "language").as_str(), Some(language));
        assert_eq!(hook_field(hook, "entry").as_str(), Some("debtmap"));
        assert_eq!(args, ["validate", "."]);
        assert_eq!(hook_field(hook, "pass_filenames").as_bool(), Some(false));
        assert_eq!(hook_field(hook, "require_serial").as_bool(), Some(true));
    }
}

#[test]
fn pre_commit_hooks_files_filter_matches_supported_sources() {
    let supported = [
        "rs", "py", "pyw", "js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts", "go", "sol",
    ];
    let ignored = ["README.md", "Cargo.toml", "src/lib.rs.bak", "notes.txt"];

    for hook in pre_commit_hooks() {
        let filter = regex::Regex::new(hook_field(&hook, "files").as_str().unwrap()).unwrap();
        assert!(filter.is_match(".debtmap.toml"));
        assert!(filter.is_match("nested/.debtmap.toml"));
        for ext in supported {
            assert!(filter.is_match(&format!("src/module.{ext}")), "{ext}");
        }
        for path in ignored {
            assert!(!filter.is_match(path), "{path}");
        }
    }
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
