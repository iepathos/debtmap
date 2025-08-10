use crate::analyzers::Analyzer;
use crate::complexity::python_patterns::analyze_python_patterns;
use crate::core::{
    ast::{Ast, PythonAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
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
                complexity: ComplexityMetrics::default(),
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

    let (cyclomatic, cognitive) = functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    });

    FileMetrics {
        path: ast.path.clone(),
        language: Language::Python,
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
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
    let suppression_context = parse_suppression_comments(source_content, Language::Python, path);

    report_unclosed_blocks(&suppression_context);

    collect_all_debt_items(
        module,
        path,
        threshold,
        functions,
        source_content,
        &suppression_context,
    )
}

fn collect_all_debt_items(
    module: &ast::Mod,
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    [
        extract_debt_items(module, path, threshold, functions),
        find_todos_and_fixmes_with_suppression(source_content, path, Some(suppression_context)),
        find_code_smells_with_suppression(source_content, path, Some(suppression_context)),
        extract_module_smell_items(path, source_content, suppression_context),
        extract_function_smell_items(module, functions, suppression_context),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn extract_module_smell_items(
    path: &Path,
    source_content: &str,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    analyze_module_smells(path, source_content.lines().count())
        .into_iter()
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn extract_function_smell_items(
    module: &ast::Mod,
    functions: &[FunctionMetrics],
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|func| !func.is_test)
        .flat_map(|func| {
            let param_count = count_python_params(module, &func.name);
            analyze_function_smells(func, param_count)
        })
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn report_unclosed_blocks(suppression_context: &SuppressionContext) {
    suppression_context
        .unclosed_blocks
        .iter()
        .for_each(|unclosed| {
            eprintln!(
                "Warning: Unclosed suppression block in {} at line {}",
                unclosed.file.display(),
                unclosed.start_line
            );
        });
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
    let mut functions = Vec::new();

    // Recursively extract functions from the module
    extract_functions_from_stmts(&module.body, path, &lines, &mut functions, 0);

    functions
}

fn extract_functions_from_stmts(
    stmts: &[ast::Stmt],
    path: &Path,
    lines: &[&str],
    functions: &mut Vec<FunctionMetrics>,
    stmt_offset: usize,
) {
    for (idx, stmt) in stmts.iter().enumerate() {
        match stmt {
            ast::Stmt::FunctionDef(func_def) => {
                let line_number =
                    estimate_line_number(lines, func_def.name.as_ref(), stmt_offset + idx);
                functions.push(FunctionMetrics {
                    name: func_def.name.to_string(),
                    file: path.to_path_buf(),
                    line: line_number,
                    cyclomatic: calculate_cyclomatic_python(&func_def.body),
                    cognitive: calculate_cognitive_python(&func_def.body),
                    nesting: calculate_nesting_python(&func_def.body),
                    length: func_def.body.len(),
                    is_test: func_def.name.starts_with("test_"),
                });

                // Recursively look for nested functions
                extract_functions_from_stmts(
                    &func_def.body,
                    path,
                    lines,
                    functions,
                    stmt_offset + idx,
                );
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                let line_number =
                    estimate_line_number(lines, func_def.name.as_ref(), stmt_offset + idx);
                functions.push(FunctionMetrics {
                    name: format!("async {}", func_def.name),
                    file: path.to_path_buf(),
                    line: line_number,
                    cyclomatic: calculate_cyclomatic_python(&func_def.body),
                    cognitive: calculate_cognitive_python(&func_def.body),
                    nesting: calculate_nesting_python(&func_def.body),
                    length: func_def.body.len(),
                    is_test: func_def.name.starts_with("test_"),
                });

                // Recursively look for nested functions
                extract_functions_from_stmts(
                    &func_def.body,
                    path,
                    lines,
                    functions,
                    stmt_offset + idx,
                );
            }
            ast::Stmt::ClassDef(class_def) => {
                // Look for methods in classes
                extract_functions_from_stmts(
                    &class_def.body,
                    path,
                    lines,
                    functions,
                    stmt_offset + idx,
                );
            }
            _ => {}
        }
    }
}

fn estimate_line_number(lines: &[&str], func_name: &str, _stmt_idx: usize) -> usize {
    let def_pattern = format!("def {func_name}");
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
    use ast::Stmt::*;

    match stmt {
        If(if_stmt) => count_if_branches(if_stmt),
        While(while_stmt) => count_loop_branches(&while_stmt.body),
        For(for_stmt) => count_loop_branches(&for_stmt.body),
        Try(try_stmt) => count_try_branches(try_stmt),
        With(with_stmt) => count_body_branches(&with_stmt.body),
        Match(match_stmt) => count_match_branches(match_stmt),
        _ => 0,
    }
}

fn count_if_branches(if_stmt: &ast::StmtIf) -> u32 {
    let base_count = 1;
    let body_count = count_body_branches(&if_stmt.body);
    let else_count = count_else_branches(&if_stmt.orelse);

    base_count + body_count + else_count
}

fn count_else_branches(orelse: &[ast::Stmt]) -> u32 {
    if orelse.is_empty() {
        return 0;
    }

    let is_elif = matches!(orelse.first(), Some(ast::Stmt::If(_)));
    let else_branch_count = if is_elif { 0 } else { 1 };
    let nested_count = count_body_branches(orelse);

    else_branch_count + nested_count
}

fn count_loop_branches(body: &[ast::Stmt]) -> u32 {
    1 + count_body_branches(body)
}

fn count_try_branches(try_stmt: &ast::StmtTry) -> u32 {
    let handler_count = try_stmt.handlers.len() as u32;
    let body_count = count_body_branches(&try_stmt.body);
    handler_count + body_count
}

fn count_body_branches(body: &[ast::Stmt]) -> u32 {
    body.iter().map(count_branches_stmt).sum()
}

fn count_match_branches(match_stmt: &ast::StmtMatch) -> u32 {
    match_stmt.cases.len().saturating_sub(1) as u32
}

fn calculate_cognitive_python(body: &[ast::Stmt]) -> u32 {
    let mut nesting = 0;
    let base_cognitive: u32 = body
        .iter()
        .map(|stmt| calculate_cognitive_stmt(stmt, &mut nesting))
        .sum();

    // Add pattern-based complexity
    let patterns = analyze_python_patterns(body);
    base_cognitive + patterns.total_complexity()
}

fn calculate_cognitive_stmt(stmt: &ast::Stmt, nesting: &mut u32) -> u32 {
    let bodies = extract_stmt_bodies(stmt);
    if bodies.is_empty() {
        return 0;
    }

    let base_cognitive = 1 + *nesting;
    *nesting += 1;
    let body_cognitive = bodies
        .into_iter()
        .flatten()
        .map(|s| calculate_cognitive_stmt(s, nesting))
        .sum::<u32>();
    *nesting -= 1;
    base_cognitive + body_cognitive
}

fn calculate_nesting_python(body: &[ast::Stmt]) -> u32 {
    body.iter()
        .map(|stmt| calculate_nesting_stmt(stmt, 0))
        .max()
        .unwrap_or(0)
}

fn calculate_nesting_stmt(stmt: &ast::Stmt, current_depth: u32) -> u32 {
    let bodies = extract_stmt_bodies(stmt);
    if bodies.is_empty() {
        return current_depth;
    }

    let next_depth = current_depth + 1;
    bodies
        .into_iter()
        .flatten()
        .map(|s| calculate_nesting_stmt(s, next_depth))
        .max()
        .unwrap_or(next_depth)
}

fn extract_stmt_bodies(stmt: &ast::Stmt) -> Vec<&[ast::Stmt]> {
    match stmt {
        ast::Stmt::If(if_stmt) => vec![&if_stmt.body[..], &if_stmt.orelse[..]],
        ast::Stmt::While(while_stmt) => vec![&while_stmt.body[..]],
        ast::Stmt::For(for_stmt) => vec![&for_stmt.body[..]],
        _ => vec![],
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
        .filter(|func| !func.is_test && func.is_complex(threshold))
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
