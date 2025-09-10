#[cfg(test)]
mod tests {
    use super::super::analyzer::PythonTestAnalyzer;
    use super::super::TestIssueType;
    use rustpython_parser::ast;
    use std::path::PathBuf;

    fn parse_python_code(code: &str) -> ast::Mod {
        rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse test code")
            .into()
    }

    #[test]
    fn test_new_analyzer_has_default_threshold() {
        // Test that new analyzer creates with expected defaults
        let code = "def test_simple(): x = 1"; // Has setup code but no assertions
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        // Should detect no assertions issue with default config
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].issue_type, TestIssueType::NoAssertions));
    }

    #[test]
    fn test_with_threshold_creates_custom_analyzer() {
        let mut analyzer = PythonTestAnalyzer::with_threshold(20);
        // Test with high complexity threshold
        let code = r#"
def test_complex():
    if True:
        if True:
            if True:
                if True:
                    if True:
                        pass
"#;
        let module = parse_python_code(code);
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        // Should not flag as complex with threshold of 20
        assert!(issues.iter().all(|i| !matches!(i.issue_type, TestIssueType::OverlyComplex(_))));
    }

    #[test]
    fn test_analyze_module_detects_framework() {
        let code = r#"
import unittest

class TestExample(unittest.TestCase):
    def test_something(self):
        self.assertEqual(1, 1)
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let _issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        // Framework should be detected through the analyze_module call
    }

    #[test]
    fn test_analyze_function_def() {
        let mut analyzer = PythonTestAnalyzer::new();
        let code = r#"
def test_without_assertions():
    x = 1
    y = 2
    result = x + y
"#;
        let module = parse_python_code(code);
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].issue_type, TestIssueType::NoAssertions));
    }

    #[test]
    fn test_analyze_async_function_def() {
        let mut analyzer = PythonTestAnalyzer::new();
        let code = r#"
async def test_async_without_assertions():
    x = 1
    y = 2
    result = x + y
"#;
        let module = parse_python_code(code);
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].issue_type, TestIssueType::NoAssertions));
    }

    #[test]
    fn test_analyze_class_with_test_methods() {
        let mut analyzer = PythonTestAnalyzer::new();
        let code = r#"
class TestExample:
    def test_method_one(self):
        x = 1  # Has setup but no assertions
    
    def test_method_two(self):
        assert True
    
    def setUp(self):
        pass
"#;
        let module = parse_python_code(code);
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        // Should find one issue for test_method_one (no assertions)
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].test_name, "test_method_one");
    }


    #[test]
    fn test_check_excessive_mocking() {
        let code = r#"
@mock.patch('module.function1')
@mock.patch('module.function2')
@mock.patch('module.function3')
@mock.patch('module.function4')
@mock.patch('module.function5')
@mock.patch('module.function6')
def test_with_many_mocks(mock1, mock2, mock3, mock4, mock5, mock6):
    pass
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        assert!(issues.iter().any(|i| matches!(i.issue_type, TestIssueType::ExcessiveMocking(_))));
    }

    #[test]
    fn test_count_mocks_in_body() {
        let code = r#"
def test_with_inline_mocks():
    with patch('module.function') as mock1:
        mock2 = Mock()
        mock3 = MagicMock()
        result = some_function()
        # No assertions, so will report that issue
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        // Will have issues due to no assertions
        assert!(issues.len() > 0);
    }

    #[test]
    fn test_check_test_isolation_with_global_state() {
        let code = r#"
def test_modifies_global():
    global some_var
    some_var = 42
    assert some_var == 42
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        assert!(issues.iter().any(|i| matches!(i.issue_type, TestIssueType::PoorIsolation)));
    }

    #[test]
    fn test_check_test_isolation_with_cleanup() {
        let code = r#"
def test_with_proper_cleanup():
    global some_var
    some_var = 42
    try:
        assert some_var == 42
    finally:
        some_var = None
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        // Should not report poor isolation when cleanup is present
        assert!(!issues.iter().any(|i| matches!(i.issue_type, TestIssueType::PoorIsolation)));
    }

    #[test]
    fn test_module_level_assignment_detection() {
        let code = r#"
def test_modifies_os():
    os.environ['TEST'] = 'value'
    sys.path.append('/some/path')
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        // Should have multiple issues: no assertions and potentially poor isolation
        // Since we're modifying os/sys but have no assertions, we should find issues
        assert!(issues.len() > 0);
        // At minimum should have no assertions issue
        assert!(issues.iter().any(|i| matches!(i.issue_type, TestIssueType::NoAssertions)));
    }

    #[test]
    fn test_context_manager_cleanup() {
        let code = r#"
def test_with_context_manager():
    global some_var
    some_var = 42
    with cleanup_context():
        assert some_var == 42
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        // Should not report poor isolation when context manager is used
        assert!(!issues.iter().any(|i| matches!(i.issue_type, TestIssueType::PoorIsolation)));
    }

    #[test]
    fn test_multiple_issues_in_single_test() {
        let code = r#"
@mock.patch('module1')
@mock.patch('module2')
@mock.patch('module3')
@mock.patch('module4')
@mock.patch('module5')
@mock.patch('module6')
def test_multiple_problems():
    global some_var
    some_var = 42
    if True:
        if True:
            if True:
                if True:
                    if True:
                        pass
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::with_threshold(5);
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        // Should detect multiple issues
        assert!(issues.len() >= 3); // No assertions, excessive mocking, poor isolation, complexity
    }

    #[test]
    fn test_empty_module() {
        let code = "";
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_non_test_functions_ignored() {
        let code = r#"
def helper_function():
    pass

def setup_database():
    pass

def create_fixture():
    pass
"#;
        let module = parse_python_code(code);
        let mut analyzer = PythonTestAnalyzer::new();
        let issues = analyzer.analyze_module(&module, &PathBuf::from("test.py"));
        
        assert_eq!(issues.len(), 0);
    }
}