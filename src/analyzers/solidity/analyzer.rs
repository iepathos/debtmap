//! Solidity analyzer implementation.

use crate::analyzers::Analyzer;
use crate::analyzers::solidity::orchestration::analyze_solidity_file;
use crate::analyzers::solidity::parser::parse_source;
use crate::config::SolidityLanguageConfig;
use crate::core::ast::Ast;
use crate::core::{ComplexityMetrics, FileMetrics, Language};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{debug, debug_span};

pub struct SolidityAnalyzer {
    complexity_threshold: u32,
    config: SolidityLanguageConfig,
}

impl SolidityAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
            config: crate::config::get_solidity_config(),
        }
    }

    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.complexity_threshold = threshold;
        self
    }

    pub fn with_config(mut self, config: SolidityLanguageConfig) -> Self {
        self.config = config;
        self
    }
}

impl Default for SolidityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SolidityAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let _span = debug_span!("parse_solidity_file", path = %path.display()).entered();
        let solidity_ast = parse_source(content, &path)?;

        debug!(
            path = %path.display(),
            bytes = content.len(),
            "Parsed Solidity file"
        );

        Ok(Ast::Solidity(solidity_ast))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Solidity(solidity_ast) => {
                analyze_solidity_file(solidity_ast, self.complexity_threshold, &self.config)
            }
            _ => empty_solidity_metrics(),
        }
    }

    fn language(&self) -> Language {
        Language::Solidity
    }
}

fn empty_solidity_metrics() -> FileMetrics {
    FileMetrics {
        path: PathBuf::new(),
        language: Language::Solidity,
        complexity: ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        total_lines: 0,
        module_scope: None,
        classes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::get_analyzer;

    const SIMPLE: &str = r#"pragma solidity ^0.8.0;

contract Token {
    function transfer(address to) public {
        require(to != address(0));
    }
}
"#;

    #[test]
    fn test_solidity_analyzer_language() {
        let analyzer = SolidityAnalyzer::new();
        assert_eq!(analyzer.language(), Language::Solidity);
    }

    #[test]
    fn test_solidity_analyzer_factory() {
        let analyzer = get_analyzer(Language::Solidity);
        assert_eq!(analyzer.language(), Language::Solidity);
    }

    #[test]
    fn test_analyze_simple_solidity_file() {
        let analyzer = SolidityAnalyzer::new();
        let ast = analyzer.parse(SIMPLE, PathBuf::from("Token.sol")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Solidity);
        assert!(!metrics.complexity.functions.is_empty());
    }

    #[test]
    fn test_foundry_test_file_skips_debt() {
        let analyzer = SolidityAnalyzer::new().with_threshold(1);
        let source = r#"pragma solidity ^0.8.0;
import "forge-std/Test.sol";

contract TokenTest is Test {
    function testTransfer() public {
        if (true) {
            revert("failed");
        }
    }
}
"#;
        let ast = analyzer
            .parse(source, PathBuf::from("Token.t.sol"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert!(metrics.complexity.functions[0].is_test);
        assert!(metrics.debt_items.is_empty());
    }

    #[test]
    fn test_tx_origin_pattern_detected() {
        let analyzer = SolidityAnalyzer::new();
        let source = r#"pragma solidity ^0.8.0;
contract Auth {
    function check() public view returns (bool) {
        return tx.origin == msg.sender;
    }
}
"#;
        let ast = analyzer.parse(source, PathBuf::from("Auth.sol")).unwrap();
        let metrics = analyzer.analyze(&ast);
        let function = &metrics.complexity.functions[0];

        assert!(
            function
                .detected_patterns
                .as_ref()
                .is_some_and(|patterns| patterns.contains(&"tx-origin-usage".to_string()))
        );
    }

    #[test]
    fn test_config_disables_tx_origin_pattern() {
        let mut config = SolidityLanguageConfig::default();
        config.security.tx_origin = false;
        let analyzer = SolidityAnalyzer::new().with_config(config);
        let source = r#"pragma solidity ^0.8.0;
contract Auth {
    function check() public view returns (bool) {
        return tx.origin == msg.sender;
    }
}
"#;
        let ast = analyzer.parse(source, PathBuf::from("Auth.sol")).unwrap();
        let metrics = analyzer.analyze(&ast);
        let function = &metrics.complexity.functions[0];

        assert!(
            !function
                .detected_patterns
                .as_ref()
                .is_some_and(|patterns| patterns.contains(&"tx-origin-usage".to_string()))
        );
        assert!(!metrics.debt_items.iter().any(|item| {
            matches!(
                &item.debt_type,
                crate::core::DebtType::CodeSmell { smell_type }
                    if smell_type.as_deref() == Some("tx-origin-usage")
            )
        }));
    }

    #[test]
    fn test_config_large_contract_threshold() {
        let config = SolidityLanguageConfig {
            large_contract_threshold: 1,
            ..Default::default()
        };
        let analyzer = SolidityAnalyzer::new().with_config(config);
        let source = r#"pragma solidity ^0.8.0;
contract ManyFunctions {
    function a() public {}
    function b() public {}
}
"#;
        let ast = analyzer
            .parse(source, PathBuf::from("ManyFunctions.sol"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert!(metrics.debt_items.iter().any(|item| {
            matches!(
                &item.debt_type,
                crate::core::DebtType::CodeSmell { smell_type }
                    if smell_type.as_deref() == Some("large-contract")
            )
        }));
    }

    #[test]
    fn test_solidity_line_suppression_filters_security_debt() {
        let analyzer = SolidityAnalyzer::new();
        let source = r#"pragma solidity ^0.8.0;
contract Auth {
    function check() public view returns (bool) { // debtmap:ignore[smell] -- reviewed legacy auth
        return tx.origin == msg.sender;
    }
}
"#;
        let ast = analyzer.parse(source, PathBuf::from("Auth.sol")).unwrap();
        let metrics = analyzer.analyze(&ast);

        assert!(!metrics.debt_items.iter().any(|item| {
            matches!(
                &item.debt_type,
                crate::core::DebtType::CodeSmell { smell_type }
                    if smell_type.as_deref() == Some("tx-origin-usage")
            )
        }));
    }

    #[test]
    fn test_solidity_function_suppression_filters_complexity_debt() {
        let analyzer = SolidityAnalyzer::new().with_threshold(1);
        let source = r#"pragma solidity ^0.8.0;
contract Branchy {
    // debtmap:ignore[complexity] -- compact state machine
    function decide(uint256 value) public returns (uint256) {
        if (value > 10) {
            return 1;
        }
        if (value > 5) {
            return 2;
        }
        return 3;
    }
}
"#;
        let ast = analyzer
            .parse(source, PathBuf::from("Branchy.sol"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert!(
            !metrics
                .debt_items
                .iter()
                .any(|item| matches!(item.debt_type, crate::core::DebtType::Complexity { .. }))
        );
    }
}
