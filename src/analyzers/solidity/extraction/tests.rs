use std::path::Path;

use crate::analyzers::solidity::dependencies::extract_dependencies;
use crate::analyzers::solidity::parser::{node_line, node_text, parse_source};
use crate::core::{Dependency, DependencyKind};

use super::extract_solidity;

const KITCHEN_SINK: &str = r#"pragma solidity ^0.8.20;
import "./Base.sol";

contract Base {
    uint256 internal value;

    modifier onlyReady() {
        _;
    }

    function read(uint256 amount) public view returns (uint256 doubled) {
        return value + amount;
    }
}

contract Vault is Base {
    mapping(address => uint256) private balances;

    constructor() {
        value = 1;
    }

    receive() external payable {}

    function deposit(uint256 amount) external onlyReady {
        balances[msg.sender] += amount;
    }
}
"#;

fn dependency_keys(dependencies: &[Dependency]) -> Vec<(String, DependencyKind)> {
    dependencies
        .iter()
        .map(|dependency| (dependency.name.clone(), dependency.kind.clone()))
        .collect()
}

#[test]
fn extracts_structural_inputs_in_one_index() {
    let ast = parse_source(KITCHEN_SINK, Path::new("contracts/Vault.sol")).unwrap();
    let extraction = extract_solidity(&ast);

    assert_eq!(extraction.contracts.len(), 2);
    assert_eq!(extraction.contracts[0].info.name, "Base");
    assert_eq!(extraction.contracts[0].info.state_variables, ["value"]);
    assert_eq!(extraction.contracts[0].info.function_count, 2);
    assert_eq!(extraction.contracts[1].info.name, "Vault");
    assert!(extraction.contracts[1].info.base_classes.is_empty());
    assert!(extraction.dependencies.iter().any(|dependency| {
        dependency.kind == DependencyKind::Inheritance && dependency.name == "Base"
    }));
    assert_eq!(extraction.contracts[1].info.state_variables, ["balances"]);
    assert_eq!(extraction.contracts[1].info.function_count, 4);
    assert_eq!(extraction.contracts[1].advisory_function_count, 3);
    assert_eq!(extraction.callables.len(), 6);
    assert!(extraction.modifier_bodies.contains_key("onlyReady"));
}

#[test]
fn extracts_natspec_signature_inputs() {
    let ast = parse_source(KITCHEN_SINK, Path::new("Vault.sol")).unwrap();
    let extraction = extract_solidity(&ast);
    let read = extraction
        .callables
        .iter()
        .find(|callable| node_text(&callable.node, &ast.source).contains("function read"))
        .unwrap();
    let signature = extraction
        .function_signatures
        .get(&node_line(&read.node))
        .unwrap();

    assert_eq!(signature.params, ["amount"]);
    assert_eq!(signature.return_slots, 1);
}

#[test]
fn extracted_dependencies_match_legacy_extractor() {
    let ast = parse_source(KITCHEN_SINK, Path::new("contracts/Vault.sol")).unwrap();
    let extraction = extract_solidity(&ast);
    let legacy = extract_dependencies(&ast);

    assert_eq!(
        dependency_keys(&extraction.dependencies),
        dependency_keys(&legacy)
    );
}
