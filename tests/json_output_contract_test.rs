use debtmap::output::unified::{
    AnalysisPolicyReceipt, LEGACY_UNIFIED_FORMAT_VERSION, UNIFIED_FORMAT_VERSION, UnifiedOutput,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_json(relative: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(manifest_path(relative)).unwrap()).unwrap()
}

fn assert_valid(schema: &Value, instance: &Value) {
    let validator = jsonschema::validator_for(schema).unwrap();
    let errors = validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn committed_fixture_matches_schema_and_serde() {
    let schema = load_json("schemas/debtmap-output-v3.schema.json");
    assert_eq!(
        schema["properties"]["format_version"]["const"],
        LEGACY_UNIFIED_FORMAT_VERSION
    );

    for path in [
        "tests/fixtures/output/unified-v3-minimal.json",
        "tests/fixtures/output/unified-v3-items.json",
    ] {
        let fixture = load_json(path);
        assert_valid(&schema, &fixture);
        let typed: UnifiedOutput = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), fixture);
    }

    let fixture = load_json("tests/fixtures/output/unified-v3-minimal.json");
    let mut wrong_version = fixture.clone();
    wrong_version["format_version"] = Value::String("4.0".to_string());
    assert!(
        !jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&wrong_version)
    );

    let mut missing_summary = fixture;
    missing_summary.as_object_mut().unwrap().remove("summary");
    assert!(
        !jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&missing_summary)
    );

    let mut wrong_uncovered_lines = load_json("tests/fixtures/output/unified-v3-items.json");
    wrong_uncovered_lines["items"][0]["metrics"]["uncovered_lines"] = Value::from(11);
    assert!(
        !jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&wrong_uncovered_lines)
    );
}

#[test]
fn committed_v4_fixture_matches_schema_and_serde() {
    let schema = load_json("schemas/debtmap-output-v4.schema.json");
    let fixture = load_json("tests/fixtures/output/unified-v4-minimal.json");

    assert_eq!(
        schema["properties"]["format_version"]["const"],
        UNIFIED_FORMAT_VERSION
    );
    assert_valid(&schema, &fixture);
    let typed: UnifiedOutput = serde_json::from_value(fixture.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), fixture);
}

#[test]
fn cli_json_matches_v4_schema_with_receipt_and_details() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("sample.rs"),
        "pub fn hotspot(x: i32) -> i32 { if x > 0 { if x > 1 { if x > 2 { if x > 3 { if x > 4 { if x > 5 { x } else { 0 } } else { 0 } } else { 0 } } else { 0 } } else { 0 } } else { 0 } }\n",
    )
    .unwrap();
    let schema = load_json("schemas/debtmap-output-v4.schema.json");

    for (name, verbose) in [("default.json", false), ("detailed.json", true)] {
        let output_path = directory.path().join(name);
        let mut command = Command::new(env!("CARGO_BIN_EXE_debtmap"));
        command
            .current_dir(directory.path())
            .env("HOME", directory.path().join(".test-home"))
            .env("XDG_CONFIG_HOME", directory.path().join(".test-config"))
            .env_remove("DEBTMAP_CONFIG")
            .args([
                "analyze",
                ".",
                "--format",
                "json",
                "--quiet",
                "--no-tui",
                "--no-context-aware",
                "--no-parallel",
                "--min-score",
                "0",
                "--output",
            ])
            .arg(&output_path);
        if verbose {
            command.arg("-vv");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value =
            serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();
        assert_valid(&schema, &report);
        assert_eq!(report["format_version"], UNIFIED_FORMAT_VERSION);
        assert_eq!(
            report["metadata"]["project_root"],
            directory
                .path()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .as_ref()
        );
        assert_eq!(report["receipt"]["evidence"]["coverage_requested"], false);
        assert_eq!(report["receipt"]["evidence"]["coverage_loaded"], false);
        assert_eq!(report["receipt"]["scope"]["discovered_files"], 1);
        assert_eq!(report["receipt"]["scope"]["analyzed_files"], 1);
        assert_eq!(report["receipt"]["scope"]["failed_files"], 0);
        assert_eq!(report["receipt"]["scope"]["status"], "complete");
        let policy: AnalysisPolicyReceipt =
            serde_json::from_value(report["receipt"]["policy"].clone()).unwrap();
        assert_eq!(
            report["receipt"]["policy_fingerprint"],
            policy.fingerprint().unwrap()
        );
        assert_eq!(
            report["summary"]["total_items"],
            report["items"].as_array().unwrap().len()
        );
        assert!(
            report["items"][0]["finding_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("dm4_") && id.len() == 68)
        );
        assert_eq!(report["items"][0].get("scoring_details").is_some(), verbose);
    }
}

#[test]
fn compare_consumes_v3_and_rejects_unknown_versions() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("comparison.json");
    let output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args(["compare", "--before"])
        .arg(manifest_path(
            "tests/fixtures/output/unified-v3-minimal.json",
        ))
        .arg("--after")
        .arg(manifest_path("tests/fixtures/output/unified-v3-items.json"))
        .args(["--format", "json", "--output"])
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let comparison: Value =
        serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();
    assert_eq!(
        comparison["project_health"]["after"]["total_debt_score"],
        75.0
    );

    let mixed_output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args(["compare", "--before"])
        .arg(manifest_path(
            "tests/fixtures/output/unified-v4-minimal.json",
        ))
        .arg("--after")
        .arg(manifest_path(
            "tests/fixtures/output/unified-v3-minimal.json",
        ))
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        mixed_output.status.success(),
        "{}",
        String::from_utf8_lossy(&mixed_output.stderr)
    );

    let unsupported = directory.path().join("unsupported.json");
    let mut report = load_json("tests/fixtures/output/unified-v3-minimal.json");
    report["format_version"] = Value::String("5.0".to_string());
    fs::write(&unsupported, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args(["compare", "--before"])
        .arg(&unsupported)
        .arg("--after")
        .arg(manifest_path(
            "tests/fixtures/output/unified-v3-minimal.json",
        ))
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("Unsupported debtmap format version")
    );

    report["format_version"] = Value::from(3);
    fs::write(&unsupported, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let malformed = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args(["compare", "--before"])
        .arg(&unsupported)
        .arg("--after")
        .arg(manifest_path(
            "tests/fixtures/output/unified-v3-minimal.json",
        ))
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(!malformed.status.success());
    assert!(String::from_utf8_lossy(&malformed.stderr).contains("Invalid debtmap format version"));
}

#[test]
fn cli_receipt_reports_partial_and_limited_scope() {
    let partial = tempfile::tempdir().unwrap();
    fs::write(partial.path().join("invalid.py"), [0xff, 0xfe]).unwrap();
    let partial_report = run_json_analysis(partial.path(), &["--languages", "python"]);
    assert_eq!(partial_report["receipt"]["scope"]["discovered_files"], 1);
    assert_eq!(partial_report["receipt"]["scope"]["failed_files"], 1);
    assert_eq!(partial_report["receipt"]["scope"]["status"], "partial");

    let limited = tempfile::tempdir().unwrap();
    fs::write(limited.path().join("a.rs"), "fn a() {}").unwrap();
    fs::write(limited.path().join("b.rs"), "fn b() {}").unwrap();
    let limited_report = run_json_analysis(limited.path(), &["--max-files", "1"]);
    assert_eq!(limited_report["receipt"]["scope"]["discovered_files"], 2);
    assert_eq!(limited_report["receipt"]["scope"]["omitted_by_limit"], 1);
    assert_eq!(limited_report["receipt"]["scope"]["status"], "limited");
}

fn run_json_analysis(directory: &Path, extra_args: &[&str]) -> Value {
    let output_path = directory.join("report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .current_dir(directory)
        .env("HOME", directory.join(".test-home"))
        .env("XDG_CONFIG_HOME", directory.join(".test-config"))
        .env_remove("DEBTMAP_CONFIG")
        .args(["analyze", ".", "--format", "json", "--quiet", "--no-tui"])
        .args(extra_args)
        .args(["--output"])
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap()
}
