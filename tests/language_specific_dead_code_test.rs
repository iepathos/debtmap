use debtmap::analyzers::{rust::RustAnalyzer, Analyzer};
use debtmap::config::{get_language_features, LanguageFeatures};
use debtmap::core::Language;
use debtmap::priority::call_graph::{CallGraph, FunctionId};
use debtmap::priority::scoring::classification::is_dead_code_with_exclusions;
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn test_rust_dead_code_disabled_by_default() {
    let rust_features = get_language_features(&Language::Rust);
    assert!(
        !rust_features.detect_dead_code,
        "Rust dead code detection should be disabled by default"
    );
}

#[test]
fn test_python_dead_code_enabled_by_default() {
    let python_features = get_language_features(&Language::Python);
    assert!(
        python_features.detect_dead_code,
        "Python dead code detection should be enabled by default"
    );
}

#[test]
fn test_javascript_dead_code_enabled_by_default() {
    let js_features = get_language_features(&Language::JavaScript);
    assert!(
        js_features.detect_dead_code,
        "JavaScript dead code detection should be enabled by default"
    );
}

#[test]
fn test_typescript_dead_code_enabled_by_default() {
    let ts_features = get_language_features(&Language::TypeScript);
    assert!(
        ts_features.detect_dead_code,
        "TypeScript dead code detection should be enabled by default"
    );
}

#[test]
fn test_rust_dead_code_always_returns_false() {
    // Create a simple Rust file with an unused function
    let rust_code = r#"
fn unused_function() {
    println!("I'm not called");
}

fn main() {
    println!("Hello, world!");
}
"#;

    // Analyze the code
    let path = PathBuf::from("test.rs");
    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(rust_code, path.clone()).unwrap();
    let analysis_result = analyzer.analyze(&ast);

    // Find the unused function
    let unused_func = analysis_result
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "unused_function")
        .expect("Should find unused_function");

    // Create a call graph with no calls to unused_function
    let mut call_graph = CallGraph::new();
    let main_id = FunctionId::new(path.clone(), "main".to_string(), 6);
    let unused_id = FunctionId::new(
        path.clone(),
        "unused_function".to_string(),
        unused_func.line,
    );

    call_graph.add_function(main_id.clone(), false, false, 1, 10);
    call_graph.add_function(unused_id.clone(), false, false, 1, 10);
    // No calls added - unused_function is not called

    // Check if dead code detection marks it as dead
    let framework_exclusions = HashSet::new();
    let is_dead = is_dead_code_with_exclusions(
        unused_func,
        &call_graph,
        &unused_id,
        &framework_exclusions,
        None,
    );

    // For Rust files, dead code detection should always return false
    assert!(
        !is_dead,
        "Rust files should never be marked as dead code because rustc handles this"
    );
}

#[test]
fn test_python_dead_code_still_detected() {
    // Test that Python dead code detection still works
    let python_features = get_language_features(&Language::Python);
    assert!(
        python_features.detect_dead_code,
        "Python dead code detection should remain enabled"
    );

    // Note: Actual Python dead code detection would require a Python analyzer
    // This test just verifies the configuration is correct
}

#[test]
fn test_language_from_path() {
    assert_eq!(
        Language::from_path(&PathBuf::from("test.rs")),
        Language::Rust
    );
    assert_eq!(
        Language::from_path(&PathBuf::from("test.py")),
        Language::Python
    );
    assert_eq!(
        Language::from_path(&PathBuf::from("test.js")),
        Language::JavaScript
    );
    assert_eq!(
        Language::from_path(&PathBuf::from("test.ts")),
        Language::TypeScript
    );
    assert_eq!(
        Language::from_path(&PathBuf::from("test.unknown")),
        Language::Unknown
    );
}

#[test]
fn test_language_features_default() {
    let default_features = LanguageFeatures::default();
    assert!(default_features.detect_dead_code);
    assert!(default_features.detect_complexity);
    assert!(default_features.detect_duplication);
}
