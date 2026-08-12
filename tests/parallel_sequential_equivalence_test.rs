//! End-to-end equivalence tests for sequential and parallel analysis modes.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const FIXTURE: &str = r#"fn shared_transform(value: usize) -> usize {
    value.saturating_mul(2).saturating_add(1)
}

fn production_entry(value: usize) -> usize {
    production_hotspot(value)
}

fn production_hotspot(value: usize) -> usize {
    if value > 0 {
        if value % 2 == 0 {
            if value > 10 {
                if value % 3 == 0 {
                    if value > 100 {
                        shared_transform(value)
                    } else {
                        shared_transform(value + 1)
                    }
                } else {
                    shared_transform(value - 1)
                }
            } else {
                shared_transform(value + 2)
            }
        } else if value % 5 == 0 {
            shared_transform(value + 3)
        } else {
            shared_transform(value + 4)
        }
    } else {
        0
    }
}
"#;

fn analyze(root: &Path, output: &Path, mode: &[&str]) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_debtmap"));
    command
        .current_dir(root)
        .arg("analyze")
        .arg(root)
        .args([
            "--format",
            "json",
            "--quiet",
            "--no-tui",
            "--no-context-aware",
            "--min-score",
            "0",
            "-vv",
        ])
        .args(mode)
        .arg("--output")
        .arg(output);
    clear_analysis_environment(&mut command);
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "analysis failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap()
}

fn clear_analysis_environment(command: &mut Command) {
    for variable in [
        "DEBTMAP_CONFIG",
        "DEBTMAP_MIN_CYCLOMATIC",
        "DEBTMAP_MIN_COGNITIVE",
        "DEBTMAP_MIN_RISK",
        "DEBTMAP_JOBS",
        "DEBTMAP_FUNCTIONAL_ANALYSIS",
        "DEBTMAP_FUNCTIONAL_ANALYSIS_PROFILE",
    ] {
        command.env_remove(variable);
    }
}

fn canonical_function_items(report: &Value, root: &Path) -> Vec<Value> {
    let mut items: Vec<_> = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["type"] == "Function")
        .map(project_function_item)
        .collect();
    for item in &mut items {
        canonicalize(item, root);
    }
    items.sort_by_key(|item| serde_json::to_string(item).unwrap());
    items
}

fn project_function_item(item: &Value) -> Value {
    serde_json::json!({
        "location": item["location"],
        "debt_type": item["debt_type"],
        "score": item["score"],
        "category": item["category"],
        "priority": item["priority"],
        "function_role": item["function_role"],
        "metrics": item["metrics"],
        "scoring_details": item["scoring_details"],
        "purity_analysis": item["purity_analysis"],
        "dependencies": item["dependencies"],
    })
}

fn canonicalize(value: &mut Value, root: &Path) {
    match value {
        Value::String(text) => normalize_text(text, root),
        Value::Array(values) => {
            for value in values.iter_mut() {
                canonicalize(value, root);
            }
            values.sort_by_key(|value| serde_json::to_string(value).unwrap());
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                canonicalize(value, root);
            }
        }
        _ => {}
    }
}

fn normalize_text(text: &mut String, root: &Path) {
    *text = text
        .replace(root.to_string_lossy().as_ref(), "<root>")
        .replace('\\', "/");
}

#[test]
fn parallel_and_sequential_function_items_have_identical_scores() {
    let fixture = TempDir::new().unwrap();
    fs::write(fixture.path().join("analysis.rs"), FIXTURE).unwrap();
    let parallel = analyze(
        fixture.path(),
        &fixture.path().join("parallel.json"),
        &["--jobs", "2", "--no-god-object"],
    );
    let sequential = analyze(
        fixture.path(),
        &fixture.path().join("sequential.json"),
        &["--no-parallel", "--no-god-object"],
    );
    let parallel_items = canonical_function_items(&parallel, fixture.path());
    let sequential_items = canonical_function_items(&sequential, fixture.path());

    assert!(parallel_items.iter().any(is_scored_production_hotspot));
    assert_eq!(parallel_items, sequential_items);
}

fn is_scored_production_hotspot(item: &Value) -> bool {
    item["location"]["function"] == "production_hotspot"
        && !item["scoring_details"].is_null()
        && item["dependencies"]["upstream_count"].as_u64().unwrap_or(0) > 0
        && item["dependencies"]["downstream_count"]
            .as_u64()
            .unwrap_or(0)
            > 0
}

fn write_god_file_fixture(root: &Path) {
    let functions = (0..51)
        .map(|index| {
            format!(
                "fn helper_{index:02}(value: usize) -> usize {{ if value > 0 && value > 1 && value > 2 && value > 3 && value > 4 && value > 5 && value > 6 && value > 7 {{ value }} else {{ {index} }} }}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(root.join("god.rs"), format!("{functions}\n")).unwrap();
}

fn canonical_god_items(report: &Value, root: &Path) -> Vec<Value> {
    let mut items: Vec<_> = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["type"] == "File" || !item["debt_type"]["GodObject"].is_null())
        .cloned()
        .collect();
    for item in &mut items {
        if let Value::Object(fields) = item {
            fields.remove("context");
        }
        canonicalize(item, root);
    }
    items.sort_by_key(|item| serde_json::to_string(item).unwrap());
    items
}

fn canonical_summary(report: &Value) -> Value {
    serde_json::json!({
        "total_items": report["summary"]["total_items"],
        "total_debt_score": report["summary"]["total_debt_score"],
        "debt_density": report["summary"]["debt_density"],
        "total_loc": report["summary"]["total_loc"],
        "by_type": report["summary"]["by_type"],
        "by_category": report["summary"]["by_category"],
        "score_distribution": report["summary"]["score_distribution"],
        "cohesion": report["summary"]["cohesion"],
    })
}

#[test]
fn parallel_and_sequential_god_file_reports_are_equivalent() {
    let fixture = TempDir::new().unwrap();
    write_god_file_fixture(fixture.path());
    let parallel = analyze(
        fixture.path(),
        &fixture.path().join("parallel.json"),
        &["--jobs", "2"],
    );
    let sequential = analyze(
        fixture.path(),
        &fixture.path().join("sequential.json"),
        &["--no-parallel"],
    );
    let parallel_items = canonical_god_items(&parallel, fixture.path());
    let sequential_items = canonical_god_items(&sequential, fixture.path());

    assert_eq!(parallel["summary"]["total_loc"], 51);
    assert_eq!(parallel["summary"]["by_type"]["File"], 1);
    assert_eq!(parallel["summary"]["by_type"]["Function"], 1);
    assert_eq!(parallel_items.len(), 2);
    assert!(parallel_items.iter().any(|item| item["type"] == "File"));
    assert!(parallel_items.iter().any(|item| {
        item["type"] == "Function"
            && item["location"]["function"] == "[file-scope]"
            && !item["debt_type"]["GodObject"].is_null()
    }));
    assert_eq!(parallel_items, sequential_items);
    assert_eq!(canonical_summary(&parallel), canonical_summary(&sequential));
}

#[test]
fn parallel_and_sequential_god_file_suppression_is_equivalent() {
    let fixture = TempDir::new().unwrap();
    write_god_file_fixture(fixture.path());
    let path = fixture.path().join("god.rs");
    let source = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        format!("// debtmap:ignore[god_object] -- generated registry\n{source}"),
    )
    .unwrap();
    let parallel = analyze(
        fixture.path(),
        &fixture.path().join("parallel.json"),
        &["--jobs", "2"],
    );
    let sequential = analyze(
        fixture.path(),
        &fixture.path().join("sequential.json"),
        &["--no-parallel"],
    );
    let parallel_items = canonical_god_items(&parallel, fixture.path());
    let sequential_items = canonical_god_items(&sequential, fixture.path());

    assert!(
        parallel_items
            .iter()
            .all(|item| { item["type"] != "Function" || item["debt_type"]["GodObject"].is_null() })
    );
    assert_eq!(parallel_items, sequential_items);
    assert_eq!(canonical_summary(&parallel), canonical_summary(&sequential));
}
