use super::{config::ComplexityConfig, Severity, TestIssueType, TestQualityIssue};
use rustpython_parser::ast::{self, Expr, Stmt};

// Pure data structure for complexity metrics
#[derive(Debug, Clone)]
struct ComplexityMetrics {
    conditionals: u32,
    loops: u32,
    assertions: u32,
    mocks: u32,
    nesting_depth: u32,
    line_count: u32,
}

pub struct TestComplexityAnalyzer {
    config: ComplexityConfig,
}

impl Default for TestComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TestComplexityAnalyzer {
    pub fn new() -> Self {
        Self {
            config: ComplexityConfig::default(),
        }
    }

    pub fn with_threshold(threshold: u32) -> Self {
        let config = ComplexityConfig {
            threshold,
            ..Default::default()
        };
        Self { config }
    }

    pub fn with_config(config: ComplexityConfig) -> Self {
        Self { config }
    }

    pub fn analyze_test_function(
        &self,
        func_def: &ast::StmtFunctionDef,
    ) -> Option<TestQualityIssue> {
        let complexity = self.calculate_complexity(&func_def.body);

        if complexity > self.config.threshold {
            Some(TestQualityIssue {
                issue_type: TestIssueType::OverlyComplex(complexity),
                test_name: func_def.name.to_string(),
                line: 1, // TODO: Extract actual line number from range
                severity: self.get_severity(complexity),
                suggestion: self.suggest_simplification(&func_def.body, complexity),
            })
        } else {
            None
        }
    }

    // Pure function: Calculate weighted contribution above threshold
    fn calculate_threshold_weight(value: u32, threshold: u32, weight: u32) -> u32 {
        if value > threshold {
            (value - threshold) * weight
        } else {
            0
        }
    }

    // Pure function: Calculate line count contribution
    fn calculate_line_contribution(line_count: u32, threshold: u32, divisor: u32) -> u32 {
        if line_count > threshold {
            (line_count - threshold) / divisor
        } else {
            0
        }
    }

    // Pure function: Collect all complexity metrics
    fn collect_complexity_metrics(&self, body: &[Stmt]) -> ComplexityMetrics {
        ComplexityMetrics {
            conditionals: self.count_conditionals(body),
            loops: self.count_loops(body),
            assertions: self.count_assertions(body),
            mocks: self.count_mocks(body),
            nesting_depth: self.calculate_max_nesting(body, 0),
            line_count: body.len() as u32,
        }
    }

    // Pure function: Calculate weighted complexity from metrics
    fn calculate_weighted_complexity(
        metrics: &ComplexityMetrics,
        config: &ComplexityConfig,
    ) -> u32 {
        let base_complexity = 1;

        // Direct contributions
        let conditional_contribution = metrics.conditionals * config.conditional_weight;
        let loop_contribution = metrics.loops * config.loop_weight;
        let mock_contribution = metrics.mocks * config.mock_weight;

        // Threshold-based contributions
        let assertion_contribution = Self::calculate_threshold_weight(
            metrics.assertions,
            config.assertion_threshold,
            config.assertion_weight,
        );

        let nesting_contribution = Self::calculate_threshold_weight(
            metrics.nesting_depth,
            config.nesting_threshold,
            config.nesting_weight,
        );

        let line_contribution = Self::calculate_line_contribution(
            metrics.line_count,
            config.line_threshold,
            config.line_divisor,
        );

        // Compose final complexity
        base_complexity
            + conditional_contribution
            + loop_contribution
            + assertion_contribution
            + mock_contribution
            + nesting_contribution
            + line_contribution
    }

    fn calculate_complexity(&self, body: &[Stmt]) -> u32 {
        // Step 1: Collect metrics (pure)
        let metrics = self.collect_complexity_metrics(body);

        // Step 2: Calculate weighted complexity (pure)
        Self::calculate_weighted_complexity(&metrics, &self.config)
    }

    fn count_conditionals(&self, body: &[Stmt]) -> u32 {
        let mut count = 0;
        for stmt in body {
            match stmt {
                Stmt::If(_) => {
                    count += 1;
                    count += self.count_conditionals_in_stmt(stmt);
                }
                Stmt::Try(try_stmt) => {
                    // Each except handler is a conditional branch
                    count += try_stmt.handlers.len() as u32;
                    count += self.count_conditionals_in_stmt(stmt);
                }
                _ => count += self.count_conditionals_in_stmt(stmt),
            }
        }
        count
    }

    fn count_conditionals_in_stmt(&self, stmt: &Stmt) -> u32 {
        match stmt {
            Stmt::If(if_stmt) => {
                self.count_conditionals(&if_stmt.body) + self.count_conditionals(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.count_conditionals(&for_stmt.body) + self.count_conditionals(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.count_conditionals(&while_stmt.body)
                    + self.count_conditionals(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.count_conditionals(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                let mut count = self.count_conditionals(&try_stmt.body);
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    count += self.count_conditionals(&h.body);
                }
                count += self.count_conditionals(&try_stmt.orelse);
                count += self.count_conditionals(&try_stmt.finalbody);
                count
            }
            _ => 0,
        }
    }

    fn count_loops(&self, body: &[Stmt]) -> u32 {
        let mut count = 0;
        for stmt in body {
            match stmt {
                Stmt::For(_) | Stmt::While(_) => {
                    count += 1;
                    count += self.count_loops_in_stmt(stmt);
                }
                _ => count += self.count_loops_in_stmt(stmt),
            }
        }
        count
    }

    fn count_loops_in_stmt(&self, stmt: &Stmt) -> u32 {
        match stmt {
            Stmt::If(if_stmt) => {
                self.count_loops(&if_stmt.body) + self.count_loops(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.count_loops(&for_stmt.body) + self.count_loops(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.count_loops(&while_stmt.body) + self.count_loops(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.count_loops(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                let mut count = self.count_loops(&try_stmt.body);
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    count += self.count_loops(&h.body);
                }
                count += self.count_loops(&try_stmt.orelse);
                count += self.count_loops(&try_stmt.finalbody);
                count
            }
            _ => 0,
        }
    }

    fn count_assertions(&self, body: &[Stmt]) -> u32 {
        let mut count = 0;
        for stmt in body {
            if self.is_assertion_stmt(stmt) {
                count += 1;
            }
            count += self.count_assertions_in_stmt(stmt);
        }
        count
    }

    fn count_assertions_in_stmt(&self, stmt: &Stmt) -> u32 {
        match stmt {
            Stmt::If(if_stmt) => {
                self.count_assertions(&if_stmt.body) + self.count_assertions(&if_stmt.orelse)
            }
            Stmt::For(for_stmt) => {
                self.count_assertions(&for_stmt.body) + self.count_assertions(&for_stmt.orelse)
            }
            Stmt::While(while_stmt) => {
                self.count_assertions(&while_stmt.body) + self.count_assertions(&while_stmt.orelse)
            }
            Stmt::With(with_stmt) => self.count_assertions(&with_stmt.body),
            Stmt::Try(try_stmt) => {
                let mut count = self.count_assertions(&try_stmt.body);
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    count += self.count_assertions(&h.body);
                }
                count += self.count_assertions(&try_stmt.orelse);
                count += self.count_assertions(&try_stmt.finalbody);
                count
            }
            _ => 0,
        }
    }

    fn is_assertion_stmt(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Assert(_) => true,
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    self.is_assertion_call(&call.func)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn is_assertion_call(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method_name = attr.attr.as_str();
                method_name.starts_with("assert") || method_name.starts_with("assert_")
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name.starts_with("assert_") || func_name == "ok_" || func_name == "eq_"
            }
            _ => false,
        }
    }

    fn count_mocks(&self, body: &[Stmt]) -> u32 {
        let mut count = 0;
        for stmt in body {
            count += self.count_mocks_in_stmt(stmt);
        }
        count
    }

    fn count_mocks_in_stmt(&self, stmt: &Stmt) -> u32 {
        match stmt {
            Stmt::With(with_stmt) => {
                let mut count = 0;
                for item in &with_stmt.items {
                    if self.is_mock_context(&item.context_expr) {
                        count += 1;
                    }
                }
                count += self.count_mocks(&with_stmt.body);
                count
            }
            Stmt::Expr(expr_stmt) => {
                if let Expr::Call(call) = &*expr_stmt.value {
                    if self.is_mock_call(&call.func) {
                        1
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            Stmt::FunctionDef(func_def) => {
                // Count mock decorators
                let mut count = 0;
                for decorator in &func_def.decorator_list {
                    if self.is_mock_decorator(decorator) {
                        count += 1;
                    }
                }
                count
            }
            _ => 0,
        }
    }

    fn is_mock_context(&self, expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            self.is_mock_call(&call.func)
        } else {
            false
        }
    }

    fn is_mock_call(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(attr) => {
                let method = attr.attr.as_str();
                method == "patch"
                    || method == "patch_object"
                    || method == "mock"
                    || method == "Mock"
            }
            Expr::Name(name) => {
                let func_name = name.id.as_str();
                func_name == "patch" || func_name == "Mock" || func_name == "MagicMock"
            }
            _ => false,
        }
    }

    fn is_mock_decorator(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call(call) => self.is_mock_call(&call.func),
            Expr::Attribute(attr) => {
                attr.attr.as_str() == "patch" || attr.attr.as_str() == "patch_object"
            }
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn calculate_max_nesting(&self, body: &[Stmt], current_depth: u32) -> u32 {
        let mut max_depth = current_depth;

        for stmt in body {
            let stmt_depth = match stmt {
                Stmt::If(if_stmt) => {
                    let if_depth = self.calculate_max_nesting(&if_stmt.body, current_depth + 1);
                    let else_depth = self.calculate_max_nesting(&if_stmt.orelse, current_depth + 1);
                    if_depth.max(else_depth)
                }
                Stmt::For(for_stmt) => {
                    let body_depth = self.calculate_max_nesting(&for_stmt.body, current_depth + 1);
                    let else_depth =
                        self.calculate_max_nesting(&for_stmt.orelse, current_depth + 1);
                    body_depth.max(else_depth)
                }
                Stmt::While(while_stmt) => {
                    let body_depth =
                        self.calculate_max_nesting(&while_stmt.body, current_depth + 1);
                    let else_depth =
                        self.calculate_max_nesting(&while_stmt.orelse, current_depth + 1);
                    body_depth.max(else_depth)
                }
                Stmt::With(with_stmt) => {
                    self.calculate_max_nesting(&with_stmt.body, current_depth + 1)
                }
                Stmt::Try(try_stmt) => {
                    let mut depth = self.calculate_max_nesting(&try_stmt.body, current_depth + 1);
                    for handler in &try_stmt.handlers {
                        let ast::ExceptHandler::ExceptHandler(h) = handler;
                        depth = depth.max(self.calculate_max_nesting(&h.body, current_depth + 1));
                    }
                    depth =
                        depth.max(self.calculate_max_nesting(&try_stmt.orelse, current_depth + 1));
                    depth = depth
                        .max(self.calculate_max_nesting(&try_stmt.finalbody, current_depth + 1));
                    depth
                }
                _ => current_depth,
            };
            max_depth = max_depth.max(stmt_depth);
        }

        max_depth
    }

    fn get_severity(&self, complexity: u32) -> Severity {
        if complexity > self.config.threshold * 3 {
            Severity::Critical
        } else if complexity > self.config.threshold * 2 {
            Severity::High
        } else if complexity > self.config.threshold {
            Severity::Medium
        } else {
            Severity::Low
        }
    }

    fn suggest_simplification(&self, body: &[Stmt], complexity: u32) -> String {
        let mut suggestions = Vec::new();

        let loop_count = self.count_loops(body);
        let assertion_count = self.count_assertions(body);
        let mock_count = self.count_mocks(body);
        let nesting_depth = self.calculate_max_nesting(body, 0);

        if loop_count > 0 {
            suggestions.push("Consider using parametrized tests instead of loops");
        }

        if assertion_count > self.config.assertion_threshold {
            suggestions.push("Split into multiple focused test functions");
        }

        if mock_count > 3 {
            suggestions.push("Reduce mocking by using test fixtures or factories");
        }

        if nesting_depth > self.config.nesting_threshold {
            suggestions.push("Extract helper functions to reduce nesting");
        }

        if body.len() as u32 > self.config.line_threshold {
            suggestions.push("Break down into smaller, more focused tests");
        }

        if suggestions.is_empty() {
            format!(
                "Test complexity score {} exceeds threshold {}",
                complexity, self.config.threshold
            )
        } else {
            suggestions.join("; ")
        }
    }
}
