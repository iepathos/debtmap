use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, File};

pub fn detect_sql_injection(file: &File, path: &Path) -> Vec<DebtItem> {
    let mut visitor = SqlInjectionVisitor::new(path);
    visitor.visit_file(file);
    visitor.debt_items
}

struct SqlInjectionVisitor {
    path: std::path::PathBuf,
    debt_items: Vec<DebtItem>,
}

impl SqlInjectionVisitor {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            debt_items: Vec::new(),
        }
    }

    fn check_sql_pattern(&mut self, expr_str: &str, line: usize) {
        let sql_keywords = [
            "SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER", "FROM", "WHERE",
        ];
        let has_sql = sql_keywords.iter().any(|kw| expr_str.contains(kw));

        if !has_sql {
            return;
        }

        // Check for string concatenation patterns
        let dangerous_patterns = [
            ("format!", "String formatting in SQL query"),
            ("push_str", "String concatenation in SQL query"),
            ("+ &", "String concatenation operator in SQL"),
            ("concat!", "Macro concatenation in SQL"),
        ];

        for (pattern, description) in dangerous_patterns {
            if expr_str.contains(pattern) {
                self.debt_items.push(DebtItem {
                    id: format!("security-sql-{}-{}", self.path.display(), line),
                    debt_type: DebtType::Security,
                    priority: Priority::Critical,
                    file: self.path.clone(),
                    line,
                    column: None,
                    message: format!("Critical: SQL injection vulnerability - {}", description),
                    context: Some(
                        "Use parameterized queries or prepared statements instead".to_string(),
                    ),
                });
                return;
            }
        }

        // Check for user input patterns
        if expr_str.contains("user_input")
            || expr_str.contains("request")
            || expr_str.contains("params")
        {
            self.debt_items.push(DebtItem {
                id: format!("security-sql-input-{}-{}", self.path.display(), line),
                debt_type: DebtType::Security,
                priority: Priority::High,
                file: self.path.clone(),
                line,
                column: None,
                message: "Potential SQL injection - user input in query".to_string(),
                context: Some(
                    "Ensure proper input sanitization and use parameterized queries".to_string(),
                ),
            });
        }
    }
}

impl<'ast> Visit<'ast> for SqlInjectionVisitor {
    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();

        // Check for database query methods
        let query_methods = ["query", "execute", "exec", "raw_query", "raw", "sql"];
        if query_methods.contains(&method_name.as_str()) {
            let expr_str = quote::quote!(#i).to_string();
            self.check_sql_pattern(&expr_str, 0);
        }

        syn::visit::visit_expr_method_call(self, i);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        // Check for SQL query builders and raw SQL
        let expr_str = quote::quote!(#expr).to_string();
        if expr_str.contains("sql!") || expr_str.contains("query!") || expr_str.contains("raw_sql")
        {
            self.check_sql_pattern(&expr_str, 0);
        }

        syn::visit::visit_expr(self, expr);
    }
}
