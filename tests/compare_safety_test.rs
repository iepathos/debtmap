//! CLI regressions: incompatible measurements must not become improvement claims.

use serde_json::{Value, json};
use std::fs;
use std::process::{Command, Output};

fn report(score: f64) -> Value {
    let mut report: Value =
        serde_json::from_str(include_str!("fixtures/output/unified-v4-minimal.json")).unwrap();
    report["summary"]["total_debt_score"] = json!(score);
    report
}

fn compare(before: &Value, after: &Value, format: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let before_path = directory.path().join("before.json");
    let after_path = directory.path().join("after.json");
    fs::write(&before_path, serde_json::to_vec(before).unwrap()).unwrap();
    fs::write(&after_path, serde_json::to_vec(after).unwrap()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args(["compare", "--before"])
        .arg(before_path)
        .arg("--after")
        .arg(after_path)
        .args(["--format", format])
        .output()
        .unwrap()
}

fn assert_rejected(output: Output, reason: &str) {
    assert!(
        !output.status.success(),
        "unsafe comparison succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains(reason), "missing {reason:?}: {stderr}");
    assert!(output.stdout.is_empty(), "must not emit a trend report");
}

#[test]
fn changed_analyzer_version_cannot_claim_improvement() {
    let mut after = report(50.0);
    after["metadata"]["debtmap_version"] = json!("0.24.0");
    assert_rejected(compare(&report(100.0), &after, "json"), "version");
}

#[test]
fn incompatible_receipts_cannot_claim_improvement() {
    for (pointer, value, reason) in [
        (
            "/receipt/policy_fingerprint",
            json!("f".repeat(64)),
            "polic",
        ),
        ("/receipt/policy/complexity_threshold", json!(99), "polic"),
        ("/receipt/evidence/coverage_loaded", json!(true), "evidence"),
        ("/receipt/selection/top", json!(10), "selection"),
        (
            "/receipt/analysis_target",
            json!("another-project"),
            "target",
        ),
        ("/receipt/execution/multi_pass", json!(true), "multi"),
    ] {
        let mut after = report(50.0);
        *after.pointer_mut(pointer).unwrap() = value;
        assert_rejected(compare(&report(100.0), &after, "json"), reason);
    }
}

#[test]
fn unknown_scope_cannot_claim_improvement() {
    for status in ["partial", "limited", "unknown"] {
        let mut after = report(50.0);
        after["receipt"]["scope"]["status"] = json!(status);
        assert_rejected(compare(&report(100.0), &after, "json"), "scope");
        assert_rejected(compare(&after, &report(100.0), "json"), "scope");
    }
}

#[test]
fn missing_analyzer_identity_cannot_claim_improvement() {
    let mut after = report(50.0);
    after["metadata"]["debtmap_version"] = json!("");
    assert_rejected(compare(&report(100.0), &after, "json"), "version");
}

#[test]
fn legacy_reports_without_receipts_cannot_claim_improvement() {
    let mut legacy = report(50.0);
    legacy["format_version"] = json!("3.0");
    legacy.as_object_mut().unwrap().remove("receipt");
    assert_rejected(compare(&report(100.0), &legacy, "json"), "receipt");
    assert_rejected(compare(&legacy, &legacy, "json"), "receipt");
}

#[test]
fn incompatible_reports_are_rejected_in_every_output_format() {
    let mut after = report(50.0);
    after["metadata"]["debtmap_version"] = json!("0.24.0");
    for format in ["json", "markdown", "terminal", "dot"] {
        assert_rejected(compare(&report(100.0), &after, format), "version");
    }
}

#[test]
fn rejected_comparison_does_not_overwrite_output_file() {
    let directory = tempfile::tempdir().unwrap();
    let before_path = directory.path().join("before.json");
    let after_path = directory.path().join("after.json");
    let output_path = directory.path().join("comparison.json");
    fs::write(&before_path, serde_json::to_vec(&report(100.0)).unwrap()).unwrap();
    let mut after = report(50.0);
    after["metadata"]["debtmap_version"] = json!("0.24.0");
    fs::write(&after_path, serde_json::to_vec(&after).unwrap()).unwrap();
    fs::write(&output_path, "previous report").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_debtmap"))
        .args(["compare", "--before"])
        .arg(before_path)
        .arg("--after")
        .arg(after_path)
        .args(["--format", "json", "--output"])
        .arg(&output_path)
        .output()
        .unwrap();
    assert_rejected(output, "version");
    assert_eq!(fs::read_to_string(output_path).unwrap(), "previous report");
}

#[test]
fn compatible_reports_still_show_real_changes() {
    let before = report(100.0);
    for (score, trend) in [
        (50.0, "Improving"),
        (100.0, "Stable"),
        (150.0, "Regressing"),
    ] {
        let mut after = report(score);
        after["metadata"]["generated_at"] = json!("2026-09-20T00:00:00Z");
        after["receipt"]["source_revision"] = json!({"commit": "new", "dirty": false});
        after["receipt"]["execution"]["parallel"] = json!(true);
        after["receipt"]["execution"]["jobs"] = json!(4);
        let output = compare(&before, &after, "json");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let comparison: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(comparison["summary"]["overall_debt_trend"], trend);
        assert_eq!(
            comparison["project_health"]["after"]["total_debt_score"],
            score
        );
    }
}
