use std::path::{Path, PathBuf};

use super::*;
use crate::analyzers::solidity::debt::natspec::{
    detect_natspec_debt, detect_natspec_debt_with_signatures,
};
use crate::analyzers::solidity::debt::security_patterns::{
    detect_contract_patterns, detect_contract_patterns_with_counts,
};
use crate::analyzers::solidity::debt::{detect_debt, detect_debt_with_extraction};
use crate::analyzers::solidity::extraction::extract_solidity;
use crate::analyzers::solidity::metrics::to_function_metrics;
use crate::analyzers::solidity::parser::parse_source;
use crate::analyzers::solidity::visitor::legacy_oracle::{
    count_advisory_callables, legacy_analyze_ast,
};

const FIXTURES: [(&str, &str); 5] = [
    (
        "SimpleToken.sol",
        include_str!("../../../tests/fixtures/solidity/simple/SimpleToken.sol"),
    ),
    (
        "ComplexVault.sol",
        include_str!("../../../tests/fixtures/solidity/complex/ComplexVault.sol"),
    ),
    (
        "InsecureBank.sol",
        include_str!("../../../tests/fixtures/solidity/security/InsecureBank.sol"),
    ),
    (
        "IERC20.sol",
        include_str!("../../../tests/fixtures/solidity/interfaces/IERC20.sol"),
    ),
    (
        "Token.t.sol",
        include_str!("../../../tests/fixtures/solidity/foundry/Token.t.sol"),
    ),
];

fn assert_analysis_equivalent(
    mut extracted: SolidityAnalysis,
    legacy: SolidityAnalysis,
    fixture: &str,
) {
    for (current, previous) in extracted.functions.iter_mut().zip(&legacy.functions) {
        normalize_entropy(current, previous);
    }
    assert_eq!(extracted, legacy, "analysis changed for {fixture}");
}

fn normalize_entropy(current: &mut SolidityFunction, previous: &SolidityFunction) {
    let (Some(current), Some(previous)) = (
        current.entropy_analysis.as_mut(),
        previous.entropy_analysis.as_ref(),
    ) else {
        return;
    };
    assert!((current.entropy_score - previous.entropy_score).abs() < 1e-12);
    assert!((current.pattern_repetition - previous.pattern_repetition).abs() < 1e-12);
    assert!((current.branch_similarity - previous.branch_similarity).abs() < 1e-12);
    assert!((current.dampening_factor - previous.dampening_factor).abs() < 1e-12);
    current.entropy_score = previous.entropy_score;
    current.pattern_repetition = previous.pattern_repetition;
    current.branch_similarity = previous.branch_similarity;
    current.dampening_factor = previous.dampening_factor;
}

#[test]
fn extracted_analysis_matches_legacy_visitor_for_solidity_fixtures() {
    let config = SolidityLanguageConfig::default();
    for (name, source) in FIXTURES {
        let ast = parse_source(source, Path::new(name)).unwrap();
        let extraction = extract_solidity(&ast);
        let legacy = legacy_analyze_ast(&ast, &config);
        let extracted = analyze_extracted(&ast, &config, &extraction);
        assert_analysis_equivalent(extracted, legacy, name);
    }
}

#[test]
fn extracted_debt_inputs_match_legacy_detectors() {
    let config = SolidityLanguageConfig::default();
    for (name, source) in FIXTURES {
        assert_debt_inputs_equivalent(name, source, &config);
    }
}

fn assert_debt_inputs_equivalent(name: &str, source: &str, config: &SolidityLanguageConfig) {
    let path = PathBuf::from(name);
    let ast = parse_source(source, &path).unwrap();
    let extraction = extract_solidity(&ast);
    let analysis = analyze_extracted(&ast, config, &extraction);
    let metrics = analysis
        .functions
        .iter()
        .map(to_function_metrics)
        .collect::<Vec<_>>();
    assert_eq!(
        detect_natspec_debt(&path, &ast, &metrics),
        detect_natspec_debt_with_signatures(
            &path,
            &ast.source,
            &metrics,
            &extraction.function_signatures,
        ),
        "NatSpec debt changed for {name}"
    );
    assert_contract_inputs(name, &ast, &extraction, config);
    let skip_debt = analysis.is_test_file;
    assert_eq!(
        detect_debt(&path, 10, &metrics, &ast, skip_debt, config),
        detect_debt_with_extraction(&path, 10, &metrics, &ast, &extraction, skip_debt, config,),
        "debt ordering changed for {name}"
    );
}

fn assert_contract_inputs(
    name: &str,
    ast: &SolidityAst,
    extraction: &SolidityExtraction<'_>,
    config: &SolidityLanguageConfig,
) {
    for contract in &extraction.contracts {
        let legacy = detect_contract_patterns(
            contract.node,
            &ast.source,
            count_advisory_callables(contract.node),
            config,
        );
        let extracted = detect_contract_patterns_with_counts(
            &ast.source,
            contract.advisory_function_count,
            contract.info.state_variable_count,
            config,
        );
        assert_eq!(legacy, extracted, "contract debt inputs changed for {name}");
    }
}
