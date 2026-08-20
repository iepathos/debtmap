//! End-to-end equivalence tests for sequential and parallel analysis modes.

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const FIXTURE: &str = r#"fn shared_transform(value: usize) -> usize {
    value.saturating_mul(2).saturating_add(1)
}

fn alternate_transform(value: usize) -> usize {
    value.saturating_sub(1)
}

fn production_entry(value: usize) -> usize {
    production_hotspot(value)
}

fn secondary_entry(value: usize) -> usize {
    production_hotspot(value + 1)
}

// debtmap:ignore[complexity] -- parity fixture suppression
fn suppressed_hotspot(value: usize) -> usize {
    if value > 0 {
        if value > 1 {
            if value > 2 {
                if value > 3 {
                    if value > 4 {
                        if value > 5 {
                            if value > 6 {
                                if value > 7 {
                                    if value > 8 {
                                        if value > 9 { value } else { 9 }
                                    } else { 8 }
                                } else { 7 }
                            } else { 6 }
                        } else { 5 }
                    } else { 4 }
                } else { 3 }
            } else { 2 }
        } else { 1 }
    } else { 0 }
}

fn production_hotspot(value: usize) -> usize {
    if value > 0 {
        if value % 2 == 0 {
            if value > 10 {
                if value % 3 == 0 {
                    if value > 100 {
                        shared_transform(value)
                    } else {
                        alternate_transform(value + 1)
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

fn profile_analysis(root: &Path, profile_output: &Path) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_debtmap"));
    command.current_dir(root).args([
        "analyze",
        root.to_str().unwrap(),
        "--profile",
        "--profile-output",
        profile_output.to_str().unwrap(),
        "--quiet",
        "--no-tui",
        "--no-context-aware",
        "--no-god-object",
    ]);
    clear_analysis_environment(&mut command);
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "analysis failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_str(&fs::read_to_string(profile_output).unwrap()).unwrap()
}

fn markdown_analysis(root: &Path, output: &Path) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_debtmap"));
    command
        .current_dir(root)
        .arg("analyze")
        .arg(root)
        .args([
            "--format",
            "markdown",
            "--quiet",
            "--no-tui",
            "--no-context-aware",
            "--min-score",
            "0",
            "--output",
        ])
        .arg(output);
    clear_analysis_environment(&mut command);
    let result = command.output().unwrap();
    assert!(result.status.success());
    fs::read_to_string(output).unwrap()
}

fn phase_named<'a>(phases: &'a [Value], name: &str) -> Option<&'a Value> {
    phases.iter().find_map(|phase| {
        let children = phase["children"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        (phase["name"] == name)
            .then_some(phase)
            .or_else(|| phase_named(children, name))
    })
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
    assert_eq!(parallel["receipt"]["suppressions"]["applied_count"], 1);
    assert_eq!(
        parallel["receipt"]["suppressions"]["records"][0]["decision"]["reason"],
        "parity fixture suppression"
    );
    let hotspot = parallel_items
        .iter()
        .find(|item| item["location"]["function"] == "production_hotspot")
        .unwrap();
    assert_eq!(
        hotspot["dependencies"]["upstream_callers"],
        serde_json::json!([
            "analysis.rs:production_entry",
            "analysis.rs:secondary_entry"
        ])
    );
    assert_eq!(
        hotspot["dependencies"]["downstream_callees"],
        serde_json::json!([
            "analysis.rs:alternate_transform",
            "analysis.rs:shared_transform"
        ])
    );
    assert_eq!(parallel_items, sequential_items);
    assert_eq!(
        canonical_report(parallel, fixture.path()),
        canonical_report(sequential, fixture.path())
    );
}

#[test]
fn profiling_report_exposes_debt_scoring_subphases() {
    let fixture = TempDir::new().unwrap();
    fs::write(fixture.path().join("analysis.rs"), FIXTURE).unwrap();
    let report = profile_analysis(fixture.path(), &fixture.path().join("profile.json"));
    let phases = report["phases"].as_array().unwrap();
    let debt_scoring = phase_named(phases, "debt_scoring").unwrap();
    let children = debt_scoring["children"].as_array().unwrap();
    let child_names: Vec<_> = children
        .iter()
        .filter_map(|child| child["name"].as_str())
        .collect();

    for expected in [
        "prepare_scoring",
        "score_functions",
        "analyze_files",
        "finalize_files",
        "sort_items",
        "calculate_impact",
    ] {
        assert!(child_names.contains(&expected), "missing {expected}");
    }
}

#[test]
fn markdown_reports_applied_suppressions() {
    let fixture = TempDir::new().unwrap();
    fs::write(fixture.path().join("analysis.rs"), FIXTURE).unwrap();

    let markdown = markdown_analysis(fixture.path(), &fixture.path().join("report.md"));

    assert!(markdown.contains("## Suppressions Applied"));
    assert!(markdown.contains("parity fixture suppression"));
    assert!(markdown.contains("directive line"));
}

#[test]
fn python_calls_contribute_to_shared_dependency_scoring() {
    let fixture = TempDir::new().unwrap();
    let source = r#"class Service:
    def alpha_entry(self, value):
        return self.hotspot(value)

    def beta_entry(self, value):
        return self.hotspot(value + 1)

    def helper(self, value):
        return value * 2

    def alternate(self, value):
        return value - 1

    def hotspot(self, value):
        if value > 0:
            if value > 1:
                if value > 2:
                    if value > 3:
                        if value > 4:
                            if value > 5:
                                if value > 6:
                                    if value > 7:
                                        if value > 8:
                                            if value > 9:
                                                if value > 10:
                                                    return self.helper(value)
                                                return self.alternate(value)
                                            return 9
                                        return 8
                                    return 7
                                return 6
                            return 5
        return 0
"#;
    fs::write(fixture.path().join("analysis.py"), source).unwrap();
    let parallel = analyze(
        fixture.path(),
        &fixture.path().join("python-parallel.json"),
        &["--jobs", "2", "--no-god-object"],
    );
    let sequential = analyze(
        fixture.path(),
        &fixture.path().join("python-sequential.json"),
        &["--no-parallel", "--no-god-object"],
    );
    assert_eq!(
        canonical_report(parallel.clone(), fixture.path()),
        canonical_report(sequential, fixture.path())
    );
    let hotspot = parallel["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["location"]["function"] == "Service.hotspot")
        .unwrap_or_else(|| panic!("{}", serde_json::to_string_pretty(&parallel).unwrap()));

    assert_eq!(hotspot["dependencies"]["upstream_count"], 2);
    assert_eq!(hotspot["dependencies"]["downstream_count"], 2);
    assert!(
        hotspot["scoring_details"]["dependency_score"]
            .as_f64()
            .unwrap()
            > 0.0
    );
}

#[test]
fn python_import_aliases_contribute_to_shared_dependency_scoring() {
    let fixture = TempDir::new().unwrap();
    fs::write(
        fixture.path().join("app.py"),
        "from helpers import hotspot as run_hotspot\n\ndef entry(value):\n    return run_hotspot(value)\n",
    )
    .unwrap();
    fs::write(
        fixture.path().join("helpers.py"),
        r#"def hotspot(value):
    if value > 0:
        if value > 1:
            if value > 2:
                if value > 3:
                    if value > 4:
                        if value > 5:
                            if value > 6:
                                if value > 7:
                                    if value > 8:
                                        if value > 9:
                                            if value > 10:
                                                return value
    return 0
"#,
    )
    .unwrap();
    let parallel = analyze(
        fixture.path(),
        &fixture.path().join("import-parallel.json"),
        &["--jobs", "2", "--no-god-object"],
    );
    let sequential = analyze(
        fixture.path(),
        &fixture.path().join("import-sequential.json"),
        &["--no-parallel", "--no-god-object"],
    );
    assert_eq!(
        canonical_report(parallel.clone(), fixture.path()),
        canonical_report(sequential, fixture.path())
    );
    let hotspot = parallel["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["location"]["function"] == "hotspot")
        .unwrap();

    assert_eq!(hotspot["dependencies"]["upstream_count"], 1);
    assert!(
        hotspot["scoring_details"]["dependency_score"]
            .as_f64()
            .unwrap()
            > 0.0
    );
}

fn canonical_report(mut report: Value, root: &Path) -> Value {
    report["metadata"]["generated_at"] = Value::String("<time>".into());
    report["receipt"]["reference_time"] = Value::String("<time>".into());
    report["receipt"]
        .as_object_mut()
        .unwrap()
        .remove("execution");
    for item in report["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("context");
    }
    canonicalize(&mut report, root);
    report
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
