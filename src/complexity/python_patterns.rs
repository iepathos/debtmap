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

impl Default for PythonPatternDetector {
    fn default() -> Self {
        Self::new()
    }
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
        use ast::Stmt::*;

        // Use visitor pattern to reduce cyclomatic complexity
        let handler: Option<fn(&mut Self, &ast::Stmt)> = match stmt {
            AsyncFunctionDef(_) | FunctionDef(_) | ClassDef(_) => Some(Self::handle_definition),
            Try(_) | With(_) | AsyncWith(_) => Some(Self::handle_context_manager),
            AsyncFor(_) | For(_) | If(_) | While(_) => Some(Self::handle_control_flow),
            Expr(_) | Return(_) | Assign(_) | AugAssign(_) => Some(Self::handle_expression),
            _ => None,
        };

        if let Some(handle) = handler {
            handle(self, stmt);
        }
    }

    fn handle_definition(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::AsyncFunctionDef(func) => self.handle_async_function(func),
            ast::Stmt::FunctionDef(func) => self.handle_function(func),
            ast::Stmt::ClassDef(class) => self.handle_class(class),
            _ => {}
        }
    }

    fn handle_context_manager(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Try(try_stmt) => self.handle_try(try_stmt),
            ast::Stmt::With(with_stmt) => self.handle_with(with_stmt),
            ast::Stmt::AsyncWith(with_stmt) => self.handle_async_with(with_stmt),
            _ => {}
        }
    }

    fn handle_control_flow(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::AsyncFor(for_stmt) => self.handle_async_for(for_stmt),
            ast::Stmt::For(for_stmt) => self.handle_for(for_stmt),
            ast::Stmt::If(if_stmt) => self.handle_if(if_stmt),
            ast::Stmt::While(while_stmt) => self.handle_while(while_stmt),
            _ => {}
        }
    }

    fn handle_expression(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Expr(expr) => self.handle_expr_stmt(expr),
            ast::Stmt::Return(ret) => self.handle_return(ret),
            ast::Stmt::Assign(assign) => self.handle_assign(assign),
            ast::Stmt::AugAssign(aug) => self.handle_aug_assign(aug),
            _ => {}
        }
    }

    fn handle_expr_stmt(&mut self, expr: &ast::StmtExpr) {
        self.analyze_expr(&expr.value);
    }

    fn handle_return(&mut self, ret: &ast::StmtReturn) {
        if let Some(value) = &ret.value {
            self.analyze_expr(value);
        }
    }

    fn handle_assign(&mut self, assign: &ast::StmtAssign) {
        self.analyze_expr(&assign.value);
    }

    fn handle_aug_assign(&mut self, aug: &ast::StmtAugAssign) {
        self.analyze_expr(&aug.value);
    }

    fn handle_async_function(&mut self, func: &ast::StmtAsyncFunctionDef) {
        self.patterns.async_await_count += 1;
        self.patterns.decorator_count += func.decorator_list.len() as u32;
        self.analyze_function_body(&func.name, &func.body);
    }

    fn handle_function(&mut self, func: &ast::StmtFunctionDef) {
        self.patterns.decorator_count += func.decorator_list.len() as u32;
        self.analyze_function_body(&func.name, &func.body);
    }

    fn analyze_function_body(&mut self, name: &ast::Identifier, body: &[ast::Stmt]) {
        if self.function_depth > 0 {
            self.patterns.nested_functions += 1;
        }
        let old_name = self.current_function_name.clone();
        let old_depth = self.function_depth;
        self.current_function_name = Some(name.to_string());
        self.function_depth += 1;
        self.analyze_body(body);
        self.current_function_name = old_name;
        self.function_depth = old_depth;
    }

    fn handle_class(&mut self, class: &ast::StmtClassDef) {
        self.patterns.decorator_count += class.decorator_list.len() as u32;
        for keyword in &class.keywords {
            if keyword.arg.as_ref().is_some_and(|arg| arg == "metaclass") {
                self.patterns.metaclass_usage += 1;
            }
        }
        self.analyze_body(&class.body);
    }

    fn handle_try(&mut self, try_stmt: &ast::StmtTry) {
        self.patterns.try_except_finally += 1;
        self.analyze_body(&try_stmt.body);
        for handler in &try_stmt.handlers {
            let ast::ExceptHandler::ExceptHandler(h) = handler;
            self.analyze_body(&h.body);
        }
        self.analyze_body(&try_stmt.orelse);
        self.analyze_body(&try_stmt.finalbody);
    }

    fn handle_with(&mut self, with_stmt: &ast::StmtWith) {
        self.patterns.with_statements += 1;
        self.analyze_body(&with_stmt.body);
    }

    fn handle_async_with(&mut self, with_stmt: &ast::StmtAsyncWith) {
        self.patterns.with_statements += 1;
        self.patterns.async_await_count += 1;
        self.analyze_body(&with_stmt.body);
    }

    fn handle_async_for(&mut self, for_stmt: &ast::StmtAsyncFor) {
        self.patterns.async_await_count += 1;
        self.analyze_expr(&for_stmt.iter);
        self.analyze_body(&for_stmt.body);
    }

    fn handle_for(&mut self, for_stmt: &ast::StmtFor) {
        self.analyze_expr(&for_stmt.iter);
        self.analyze_body(&for_stmt.body);
    }

    fn handle_if(&mut self, if_stmt: &ast::StmtIf) {
        self.analyze_expr(&if_stmt.test);
        self.analyze_body(&if_stmt.body);
        self.analyze_body(&if_stmt.orelse);
    }

    fn handle_while(&mut self, while_stmt: &ast::StmtWhile) {
        self.analyze_expr(&while_stmt.test);
        self.analyze_body(&while_stmt.body);
    }

    fn analyze_expr(&mut self, expr: &ast::Expr) {
        use ast::Expr::*;
        match expr {
            Lambda(_) => self.patterns.lambda_count += 1,
            GeneratorExp(_) => self.handle_generator(),
            ListComp(_) | SetComp(_) | DictComp(_) => self.handle_comprehension(),
            Await(_) => self.patterns.async_await_count += 1,
            Yield(_) | YieldFrom(_) => self.patterns.generator_count += 1,
            Call(call) => self.handle_call(call),
            BinOp(binop) => self.handle_binop(binop),
            UnaryOp(unary) => self.analyze_expr(&unary.operand),
            IfExp(if_exp) => self.handle_if_exp(if_exp),
            _ => {}
        }
    }

    fn handle_generator(&mut self) {
        self.patterns.generator_count += 1;
        self.comprehension_depth += 1;
        // Note: Would need to recursively analyze comprehension here
        self.comprehension_depth -= 1;
    }

    fn handle_comprehension(&mut self) {
        self.comprehension_depth += 1;
        if self.comprehension_depth > 1 {
            self.patterns.comprehension_depth = self
                .patterns
                .comprehension_depth
                .max(self.comprehension_depth);
        }
        // Note: Would need to recursively analyze comprehension here
        self.comprehension_depth -= 1;
    }

    fn handle_call(&mut self, call: &ast::ExprCall) {
        if let Some(ref func_name) = self.current_function_name {
            if let ast::Expr::Name(name) = &*call.func {
                if name.id == *func_name {
                    self.patterns.recursive_calls += 1;
                }
            }
        }
        for arg in &call.args {
            self.analyze_expr(arg);
        }
    }

    fn handle_binop(&mut self, binop: &ast::ExprBinOp) {
        self.analyze_expr(&binop.left);
        self.analyze_expr(&binop.right);
    }

    fn handle_if_exp(&mut self, if_exp: &ast::ExprIfExp) {
        self.analyze_expr(&if_exp.test);
        self.analyze_expr(&if_exp.body);
        self.analyze_expr(&if_exp.orelse);
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
        let patterns = PythonPatternComplexity {
            async_await_count: 2,
            decorator_count: 3,
            generator_count: 1,
            ..Default::default()
        };

        assert_eq!(patterns.total_complexity(), 2 * 2 + 3 + 2);
    }
}
