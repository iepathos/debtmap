use crate::analyzers::Analyzer;
use crate::core::{
    ast::{Ast, PythonAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::parse_suppression_comments;
use anyhow::Result;
use rustpython_parser::ast;
use std::path::{Path, PathBuf};

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

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
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
    let source_content = std::fs::read_to_string(&ast.path).unwrap_or_default();
    let functions = extract_function_metrics(&ast.module, &ast.path, &source_content);
    let debt_items = create_python_debt_items(
        &ast.module,
        &ast.path,
        threshold,
        &functions,
        &source_content,
    );
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

fn create_python_debt_items(
    module: &ast::Mod,
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
) -> Vec<DebtItem> {
    // Parse suppression comments
    let suppression_context = parse_suppression_comments(source_content, Language::Python, &path.to_path_buf());

    let complexity_items = extract_debt_items(module, path, threshold, functions);
    let todo_items =
        find_todos_and_fixmes_with_suppression(source_content, path, Some(&suppression_context));
    let code_smell_items =
        find_code_smells_with_suppression(source_content, path, Some(&suppression_context));

    let module_smells = analyze_module_smells(path, source_content.lines().count())
        .into_iter()
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type));

    let function_smells = functions
        .iter()
        .flat_map(|func| {
            let param_count = count_python_params(module, &func.name);
            analyze_function_smells(func, param_count)
        })
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type));

    // Report unclosed blocks as warnings
    for unclosed in &suppression_context.unclosed_blocks {
        eprintln!(
            "Warning: Unclosed suppression block in {} at line {}",
            unclosed.file.display(),
            unclosed.start_line
        );
    }

    complexity_items
        .into_iter()
        .chain(todo_items)
        .chain(code_smell_items)
        .chain(module_smells)
        .chain(function_smells)
        .collect()
}

fn extract_function_metrics(
    module: &ast::Mod,
    path: &Path,
    source_content: &str,
) -> Vec<FunctionMetrics> {
    let ast::Mod::Module(module) = module else {
        return Vec::new();
    };

    let lines: Vec<&str> = source_content.lines().collect();

    module
        .body
        .iter()
        .enumerate()
        .filter_map(|(stmt_idx, stmt)| match stmt {
            ast::Stmt::FunctionDef(func_def) => {
                let line_number =
                    estimate_line_number(&lines, &func_def.name.to_string(), stmt_idx);
                Some(FunctionMetrics {
                    name: func_def.name.to_string(),
                    file: path.to_path_buf(),
                    line: line_number,
                    cyclomatic: calculate_cyclomatic_python(&func_def.body),
                    cognitive: calculate_cognitive_python(&func_def.body),
                    nesting: calculate_nesting_python(&func_def.body),
                    length: func_def.body.len(),
                })
            }
            _ => None,
        })
        .collect()
}

fn estimate_line_number(lines: &[&str], func_name: &str, _stmt_idx: usize) -> usize {
    let def_pattern = format!("def {}", func_name);
    lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.trim_start().starts_with(&def_pattern))
        .map(|(idx, _)| idx + 1) // Line numbers are 1-based
        .unwrap_or(1) // Default to line 1 if not found
}

fn count_python_params(module: &ast::Mod, func_name: &str) -> usize {
    let ast::Mod::Module(module) = module else {
        return 0;
    };

    module
        .body
        .iter()
        .find_map(|stmt| match stmt {
            ast::Stmt::FunctionDef(func_def) if func_def.name.to_string() == func_name => {
                Some(func_def.args.args.len())
            }
            _ => None,
        })
        .unwrap_or(0)
}

fn calculate_cyclomatic_python(body: &[ast::Stmt]) -> u32 {
    1 + body.iter().map(count_branches_stmt).sum::<u32>()
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
    let mut nesting = 0;
    body.iter()
        .map(|stmt| calculate_cognitive_stmt(stmt, &mut nesting))
        .sum()
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
    body.iter()
        .map(|stmt| calculate_nesting_stmt(stmt, 0))
        .max()
        .unwrap_or(0)
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
    _path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|func| func.is_complex(threshold))
        .map(|func| create_python_complexity_debt_item(func, threshold))
        .collect()
}

fn create_python_complexity_debt_item(func: &FunctionMetrics, threshold: u32) -> DebtItem {
    DebtItem {
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
    }
}

fn extract_dependencies(module: &ast::Mod) -> Vec<Dependency> {
    let ast::Mod::Module(module) = module else {
        return Vec::new();
    };

    module
        .body
        .iter()
        .flat_map(extract_stmt_dependencies)
        .collect()
}

fn extract_stmt_dependencies(stmt: &ast::Stmt) -> Vec<Dependency> {
    match stmt {
        ast::Stmt::Import(import) => import
            .names
            .iter()
            .map(|alias| Dependency {
                name: alias.name.to_string(),
                kind: DependencyKind::Import,
            })
            .collect(),
        ast::Stmt::ImportFrom(import_from) => import_from
            .module
            .as_ref()
            .map(|module| Dependency {
                name: module.to_string(),
                kind: DependencyKind::Module,
            })
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}
