//! Tree-sitter parser integration for Solidity.

use crate::core::ast::SolidityAst;
use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language as TsLanguage, Parser, Tree};

fn get_language() -> TsLanguage {
    tree_sitter_solidity::LANGUAGE.into()
}

pub fn parse_source(content: &str, path: &Path) -> Result<SolidityAst> {
    let mut parser = Parser::new();
    let language = get_language();

    parser
        .set_language(&language)
        .context("Failed to set tree-sitter Solidity language")?;

    let tree = parser
        .parse(content, None)
        .context("Failed to parse Solidity source")?;

    if has_parse_errors(&tree) {
        anyhow::bail!("Solidity parse tree contains syntax errors");
    }

    Ok(SolidityAst {
        tree,
        path: path.to_path_buf(),
        source: content.to_string(),
    })
}

pub fn has_parse_errors(tree: &Tree) -> bool {
    tree.root_node().has_error()
}

pub fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    &source[start..end]
}

pub fn node_line(node: &tree_sitter::Node) -> usize {
    node.start_position().row + 1
}

pub fn node_column(node: &tree_sitter::Node) -> usize {
    node.start_position().column + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const SIMPLE_CONTRACT: &str = r#"// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Token {
    function transfer(address to) public {}
}
"#;

    const INTERFACE: &str = r#"pragma solidity ^0.8.0;

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
}
"#;

    const LIBRARY: &str = r#"pragma solidity ^0.8.0;

library SafeMath {
    function add(uint256 a, uint256 b) internal pure returns (uint256) {
        return a + b;
    }
}
"#;

    const WITH_IMPORT: &str = r#"pragma solidity ^0.8.0;
import "./Token.sol";

contract Vault is Token {
    modifier onlyOwner() {
        _;
    }

    function deposit() public onlyOwner {}
}
"#;

    #[test]
    fn test_parse_simple_contract() {
        let ast = parse_source(SIMPLE_CONTRACT, &PathBuf::from("Token.sol")).unwrap();
        assert!(!has_parse_errors(&ast.tree));
        assert_eq!(ast.tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_interface() {
        let ast = parse_source(INTERFACE, &PathBuf::from("IERC20.sol")).unwrap();
        assert!(!has_parse_errors(&ast.tree));
    }

    #[test]
    fn test_parse_library() {
        let ast = parse_source(LIBRARY, &PathBuf::from("SafeMath.sol")).unwrap();
        assert!(!has_parse_errors(&ast.tree));
    }

    #[test]
    fn test_parse_import_and_modifier() {
        let ast = parse_source(WITH_IMPORT, &PathBuf::from("Vault.sol")).unwrap();
        assert!(!has_parse_errors(&ast.tree));
    }

    #[test]
    fn test_parse_errors_on_invalid_source() {
        let result = parse_source("contract { broken", &PathBuf::from("Bad.sol"));
        assert!(result.is_err());
    }
}
