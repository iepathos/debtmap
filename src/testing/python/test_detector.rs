//! Comprehensive test function detection for Python
//!
//! This module provides comprehensive test detection that goes beyond simple name matching.
//! It recognizes test patterns from pytest, unittest, nose, and doctest frameworks.

use rustpython_parser::ast;
use std::path::Path;

/// Result of test detection with detailed information
#[derive(Debug, Clone, PartialEq)]
pub struct TestDetectionResult {
    pub is_test: bool,
    pub test_type: Option<TestType>,
    pub framework: Option<super::TestFramework>,
    pub confidence: f32,
}

/// Types of test-related functions
#[derive(Debug, Clone, PartialEq)]
pub enum TestType {
    /// Standard test function
    TestFunction,
    /// Test method in a test class
    TestMethod,
    /// Fixture/setup function
    Fixture,
    /// Helper function in test context
    Helper,
    /// Setup function (setUp, setUpClass, etc.)
    Setup,
    /// Teardown function (tearDown, tearDownClass, etc.)
    Teardown,
    /// Parameterized test
    Parameterized,
}

/// Context for test detection
#[derive(Debug, Clone)]
pub struct TestContext {
    pub in_test_class: bool,
    pub class_name: Option<String>,
    pub is_test_file: bool,
}

impl TestContext {
    pub fn new() -> Self {
        Self {
            in_test_class: false,
            class_name: None,
            is_test_file: false,
        }
    }

    pub fn with_test_file(mut self, is_test_file: bool) -> Self {
        self.is_test_file = is_test_file;
        self
    }

    pub fn with_class(mut self, class_name: String, is_test_class: bool) -> Self {
        self.class_name = Some(class_name);
        self.in_test_class = is_test_class;
        self
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Main test detector
pub struct PythonTestDetector;

impl PythonTestDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect if a function is a test
    pub fn detect_test(
        &self,
        func: &ast::StmtFunctionDef,
        _context: &TestContext,
    ) -> TestDetectionResult {
        let context = _context;
        // Check decorators first (highest priority)
        if let Some(result) = self.check_decorators(&func.decorator_list, context) {
            return result;
        }

        // Check function name patterns
        if let Some(result) = self.check_function_name(&func.name, context) {
            return result;
        }

        // Check docstring for doctests
        if self.has_doctest(&func.body) {
            return TestDetectionResult {
                is_test: true,
                test_type: Some(TestType::TestFunction),
                framework: Some(super::TestFramework::Doctest),
                confidence: 0.9,
            };
        }

        // Not a test
        TestDetectionResult {
            is_test: false,
            test_type: None,
            framework: None,
            confidence: 1.0,
        }
    }

    /// Detect if an async function is a test
    pub fn detect_async_test(
        &self,
        func: &ast::StmtAsyncFunctionDef,
        _context: &TestContext,
    ) -> TestDetectionResult {
        let context = _context;
        // Check decorators first
        if let Some(result) = self.check_decorators(&func.decorator_list, context) {
            return result;
        }

        // Check function name patterns
        if let Some(result) = self.check_function_name(&func.name, context) {
            return result;
        }

        // Check docstring for doctests
        if self.has_doctest(&func.body) {
            return TestDetectionResult {
                is_test: true,
                test_type: Some(TestType::TestFunction),
                framework: Some(super::TestFramework::Doctest),
                confidence: 0.9,
            };
        }

        // Not a test
        TestDetectionResult {
            is_test: false,
            test_type: None,
            framework: None,
            confidence: 1.0,
        }
    }

    /// Check if a file path indicates a test file
    pub fn is_test_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check file name patterns
        if file_name.starts_with("test_") || file_name.ends_with("_test.py") {
            return true;
        }

        // Check directory patterns
        if let Some(parent) = path.parent() {
            let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if parent_name == "tests" || parent_name == "test" {
                return true;
            }
        }

        // Special pytest files
        if file_name == "conftest.py" {
            return true;
        }

        false
    }

    /// Check decorators for test markers
    fn check_decorators(
        &self,
        decorators: &[ast::Expr],
        _context: &TestContext,
    ) -> Option<TestDetectionResult> {
        for decorator in decorators {
            // Check for pytest decorators
            if let Some(name) = Self::extract_decorator_name(decorator) {
                match name.as_str() {
                    // Pytest fixtures
                    "fixture" | "pytest.fixture" => {
                        return Some(TestDetectionResult {
                            is_test: true,
                            test_type: Some(TestType::Fixture),
                            framework: Some(super::TestFramework::Pytest),
                            confidence: 1.0,
                        });
                    }
                    // Pytest marks
                    "parametrize" | "pytest.mark.parametrize" => {
                        return Some(TestDetectionResult {
                            is_test: true,
                            test_type: Some(TestType::Parameterized),
                            framework: Some(super::TestFramework::Pytest),
                            confidence: 1.0,
                        });
                    }
                    // Other pytest marks (skip, xfail, etc.)
                    s if s.starts_with("pytest.mark.") => {
                        return Some(TestDetectionResult {
                            is_test: true,
                            test_type: Some(TestType::TestFunction),
                            framework: Some(super::TestFramework::Pytest),
                            confidence: 0.95,
                        });
                    }
                    // Unittest skip decorators
                    "skip"
                    | "skipIf"
                    | "skipUnless"
                    | "unittest.skip"
                    | "unittest.skipIf"
                    | "unittest.skipUnless" => {
                        return Some(TestDetectionResult {
                            is_test: true,
                            test_type: Some(TestType::TestFunction),
                            framework: Some(super::TestFramework::Unittest),
                            confidence: 0.95,
                        });
                    }
                    // Nose decorators
                    "with_setup" | "nose.tools.with_setup" => {
                        return Some(TestDetectionResult {
                            is_test: true,
                            test_type: Some(TestType::TestFunction),
                            framework: Some(super::TestFramework::Nose),
                            confidence: 0.95,
                        });
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// Check function name for test patterns
    fn check_function_name(
        &self,
        name: &ast::Identifier,
        context: &TestContext,
    ) -> Option<TestDetectionResult> {
        let name_str = name.as_str();

        // Standard test function pattern
        if name_str.starts_with("test_") {
            let test_type = if context.in_test_class {
                TestType::TestMethod
            } else {
                TestType::TestFunction
            };

            return Some(TestDetectionResult {
                is_test: true,
                test_type: Some(test_type),
                framework: Some(super::TestFramework::Pytest),
                confidence: 0.9,
            });
        }

        // Unittest setup/teardown methods
        if context.in_test_class {
            match name_str {
                "setUp" | "tearDown" => {
                    return Some(TestDetectionResult {
                        is_test: true,
                        test_type: if name_str == "setUp" {
                            Some(TestType::Setup)
                        } else {
                            Some(TestType::Teardown)
                        },
                        framework: Some(super::TestFramework::Unittest),
                        confidence: 1.0,
                    });
                }
                "setUpClass" | "tearDownClass" | "setUpModule" | "tearDownModule" => {
                    return Some(TestDetectionResult {
                        is_test: true,
                        test_type: if name_str.starts_with("setUp") {
                            Some(TestType::Setup)
                        } else {
                            Some(TestType::Teardown)
                        },
                        framework: Some(super::TestFramework::Unittest),
                        confidence: 1.0,
                    });
                }
                _ => {}
            }
        }

        None
    }

    /// Check if a function has doctests in its docstring
    fn has_doctest(&self, body: &[ast::Stmt]) -> bool {
        // Check first statement for docstring
        if let Some(ast::Stmt::Expr(expr)) = body.first() {
            if let ast::Expr::Constant(constant) = &*expr.value {
                // Check if docstring contains ">>>" (doctest marker)
                if let ast::Constant::Str(s) = &constant.value {
                    return s.contains(">>>");
                }
            }
        }
        false
    }

    /// Extract decorator name from expression
    fn extract_decorator_name(expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => {
                // Handle chained attributes like pytest.mark.skip
                let mut parts = vec![attr.attr.to_string()];
                let mut current = &*attr.value;

                loop {
                    match current {
                        ast::Expr::Attribute(inner_attr) => {
                            parts.push(inner_attr.attr.to_string());
                            current = &*inner_attr.value;
                        }
                        ast::Expr::Name(name) => {
                            parts.push(name.id.to_string());
                            break;
                        }
                        _ => break,
                    }
                }

                parts.reverse();
                Some(parts.join("."))
            }
            ast::Expr::Call(call) => {
                // Handle decorator calls like @pytest.mark.parametrize(...)
                Self::extract_decorator_name(&call.func)
            }
            _ => None,
        }
    }
}

impl Default for PythonTestDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::python::TestFramework;

    fn parse_function(code: &str) -> ast::StmtFunctionDef {
        let module = rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse");

        if let ast::Mod::Module(ast::ModModule { body, .. }) = module {
            if let Some(ast::Stmt::FunctionDef(func)) = body.first() {
                return func.clone();
            }
        }
        panic!("Expected function definition");
    }

    #[test]
    fn test_detect_test_function_by_name() {
        let code = r#"
def test_example():
    assert True
"#;
        let func = parse_function(code);
        let detector = PythonTestDetector::new();
        let context = TestContext::new();

        let result = detector.detect_test(&func, &context);
        assert!(result.is_test);
        assert_eq!(result.test_type, Some(TestType::TestFunction));
    }

    #[test]
    fn test_detect_pytest_fixture() {
        let code = r#"
@pytest.fixture
def example_fixture():
    return 42
"#;
        let func = parse_function(code);
        let detector = PythonTestDetector::new();
        let context = TestContext::new();

        let result = detector.detect_test(&func, &context);
        assert!(result.is_test);
        assert_eq!(result.test_type, Some(TestType::Fixture));
        assert_eq!(result.framework, Some(TestFramework::Pytest));
    }

    #[test]
    fn test_detect_setup_method() {
        let code = r#"
def setUp(self):
    self.data = []
"#;
        let func = parse_function(code);
        let detector = PythonTestDetector::new();
        let context = TestContext::new().with_class("TestExample".to_string(), true);

        let result = detector.detect_test(&func, &context);
        assert!(result.is_test);
        assert_eq!(result.test_type, Some(TestType::Setup));
        assert_eq!(result.framework, Some(TestFramework::Unittest));
    }

    #[test]
    fn test_non_test_function() {
        let code = r#"
def helper_function():
    return 42
"#;
        let func = parse_function(code);
        let detector = PythonTestDetector::new();
        let context = TestContext::new();

        let result = detector.detect_test(&func, &context);
        assert!(!result.is_test);
    }

    #[test]
    fn test_is_test_file() {
        let detector = PythonTestDetector::new();

        assert!(detector.is_test_file(Path::new("test_example.py")));
        assert!(detector.is_test_file(Path::new("example_test.py")));
        assert!(detector.is_test_file(Path::new("tests/example.py")));
        assert!(detector.is_test_file(Path::new("test/example.py")));
        assert!(detector.is_test_file(Path::new("conftest.py")));

        assert!(!detector.is_test_file(Path::new("example.py")));
        assert!(!detector.is_test_file(Path::new("src/example.py")));
    }
}
