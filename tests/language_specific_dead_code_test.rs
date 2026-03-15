use debtmap::analyzers::{rust::RustAnalyzer, Analyzer};
use debtmap::config::{get_language_features, LanguageFeatures};
use debtmap::core::Language;
use std::path::PathBuf;

#[test]
fn test_dead_code_disabled_by_default() {
    // Dead code detection is disabled by default for all languages (dependency tracking prioritized)
    assert!(!get_language_features(&Language::Rust).detect_dead_code);
    assert!(!get_language_features(&Language::Python).detect_dead_code);
    assert!(!get_language_features(&Language::TypeScript).detect_dead_code);
}

#[test]
fn test_rust_dead_code_always_returns_false() {
    let rust_code = r#"
fn unused_function() {
    println!("I'm not called");
}

fn main() {
    println!("Hello, world!");
}
"#;

    let path = PathBuf::from("test.rs");
    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(rust_code, path.clone()).unwrap();
    let analysis_result = analyzer.analyze(&ast);

    let unused_func = analysis_result
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "unused_function")
        .unwrap();

    // Verify metrics are extracted correctly even for unused code
    assert!(unused_func.cyclomatic >= 1);
    assert_eq!(unused_func.name, "unused_function");
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
}

#[test]
fn test_language_features_default() {
    let default_features = LanguageFeatures::default();
    assert!(!default_features.detect_dead_code);
    assert!(default_features.detect_complexity);
    assert!(default_features.detect_duplication);
}
