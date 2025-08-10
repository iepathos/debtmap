use rustpython_parser::ast;

/// Python-specific pattern complexity detection
#[derive(Debug, Clone, Default)]
pub struct PythonPatternComplexity {
    pub async_await_count: u32,
    pub decorator_count: u32,
    pub generator_count: u32,
    pub comprehension_depth: u32,
    pub lambda_count: u32,
    pub try_except_finally: u32,
    pub with_statements: u32,
    pub metaclass_usage: u32,
    pub recursive_calls: u32,
    pub nested_functions: u32,
}

impl PythonPatternComplexity {
    pub fn total_complexity(&self) -> u32 {
        self.async_await_count * 2
            + self.decorator_count
            + self.generator_count * 2
            + self.comprehension_depth * 2
            + self.lambda_count
            + self.try_except_finally
            + self.with_statements
            + self.metaclass_usage * 3
            + self.recursive_calls * 3
            + self.nested_functions * 2
    }
}

pub struct PythonPatternDetector {
    pub patterns: PythonPatternComplexity,
    current_function_name: Option<String>,
    comprehension_depth: u32,
    function_depth: u32,
}

impl PythonPatternDetector {
    pub fn new() -> Self {
        Self {
            patterns: PythonPatternComplexity::default(),
            current_function_name: None,
            comprehension_depth: 0,
            function_depth: 0,
        }
    }

    pub fn analyze_body(&mut self, body: &[ast::Stmt]) {
        for stmt in body {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            // Async function detection
            ast::Stmt::AsyncFunctionDef(func) => {
                self.patterns.async_await_count += 1;
                self.patterns.decorator_count += func.decorator_list.len() as u32;

                // Check for nested functions
                if self.function_depth > 0 {
                    self.patterns.nested_functions += 1;
                }

                let old_name = self.current_function_name.clone();
                let old_depth = self.function_depth;
                self.current_function_name = Some(func.name.to_string());
                self.function_depth += 1;

                self.analyze_body(&func.body);

                self.current_function_name = old_name;
                self.function_depth = old_depth;
            }

            // Regular function with decorators
            ast::Stmt::FunctionDef(func) => {
                self.patterns.decorator_count += func.decorator_list.len() as u32;

                // Check for nested functions
                if self.function_depth > 0 {
                    self.patterns.nested_functions += 1;
                }

                let old_name = self.current_function_name.clone();
                let old_depth = self.function_depth;
                self.current_function_name = Some(func.name.to_string());
                self.function_depth += 1;

                self.analyze_body(&func.body);

                self.current_function_name = old_name;
                self.function_depth = old_depth;
            }

            // Class definition (check for metaclass)
            ast::Stmt::ClassDef(class) => {
                self.patterns.decorator_count += class.decorator_list.len() as u32;

                // Check for metaclass usage
                for keyword in &class.keywords {
                    if keyword.arg.as_ref().map_or(false, |arg| arg == "metaclass") {
                        self.patterns.metaclass_usage += 1;
                    }
                }

                self.analyze_body(&class.body);
            }

            // Try/except/finally blocks
            ast::Stmt::Try(try_stmt) => {
                self.patterns.try_except_finally += 1;
                self.analyze_body(&try_stmt.body);
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    self.analyze_body(&h.body);
                }
                self.analyze_body(&try_stmt.orelse);
                self.analyze_body(&try_stmt.finalbody);
            }

            // With statements (context managers)
            ast::Stmt::With(with_stmt) => {
                self.patterns.with_statements += 1;
                self.analyze_body(&with_stmt.body);
            }

            ast::Stmt::AsyncWith(with_stmt) => {
                self.patterns.with_statements += 1;
                self.patterns.async_await_count += 1;
                self.analyze_body(&with_stmt.body);
            }

            // For loops (check for async)
            ast::Stmt::AsyncFor(for_stmt) => {
                self.patterns.async_await_count += 1;
                self.analyze_expr(&for_stmt.iter);
                self.analyze_body(&for_stmt.body);
            }

            ast::Stmt::For(for_stmt) => {
                self.analyze_expr(&for_stmt.iter);
                self.analyze_body(&for_stmt.body);
            }

            // Expression statements
            ast::Stmt::Expr(expr) => {
                self.analyze_expr(&expr.value);
            }

            // Other statements that might contain expressions
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.analyze_expr(value);
                }
            }

            ast::Stmt::Assign(assign) => {
                self.analyze_expr(&assign.value);
            }

            ast::Stmt::AugAssign(aug) => {
                self.analyze_expr(&aug.value);
            }

            ast::Stmt::If(if_stmt) => {
                self.analyze_expr(&if_stmt.test);
                self.analyze_body(&if_stmt.body);
                self.analyze_body(&if_stmt.orelse);
            }

            ast::Stmt::While(while_stmt) => {
                self.analyze_expr(&while_stmt.test);
                self.analyze_body(&while_stmt.body);
            }

            _ => {}
        }
    }

    fn analyze_expr(&mut self, expr: &ast::Expr) {
        match expr {
            // Lambda expressions
            ast::Expr::Lambda(_) => {
                self.patterns.lambda_count += 1;
            }

            // Generator expressions
            ast::Expr::GeneratorExp(_) => {
                self.patterns.generator_count += 1;
                self.comprehension_depth += 1;
                // Note: Would need to recursively analyze comprehension here
                self.comprehension_depth -= 1;
            }

            // List/Set/Dict comprehensions
            ast::Expr::ListComp(_) | ast::Expr::SetComp(_) | ast::Expr::DictComp(_) => {
                self.comprehension_depth += 1;
                if self.comprehension_depth > 1 {
                    // Nested comprehension adds significant complexity
                    self.patterns.comprehension_depth = self
                        .patterns
                        .comprehension_depth
                        .max(self.comprehension_depth);
                }
                // Note: Would need to recursively analyze comprehension here
                self.comprehension_depth -= 1;
            }

            // Await expressions
            ast::Expr::Await(_) => {
                self.patterns.async_await_count += 1;
            }

            // Yield/YieldFrom (generators)
            ast::Expr::Yield(_) | ast::Expr::YieldFrom(_) => {
                self.patterns.generator_count += 1;
            }

            // Function calls (check for recursion)
            ast::Expr::Call(call) => {
                if let Some(ref func_name) = self.current_function_name {
                    if let ast::Expr::Name(name) = &*call.func {
                        if name.id.to_string() == *func_name {
                            self.patterns.recursive_calls += 1;
                        }
                    }
                }

                // Recursively analyze arguments
                for arg in &call.args {
                    self.analyze_expr(arg);
                }
            }

            // Other expressions that might contain nested expressions
            ast::Expr::BinOp(binop) => {
                self.analyze_expr(&binop.left);
                self.analyze_expr(&binop.right);
            }

            ast::Expr::UnaryOp(unary) => {
                self.analyze_expr(&unary.operand);
            }

            ast::Expr::IfExp(if_exp) => {
                self.analyze_expr(&if_exp.test);
                self.analyze_expr(&if_exp.body);
                self.analyze_expr(&if_exp.orelse);
            }

            _ => {}
        }
    }
}

/// Analyze Python AST for patterns
pub fn analyze_python_patterns(body: &[ast::Stmt]) -> PythonPatternComplexity {
    let mut detector = PythonPatternDetector::new();
    detector.analyze_body(body);
    detector.patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_pattern_complexity_total() {
        let mut patterns = PythonPatternComplexity::default();
        patterns.async_await_count = 2;
        patterns.decorator_count = 3;
        patterns.generator_count = 1;

        assert_eq!(patterns.total_complexity(), 2 * 2 + 3 + 1 * 2);
    }
}
