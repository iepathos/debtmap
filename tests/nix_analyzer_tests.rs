use std::fs;
use std::path::{Path, PathBuf};

use debtmap::analyzers::nix::NixAnalyzer;
use debtmap::analyzers::{Analyzer, get_analyzer};
use debtmap::core::{DebtType, Language};
use debtmap::io::walker::FileWalker;
use debtmap::utils::language_parser::{default_languages, parse_languages, parse_single_language};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/nix")
        .join(relative)
}

fn analyze_fixture(relative: &str) -> debtmap::core::FileMetrics {
    let path = fixture_path(relative);
    let content = fs::read_to_string(&path).expect("fixture should exist");
    let analyzer = get_analyzer(Language::Nix);
    let ast = analyzer.parse(&content, path).expect("parse fixture");
    analyzer.analyze(&ast)
}

fn function_names(metrics: &debtmap::core::FileMetrics) -> Vec<&str> {
    metrics
        .complexity
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect()
}

#[test]
fn test_language_parser_supports_nix() {
    assert_eq!(parse_single_language("nix"), Some(Language::Nix));
    assert_eq!(parse_single_language("NIX"), Some(Language::Nix));
}

#[test]
fn test_default_languages_include_nix() {
    let defaults = default_languages();
    assert_eq!(defaults.len(), 7);
    assert!(defaults.contains(&Language::Nix));
}

#[test]
fn test_parse_languages_nix_only() {
    let result = parse_languages(Some(vec!["nix".to_string()]));
    assert_eq!(result, vec![Language::Nix]);
}

#[test]
fn test_get_analyzer_routes_to_nix() {
    let analyzer = get_analyzer(Language::Nix);
    assert_eq!(analyzer.language(), Language::Nix);
}

#[test]
fn test_analyze_flake_fixture() {
    let metrics = analyze_fixture("flake.nix");

    assert_eq!(metrics.language, Language::Nix);
    assert_eq!(function_names(&metrics), vec!["outputs"]);
    assert_eq!(metrics.complexity.functions[0].cyclomatic, 1);
    assert_eq!(
        metrics
            .dependencies
            .iter()
            .map(|dependency| dependency.name.as_str())
            .collect::<Vec<_>>(),
        vec!["github:NixOS/nixpkgs"]
    );
}

#[test]
fn test_analyze_package_fixture() {
    let metrics = analyze_fixture("hello/default.nix");

    assert_eq!(function_names(&metrics), vec!["default"]);
    assert_eq!(metrics.dependencies[0].name, "./src.nix");
}

#[test]
fn test_analyze_module_fixture() {
    let metrics = analyze_fixture("module.nix");
    let mk_service = metrics
        .complexity
        .functions
        .iter()
        .find(|function| function.name == "mkService")
        .expect("mkService");

    assert_eq!(function_names(&metrics), vec!["module", "mkService"]);
    assert_eq!(mk_service.cyclomatic, 3);
    assert!(
        metrics
            .complexity
            .functions
            .iter()
            .all(|function| function.is_test)
    );
    assert!(metrics.debt_items.is_empty());
}

#[test]
fn test_complexity_debt_for_package_path() {
    let analyzer = NixAnalyzer::new().with_threshold(1);
    let source = "{ f = a: if a then 1 else 0; }";
    let path = PathBuf::from("pkgs/example.nix");
    let ast = analyzer.parse(source, path).expect("parse source");
    let metrics = analyzer.analyze(&ast);

    assert_eq!(metrics.complexity.functions[0].cyclomatic, 2);
    assert!(
        metrics
            .debt_items
            .iter()
            .any(|item| { matches!(item.debt_type, DebtType::Complexity { cyclomatic: 2, .. }) })
    );
}

#[test]
fn test_file_walker_discovers_nix_files() {
    let files = FileWalker::new(fixture_path(""))
        .with_languages(vec![Language::Nix])
        .walk()
        .expect("walk fixtures");

    assert!(files.iter().any(|path| path.ends_with("flake.nix")));
    assert!(files.iter().any(|path| path.ends_with("default.nix")));
    assert!(files.iter().any(|path| path.ends_with("module.nix")));
}

#[test]
fn test_language_from_extension_nix() {
    assert_eq!(Language::from_extension("nix"), Language::Nix);
    assert_eq!(
        Language::from_path(Path::new("pkgs/hello/default.nix")),
        Language::Nix
    );
}
