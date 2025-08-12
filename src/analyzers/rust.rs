use crate::analyzers::Analyzer;
use crate::complexity::{
    cognitive::calculate_cognitive_with_patterns, cyclomatic::calculate_cyclomatic,
};
use crate::core::{
    ast::{Ast, RustAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use crate::priority::call_graph::CallGraph;
use anyhow::Result;
use quote::ToTokens;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
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
                complexity: ComplexityMetrics::default(),
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

pub fn extract_rust_call_graph(ast: &RustAst) -> CallGraph {
    use super::rust_call_graph::extract_call_graph;
    extract_call_graph(&ast.file, &ast.path)
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

    let functions = visitor.functions;
    let (cyclomatic, cognitive) = functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    });

    FileMetrics {
        path: ast.path.clone(),
        language: Language::Rust,
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

fn create_debt_items(
    file: &syn::File,
    path: &std::path::Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
) -> Vec<DebtItem> {
    let suppression_context = parse_suppression_comments(source_content, Language::Rust, path);

    report_rust_unclosed_blocks(&suppression_context);

    collect_all_rust_debt_items(
        file,
        path,
        threshold,
        functions,
        source_content,
        &suppression_context,
    )
}

fn collect_all_rust_debt_items(
    file: &syn::File,
    path: &std::path::Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    [
        extract_debt_items(file, path, threshold, functions),
        find_todos_and_fixmes_with_suppression(source_content, path, Some(suppression_context)),
        find_code_smells_with_suppression(source_content, path, Some(suppression_context)),
        extract_rust_module_smell_items(path, source_content, suppression_context),
        extract_rust_function_smell_items(functions, suppression_context),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn extract_rust_module_smell_items(
    path: &std::path::Path,
    source_content: &str,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    analyze_module_smells(path, source_content.lines().count())
        .into_iter()
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn extract_rust_function_smell_items(
    functions: &[FunctionMetrics],
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    functions
        .iter()
        .flat_map(|func| analyze_function_smells(func, 0))
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn report_rust_unclosed_blocks(suppression_context: &SuppressionContext) {
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

struct FunctionVisitor {
    functions: Vec<FunctionMetrics>,
    current_file: PathBuf,
    #[allow(dead_code)]
    source_content: String,
    in_test_module: bool,
}

impl FunctionVisitor {
    fn new(file: PathBuf, source_content: String) -> Self {
        Self {
            functions: Vec::new(),
            current_file: file,
            source_content,
            in_test_module: false,
        }
    }

    fn get_line_number(&self, span: syn::__private::Span) -> usize {
        // Use proc-macro2's span-locations feature to get actual line numbers
        span.start().line
    }

    fn analyze_function(&mut self, name: String, item_fn: &syn::ItemFn, line: usize) {
        // Check if this is a test function (either has #[test] attribute or is in a test module)
        let has_test_attribute = item_fn.attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || (attr.path().is_ident("cfg")
                    && attr.meta.to_token_stream().to_string().contains("test"))
        });

        let is_test = has_test_attribute || self.in_test_module;

        // Extract visibility information
        let visibility = match &item_fn.vis {
            syn::Visibility::Public(_) => Some("pub".to_string()),
            syn::Visibility::Restricted(restricted) => {
                if restricted.path.is_ident("crate") {
                    Some("pub(crate)".to_string())
                } else {
                    Some(format!("pub({})", quote::quote!(#restricted.path)))
                }
            }
            syn::Visibility::Inherited => None,
        };

        let metrics = FunctionMetrics {
            name,
            file: self.current_file.clone(),
            line,
            cyclomatic: calculate_cyclomatic_syn(&item_fn.block),
            cognitive: calculate_cognitive_syn(&item_fn.block),
            nesting: calculate_nesting(&item_fn.block),
            length: count_function_lines(item_fn),
            is_test,
            visibility,
        };

        self.functions.push(metrics);
    }
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_mod(&mut self, item_mod: &'ast syn::ItemMod) {
        // Check if this is a test module (has #[cfg(test)] attribute)
        let is_test_mod = item_mod.attrs.iter().any(|attr| {
            attr.path().is_ident("cfg") && attr.meta.to_token_stream().to_string().contains("test")
        });

        let was_in_test_module = self.in_test_module;
        if is_test_mod {
            self.in_test_module = true;
        }

        // Continue visiting the module content
        syn::visit::visit_item_mod(self, item_mod);

        // Restore the previous state when leaving the module
        self.in_test_module = was_in_test_module;
    }

    fn visit_item_fn(&mut self, item_fn: &'ast syn::ItemFn) {
        let name = item_fn.sig.ident.to_string();
        let line = self.get_line_number(item_fn.sig.ident.span());
        self.analyze_function(name, item_fn, line);
        // Continue visiting to find nested functions
        syn::visit::visit_item_fn(self, item_fn);
    }

    fn visit_impl_item_fn(&mut self, impl_fn: &'ast syn::ImplItemFn) {
        let name = impl_fn.sig.ident.to_string();
        let line = self.get_line_number(impl_fn.sig.ident.span());
        // Use the actual visibility from impl_fn
        let vis = impl_fn.vis.clone();
        let item_fn = syn::ItemFn {
            attrs: impl_fn.attrs.clone(),
            vis,
            sig: impl_fn.sig.clone(),
            block: Box::new(impl_fn.block.clone()),
        };
        self.analyze_function(name, &item_fn, line);
        // Continue visiting to find nested items
        syn::visit::visit_impl_item_fn(self, impl_fn);
    }

    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        // Also count closures as functions, but only if they're non-trivial
        if let syn::Expr::Closure(closure) = expr {
            // Convert closure body to a block for analysis
            let block = match &*closure.body {
                syn::Expr::Block(expr_block) => expr_block.block.clone(),
                _ => {
                    // Wrap single expression in a block
                    syn::Block {
                        brace_token: Default::default(),
                        stmts: vec![syn::Stmt::Expr(*closure.body.clone(), None)],
                    }
                }
            };

            // Calculate metrics first to determine if closure is trivial
            let cyclomatic = calculate_cyclomatic(&block);
            let cognitive = calculate_cognitive_syn(&block);
            let nesting = calculate_nesting(&block);
            let length = count_lines(&block);

            // Only track substantial closures:
            // - Cognitive complexity > 1 (has some logic)
            // - OR length > 1 (multi-line)
            // - OR cyclomatic > 1 (has branches)
            if cognitive > 1 || length > 1 || cyclomatic > 1 {
                let name = format!("<closure@{}>", self.functions.len());
                let line = self.get_line_number(closure.body.span());

                let metrics = FunctionMetrics {
                    name,
                    file: self.current_file.clone(),
                    line,
                    cyclomatic,
                    cognitive,
                    nesting,
                    length,
                    is_test: self.in_test_module, // Closures in test modules are test-related
                    visibility: None,             // Closures are always private
                };

                self.functions.push(metrics);
            }
        }

        // Continue visiting
        syn::visit::visit_expr(self, expr);
    }
}

fn calculate_cyclomatic_syn(block: &syn::Block) -> u32 {
    calculate_cyclomatic(block)
}

fn calculate_cognitive_syn(block: &syn::Block) -> u32 {
    // Use the enhanced version that includes pattern detection
    let (total, _patterns) = calculate_cognitive_with_patterns(block);
    total
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
    // Get the span of the entire block to calculate actual source lines
    let span = block.span();
    let start_line = span.start().line;
    let end_line = span.end().line;

    // Calculate the number of lines the function spans
    if end_line >= start_line {
        end_line - start_line + 1
    } else {
        1 // Fallback for edge cases
    }
}

fn count_function_lines(item_fn: &syn::ItemFn) -> usize {
    // Get the span of the entire function (from signature to end of body)
    let span = item_fn.span();
    let start_line = span.start().line;
    let end_line = span.end().line;

    // Calculate the number of lines the function spans
    if end_line >= start_line {
        end_line - start_line + 1
    } else {
        1 // Fallback for edge cases
    }
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
