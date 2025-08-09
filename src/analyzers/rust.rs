use crate::analyzers::Analyzer;
use crate::complexity::{cognitive::calculate_cognitive, cyclomatic::calculate_cyclomatic};
use crate::core::{
    ast::{Ast, RustAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::find_todos_and_fixmes;
use anyhow::Result;
use std::path::PathBuf;
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
    let mut visitor = FunctionVisitor::new(ast.path.clone());
    visitor.visit_file(&ast.file);

    let debt_items = extract_debt_items(&ast.file, &ast.path, threshold, &visitor.functions);
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

struct FunctionVisitor {
    functions: Vec<FunctionMetrics>,
    current_file: PathBuf,
}

impl FunctionVisitor {
    fn new(file: PathBuf) -> Self {
        Self {
            functions: Vec::new(),
            current_file: file,
        }
    }

    fn analyze_function(&mut self, name: String, item_fn: &syn::ItemFn, line: usize) {
        let mut metrics = FunctionMetrics::new(name, self.current_file.clone(), line);

        metrics.cyclomatic = calculate_cyclomatic_syn(&item_fn.block);
        metrics.cognitive = calculate_cognitive_syn(&item_fn.block);
        metrics.nesting = calculate_nesting(&item_fn.block);
        metrics.length = count_lines(&item_fn.block);

        self.functions.push(metrics);
    }
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_fn(&mut self, item_fn: &'ast syn::ItemFn) {
        let name = item_fn.sig.ident.to_string();
        let line = 0;
        self.analyze_function(name, item_fn, line);
        syn::visit::visit_item_fn(self, item_fn);
    }

    fn visit_impl_item_fn(&mut self, impl_fn: &'ast syn::ImplItemFn) {
        let name = impl_fn.sig.ident.to_string();
        let line = 0;
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
    file: &syn::File,
    path: &PathBuf,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    let mut items = Vec::new();

    let file_content = quote::quote! { #file }.to_string();
    items.extend(find_todos_and_fixmes(&file_content, path));

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

fn extract_dependencies(file: &syn::File) -> Vec<Dependency> {
    let mut deps = Vec::new();

    for item in &file.items {
        if let Item::Use(use_item) = item {
            if let Some(dep_name) = extract_use_name(&use_item.tree) {
                deps.push(Dependency {
                    name: dep_name,
                    kind: DependencyKind::Import,
                });
            }
        }
    }

    deps
}

fn extract_use_name(tree: &syn::UseTree) -> Option<String> {
    match tree {
        syn::UseTree::Path(path) => Some(path.ident.to_string()),
        syn::UseTree::Name(name) => Some(name.ident.to_string()),
        _ => None,
    }
}
