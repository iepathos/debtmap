#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;
    use crate::complexity::entropy_core::{EntropyToken, TokenCategory};
    use rustpython_parser::{ast, Parse};

    fn parse_python_expr(code: &str) -> ast::Expr {
        let full_code = format!("x = {}", code);
        let parsed = ast::Suite::parse(&full_code, "<test>").unwrap();
        match &parsed[0] {
            ast::Stmt::Assign(assign) => assign.value.as_ref().clone(),
            _ => panic!("Expected assignment statement"),
        }
    }

    #[test]
    fn test_extract_bool_op_tokens() {
        let analyzer = PythonEntropyAnalyzer::new("");
        let expr = parse_python_expr("True and False or True");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have boolean operators and literals
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Operator)));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Literal)));
    }

    #[test]
    fn test_extract_bin_op_tokens() {
        let analyzer = PythonEntropyAnalyzer::new("");
        let expr = parse_python_expr("5 + 3 * 2");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have binary operators and number literals
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Operator)));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Literal)));
    }

    #[test]
    fn test_extract_unary_op_tokens() {
        let analyzer = PythonEntropyAnalyzer::new("");
        let expr = parse_python_expr("-x");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have unary operator
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Operator)));
    }

    #[test]
    fn test_extract_lambda_tokens() {
        let analyzer = PythonEntropyAnalyzer::new("");
        let expr = parse_python_expr("lambda x: x * 2");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have lambda keyword
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Keyword)));
    }

    #[test]
    fn test_extract_if_exp_tokens() {
        let analyzer = PythonEntropyAnalyzer::new("");
        let expr = parse_python_expr("5 if True else 3");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have control flow token
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::ControlFlow)));
    }

    #[test]
    fn test_extract_list_comp_tokens() {
        let analyzer = PythonEntropyAnalyzer::new("");
        let expr = parse_python_expr("[x for x in range(10)]");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&expr, &mut tokens);

        // Should have extracted tokens from the comprehension
        assert!(!tokens.is_empty());
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Operator)));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.to_category(), TokenCategory::Literal)));
    }

    #[test]
    fn test_operator_token_extraction() {
        let analyzer = PythonEntropyAnalyzer::new("");

        // Test each operator type (using operator names to match implementation)
        let test_cases = vec![
            ("True and False", "and"),
            ("1 + 2", "Add"),
            ("not True", "not"),
            ("x == y", "=="),
        ];

        for (expr_str, expected_op) in test_cases {
            let expr = parse_python_expr(expr_str);
            let mut tokens = Vec::new();
            analyzer.extract_tokens_from_expr(&expr, &mut tokens);

            assert!(
                !tokens.is_empty(),
                "Failed to extract tokens for {}",
                expr_str
            );

            // Check if any token contains the expected operator
            let has_operator = tokens.iter().any(|t| {
                let value = t.value();
                value.contains(expected_op) || value == expected_op
            });

            assert!(
                has_operator,
                "Expected operator '{}' not found in tokens for {}. Found tokens: {:?}",
                expected_op,
                expr_str,
                tokens.iter().map(|t| t.value()).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_control_flow_token_extraction() {
        let analyzer = PythonEntropyAnalyzer::new("");

        // Test control flow expressions
        let lambda = parse_python_expr("lambda x: x + 1");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&lambda, &mut tokens);
        assert!(tokens.iter().any(|t| t.value() == "lambda"));

        let if_exp = parse_python_expr("1 if True else 2");
        tokens.clear();
        analyzer.extract_tokens_from_expr(&if_exp, &mut tokens);
        assert!(tokens.iter().any(|t| t.value() == "if"));
    }

    #[test]
    fn test_comprehension_token_extraction() {
        let analyzer = PythonEntropyAnalyzer::new("");

        // Test comprehension expressions
        let list_comp = parse_python_expr("[x for x in range(10)]");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&list_comp, &mut tokens);
        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.value() == "list_comp"));
    }

    #[test]
    fn test_collection_token_extraction() {
        let analyzer = PythonEntropyAnalyzer::new("");

        // Test collection expressions
        let list = parse_python_expr("[1, 2, 3]");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&list, &mut tokens);
        assert!(tokens.iter().any(|t| t.value() == "list"));

        let dict = parse_python_expr("{'a': 1}");
        tokens.clear();
        analyzer.extract_tokens_from_expr(&dict, &mut tokens);
        assert!(tokens.iter().any(|t| t.value() == "dict"));
    }

    #[test]
    fn test_edge_cases_and_complex_expressions() {
        let analyzer = PythonEntropyAnalyzer::new("");

        // Test deeply nested expression
        let nested = parse_python_expr("{'key': [x * 2 for x in (1, 2, 3) if x > 1]}");
        let mut tokens = Vec::new();
        analyzer.extract_tokens_from_expr(&nested, &mut tokens);

        // Should handle nested structures correctly
        assert!(tokens.iter().any(|t| t.value() == "dict"));
        assert!(tokens.iter().any(|t| t.value() == "list_comp"));
        assert!(tokens.iter().any(|t| t.value() == "tuple"));
    }
}
