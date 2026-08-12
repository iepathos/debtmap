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
            "--no-god-object",
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
        &["--jobs", "2"],
    );
    let sequential = analyze(
        fixture.path(),
        &fixture.path().join("sequential.json"),
        &["--no-parallel"],
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
