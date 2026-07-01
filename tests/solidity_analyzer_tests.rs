use std::fs;
use std::path::{Path, PathBuf};

use debtmap::analyzers::{Analyzer, get_analyzer};
use debtmap::core::Language;
use debtmap::io::walker::FileWalker;
use debtmap::utils::language_parser::{default_languages, parse_languages, parse_single_language};

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/solidity")
        .join(relative)
}

fn analyze_fixture(relative: &str) -> debtmap::core::FileMetrics {
    let path = fixture_path(relative);
    let content = fs::read_to_string(&path).expect("fixture should exist");
    let analyzer = get_analyzer(Language::Solidity);
    let ast = analyzer.parse(&content, path).expect("parse fixture");
    analyzer.analyze(&ast)
}

#[test]
fn test_language_parser_supports_solidity() {
    assert_eq!(parse_single_language("solidity"), Some(Language::Solidity));
    assert_eq!(parse_single_language("sol"), Some(Language::Solidity));
    assert_eq!(parse_single_language("SOL"), Some(Language::Solidity));
}

#[test]
fn test_default_languages_include_solidity() {
    let defaults = default_languages();
    assert_eq!(defaults.len(), 6);
    assert!(defaults.contains(&Language::Solidity));
}

#[test]
fn test_parse_languages_solidity_only() {
    let result = parse_languages(Some(vec!["sol".to_string()]));
    assert_eq!(result, vec![Language::Solidity]);
}

#[test]
fn test_get_analyzer_routes_to_solidity() {
    let analyzer = get_analyzer(Language::Solidity);
    assert_eq!(analyzer.language(), Language::Solidity);
}

#[test]
fn test_analyze_simple_token_fixture() {
    let metrics = analyze_fixture("simple/SimpleToken.sol");

    assert_eq!(metrics.language, Language::Solidity);
    assert!(
        metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name.contains("mint"))
    );
    assert!(metrics.dependencies.is_empty() || metrics.path.ends_with("SimpleToken.sol"));
}

#[test]
fn test_analyze_complex_vault_emits_complexity_debt() {
    let analyzer = debtmap::analyzers::solidity::SolidityAnalyzer::new().with_threshold(2);
    let path = fixture_path("complex/ComplexVault.sol");
    let content = fs::read_to_string(&path).unwrap();
    let ast = analyzer.parse(&content, path).unwrap();
    let metrics = analyzer.analyze(&ast);

    assert!(metrics.complexity.functions[0].cyclomatic > 2);
    assert!(
        metrics
            .debt_items
            .iter()
            .any(|item| { matches!(item.debt_type, debtmap::core::DebtType::Complexity { .. }) })
    );
}

#[test]
fn test_analyze_security_fixture_detects_tx_origin() {
    let metrics = analyze_fixture("security/InsecureBank.sol");
    let withdraw = metrics
        .complexity
        .functions
        .iter()
        .find(|function| function.name.contains("withdraw"))
        .expect("withdraw function");

    assert!(
        withdraw
            .detected_patterns
            .as_ref()
            .is_some_and(|patterns| patterns.contains(&"tx-origin-usage".to_string()))
    );
}

#[test]
fn test_foundry_fixture_skips_debt() {
    let metrics = analyze_fixture("foundry/Token.t.sol");
    assert!(metrics.complexity.functions.iter().all(|f| f.is_test));
    assert!(metrics.debt_items.is_empty());
}

#[test]
fn test_file_walker_discovers_sol_files() {
    let root = fixture_path("");
    let files = FileWalker::new(root)
        .with_languages(vec![Language::Solidity])
        .walk()
        .expect("walk fixtures");

    assert!(files.iter().any(|path| path.ends_with("SimpleToken.sol")));
    assert!(files.iter().any(|path| path.ends_with("Token.t.sol")));
}

#[test]
fn test_language_from_extension_sol() {
    assert_eq!(Language::from_extension("sol"), Language::Solidity);
    assert_eq!(
        Language::from_path(Path::new("contracts/Token.sol")),
        Language::Solidity
    );
}
