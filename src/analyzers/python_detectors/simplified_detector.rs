// Simplified Python detector implementation that compiles with current rustpython-parser
use crate::core::{DebtItem, DebtType, Priority};
use rustpython_parser::ast;
use std::path::PathBuf;

pub struct SimplifiedPythonDetector {
    pub path: PathBuf,
    pub debt_items: Vec<DebtItem>,
}

impl SimplifiedPythonDetector {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            debt_items: Vec::new(),
        }
    }

    pub fn analyze_module(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(module) = module {
            self.analyze_statements(&module.body);
        }
    }

    fn analyze_statements(&mut self, stmts: &[ast::Stmt]) {
        for stmt in stmts {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::FunctionDef(func_def) => {
                self.check_function_patterns(func_def);
            }
            ast::Stmt::ClassDef(class_def) => {
                self.check_class_patterns(class_def);
            }
            ast::Stmt::For(for_stmt) => {
                self.check_loop_patterns(&for_stmt.body);
                // Also analyze statements inside the loop
                self.analyze_statements(&for_stmt.body);
            }
            ast::Stmt::While(while_stmt) => {
                self.check_loop_patterns(&while_stmt.body);
                // Also analyze statements inside the loop
                self.analyze_statements(&while_stmt.body);
            }
            _ => {}
        }
    }

    fn check_function_patterns(&mut self, func_def: &ast::StmtFunctionDef) {
        // Check for mutable default arguments
        for default in func_def.args.defaults() {
            if self.is_mutable_default(default) {
                self.debt_items.push(DebtItem {
                    id: format!("py-mutable-default-{}", self.debt_items.len()),
                    debt_type: DebtType::CodeOrganization,
                    priority: Priority::High,
                    file: self.path.clone(),
                    line: 1, // Line tracking would require more complex implementation
                    column: None,
                    message: "Mutable default argument detected".to_string(),
                    context: None,
                });
            }
        }
    }

    fn check_class_patterns(&mut self, class_def: &ast::StmtClassDef) {
        let method_count = class_def
            .body
            .iter()
            .filter(|stmt| {
                matches!(
                    stmt,
                    ast::Stmt::FunctionDef(_) | ast::Stmt::AsyncFunctionDef(_)
                )
            })
            .count();

        if method_count > 20 {
            self.debt_items.push(DebtItem {
                id: format!("py-god-class-{}", self.debt_items.len()),
                debt_type: DebtType::CodeOrganization,
                priority: Priority::High,
                file: self.path.clone(),
                line: 1,
                column: None,
                message: format!(
                    "Class '{}' has {} methods (God Object)",
                    class_def.name.as_str(),
                    method_count
                ),
                context: None,
            });
        }
    }

    fn check_loop_patterns(&mut self, body: &[ast::Stmt]) {
        // Check for nested loops
        for stmt in body {
            if matches!(stmt, ast::Stmt::For(_) | ast::Stmt::While(_)) {
                self.debt_items.push(DebtItem {
                    id: format!("py-nested-loop-{}", self.debt_items.len()),
                    debt_type: DebtType::Complexity,
                    priority: Priority::Medium,
                    file: self.path.clone(),
                    line: 1,
                    column: None,
                    message: "Nested loop detected - increases code complexity".to_string(),
                    context: None,
                });
            }
        }
    }

    fn is_mutable_default(&self, expr: &ast::Expr) -> bool {
        matches!(
            expr,
            ast::Expr::List(_) | ast::Expr::Dict(_) | ast::Expr::Set(_)
        )
    }

    pub fn get_debt_items(self) -> Vec<DebtItem> {
        self.debt_items
    }
}
