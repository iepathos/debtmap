#[cfg(test)]
mod tests {
    use super::super::assertion_patterns::AssertionDetector;
    use super::super::{TestFramework, TestIssueType};
    use rustpython_parser::ast;

    fn parse_function(code: &str) -> ast::StmtFunctionDef {
        let full_code = format!(
            "def test_func():\n{}",
            code.lines()
                .map(|l| format!("    {}", l))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let module: ast::Mod =
            rustpython_parser::parse(&full_code, rustpython_parser::Mode::Module, "<test>")
                .expect("Failed to parse");

        if let ast::Mod::Module(ast::ModModule { body, .. }) = module {
            if let Some(ast::Stmt::FunctionDef(func)) = body.into_iter().next() {
                return func;
            }
        }
        panic!("Failed to extract function");
    }

    #[test]
    fn test_new_detector_with_framework() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        // Test is mainly for constructor
        let func = parse_function("pass");
        let issue = detector.analyze_test_function(&func);
        assert!(issue.is_none()); // Empty function has no setup/action
    }

    #[test]
    fn test_detect_missing_assertions_with_setup() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("x = 1\ny = 2\nresult = x + y");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_some());
        let issue = issue.unwrap();
        assert!(matches!(issue.issue_type, TestIssueType::NoAssertions));
    }

    #[test]
    fn test_detect_pytest_assert() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("x = 1\nassert x == 1");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion
    }

    #[test]
    fn test_detect_unittest_assertion() {
        let detector = AssertionDetector::new(TestFramework::Unittest);
        let func = parse_function("x = 1\nself.assertEqual(x, 1)");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion
    }

    #[test]
    fn test_detect_nose_assertion() {
        let detector = AssertionDetector::new(TestFramework::Nose);
        let func = parse_function("x = 1\nassert_equal(x, 1)");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion
    }

    #[test]
    fn test_unknown_framework_checks_all() {
        let detector = AssertionDetector::new(TestFramework::Unknown);

        // Test with pytest style
        let func = parse_function("x = 1\nassert x == 1");
        assert!(detector.analyze_test_function(&func).is_none());

        // Test with unittest style
        let func = parse_function("x = 1\nself.assertEqual(x, 1)");
        assert!(detector.analyze_test_function(&func).is_none());

        // Test with nose style
        let func = parse_function("x = 1\nassert_equal(x, 1)");
        assert!(detector.analyze_test_function(&func).is_none());
    }

    #[test]
    fn test_assertions_in_if_statement() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("x = 1\nif x > 0:\n    assert x == 1");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion in if block
    }

    #[test]
    fn test_assertions_in_for_loop() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("for i in range(3):\n    assert i >= 0");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion in loop
    }

    #[test]
    fn test_assertions_in_while_loop() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("i = 0\nwhile i < 3:\n    assert i >= 0\n    i += 1");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion in while loop
    }

    #[test]
    fn test_assertions_in_with_statement() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("with open('file') as f:\n    data = f.read()\n    assert data");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion in with block
    }

    #[test]
    fn test_assertions_in_try_block() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("try:\n    x = 1\n    assert x == 1\nexcept:\n    pass");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion in try block
    }

    #[test]
    fn test_assertions_in_except_block() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func =
            parse_function("try:\n    x = 1/0\nexcept ZeroDivisionError as e:\n    assert str(e)");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion in except block
    }

    #[test]
    fn test_pytest_raises_context_manager() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("with pytest.raises(ValueError):\n    raise ValueError('test')");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // pytest.raises is an assertion
    }

    #[test]
    fn test_pytest_warns_context_manager() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function(
            "with pytest.warns(UserWarning):\n    warnings.warn('test', UserWarning)",
        );
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // pytest.warns is an assertion
    }

    #[test]
    fn test_unittest_self_assertions() {
        let detector = AssertionDetector::new(TestFramework::Unittest);

        // Test various unittest assertions
        let assertions = vec![
            "self.assertEqual(1, 1)",
            "self.assertTrue(True)",
            "self.assertFalse(False)",
            "self.assertIsNone(None)",
            "self.assertIsNotNone(1)",
            "self.assertIn('a', 'abc')",
            "self.assertRaises(ValueError)",
        ];

        for assertion in assertions {
            let func = parse_function(&format!("x = 1\n{}", assertion));
            assert!(
                detector.analyze_test_function(&func).is_none(),
                "Failed for assertion: {}",
                assertion
            );
        }
    }

    #[test]
    fn test_nose_assert_functions() {
        let detector = AssertionDetector::new(TestFramework::Nose);

        let assertions = vec![
            "assert_equal(1, 1)",
            "assert_true(True)",
            "assert_false(False)",
            "assert_is_none(None)",
            "ok_(True)",
            "eq_(1, 1)",
        ];

        for assertion in assertions {
            let func = parse_function(&format!("x = 1\n{}", assertion));
            assert!(
                detector.analyze_test_function(&func).is_none(),
                "Failed for assertion: {}",
                assertion
            );
        }
    }

    #[test]
    fn test_doctest_framework() {
        let detector = AssertionDetector::new(TestFramework::Doctest);
        let func = parse_function("x = 1\ny = 2");
        let issue = detector.analyze_test_function(&func);

        // Doctest has setup code but no assertions, however since it's Doctest,
        // it should report the issue (because Doctest patterns are in comments, not code)
        assert!(issue.is_some());
    }

    #[test]
    fn test_empty_test_function() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("pass");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // No setup or action code
    }

    #[test]
    fn test_only_comments() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("# This is a comment\npass");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none());
    }

    #[test]
    fn test_setup_without_assertions() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("data = {'key': 'value'}\nresult = process(data)");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_some());
        if let Some(issue) = issue {
            assert!(matches!(issue.issue_type, TestIssueType::NoAssertions));
        }
    }

    #[test]
    fn test_action_without_assertions() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("do_something()\nprocess_data()");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_some());
        if let Some(issue) = issue {
            assert!(matches!(issue.issue_type, TestIssueType::NoAssertions));
        }
    }

    #[test]
    fn test_suggestion_for_unittest() {
        let detector = AssertionDetector::new(TestFramework::Unittest);
        let func = parse_function("result = calculate()");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_some());
        if let Some(issue) = issue {
            assert!(issue.suggestion.contains("self.assert"));
        }
    }

    #[test]
    fn test_suggestion_for_pytest() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function("result = calculate()");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_some());
        if let Some(issue) = issue {
            assert!(issue.suggestion.contains("assert"));
        }
    }

    #[test]
    fn test_suggestion_for_nose() {
        let detector = AssertionDetector::new(TestFramework::Nose);
        let func = parse_function("result = calculate()");
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_some());
        if let Some(issue) = issue {
            assert!(issue.suggestion.contains("assert_"));
        }
    }

    #[test]
    fn test_complex_nested_assertions() {
        let detector = AssertionDetector::new(TestFramework::Pytest);
        let func = parse_function(
            r#"
data = prepare_data()
if data:
    for item in data:
        try:
            result = process(item)
            if result:
                assert result > 0
        except Exception:
            pass
"#,
        );
        let issue = detector.analyze_test_function(&func);

        assert!(issue.is_none()); // Has assertion deep in nesting
    }
}
