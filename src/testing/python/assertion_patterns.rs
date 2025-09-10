use super::{Severity, TestFramework, TestIssueType, TestQualityIssue};
use rustpython_parser::ast::{self, Expr, Stmt};

pub struct AssertionDetector {
    framework: TestFramework,
}

impl AssertionDetector {
    pub fn new(framework: TestFramework) -> Self {
        Self { framework }
    }

    pub fn analyze_test_function(
        &self,
        func_def: &ast::StmtFunctionDef,
    ) -> Option<TestQualityIssue> {
        let has_assertions = self.has_assertions(&func_def.body);
        let has_setup = self.has_setup_code(&func_def.body);
        let has_action = self.has_action_code(&func_def.body);

        if !has_assertions && (has_setup || has_action) {
            Some(TestQualityIssue {
                issue_type: TestIssueType::NoAssertions,
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: Severity::High,
                suggestion: self.suggest_assertions(&func_def.body),
            })
        } else {
            None
        }
    }

    fn has_assertions(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.is_assertion(stmt) {
                return true;
            }
            // Check nested statements
            if self.has_assertions_in_stmt(stmt) {
                return true;
            }
        }
        false
    }

    fn has_assertions_in_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If(if_stmt) => {
                self.has_assertions(&if_stmt.body) || self.has_assertions(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.has_assertions(&for_stmt.body) || self.has_assertions(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.has_assertions(&while_stmt.body) || self.has_assertions(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.has_assertions(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                self.has_assertions(&try_stmt.body)
                    || try_stmt.handlers.iter().any(|handler| {
                        let ast::ExceptHandler::ExceptHandler(h) = handler;
                        self.has_assertions(&h.body)
                    })
                    || self.has_assertions(&try_stmt.orelse)
                    || self.has_assertions(&try_stmt.finalbody)
            }
            _ => false,
        }
    }

    fn is_assertion(&self, stmt: &Stmt) -> bool {
        match &self.framework {
            TestFramework::Unittest => self.is_unittest_assertion(stmt),
            TestFramework::Pytest => self.is_pytest_assertion(stmt),
            TestFramework::Nose => self.is_nose_assertion(stmt),
            TestFramework::Doctest => false, // Doctest assertions are in comments
            TestFramework::Unknown => {
                // Try to detect any common assertion pattern
                self.is_unittest_assertion(stmt)
                    || self.is_pytest_assertion(stmt)
                    || self.is_nose_assertion(stmt)
            }
        }
    }

    fn is_unittest_assertion(&self, stmt: &Stmt) -> bool {
        if let Stmt::Expr(expr_stmt) = stmt {
            if let Expr::Call(call) = &*expr_stmt.value {
                if let Expr::Attribute(attr) = &*call.func {
                    let method_name = attr.attr.as_str();
                    // Check for self.assert* methods
                    if let Expr::Name(name) = &*attr.value {
                        if name.id.as_str() == "self" && method_name.starts_with("assert") {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn is_pytest_assertion(&self, stmt: &Stmt) -> bool {
        // Pytest uses plain assert statements
        if let Stmt::Assert(_) = stmt {
            return true;
        }
        // Also check for pytest.raises, pytest.warns, etc.
        if let Stmt::With(with_stmt) = stmt {
            for item in &with_stmt.items {
                if let Expr::Call(call) = &item.context_expr {
                    if self.is_pytest_context_manager(&call.func) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn is_pytest_context_manager(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let attr_name = attr.attr.as_str();
                (attr_name == "raises" || attr_name == "warns" || attr_name == "deprecated_call")
                    && self.is_pytest_module(&*attr.value)
            }
            _ => false,
        }
    }

    fn is_pytest_module(&self, expr: &Expr) -> bool {
        if let Expr::Name(name) = expr {
            name.id.as_str() == "pytest" || name.id.as_str() == "py"
        } else {
            false
        }
    }

    fn is_nose_assertion(&self, stmt: &Stmt) -> bool {
        if let Stmt::Expr(expr_stmt) = stmt {
            if let Expr::Call(call) = &*expr_stmt.value {
                // Check for nose.tools assert functions
                if let Expr::Name(name) = &*call.func {
                    let func_name = name.id.as_str();
                    if func_name.starts_with("assert_") || func_name == "ok_" || func_name == "eq_"
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn has_setup_code(&self, body: &[Stmt]) -> bool {
        // Look for variable assignments, object creation, etc.
        for stmt in body {
            match stmt {
                Stmt::Assign(_) | Stmt::AnnAssign(_) | Stmt::AugAssign(_) => return true,
                Stmt::Expr(expr_stmt) => {
                    if let Expr::Call(_) = &*expr_stmt.value {
                        // Calls that aren't assertions are likely setup
                        if !self.is_assertion(stmt) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn has_action_code(&self, body: &[Stmt]) -> bool {
        // Look for function calls that aren't assertions
        let mut call_count = 0;
        for stmt in body {
            if let Stmt::Expr(expr_stmt) = stmt {
                if let Expr::Call(_) = &*expr_stmt.value {
                    if !self.is_assertion(stmt) {
                        call_count += 1;
                    }
                }
            }
        }
        call_count > 0
    }

    fn suggest_assertions(&self, body: &[Stmt]) -> String {
        let mut suggestions = Vec::new();

        // Look for variables that might need assertions
        for stmt in body {
            if let Stmt::Assign(assign) = stmt {
                if let Some(target) = assign.targets.first() {
                    if let Expr::Name(name) = target {
                        let var_name = name.id.as_str();
                        match &self.framework {
                            TestFramework::Unittest => {
                                suggestions.push(format!("self.assertIsNotNone({})", var_name));
                                suggestions.push(format!(
                                    "self.assertEqual({}, expected_value)",
                                    var_name
                                ));
                            }
                            TestFramework::Pytest | TestFramework::Unknown => {
                                suggestions.push(format!("assert {} is not None", var_name));
                                suggestions.push(format!("assert {} == expected_value", var_name));
                            }
                            TestFramework::Nose => {
                                suggestions.push(format!("assert_is_not_none({})", var_name));
                                suggestions
                                    .push(format!("assert_equal({}, expected_value)", var_name));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if suggestions.is_empty() {
            match &self.framework {
                TestFramework::Unittest => {
                    "Add unittest assertions like self.assertEqual() or self.assertTrue()"
                        .to_string()
                }
                TestFramework::Pytest => {
                    "Add assert statements to verify expected behavior".to_string()
                }
                TestFramework::Nose => {
                    "Add nose.tools assertions like assert_equal() or ok_()".to_string()
                }
                _ => "Add assertions to verify the test outcome".to_string(),
            }
        } else {
            format!("Consider adding: {}", suggestions.join(" or "))
        }
    }
}
