use crate::analyzers::Analyzer;
use crate::core::{
    ast::{Ast, PythonAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use anyhow::Result;
use rustpython_parser::ast;
use std::path::PathBuf;

pub struct PythonAnalyzer {
    complexity_threshold: u32,
}

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
        }
    }
}

impl Analyzer for PythonAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let module = rustpython_parser::parse(content, rustpython_parser::Mode::Module, "<module>")
            .map_err(|e| anyhow::anyhow!("Python parse error: {:?}", e))?;
        Ok(Ast::Python(PythonAst { module, path }))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Python(python_ast) => analyze_python_file(python_ast, self.complexity_threshold),
            _ => FileMetrics {
                path: PathBuf::new(),
                language: Language::Python,
                complexity: ComplexityMetrics { functions: vec![] },
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
            },
        }
    }

    fn language(&self) -> Language {
        Language::Python
    }
}

fn analyze_python_file(ast: &PythonAst, threshold: u32) -> FileMetrics {
    let functions = extract_function_metrics(&ast.module, &ast.path);
    let debt_items = extract_debt_items(&ast.module, &ast.path, threshold, &functions);
    let dependencies = extract_dependencies(&ast.module);

    FileMetrics {
        path: ast.path.clone(),
        language: Language::Python,
        complexity: ComplexityMetrics { functions },
        debt_items,
        dependencies,
        duplications: vec![],
    }
}

fn extract_function_metrics(module: &ast::Mod, path: &PathBuf) -> Vec<FunctionMetrics> {
    let mut functions = Vec::new();

    if let ast::Mod::Module(module) = module {
        for stmt in &module.body {
            if let ast::Stmt::FunctionDef(func_def) = stmt {
                let mut metrics = FunctionMetrics::new(func_def.name.to_string(), path.clone(), 0);

                metrics.cyclomatic = calculate_cyclomatic_python(&func_def.body);
                metrics.cognitive = calculate_cognitive_python(&func_def.body);
                metrics.nesting = calculate_nesting_python(&func_def.body);
                metrics.length = func_def.body.len();

                functions.push(metrics);
            }
        }
    }

    functions
}

fn calculate_cyclomatic_python(body: &[ast::Stmt]) -> u32 {
    let mut complexity = 1;

    for stmt in body {
        complexity += count_branches_stmt(stmt);
    }

    complexity
}

fn count_branches_stmt(stmt: &ast::Stmt) -> u32 {
    match stmt {
        ast::Stmt::If(_) => 1,
        ast::Stmt::While(_) => 1,
        ast::Stmt::For(_) => 1,
        ast::Stmt::Try(try_stmt) => try_stmt.handlers.len() as u32,
        ast::Stmt::With(with_stmt) => {
            let mut count = 0;
            for stmt in &with_stmt.body {
                count += count_branches_stmt(stmt);
            }
            count
        }
        _ => 0,
    }
}

fn calculate_cognitive_python(body: &[ast::Stmt]) -> u32 {
    let mut cognitive = 0;
    let mut nesting = 0;

    for stmt in body {
        cognitive += calculate_cognitive_stmt(stmt, &mut nesting);
    }

    cognitive
}

fn calculate_cognitive_stmt(stmt: &ast::Stmt, nesting: &mut u32) -> u32 {
    match stmt {
        ast::Stmt::If(if_stmt) => {
            let mut cognitive = 1 + *nesting;
            *nesting += 1;
            for s in &if_stmt.body {
                cognitive += calculate_cognitive_stmt(s, nesting);
            }
            for s in &if_stmt.orelse {
                cognitive += calculate_cognitive_stmt(s, nesting);
            }
            *nesting -= 1;
            cognitive
        }
        ast::Stmt::While(while_stmt) => {
            let mut cognitive = 1 + *nesting;
            *nesting += 1;
            for s in &while_stmt.body {
                cognitive += calculate_cognitive_stmt(s, nesting);
            }
            *nesting -= 1;
            cognitive
        }
        ast::Stmt::For(for_stmt) => {
            let mut cognitive = 1 + *nesting;
            *nesting += 1;
            for s in &for_stmt.body {
                cognitive += calculate_cognitive_stmt(s, nesting);
            }
            *nesting -= 1;
            cognitive
        }
        _ => 0,
    }
}

fn calculate_nesting_python(body: &[ast::Stmt]) -> u32 {
    let mut max_nesting = 0;

    for stmt in body {
        max_nesting = max_nesting.max(calculate_nesting_stmt(stmt, 0));
    }

    max_nesting
}

fn calculate_nesting_stmt(stmt: &ast::Stmt, current_depth: u32) -> u32 {
    match stmt {
        ast::Stmt::If(if_stmt) => {
            let mut max = current_depth + 1;
            for s in &if_stmt.body {
                max = max.max(calculate_nesting_stmt(s, current_depth + 1));
            }
            for s in &if_stmt.orelse {
                max = max.max(calculate_nesting_stmt(s, current_depth + 1));
            }
            max
        }
        ast::Stmt::While(while_stmt) => {
            let mut max = current_depth + 1;
            for s in &while_stmt.body {
                max = max.max(calculate_nesting_stmt(s, current_depth + 1));
            }
            max
        }
        ast::Stmt::For(for_stmt) => {
            let mut max = current_depth + 1;
            for s in &for_stmt.body {
                max = max.max(calculate_nesting_stmt(s, current_depth + 1));
            }
            max
        }
        _ => current_depth,
    }
}

fn extract_debt_items(
    _module: &ast::Mod,
    _path: &PathBuf,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    let mut items = Vec::new();

    for func in functions {
        if func.is_complex(threshold) {
            items.push(DebtItem {
                id: format!("complexity-{}-{}", func.file.display(), func.line),
                debt_type: DebtType::Complexity,
                priority: if func.cyclomatic > threshold * 2 {
                    Priority::High
                } else {
                    Priority::Medium
                },
                file: func.file.clone(),
                line: func.line,
                message: format!(
                    "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
                    func.name, func.cyclomatic, func.cognitive
                ),
                context: None,
            });
        }
    }

    items
}

fn extract_dependencies(module: &ast::Mod) -> Vec<Dependency> {
    let mut deps = Vec::new();

    if let ast::Mod::Module(module) = module {
        for stmt in &module.body {
            match stmt {
                ast::Stmt::Import(import) => {
                    for alias in &import.names {
                        deps.push(Dependency {
                            name: alias.name.to_string(),
                            kind: DependencyKind::Import,
                        });
                    }
                }
                ast::Stmt::ImportFrom(import_from) => {
                    if let Some(module) = &import_from.module {
                        deps.push(Dependency {
                            name: module.to_string(),
                            kind: DependencyKind::Module,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    deps
}
