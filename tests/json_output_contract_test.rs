use debtmap::output::unified::{UNIFIED_FORMAT_VERSION, UnifiedOutput};
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
        UNIFIED_FORMAT_VERSION
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
fn cli_json_matches_v3_schema_with_and_without_details() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("sample.rs"),
        "pub fn hotspot(x: i32) -> i32 { if x > 0 { if x > 1 { if x > 2 { if x > 3 { if x > 4 { if x > 5 { x } else { 0 } } else { 0 } } else { 0 } } else { 0 } } else { 0 } } else { 0 } }\n",
    )
    .unwrap();
    let schema = load_json("schemas/debtmap-output-v3.schema.json");

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
            report["summary"]["total_items"],
            report["items"].as_array().unwrap().len()
        );
        assert_eq!(report["items"][0].get("scoring_details").is_some(), verbose);
    }
}

#[test]
fn compare_consumes_v3_summary_and_rejects_unknown_versions() {
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

    let unsupported = directory.path().join("unsupported.json");
    let mut report = load_json("tests/fixtures/output/unified-v3-minimal.json");
    report["format_version"] = Value::String("4.0".to_string());
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
