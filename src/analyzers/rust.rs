use crate::analyzers::Analyzer;
use crate::complexity::{cognitive::calculate_cognitive, cyclomatic::calculate_cyclomatic};
use crate::core::{
    ast::{Ast, RustAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::parse_suppression_comments;
use anyhow::Result;
use std::path::{Path, PathBuf};
use syn::{visit::Visit, Item};

pub struct RustAnalyzer {
    complexity_threshold: u32,
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
        }
    }
}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RustAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let file = syn::parse_str::<syn::File>(content)?;
        Ok(Ast::Rust(RustAst { file, path }))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Rust(rust_ast) => analyze_rust_file(rust_ast, self.complexity_threshold),
            _ => FileMetrics {
                path: PathBuf::new(),
                language: Language::Rust,
                complexity: ComplexityMetrics { functions: vec![] },
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
            },
        }
    }

    fn language(&self) -> Language {
        Language::Rust
    }
}

fn analyze_rust_file(ast: &RustAst, threshold: u32) -> FileMetrics {
    let source_content = std::fs::read_to_string(&ast.path).unwrap_or_default();
    let mut visitor = FunctionVisitor::new(ast.path.clone(), source_content.clone());
    visitor.visit_file(&ast.file);

    let debt_items = create_debt_items(
        &ast.file,
        &ast.path,
        threshold,
        &visitor.functions,
        &source_content,
    );
    let dependencies = extract_dependencies(&ast.file);

    FileMetrics {
        path: ast.path.clone(),
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions: visitor.functions,
        },
        debt_items,
        dependencies,
        duplications: vec![],
    }
}

fn create_debt_items(
    file: &syn::File,
    path: &std::path::Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
) -> Vec<DebtItem> {
    // Parse suppression comments
    let suppression_context = parse_suppression_comments(source_content, Language::Rust, path);

    let complexity_items = extract_debt_items(file, path, threshold, functions);
    let todo_items =
        find_todos_and_fixmes_with_suppression(source_content, path, Some(&suppression_context));
    let code_smell_items =
        find_code_smells_with_suppression(source_content, path, Some(&suppression_context));

    let module_smells = analyze_module_smells(path, source_content.lines().count())
        .into_iter()
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect::<Vec<_>>();

    let function_smells = functions
        .iter()
        .flat_map(|func| analyze_function_smells(func, 0))
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect::<Vec<_>>();

    // Report unclosed blocks as warnings
    for unclosed in &suppression_context.unclosed_blocks {
        eprintln!(
            "Warning: Unclosed suppression block in {} at line {}",
            unclosed.file.display(),
            unclosed.start_line
        );
    }

    [
        complexity_items,
        todo_items,
        code_smell_items,
        module_smells,
        function_smells,
    ]
    .into_iter()
    .flatten()
    .collect()
}

struct FunctionVisitor {
    functions: Vec<FunctionMetrics>,
    current_file: PathBuf,
    #[allow(dead_code)]
    source_content: String,
}

impl FunctionVisitor {
    fn new(file: PathBuf, source_content: String) -> Self {
        Self {
            functions: Vec::new(),
            current_file: file,
            source_content,
        }
    }

    fn get_line_number(&self, _span: syn::__private::Span) -> usize {
        // For now, return a default line number
        // Proper line number tracking would require more sophisticated span handling
        1
    }

    fn analyze_function(&mut self, name: String, item_fn: &syn::ItemFn, line: usize) {
        let metrics = FunctionMetrics {
            name,
            file: self.current_file.clone(),
            line,
            cyclomatic: calculate_cyclomatic_syn(&item_fn.block),
            cognitive: calculate_cognitive_syn(&item_fn.block),
            nesting: calculate_nesting(&item_fn.block),
            length: count_lines(&item_fn.block),
        };

        self.functions.push(metrics);
    }
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_fn(&mut self, item_fn: &'ast syn::ItemFn) {
        let name = item_fn.sig.ident.to_string();
        let line = self.get_line_number(item_fn.sig.ident.span());
        self.analyze_function(name, item_fn, line);
        syn::visit::visit_item_fn(self, item_fn);
    }

    fn visit_impl_item_fn(&mut self, impl_fn: &'ast syn::ImplItemFn) {
        let name = impl_fn.sig.ident.to_string();
        let line = self.get_line_number(impl_fn.sig.ident.span());
        let item_fn = syn::ItemFn {
            attrs: impl_fn.attrs.clone(),
            vis: syn::Visibility::Inherited,
            sig: impl_fn.sig.clone(),
            block: Box::new(impl_fn.block.clone()),
        };
        self.analyze_function(name, &item_fn, line);
        syn::visit::visit_impl_item_fn(self, impl_fn);
    }
}

fn calculate_cyclomatic_syn(block: &syn::Block) -> u32 {
    calculate_cyclomatic(block)
}

fn calculate_cognitive_syn(block: &syn::Block) -> u32 {
    calculate_cognitive(block)
}

fn calculate_nesting(block: &syn::Block) -> u32 {
    struct NestingVisitor {
        current_depth: u32,
        max_depth: u32,
    }

    impl<'ast> Visit<'ast> for NestingVisitor {
        fn visit_block(&mut self, block: &'ast syn::Block) {
            self.current_depth += 1;
            self.max_depth = self.max_depth.max(self.current_depth);
            syn::visit::visit_block(self, block);
            self.current_depth -= 1;
        }
    }

    let mut visitor = NestingVisitor {
        current_depth: 0,
        max_depth: 0,
    };
    visitor.visit_block(block);
    visitor.max_depth
}

fn count_lines(block: &syn::Block) -> usize {
    let tokens = quote::quote! { #block };
    tokens.to_string().lines().count()
}

fn extract_debt_items(
    _file: &syn::File,
    _path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|func| func.is_complex(threshold))
        .map(|func| create_complexity_debt_item(func, threshold))
        .collect()
}

fn create_complexity_debt_item(func: &FunctionMetrics, threshold: u32) -> DebtItem {
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

fn extract_dependencies(file: &syn::File) -> Vec<Dependency> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Use(use_item) => extract_use_name(&use_item.tree).map(|name| Dependency {
                name,
                kind: DependencyKind::Import,
            }),
            _ => None,
        })
        .collect()
}

fn extract_use_name(tree: &syn::UseTree) -> Option<String> {
    match tree {
        syn::UseTree::Path(path) => Some(path.ident.to_string()),
        syn::UseTree::Name(name) => Some(name.ident.to_string()),
        _ => None,
    }
}
