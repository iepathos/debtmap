#[cfg(test)]
mod tests {
    use super::super::complexity_analyzer::TestComplexityAnalyzer;
    use super::super::{TestIssueType, Severity};
    use rustpython_parser::ast;

    fn parse_function(code: &str) -> ast::StmtFunctionDef {
        let full_code = format!("def test_func():\n{}", 
            code.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n"));
        let module: ast::Mod = rustpython_parser::parse(&full_code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse")
            .into();
        
        if let ast::Mod::Module(ast::ModModule { body, .. }) = module {
            if let Some(ast::Stmt::FunctionDef(func)) = body.into_iter().next() {
                return func;
            }
        }
        panic!("Failed to extract function");
    }

    #[test]
    fn test_new_analyzer_default_threshold() {
        let analyzer = TestComplexityAnalyzer::new();
        let func = parse_function("pass");
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_none()); // Simple function below default threshold
    }

    #[test]
    fn test_with_threshold_custom_value() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
if True:
    if True:
        if True:
            pass
"#);
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_some()); // Should exceed threshold of 5
    }

    #[test]
    fn test_simple_function_no_complexity() {
        let analyzer = TestComplexityAnalyzer::new();
        let func = parse_function("x = 1\ny = 2\nassert x + y == 3");
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_none());
    }

    #[test]
    fn test_single_conditional_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(2);
        let func = parse_function(r#"
if condition:
    assert True
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base complexity 1 + conditional 2 = 3, exceeds threshold of 2
        assert!(issue.is_some());
    }

    #[test]
    fn test_multiple_conditionals_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
if a:
    pass
if b:
    pass
if c:
    pass
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base 1 + 3 conditionals * 2 = 7, exceeds threshold of 5
        assert!(issue.is_some());
    }

    #[test]
    fn test_nested_conditionals_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(8);
        let func = parse_function(r#"
if a:
    if b:
        if c:
            assert True
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base 1 + 3 conditionals * 2 + nesting penalty = should exceed
        assert!(issue.is_some());
    }

    #[test]
    fn test_single_loop_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(3);
        let func = parse_function(r#"
for i in range(10):
    assert i >= 0
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base 1 + loop 3 = 4, exceeds threshold of 3
        assert!(issue.is_some());
    }

    #[test]
    fn test_nested_loops_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(7);
        let func = parse_function(r#"
for i in range(10):
    for j in range(10):
        assert i * j >= 0
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base 1 + 2 loops * 3 = 7, plus nesting penalty
        assert!(issue.is_some());
    }

    #[test]
    fn test_while_loop_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(3);
        let func = parse_function(r#"
i = 0
while i < 10:
    assert i >= 0
    i += 1
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base 1 + while loop 3 = 4, exceeds threshold of 3
        assert!(issue.is_some());
    }

    #[test]
    fn test_many_assertions_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
assert a == 1
assert b == 2
assert c == 3
assert d == 4
assert e == 5
assert f == 6
assert g == 7
assert h == 8
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base 1 + (8 assertions - 5) * 1 = 4, below threshold of 5
        assert!(issue.is_none());
    }

    #[test]
    fn test_mock_decorators_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
with patch('module1'):
    with patch('module2'):
        with patch('module3'):
            assert True
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Base 1 + 3 mocks * 2 + nesting penalty
        assert!(issue.is_some());
    }

    #[test]
    fn test_try_except_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
try:
    risky_operation()
    assert True
except ValueError:
    assert False
except KeyError:
    assert False
finally:
    cleanup()
"#);
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_some());
    }

    #[test]
    fn test_deep_nesting_penalty() {
        let analyzer = TestComplexityAnalyzer::with_threshold(10);
        let func = parse_function(r#"
if a:
    if b:
        if c:
            if d:
                assert True
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Deep nesting should add penalty
        assert!(issue.is_some());
        if let Some(issue) = issue {
            if let TestIssueType::OverlyComplex(score) = issue.issue_type {
                assert!(score > 10);
            }
        }
    }

    #[test]
    fn test_long_function_penalty() {
        let lines: Vec<String> = (0..25).map(|i| format!("var_{} = {}", i, i)).collect();
        let code = lines.join("\n");
        let analyzer = TestComplexityAnalyzer::with_threshold(3);
        let func = parse_function(&code);
        let issue = analyzer.analyze_test_function(&func);
        // Should add penalty for >20 lines
        assert!(issue.is_some());
    }

    #[test]
    fn test_severity_levels() {
        let analyzer = TestComplexityAnalyzer::with_threshold(10);
        
        // Medium severity (just over threshold)
        let func = parse_function(r#"
if a:
    pass
if b:
    pass
if c:
    pass
if d:
    pass
if e:
    pass
if f:
    pass
"#);
        if let Some(issue) = analyzer.analyze_test_function(&func) {
            assert_eq!(issue.severity, Severity::Medium);
        }
        
        // Create very complex function for high/critical severity
        let complex_code = r#"
for i in range(10):
    for j in range(10):
        if i > 5:
            if j > 5:
                for k in range(10):
                    if k > 5:
                        assert True
"#;
        let func = parse_function(complex_code);
        if let Some(issue) = analyzer.analyze_test_function(&func) {
            assert!(matches!(issue.severity, Severity::High | Severity::Critical));
        }
    }

    #[test]
    fn test_suggestions_for_loops() {
        let analyzer = TestComplexityAnalyzer::with_threshold(3);
        let func = parse_function(r#"
for i in range(10):
    assert i >= 0
"#);
        if let Some(issue) = analyzer.analyze_test_function(&func) {
            assert!(issue.suggestion.contains("parametrized"));
        }
    }

    #[test]
    fn test_suggestions_for_many_assertions() {
        let analyzer = TestComplexityAnalyzer::with_threshold(1);
        let func = parse_function(r#"
assert a == 1
assert b == 2
assert c == 3
assert d == 4
assert e == 5
assert f == 6
assert g == 7
"#);
        if let Some(issue) = analyzer.analyze_test_function(&func) {
            assert!(issue.suggestion.contains("multiple focused test"));
        }
    }

    #[test]
    fn test_suggestions_for_mocking() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
with patch('mod1'):
    with patch('mod2'):
        with patch('mod3'):
            with patch('mod4'):
                assert True
"#);
        if let Some(issue) = analyzer.analyze_test_function(&func) {
            assert!(issue.suggestion.contains("fixtures"));
        }
    }

    #[test]
    fn test_suggestions_for_deep_nesting() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
if a:
    if b:
        if c:
            assert True
"#);
        if let Some(issue) = analyzer.analyze_test_function(&func) {
            assert!(issue.suggestion.contains("helper functions"));
        }
    }

    #[test]
    fn test_suggestions_for_long_tests() {
        let lines: Vec<String> = (0..25).map(|i| format!("var_{} = {}", i, i)).collect();
        let code = lines.join("\n");
        let analyzer = TestComplexityAnalyzer::with_threshold(3);
        let func = parse_function(&code);
        
        if let Some(issue) = analyzer.analyze_test_function(&func) {
            assert!(issue.suggestion.contains("smaller"));
        }
    }

    #[test]
    fn test_combined_complexity_factors() {
        let analyzer = TestComplexityAnalyzer::with_threshold(10);
        let func = parse_function(r#"
with patch('module'):
    for i in range(5):
        if i > 0:
            try:
                result = process(i)
                assert result > 0
                if result > 10:
                    assert result < 100
            except ValueError:
                assert False
"#);
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_some());
        if let Some(issue) = issue {
            if let TestIssueType::OverlyComplex(score) = issue.issue_type {
                assert!(score > 10);
            }
        }
    }

    #[test]
    fn test_empty_function_minimal_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(1);
        let func = parse_function("pass");
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_none()); // Base complexity of 1 should not exceed threshold of 1
    }

    #[test]
    fn test_with_statement_nesting() {
        let analyzer = TestComplexityAnalyzer::with_threshold(7);
        let func = parse_function(r#"
with open('file1'):
    with open('file2'):
        with open('file3'):
            data = read_data()
            assert data
"#);
        let issue = analyzer.analyze_test_function(&func);
        // Should count nesting depth
        assert!(issue.is_some());
    }

    #[test]
    fn test_for_else_complexity() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
for i in range(10):
    if i == 5:
        break
else:
    assert False
"#);
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_some());
    }

    #[test]
    fn test_complex_boolean_expressions() {
        let analyzer = TestComplexityAnalyzer::with_threshold(5);
        let func = parse_function(r#"
if a and b or c and not d:
    if e or f and g:
        assert True
"#);
        let issue = analyzer.analyze_test_function(&func);
        assert!(issue.is_some());
    }
}