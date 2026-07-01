use std::fs;
use std::path::{Path, PathBuf};

use debtmap::analyzers::{Analyzer, get_analyzer};
use debtmap::core::{DebtType, Language, LanguageSpecificData, PurityLevel};
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

fn analyze_source(name: &str, source: &str) -> debtmap::core::FileMetrics {
    let analyzer = debtmap::analyzers::solidity::SolidityAnalyzer::new();
    let path = PathBuf::from(name);
    let ast = analyzer.parse(source, path).expect("parse source");
    analyzer.analyze(&ast)
}

fn analyze_source_with_config(
    name: &str,
    source: &str,
    config: debtmap::config::SolidityLanguageConfig,
) -> debtmap::core::FileMetrics {
    let analyzer = debtmap::analyzers::solidity::SolidityAnalyzer::new().with_config(config);
    let path = PathBuf::from(name);
    let ast = analyzer.parse(source, path).expect("parse source");
    analyzer.analyze(&ast)
}

fn has_pattern(metrics: &debtmap::core::FileMetrics, pattern: &str) -> bool {
    metrics.debt_items.iter().any(|item| {
        matches!(
            &item.debt_type,
            DebtType::CodeSmell { smell_type } if smell_type.as_deref() == Some(pattern)
        )
    })
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

#[test]
fn test_security_patterns_positive_and_negative_fixtures() {
    let cases = [
        (
            "tx-origin-usage",
            r#"pragma solidity 0.8.20; contract C { function f() internal view returns (bool) { return tx.origin == msg.sender; } }"#,
            r#"pragma solidity 0.8.20; contract C { function f() internal view returns (bool) { return msg.sender != address(0); } }"#,
        ),
        (
            "unchecked-low-level-call",
            r#"pragma solidity 0.8.20; contract C { function f(address target) internal { target.call{value: 0}(""); } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(address target) internal view returns (address) { return target; } }"#,
        ),
        (
            "external-call-before-state-update",
            r#"pragma solidity 0.8.20; contract C { mapping(address => uint256) balances; function f() internal { payable(msg.sender).transfer(1); balances[msg.sender] = 0; } }"#,
            r#"pragma solidity 0.8.20; contract C { mapping(address => uint256) balances; function f() internal { balances[msg.sender] = 0; payable(msg.sender).transfer(1); } }"#,
        ),
        (
            "delegatecall-usage",
            r#"pragma solidity 0.8.20; contract C { function f(address target) internal { target.delegatecall(""); } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(address target) internal view returns (address) { return target; } }"#,
        ),
        (
            "selfdestruct-usage",
            r#"pragma solidity 0.8.20; contract C { function f(address payable target) internal { selfdestruct(target); } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(address payable target) internal view returns (address) { return target; } }"#,
        ),
        (
            "assembly-block",
            r#"pragma solidity 0.8.20; contract C { function f() internal pure returns (uint256 x) { assembly { x := 1 } } }"#,
            r#"pragma solidity 0.8.20; contract C { function f() internal pure returns (uint256) { return 1; } }"#,
        ),
        (
            "unbounded-loop",
            r#"pragma solidity 0.8.20; contract C { address[] users; function f() internal { for (uint256 i = 0; i < users.length; i++) { } } }"#,
            r#"pragma solidity 0.8.20; contract C { function f() internal { for (uint256 i = 0; i < 10; i++) { } } }"#,
        ),
        (
            "missing-access-control",
            r#"pragma solidity 0.8.20; contract C { function setValue(uint256 value) public { value; } }"#,
            r#"pragma solidity 0.8.20; contract C { modifier onlyOwner() { _; } function setValue(uint256 value) public onlyOwner { value; } }"#,
        ),
        (
            "hardcoded-address",
            r#"pragma solidity 0.8.20; contract C { function f() internal pure returns (address) { return 0x1234567890123456789012345678901234567890; } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(address target) internal pure returns (address) { return target; } }"#,
        ),
        (
            "floating-pragma",
            r#"pragma solidity ^0.8.20; contract C { function f() internal {} }"#,
            r#"pragma solidity 0.8.20; contract C { function f() internal {} }"#,
        ),
    ];

    for (pattern, positive, negative) in cases {
        assert!(
            has_pattern(&analyze_source("Positive.sol", positive), pattern),
            "expected positive fixture to detect {pattern}"
        );
        assert!(
            !has_pattern(&analyze_source("Negative.sol", negative), pattern),
            "expected negative fixture not to detect {pattern}"
        );
    }
}

#[test]
fn test_large_contract_pattern_positive_and_negative() {
    let source = r#"pragma solidity 0.8.20;
contract C {
    function a() internal {}
    function b() internal {}
}
"#;
    let low_threshold = debtmap::config::SolidityLanguageConfig {
        large_contract_threshold: 1,
        ..Default::default()
    };
    let high_threshold = debtmap::config::SolidityLanguageConfig {
        large_contract_threshold: 10,
        ..Default::default()
    };

    assert!(has_pattern(
        &analyze_source_with_config("Large.sol", source, low_threshold),
        "large-contract"
    ));
    assert!(!has_pattern(
        &analyze_source_with_config("Small.sol", source, high_threshold),
        "large-contract"
    ));
}

#[test]
fn test_solidity_entropy_metrics_are_populated() {
    let source = r#"pragma solidity 0.8.20;
contract C {
    function validate(address a, address b, address c) public pure {
        require(a != address(0));
        require(b != address(0));
        require(c != address(0));
    }
}
"#;
    let metrics = analyze_source("Entropy.sol", source);
    let function = metrics
        .complexity
        .functions
        .iter()
        .find(|function| function.name.contains("validate"))
        .expect("validate function");

    assert!(function.entropy_score.is_some());
    assert!(function.entropy_analysis.is_some());
    assert!(function.adjusted_complexity.is_some());
}

#[test]
fn test_advanced_advisory_patterns_positive_and_negative() {
    let cases = [
        (
            "unchecked-arithmetic",
            r#"pragma solidity 0.8.20; contract C { function f(uint256 x) internal pure returns (uint256) { unchecked { return x + 1; } } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(uint256 x) internal pure returns (uint256) { return x + 1; } }"#,
        ),
        (
            "unsafe-erc20-transfer",
            r#"pragma solidity 0.8.20; interface IERC20 { function transfer(address,uint256) external returns (bool); } contract C { function f(IERC20 token, address to) internal { token.transfer(to, 1); } }"#,
            r#"pragma solidity 0.8.20; interface IERC20 { function transfer(address,uint256) external returns (bool); } contract C { function f(IERC20 token, address to) internal { require(token.transfer(to, 1)); } }"#,
        ),
        (
            "push-without-length-cap",
            r#"pragma solidity 0.8.20; contract C { uint256[] values; function f(uint256 value) internal { values.push(value); } }"#,
            r#"pragma solidity 0.8.20; contract C { uint256[] values; function f(uint256 value) internal { require(values.length < 10); values.push(value); } }"#,
        ),
        (
            "block-timestamp-dependency",
            r#"pragma solidity 0.8.20; contract C { function f() internal view returns (uint256) { return block.timestamp; } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(uint256 nowish) internal pure returns (uint256) { return nowish; } }"#,
        ),
        (
            "tx-gas-price-dependency",
            r#"pragma solidity 0.8.20; contract C { function f() internal view returns (uint256) { return tx.gasprice; } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(uint256 gasPrice) internal pure returns (uint256) { return gasPrice; } }"#,
        ),
        (
            "encode-packed-collision",
            r#"pragma solidity 0.8.20; contract C { function f(string memory a, string memory b) internal pure returns (bytes memory) { return abi.encodePacked(a, b); } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(string memory a, string memory b) internal pure returns (bytes memory) { return abi.encode(a, b); } }"#,
        ),
        (
            "delegatecall-in-constructor",
            r#"pragma solidity 0.8.20; contract C { constructor(address target) { target.delegatecall(""); } }"#,
            r#"pragma solidity 0.8.20; contract C { function f(address target) internal { target.delegatecall(""); } }"#,
        ),
    ];

    for (pattern, positive, negative) in cases {
        assert!(
            has_pattern(&analyze_source("AdvancedPositive.sol", positive), pattern),
            "expected positive fixture to detect {pattern}"
        );
        assert!(
            !has_pattern(&analyze_source("AdvancedNegative.sol", negative), pattern),
            "expected negative fixture not to detect {pattern}"
        );
    }
}

#[test]
fn test_natspec_debt_for_public_function_without_docs() {
    let undocumented = analyze_source(
        "Undocumented.sol",
        r#"pragma solidity 0.8.20;
contract C {
    function f() public {}
}
"#,
    );
    let documented = analyze_source(
        "Documented.sol",
        r#"pragma solidity 0.8.20;
contract C {
    /// @notice Does the thing.
    function f() public {}
}
"#,
    );

    assert!(has_pattern(&undocumented, "missing-natspec"));
    assert!(!has_pattern(&documented, "missing-natspec"));
}

#[test]
fn test_solidity_language_specific_data_and_purity() {
    let metrics = analyze_source(
        "Metadata.sol",
        r#"pragma solidity 0.8.20;
contract C {
    /// @notice Reads value.
    function f() public view returns (uint256) {
        return 1;
    }
}
"#,
    );
    let function = metrics
        .complexity
        .functions
        .iter()
        .find(|function| function.name == "C.f")
        .expect("function");

    assert_eq!(function.purity_level, Some(PurityLevel::ReadOnly));
    match function.language_specific.as_ref() {
        Some(LanguageSpecificData::Solidity(data)) => {
            assert_eq!(data.state_mutability.as_deref(), Some("view"));
            assert!(!data.is_payable);
        }
        other => panic!("expected Solidity language-specific data, got {other:?}"),
    }
}
