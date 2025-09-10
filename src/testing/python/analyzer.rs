use super::{
    assertion_patterns::AssertionDetector, complexity_analyzer::TestComplexityAnalyzer,
    config::PythonTestConfig, flaky_patterns::FlakyPatternDetector,
    framework_detector::detect_test_framework, TestFramework, TestQualityIssue,
};
use rustpython_parser::ast::{self, Stmt};
use std::path::PathBuf;

pub struct PythonTestAnalyzer {
    framework: TestFramework,
    config: PythonTestConfig,
}

impl PythonTestAnalyzer {
    pub fn new() -> Self {
        Self {
            framework: TestFramework::Unknown,
            config: PythonTestConfig::default(),
        }
    }

    pub fn with_config(config: PythonTestConfig) -> Self {
        Self {
            framework: TestFramework::Unknown,
            config,
        }
    }

    pub fn with_threshold(threshold: u32) -> Self {
        let mut config = PythonTestConfig::default();
        config.complexity.threshold = threshold;
        Self {
            framework: TestFramework::Unknown,
            config,
        }
    }

    pub fn analyze_module(&mut self, module: &ast::Mod, _path: &PathBuf) -> Vec<TestQualityIssue> {
        // Detect the test framework first
        self.framework = detect_test_framework(module);

        let mut all_issues = Vec::new();

        if let ast::Mod::Module(ast::ModModule { body, .. }) = module {
            for stmt in body {
                if let Some(issues) = self.analyze_statement(stmt) {
                    all_issues.extend(issues);
                }
            }
        }

        all_issues
    }

    fn analyze_statement(&self, stmt: &Stmt) -> Option<Vec<TestQualityIssue>> {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                if self.is_test_function(&func_def.name) {
                    Some(self.analyze_test_function(func_def))
                } else {
                    None
                }
            }
            Stmt::ClassDef(class_def) => {
                if self.is_test_class(class_def) {
                    let mut issues = Vec::new();
                    for stmt in &class_def.body {
                        if let Stmt::FunctionDef(func_def) = stmt {
                            if self.is_test_method(&func_def.name) {
                                issues.extend(self.analyze_test_function(func_def));
                            }
                        }
                    }
                    Some(issues)
                } else {
                    None
                }
            }
            Stmt::AsyncFunctionDef(func_def) => {
                if self.is_test_function(&func_def.name) {
                    // Convert async function def to regular for analysis
                    let regular_func = ast::StmtFunctionDef {
                        name: func_def.name.clone(),
                        args: func_def.args.clone(),
                        body: func_def.body.clone(),
                        decorator_list: func_def.decorator_list.clone(),
                        returns: func_def.returns.clone(),
                        type_comment: func_def.type_comment.clone(),
                        type_params: func_def.type_params.clone(),
                        range: func_def.range,
                    };
                    Some(self.analyze_test_function(&regular_func))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn analyze_test_function(&self, func_def: &ast::StmtFunctionDef) -> Vec<TestQualityIssue> {
        let mut issues = Vec::new();

        // Check for assertions
        let assertion_detector = AssertionDetector::new(self.framework.clone());
        if let Some(issue) = assertion_detector.analyze_test_function(func_def) {
            issues.push(issue);
        }

        // Check complexity
        let complexity_analyzer = TestComplexityAnalyzer::with_config(self.config.complexity.clone());
        if let Some(issue) = complexity_analyzer.analyze_test_function(func_def) {
            issues.push(issue);
        }

        // Check for flaky patterns
        let flaky_detector = FlakyPatternDetector::new();
        issues.extend(flaky_detector.analyze_test_function(func_def));

        // Check for excessive mocking
        if let Some(issue) = self.check_excessive_mocking(func_def) {
            issues.push(issue);
        }

        // Check for poor isolation
        if let Some(issue) = self.check_test_isolation(func_def) {
            issues.push(issue);
        }

        issues
    }

    fn is_test_function(&self, name: &str) -> bool {
        match &self.framework {
            TestFramework::Unittest => name.starts_with("test_") || name.starts_with("test"),
            TestFramework::Pytest | TestFramework::Nose => {
                name.starts_with("test_") || name.starts_with("test")
            }
            TestFramework::Doctest => false, // Doctest doesn't use function names
            TestFramework::Unknown => {
                // Default pattern
                name.starts_with("test_") || name.starts_with("test")
            }
        }
    }

    fn is_test_method(&self, name: &str) -> bool {
        // Similar to is_test_function but for class methods
        self.is_test_function(name)
            && name != "setUp"
            && name != "tearDown"
            && name != "setUpClass"
            && name != "tearDownClass"
    }

    fn is_test_class(&self, class_def: &ast::StmtClassDef) -> bool {
        // Check if class name follows test naming convention
        let name = class_def.name.as_str();
        if name.starts_with("Test") || name.ends_with("Test") || name.ends_with("Tests") {
            return true;
        }

        // Check if it inherits from TestCase
        for base in &class_def.bases {
            if let ast::Expr::Name(name) = base {
                if name.id.as_str() == "TestCase" {
                    return true;
                }
            } else if let ast::Expr::Attribute(attr) = base {
                if attr.attr.as_str() == "TestCase" {
                    return true;
                }
            }
        }

        false
    }

    fn check_excessive_mocking(&self, func_def: &ast::StmtFunctionDef) -> Option<TestQualityIssue> {
        let mock_count = self.count_mocks(func_def);

        if mock_count > self.config.mocking.max_mocks {
            Some(TestQualityIssue {
                issue_type: super::TestIssueType::ExcessiveMocking(mock_count),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: super::Severity::Medium,
                suggestion: "Consider using test fixtures, factories, or real objects to reduce mocking complexity".to_string(),
            })
        } else {
            None
        }
    }

    fn count_mocks(&self, func_def: &ast::StmtFunctionDef) -> usize {
        let mut count = 0;

        // Count @mock decorators
        for decorator in &func_def.decorator_list {
            if self.is_mock_decorator(decorator) {
                count += 1;
            }
        }

        // Count mock usage in body
        count += self.count_mocks_in_body(&func_def.body);

        count
    }

    fn is_mock_decorator(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call(call) => {
                if let ast::Expr::Attribute(attr) = &*call.func {
                    attr.attr.as_str() == "patch" || attr.attr.as_str() == "patch_object"
                } else if let ast::Expr::Name(name) = &*call.func {
                    name.id.as_str() == "patch" || name.id.as_str() == "mock"
                } else {
                    false
                }
            }
            ast::Expr::Attribute(attr) => {
                attr.attr.as_str() == "patch" || attr.attr.as_str() == "patch_object"
            }
            _ => false,
        }
    }

    fn count_mocks_in_body(&self, body: &[Stmt]) -> usize {
        let mut count = 0;

        for stmt in body {
            count += self.count_mocks_in_stmt(stmt);
        }

        count
    }

    fn count_mocks_in_stmt(&self, stmt: &Stmt) -> usize {
        match stmt {
            Stmt::With(with_stmt) => {
                let mut count = 0;
                for item in &with_stmt.items {
                    if let ast::Expr::Call(call) = &item.context_expr {
                        if self.is_mock_call(&call.func) {
                            count += 1;
                        }
                    }
                }
                count += self.count_mocks_in_body(&with_stmt.body);
                count
            }
            Stmt::Assign(assign) => {
                if let ast::Expr::Call(call) = &*assign.value {
                    if self.is_mock_call(&call.func) {
                        1
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn is_mock_call(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Attribute(attr) => {
                attr.attr.as_str() == "patch"
                    || attr.attr.as_str() == "patch_object"
                    || attr.attr.as_str() == "Mock"
                    || attr.attr.as_str() == "MagicMock"
            }
            ast::Expr::Name(name) => {
                name.id.as_str() == "patch"
                    || name.id.as_str() == "Mock"
                    || name.id.as_str() == "MagicMock"
                    || name.id.as_str() == "PropertyMock"
            }
            _ => false,
        }
    }

    fn check_test_isolation(&self, func_def: &ast::StmtFunctionDef) -> Option<TestQualityIssue> {
        if self.has_global_state_modification(&func_def.body) && !self.has_cleanup(&func_def.body) {
            Some(TestQualityIssue {
                issue_type: super::TestIssueType::PoorIsolation,
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: super::Severity::High,
                suggestion: "Ensure test cleans up global state or use fixtures for isolation"
                    .to_string(),
            })
        } else {
            None
        }
    }

    fn has_global_state_modification(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            match stmt {
                Stmt::Global(_) | Stmt::Nonlocal(_) => return true,
                Stmt::Assign(assign) => {
                    // Check if assigning to module-level variables
                    for target in &assign.targets {
                        if self.is_module_level_assignment(target) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn is_module_level_assignment(&self, expr: &ast::Expr) -> bool {
        if let ast::Expr::Attribute(attr) = expr {
            // Check if assigning to attributes of imported modules
            if let ast::Expr::Name(name) = &*attr.value {
                // Common module names that shouldn't be modified in tests
                let module_name = name.id.as_str();
                return module_name == "os"
                    || module_name == "sys"
                    || module_name == "settings"
                    || module_name == "config";
            }
        }
        false
    }

    fn has_cleanup(&self, body: &[Stmt]) -> bool {
        // Check for try/finally blocks or context managers that might handle cleanup
        for stmt in body {
            match stmt {
                Stmt::Try(try_stmt) => {
                    if !try_stmt.finalbody.is_empty() {
                        return true;
                    }
                }
                Stmt::With(_) => {
                    // Context managers often handle cleanup
                    return true;
                }
                _ => {}
            }
        }
        false
    }
}
