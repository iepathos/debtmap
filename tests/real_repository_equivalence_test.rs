//! Single-threaded and parallel analysis of pinned open-source repositories
//! must produce identical reports.
//!
//! These tests clone over the network, so they are ignored by default:
//! `just test-real-repos`.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn checkout(name: &str, url: &str, commit: &str) -> PathBuf {
    // Not under Cargo's target directory: debtmap reports nothing for files under `target/`.
    let root = std::env::temp_dir().join("debtmap-real-repos").join(name);
    if git(&root, &["rev-parse", "HEAD"]).is_some_and(|head| head.trim() == commit) {
        return root;
    }
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for args in [
        &["init", "-q"][..],
        &["fetch", "-q", "--depth", "1", url, commit],
        &["checkout", "-q", "FETCH_HEAD"],
    ] {
        git(&root, args).unwrap_or_else(|| panic!("git {args:?} failed for {name}"));
    }
    root
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn analyze(root: &Path, jobs: &str) -> Value {
    let output = root.with_extension(format!("j{jobs}.json"));
    let result = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .current_dir(root)
        .args(["analyze", "--format", "json", "--quiet", "--no-tui"])
        .args(["--jobs", jobs, "--output"])
        .arg(&output)
        .arg(root)
        .env_remove("DEBTMAP_CONFIG")
        .env_remove("DEBTMAP_JOBS")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "analysis failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    canonical(serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap())
}

fn canonical(mut report: Value) -> Value {
    report["metadata"]["generated_at"] = Value::Null;
    report["receipt"]["reference_time"] = Value::Null;
    report["receipt"]["execution"]["jobs"] = Value::Null;
    report
}

fn assert_equivalent(name: &str, url: &str, commit: &str) {
    let root = checkout(name, url, commit);
    let sequential = analyze(&root, "1");
    let parallel = analyze(&root, "2");
    assert!(!sequential["items"].as_array().unwrap().is_empty());
    assert!(
        sequential == parallel,
        "{name}: --jobs 1 and --jobs 2 reports differ; compare {} with {}",
        root.with_extension("j1.json").display(),
        root.with_extension("j2.json").display(),
    );
}

#[test]
#[ignore = "network: clones tokio-rs/axum"]
fn axum_rust() {
    assert_equivalent(
        "axum",
        "https://github.com/tokio-rs/axum.git",
        "e57b547eb95de74b73b34719dd5926254a085634",
    );
}

#[test]
#[ignore = "network: clones clap-rs/clap"]
fn clap_rust() {
    assert_equivalent(
        "clap",
        "https://github.com/clap-rs/clap.git",
        "ccc89b83875474399e9d9d70841163f6319d1a41",
    );
}

#[test]
#[ignore = "network: clones spf13/cobra"]
fn cobra_go() {
    assert_equivalent(
        "cobra",
        "https://github.com/spf13/cobra.git",
        "adbc8813901bba65827259daa8e22ff94ec1f30e",
    );
}

#[test]
#[ignore = "network: clones oclif/core"]
fn oclif_core_typescript() {
    assert_equivalent(
        "oclif-core",
        "https://github.com/oclif/core.git",
        "cdf75d84009427cce67e90f12479566c8e6a6dbd",
    );
}

#[test]
#[ignore = "network: clones pallets/click"]
fn click_python() {
    assert_equivalent(
        "click",
        "https://github.com/pallets/click.git",
        "2247b35ea1c47c727d7a06e51fa280e12a863ff6",
    );
}

#[test]
#[ignore = "network: clones transmissions11/solmate"]
fn solmate_solidity() {
    assert_equivalent(
        "solmate",
        "https://github.com/transmissions11/solmate.git",
        "89365b880c4f3c786bdd453d4b8e8fe410344a69",
    );
}
