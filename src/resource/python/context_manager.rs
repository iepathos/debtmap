/// Context manager usage detection for Python resources
use super::{
    AffectedScope, ImpactLevel, PythonResourceDetector, PythonResourceIssueType, ResourceImpact,
    ResourceIssue, ResourceLocation, ResourceSeverity,
};
use rustpython_parser::ast::{self, Expr, Stmt};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct PythonContextManagerDetector {
    resource_types: HashSet<String>,
    _safe_patterns: Vec<String>,
    resource_functions: HashMap<String, String>, // function -> resource type
}

impl Default for PythonContextManagerDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonContextManagerDetector {
    pub fn new() -> Self {
        let mut resource_types = HashSet::new();
        // Common resource types that should use context managers
        resource_types.insert("open".to_string());
        resource_types.insert("file".to_string());
        resource_types.insert("socket".to_string());
        resource_types.insert("connect".to_string());
        resource_types.insert("urlopen".to_string());
        resource_types.insert("Session".to_string());
        resource_types.insert("Connection".to_string());
        resource_types.insert("Lock".to_string());
        resource_types.insert("Semaphore".to_string());
        resource_types.insert("Pool".to_string());
        resource_types.insert("TemporaryFile".to_string());
        resource_types.insert("NamedTemporaryFile".to_string());

        let safe_patterns = vec![
            "with".to_string(),
            "contextmanager".to_string(),
            "__enter__".to_string(),
            "__exit__".to_string(),
        ];

        let mut resource_functions = HashMap::new();
        resource_functions.insert("open".to_string(), "file".to_string());
        resource_functions.insert("socket".to_string(), "socket".to_string());
        resource_functions.insert("urlopen".to_string(), "url connection".to_string());
        resource_functions.insert("connect".to_string(), "database connection".to_string());
        resource_functions.insert("Lock".to_string(), "lock".to_string());
        resource_functions.insert("Pool".to_string(), "connection pool".to_string());

        Self {
            resource_types,
            _safe_patterns: safe_patterns,
            resource_functions,
        }
    }

    fn check_statement(&self, stmt: &Stmt, issues: &mut Vec<ResourceIssue>) {
        match stmt {
            Stmt::Assign(assign) => {
                // Check for resource assignments without context manager
                if let Some(resource_info) = self.extract_resource_from_expr(&assign.value) {
                    let var_name = self.extract_target_name(&assign.targets);
                    issues.push(ResourceIssue {
                        issue_type: PythonResourceIssueType::MissingContextManager {
                            resource_type: resource_info.1,
                            variable_name: var_name,
                        },
                        severity: ResourceSeverity::High,
                        location: ResourceLocation {
                            line: 1, // TODO: Track actual line numbers
                            column: 0,
                            end_line: None,
                            end_column: None,
                        },
                        suggestion: format!("Use 'with {}(...) as ...:'", resource_info.0),
                    });
                }
            }
            Stmt::With(with_stmt) => {
                // With statements are safe, check body recursively
                for stmt in &with_stmt.body {
                    self.check_statement(stmt, issues);
                }
            }
            Stmt::FunctionDef(func) => {
                // Check function body
                for stmt in &func.body {
                    self.check_statement(stmt, issues);
                }
            }
            Stmt::ClassDef(class) => {
                // Check class body
                for stmt in &class.body {
                    self.check_statement(stmt, issues);
                }
            }
            Stmt::If(if_stmt) => {
                // Check if branches
                for stmt in &if_stmt.body {
                    self.check_statement(stmt, issues);
                }
                for stmt in &if_stmt.orelse {
                    self.check_statement(stmt, issues);
                }
            }
            Stmt::Try(try_stmt) => {
                // Check try/except/finally blocks
                for stmt in &try_stmt.body {
                    self.check_statement(stmt, issues);
                }
                // handlers have a different structure, skip for now
                for stmt in &try_stmt.orelse {
                    self.check_statement(stmt, issues);
                }
                for stmt in &try_stmt.finalbody {
                    self.check_statement(stmt, issues);
                }
            }
            Stmt::For(for_stmt) => {
                for stmt in &for_stmt.body {
                    self.check_statement(stmt, issues);
                }
                for stmt in &for_stmt.orelse {
                    self.check_statement(stmt, issues);
                }
            }
            Stmt::While(while_stmt) => {
                for stmt in &while_stmt.body {
                    self.check_statement(stmt, issues);
                }
                for stmt in &while_stmt.orelse {
                    self.check_statement(stmt, issues);
                }
            }
            _ => {}
        }
    }

    fn extract_resource_from_expr(&self, expr: &Expr) -> Option<(String, String)> {
        if let Expr::Call(call) = expr {
            if let Expr::Name(name) = &call.func.as_ref() {
                let func_name = &name.id;
                let func_name_str = func_name.to_string();
                if self.resource_functions.contains_key(&func_name_str) {
                    return Some((
                        func_name_str.clone(),
                        self.resource_functions[&func_name_str].clone(),
                    ));
                }
            } else if let Expr::Attribute(attr) = &call.func.as_ref() {
                let attr_name = &attr.attr;
                let attr_name_str = attr_name.to_string();
                if self.resource_types.contains(&attr_name_str) {
                    return Some((attr_name_str.clone(), attr_name_str.clone()));
                }
            }
        }
        None
    }

    fn extract_target_name(&self, targets: &[Expr]) -> String {
        if let Some(Expr::Name(name)) = targets.first() {
            return name.id.to_string();
        }
        "<unknown>".to_string()
    }
}

impl PythonResourceDetector for PythonContextManagerDetector {
    fn detect_issues(&self, module: &ast::Mod, _path: &Path) -> Vec<ResourceIssue> {
        let mut issues = Vec::new();

        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                self.check_statement(stmt, &mut issues);
            }
        }

        issues
    }

    fn assess_resource_impact(&self, issue: &ResourceIssue) -> ResourceImpact {
        let impact_level = match issue.severity {
            ResourceSeverity::Critical => ImpactLevel::Critical,
            ResourceSeverity::High => ImpactLevel::High,
            ResourceSeverity::Medium => ImpactLevel::Medium,
            ResourceSeverity::Low => ImpactLevel::Low,
        };

        ResourceImpact {
            impact_level,
            affected_scope: AffectedScope::Function,
            estimated_severity: match issue.severity {
                ResourceSeverity::Critical => 1.0,
                ResourceSeverity::High => 0.8,
                ResourceSeverity::Medium => 0.5,
                ResourceSeverity::Low => 0.3,
            },
        }
    }
}
