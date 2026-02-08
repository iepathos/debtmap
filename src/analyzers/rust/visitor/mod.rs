//! Function visitor implementation
//!
//! The FunctionVisitor walks the AST and collects function metrics.

pub mod closure_analysis;
pub mod function_analysis;
pub mod helpers;

use quote::ToTokens;
use std::path::PathBuf;
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::metadata::classify_test_file;
use super::types::{AnalysisResult, EnhancedFunctionAnalysis};
use crate::complexity::threshold_manager::{ComplexityThresholds, ThresholdPreset};
use crate::core::FunctionMetrics;

pub use closure_analysis::{
    build_closure_metrics, calculate_closure_complexity, convert_closure_to_block,
    generate_closure_name, is_substantial_closure,
};
pub use function_analysis::create_function_analysis_data;
pub use helpers::{create_function_context, get_line_number};

/// Visitor that collects function metrics from Rust AST
pub struct FunctionVisitor {
    pub functions: Vec<FunctionMetrics>,
    pub current_file: PathBuf,
    pub source_content: String,
    pub in_test_module: bool,
    pub current_function: Option<String>,
    pub current_impl_type: Option<String>,
    pub current_impl_is_trait: bool,
    pub current_trait_name: Option<String>,
    pub file_ast: Option<syn::File>,
    pub enhanced_analysis: Vec<EnhancedFunctionAnalysis>,
    pub enhanced_thresholds: ComplexityThresholds,
    pub enable_functional_analysis: bool,
    pub enable_rust_patterns: bool,
}

impl FunctionVisitor {
    pub fn new(file: PathBuf, source_content: String) -> Self {
        // Check if this file is a test file based on its path
        let is_test_file = classify_test_file(&file.to_string_lossy());

        Self {
            functions: Vec::new(),
            current_file: file,
            source_content,
            in_test_module: is_test_file,
            current_function: None,
            current_impl_type: None,
            current_impl_is_trait: false,
            current_trait_name: None,
            file_ast: None,
            enhanced_analysis: Vec::new(),
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            enable_functional_analysis: false,
            enable_rust_patterns: false,
        }
    }

    fn analyze_function(
        &mut self,
        name: String,
        item_fn: &syn::ItemFn,
        line: usize,
        is_trait_method: bool,
    ) {
        let context = create_function_context(
            name.clone(),
            self.current_file.clone(),
            line,
            is_trait_method,
            self.in_test_module,
            self.current_impl_type.clone(),
            self.current_trait_name.clone(),
        );

        let analysis_data = create_function_analysis_data(
            &name,
            item_fn,
            line,
            is_trait_method,
            context,
            self.file_ast.as_ref(),
            &self.source_content,
            &self.enhanced_thresholds,
            self.enable_functional_analysis,
            self.enable_rust_patterns,
        );

        self.enhanced_analysis.push(analysis_data.enhanced_analysis);
        self.functions.push(analysis_data.metrics);
    }

    fn analyze_closure(&mut self, closure: &syn::ExprClosure) {
        let block = convert_closure_to_block(closure);
        let complexity_metrics = calculate_closure_complexity(&block);

        if is_substantial_closure(&complexity_metrics) {
            let name =
                generate_closure_name(self.current_function.as_deref(), self.functions.len());
            let line = get_line_number(closure.body.span());

            let metrics = build_closure_metrics(
                closure,
                &block,
                &complexity_metrics,
                name,
                line,
                self.current_file.clone(),
                self.in_test_module,
            );
            self.functions.push(metrics);
        }
    }
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_impl(&mut self, item_impl: &'ast syn::ItemImpl) {
        // Extract the type name from the impl block
        let impl_type = if let syn::Type::Path(type_path) = &*item_impl.self_ty {
            type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
        } else {
            None
        };

        // Check if this is a trait implementation and extract trait name
        let (is_trait_impl, trait_name) = if let Some((_, trait_path, _)) = &item_impl.trait_ {
            let name = trait_path.segments.last().map(|seg| seg.ident.to_string());
            (true, name)
        } else {
            (false, None)
        };

        // Store the current impl type and trait status
        let prev_impl_type = self.current_impl_type.clone();
        let prev_impl_is_trait = self.current_impl_is_trait;
        let prev_trait_name = self.current_trait_name.clone();
        self.current_impl_type = impl_type;
        self.current_impl_is_trait = is_trait_impl;
        self.current_trait_name = trait_name;

        // Continue visiting the impl block
        syn::visit::visit_item_impl(self, item_impl);

        // Restore previous impl type and trait status
        self.current_impl_type = prev_impl_type;
        self.current_impl_is_trait = prev_impl_is_trait;
        self.current_trait_name = prev_trait_name;
    }

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
        let line = get_line_number(item_fn.sig.ident.span());
        self.analyze_function(name.clone(), item_fn, line, false);

        // Track the current function for closures
        let prev_function = self.current_function.clone();
        self.current_function = Some(name);

        // Continue visiting to find nested functions
        syn::visit::visit_item_fn(self, item_fn);

        // Restore previous function context
        self.current_function = prev_function;
    }

    fn visit_impl_item_fn(&mut self, impl_fn: &'ast syn::ImplItemFn) {
        // Construct the full function name including the impl type
        let method_name = impl_fn.sig.ident.to_string();
        let name = if let Some(ref impl_type) = self.current_impl_type {
            format!("{impl_type}::{method_name}")
        } else {
            method_name.clone()
        };

        let line = get_line_number(impl_fn.sig.ident.span());

        // For trait implementations, methods inherit the trait's visibility
        let vis = if self.current_impl_is_trait {
            syn::Visibility::Public(syn::Token![pub](impl_fn.sig.ident.span()))
        } else {
            impl_fn.vis.clone()
        };

        let item_fn = syn::ItemFn {
            attrs: impl_fn.attrs.clone(),
            vis,
            sig: impl_fn.sig.clone(),
            block: Box::new(impl_fn.block.clone()),
        };
        self.analyze_function(name.clone(), &item_fn, line, self.current_impl_is_trait);

        // Track the current function for closures
        let prev_function = self.current_function.clone();
        self.current_function = Some(name);

        // Continue visiting to find nested items
        syn::visit::visit_impl_item_fn(self, impl_fn);

        // Restore previous function context
        self.current_function = prev_function;
    }

    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        if let syn::Expr::Closure(closure) = expr {
            self.analyze_closure(closure);
        }
        syn::visit::visit_expr(self, expr);
    }
}

/// Create and configure a visitor
pub fn create_configured_visitor(
    path: PathBuf,
    source_content: String,
    enhanced_thresholds: ComplexityThresholds,
    file_ast: Option<syn::File>,
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
) -> FunctionVisitor {
    let mut visitor = FunctionVisitor::new(path, source_content);
    visitor.file_ast = file_ast;
    visitor.enhanced_thresholds = enhanced_thresholds;
    visitor.enable_functional_analysis = enable_functional_analysis;
    visitor.enable_rust_patterns = enable_rust_patterns;
    visitor
}

/// Analyze AST with content
pub fn analyze_ast_with_content(
    ast: &crate::core::ast::RustAst,
    source_content: &str,
    enhanced_thresholds: &ComplexityThresholds,
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
) -> AnalysisResult {
    let mut visitor = create_configured_visitor(
        ast.path.clone(),
        source_content.to_string(),
        enhanced_thresholds.clone(),
        Some(ast.file.clone()),
        enable_functional_analysis,
        enable_rust_patterns,
    );

    visitor.visit_file(&ast.file);

    AnalysisResult {
        functions: visitor.functions,
        enhanced_analysis: visitor.enhanced_analysis,
    }
}
