//! Single-pass file extraction for unified analysis.
//!
//! This module implements `UnifiedFileExtractor`, which parses a Rust file
//! exactly once and extracts all data needed by downstream analysis phases.
//! This replaces the previous approach of parsing files multiple times across
//! different analyzers, avoiding proc-macro2 SourceMap overflow on large codebases.
//!
//! # Usage
//!
//! ```rust,ignore
//! use debtmap::extraction::{UnifiedFileExtractor, ExtractedFileData};
//! use std::path::Path;
//!
//! let content = std::fs::read_to_string("src/main.rs")?;
//! let data = UnifiedFileExtractor::extract(Path::new("src/main.rs"), &content)?;
//!
//! // Use extracted data across multiple analysis phases
//! for func in &data.functions {
//!     println!("Function {} at line {}, complexity: {}", func.name, func.line, func.cyclomatic);
//! }
//! ```

use crate::analyzers::io_detector::detect_io_operations_from_block;
use crate::analyzers::purity_detector::PurityDetector;
use crate::complexity::entropy::EntropyAnalyzer;
use crate::complexity::entropy_core::EntropyScore;
use crate::complexity::{cognitive::calculate_cognitive, cyclomatic::calculate_cyclomatic};
use crate::config::get_entropy_config;
use crate::core::parsing::reset_span_locations;
use crate::extraction::types::{
    CallSite, CallType, DetectedPattern, ExtractedFileData, ExtractedFunctionData,
    ExtractedImplData, ExtractedStructData, FieldInfo, ImportInfo, IoOperation, IoType, MethodInfo,
    PatternType, PurityAnalysisData, PurityLevel, TransformationPattern,
};
use anyhow::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;

/// Single-pass extractor for all file data.
///
/// Parses a file once and extracts:
/// - All function/method data with complexity and purity
/// - All struct definitions
/// - All impl blocks
/// - All imports
/// - Test region line counts (Spec 214)
///
/// The extractor resets the proc-macro2 SourceMap after extraction
/// to prevent overflow when processing large codebases.
pub struct UnifiedFileExtractor {
    /// Number of lines in content
    line_count: usize,
    /// Accumulated test lines count (Spec 214)
    test_lines: usize,
}

impl UnifiedFileExtractor {
    /// Extract all data from a file in a single pass.
    ///
    /// Parses the file once and extracts:
    /// - All function/method data with complexity and purity
    /// - All struct definitions
    /// - All impl blocks
    /// - All imports
    ///
    /// Resets SourceMap after extraction to prevent overflow.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be parsed.
    pub fn extract(path: &Path, content: &str) -> Result<ExtractedFileData> {
        let ast = syn::parse_file(content)
            .map_err(|e| anyhow::anyhow!("Parse error in {}: {}", path.display(), e))?;

        let mut extractor = Self {
            line_count: content.lines().count(),
            test_lines: 0,
        };

        let data = extractor.extract_from_ast(path, &ast);

        // Reset SourceMap to prevent overflow
        reset_span_locations();

        Ok(data)
    }

    /// Extract from multiple files in parallel with batched SourceMap resets.
    ///
    /// Processes files in batches, resetting SourceMap between batches
    /// to prevent overflow on large codebases.
    ///
    /// # Arguments
    ///
    /// * `files` - Slice of (path, content) tuples
    /// * `batch_size` - Number of files per batch (default: 200)
    pub fn extract_batch(
        files: &[(PathBuf, String)],
        batch_size: usize,
    ) -> Vec<(PathBuf, Result<ExtractedFileData>)> {
        let mut results = Vec::with_capacity(files.len());

        for batch in files.chunks(batch_size) {
            // Extract in parallel
            let batch_results: Vec<_> = batch
                .par_iter()
                .map(|(path, content)| {
                    let result = Self::extract(path, content);
                    (path.clone(), result)
                })
                .collect();

            results.extend(batch_results);

            // Reset after each batch
            reset_span_locations();
        }

        results
    }

    fn extract_from_ast(&mut self, path: &Path, ast: &syn::File) -> ExtractedFileData {
        let mut data = ExtractedFileData::empty(path.to_path_buf());
        data.total_lines = self.line_count;

        // Extract imports
        data.imports = self.extract_imports(ast);

        // Visit all items
        for item in &ast.items {
            match item {
                syn::Item::Fn(item_fn) => {
                    let func_data = self.extract_function(item_fn, None, false);
                    data.functions.push(func_data);
                }
                syn::Item::Struct(item_struct) => {
                    let struct_data = self.extract_struct(item_struct);
                    data.structs.push(struct_data);
                }
                syn::Item::Impl(item_impl) => {
                    let (impl_data, methods) = self.extract_impl(item_impl, false);
                    data.impls.push(impl_data);
                    data.functions.extend(methods);
                }
                syn::Item::Mod(item_mod) => {
                    // Check for #[cfg(test)]
                    let is_test_mod = item_mod.attrs.iter().any(|attr| {
                        attr.path().is_ident("cfg")
                            && attr
                                .meta
                                .require_list()
                                .ok()
                                .is_some_and(|list| list.tokens.to_string().contains("test"))
                    });

                    if let Some((brace, items)) = &item_mod.content {
                        // Spec 214: Track test module line ranges
                        if is_test_mod {
                            let start_line = item_mod.mod_token.span.start().line;
                            let end_line = brace.span.close().start().line;
                            let test_mod_lines = end_line.saturating_sub(start_line) + 1;
                            self.test_lines += test_mod_lines;
                        }
                        self.extract_module_items(items, &mut data, is_test_mod);
                    }
                }
                _ => {}
            }
        }

        // Set test_lines from accumulated count (Spec 214)
        data.test_lines = self.test_lines;

        // Detect patterns from extracted data (spec 204: avoid re-parsing)
        data.detected_patterns = Self::detect_patterns_from_extracted(&data);

        data
    }

    /// Detect code patterns from already-extracted data.
    ///
    /// Spec 204: Pattern detection is done during extraction to avoid re-parsing.
    /// This uses the same thresholds as `complexity::pure::detect_patterns_pure`.
    fn detect_patterns_from_extracted(data: &ExtractedFileData) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        // God object detection: structs with >5 fields
        for s in &data.structs {
            if s.fields.len() > 5 {
                patterns.push(DetectedPattern::GodObject {
                    name: s.name.clone(),
                    field_count: s.fields.len(),
                });
            }
        }

        // Function patterns (excluding test functions)
        for func in &data.functions {
            if func.is_test || func.in_test_module {
                continue;
            }

            // Long function detection: >50 lines (using length field)
            if func.length > 50 {
                patterns.push(DetectedPattern::LongFunction {
                    name: func.name.clone(),
                    lines: func.length,
                });
            }

            // Many parameters detection: >5 parameters
            if func.parameter_names.len() > 5 {
                patterns.push(DetectedPattern::ManyParameters {
                    name: func.name.clone(),
                    param_count: func.parameter_names.len(),
                });
            }

            // Deep nesting detection: >4 levels
            if func.nesting > 4 {
                patterns.push(DetectedPattern::DeepNesting {
                    function_name: func.name.clone(),
                    depth: func.nesting,
                });
            }
        }

        patterns
    }

    fn extract_module_items(
        &mut self,
        items: &[syn::Item],
        data: &mut ExtractedFileData,
        in_test_module: bool,
    ) {
        for item in items {
            match item {
                syn::Item::Fn(item_fn) => {
                    let func_data = self.extract_function(item_fn, None, in_test_module);
                    data.functions.push(func_data);
                }
                syn::Item::Struct(item_struct) => {
                    let struct_data = self.extract_struct(item_struct);
                    data.structs.push(struct_data);
                }
                syn::Item::Impl(item_impl) => {
                    let (impl_data, methods) = self.extract_impl(item_impl, in_test_module);
                    data.impls.push(impl_data);
                    data.functions.extend(methods);
                }
                syn::Item::Mod(item_mod) => {
                    // Nested module - check for additional #[cfg(test)]
                    let is_nested_test = item_mod.attrs.iter().any(|attr| {
                        attr.path().is_ident("cfg")
                            && attr
                                .meta
                                .require_list()
                                .ok()
                                .is_some_and(|list| list.tokens.to_string().contains("test"))
                    });
                    let is_test_mod = in_test_module || is_nested_test;

                    if let Some((brace, items)) = &item_mod.content {
                        // Spec 214: Track nested test module line ranges
                        // Only count if this is a new test module (not already in test context)
                        if is_nested_test && !in_test_module {
                            let start_line = item_mod.mod_token.span.start().line;
                            let end_line = brace.span.close().start().line;
                            let test_mod_lines = end_line.saturating_sub(start_line) + 1;
                            self.test_lines += test_mod_lines;
                        }
                        self.extract_module_items(items, data, is_test_mod);
                    }
                }
                _ => {}
            }
        }
    }

    fn extract_function(
        &self,
        item_fn: &syn::ItemFn,
        impl_type: Option<&str>,
        in_test_module: bool,
    ) -> ExtractedFunctionData {
        let name = item_fn.sig.ident.to_string();
        let qualified_name = impl_type
            .map(|t| format!("{}::{}", t, name))
            .unwrap_or_else(|| name.clone());

        let line = self.span_to_line(&item_fn.sig.ident.span());
        let end_line = self.estimate_end_line_fn(item_fn);
        let length = end_line.saturating_sub(line) + 1;

        // Calculate complexity
        let (cyclomatic, cognitive, nesting) = self.calculate_complexity(&item_fn.block);

        // Extract purity analysis
        let purity_analysis = self.extract_purity(item_fn);

        // Extract I/O operations
        let io_operations = self.extract_io_operations(&item_fn.block);

        // Extract parameters
        let parameter_names = self.extract_parameters(&item_fn.sig);

        // Extract transformation patterns
        let transformation_patterns = self.extract_transformations(&item_fn.block);

        // Extract calls
        let calls = self.extract_calls(&item_fn.block);

        // Extract metadata
        let is_test = self.is_test_function(&item_fn.attrs) || in_test_module;
        let is_async = item_fn.sig.asyncness.is_some();
        let visibility = self.extract_visibility(&item_fn.vis);

        // Calculate entropy if enabled
        let entropy_score = calculate_entropy_if_enabled(&item_fn.block);

        ExtractedFunctionData {
            name,
            qualified_name,
            line,
            end_line,
            length,
            cyclomatic,
            cognitive,
            nesting,
            entropy_score,
            purity_analysis,
            io_operations,
            parameter_names,
            transformation_patterns,
            calls,
            is_test,
            is_async,
            visibility,
            is_trait_method: false,
            in_test_module,
        }
    }

    fn extract_impl_method(
        &self,
        impl_fn: &syn::ImplItemFn,
        impl_type: &str,
        in_test_module: bool,
        is_trait_impl: bool,
    ) -> ExtractedFunctionData {
        let name = impl_fn.sig.ident.to_string();
        let qualified_name = format!("{}::{}", impl_type, name);

        let line = self.span_to_line(&impl_fn.sig.ident.span());
        let end_line = self.estimate_end_line_impl_fn(impl_fn);
        let length = end_line.saturating_sub(line) + 1;

        // Calculate complexity
        let (cyclomatic, cognitive, nesting) = self.calculate_complexity(&impl_fn.block);

        // Extract purity analysis for impl method
        let purity_analysis = self.extract_purity_impl_method(impl_fn);

        // Extract I/O operations
        let io_operations = self.extract_io_operations(&impl_fn.block);

        // Extract parameters
        let parameter_names = self.extract_parameters(&impl_fn.sig);

        // Extract transformation patterns
        let transformation_patterns = self.extract_transformations(&impl_fn.block);

        // Extract calls
        let calls = self.extract_calls(&impl_fn.block);

        // Extract metadata
        let is_test = self.is_test_function(&impl_fn.attrs) || in_test_module;
        let is_async = impl_fn.sig.asyncness.is_some();
        let visibility = self.extract_impl_visibility(&impl_fn.vis);

        // Calculate entropy if enabled
        let entropy_score = calculate_entropy_if_enabled(&impl_fn.block);

        ExtractedFunctionData {
            name,
            qualified_name,
            line,
            end_line,
            length,
            cyclomatic,
            cognitive,
            nesting,
            entropy_score,
            purity_analysis,
            io_operations,
            parameter_names,
            transformation_patterns,
            calls,
            is_test,
            is_async,
            visibility,
            is_trait_method: is_trait_impl,
            in_test_module,
        }
    }

    fn extract_purity(&self, item_fn: &syn::ItemFn) -> PurityAnalysisData {
        let mut detector = PurityDetector::new();
        let analysis = detector.is_pure_function(item_fn);

        // Extract purity_level first since we'll move var_names later
        let purity_level = Self::to_purity_level(&analysis);

        PurityAnalysisData {
            is_pure: analysis.is_pure,
            has_mutations: analysis.total_mutations > 0,
            has_io_operations: analysis.reasons.iter().any(|r| {
                matches!(
                    r,
                    crate::analyzers::purity_detector::ImpurityReason::IOOperations
                )
            }),
            has_unsafe: analysis.reasons.iter().any(|r| {
                matches!(
                    r,
                    crate::analyzers::purity_detector::ImpurityReason::UnsafeCode
                )
            }),
            local_mutations: analysis
                .live_mutations
                .iter()
                .map(|m| m.target.clone())
                .collect(),
            upvalue_mutations: Vec::new(), // Not directly available from PurityAnalysis
            total_mutations: analysis.total_mutations,
            var_names: analysis.var_names.into_iter().enumerate().collect(),
            confidence: analysis.confidence,
            purity_level,
        }
    }

    fn extract_purity_impl_method(&self, impl_fn: &syn::ImplItemFn) -> PurityAnalysisData {
        let mut detector = PurityDetector::new();
        let analysis = detector.is_pure_impl_method(impl_fn);

        // Extract purity_level first since we'll move var_names later
        let purity_level = Self::to_purity_level(&analysis);

        PurityAnalysisData {
            is_pure: analysis.is_pure,
            has_mutations: analysis.total_mutations > 0,
            has_io_operations: analysis.reasons.iter().any(|r| {
                matches!(
                    r,
                    crate::analyzers::purity_detector::ImpurityReason::IOOperations
                )
            }),
            has_unsafe: analysis.reasons.iter().any(|r| {
                matches!(
                    r,
                    crate::analyzers::purity_detector::ImpurityReason::UnsafeCode
                )
            }),
            local_mutations: analysis
                .live_mutations
                .iter()
                .map(|m| m.target.clone())
                .collect(),
            upvalue_mutations: Vec::new(),
            total_mutations: analysis.total_mutations,
            var_names: analysis.var_names.into_iter().enumerate().collect(),
            confidence: analysis.confidence,
            purity_level,
        }
    }

    fn to_purity_level(
        analysis: &crate::analyzers::purity_detector::PurityAnalysis,
    ) -> PurityLevel {
        match analysis.purity_level {
            crate::core::PurityLevel::StrictlyPure => PurityLevel::StrictlyPure,
            crate::core::PurityLevel::LocallyPure => PurityLevel::LocallyPure,
            crate::core::PurityLevel::ReadOnly => PurityLevel::ReadOnly,
            crate::core::PurityLevel::Impure => PurityLevel::Impure,
        }
    }

    fn extract_io_operations(&self, block: &syn::Block) -> Vec<IoOperation> {
        detect_io_operations_from_block(block)
            .into_iter()
            .map(|op| IoOperation {
                io_type: Self::convert_io_type(&op.operation_type),
                description: op.operation_type,
                line: op.line,
            })
            .collect()
    }

    fn convert_io_type(operation_type: &str) -> IoType {
        match operation_type {
            "file_io" => IoType::File,
            "console" => IoType::Console,
            "network" => IoType::Network,
            "database" => IoType::Database,
            "async_io" => IoType::AsyncIO,
            "environment" => IoType::Environment,
            "system" => IoType::System,
            _ => IoType::System,
        }
    }

    fn extract_parameters(&self, sig: &syn::Signature) -> Vec<String> {
        sig.inputs
            .iter()
            .filter_map(|arg| match arg {
                syn::FnArg::Typed(pat_type) => {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        Some(pat_ident.ident.to_string())
                    } else {
                        None
                    }
                }
                syn::FnArg::Receiver(_) => Some("self".to_string()),
            })
            .collect()
    }

    fn extract_transformations(&self, block: &syn::Block) -> Vec<TransformationPattern> {
        let mut visitor = TransformationVisitor::new();
        visitor.visit_block(block);
        visitor.patterns
    }

    fn extract_calls(&self, block: &syn::Block) -> Vec<CallSite> {
        let mut visitor = CallVisitor::new();
        visitor.visit_block(block);
        visitor.calls
    }

    fn calculate_complexity(&self, block: &syn::Block) -> (u32, u32, u32) {
        let cyclomatic = calculate_cyclomatic(block);
        let cognitive = calculate_cognitive(block);
        let nesting = self.calculate_max_nesting(block);
        (cyclomatic, cognitive, nesting)
    }

    fn calculate_max_nesting(&self, block: &syn::Block) -> u32 {
        crate::complexity::pure::calculate_max_nesting_depth(block)
    }

    fn extract_struct(&self, item_struct: &syn::ItemStruct) -> ExtractedStructData {
        let name = item_struct.ident.to_string();
        let line = self.span_to_line(&item_struct.ident.span());
        let is_public = matches!(item_struct.vis, syn::Visibility::Public(_));

        let fields = match &item_struct.fields {
            syn::Fields::Named(fields_named) => fields_named
                .named
                .iter()
                .map(|field| FieldInfo {
                    name: field
                        .ident
                        .as_ref()
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    type_str: quote::quote!(#field.ty).to_string(),
                    is_public: matches!(field.vis, syn::Visibility::Public(_)),
                })
                .collect(),
            syn::Fields::Unnamed(fields_unnamed) => fields_unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(idx, field)| FieldInfo {
                    name: format!("{}", idx),
                    type_str: quote::quote!(#field.ty).to_string(),
                    is_public: matches!(field.vis, syn::Visibility::Public(_)),
                })
                .collect(),
            syn::Fields::Unit => Vec::new(),
        };

        ExtractedStructData {
            name,
            line,
            fields,
            is_public,
        }
    }

    fn extract_impl(
        &self,
        item_impl: &syn::ItemImpl,
        in_test_module: bool,
    ) -> (ExtractedImplData, Vec<ExtractedFunctionData>) {
        let type_name = Self::extract_type_name(&item_impl.self_ty);
        let trait_name = item_impl.trait_.as_ref().map(|(_, path, _)| {
            path.segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default()
        });
        let line = self.span_to_line(&item_impl.self_ty.span());
        let is_trait_impl = trait_name.is_some();

        let mut methods = Vec::new();
        let mut method_infos = Vec::new();

        for item in &item_impl.items {
            if let syn::ImplItem::Fn(impl_fn) = item {
                let method_line = self.span_to_line(&impl_fn.sig.ident.span());
                let is_public = matches!(impl_fn.vis, syn::Visibility::Public(_));

                method_infos.push(MethodInfo {
                    name: impl_fn.sig.ident.to_string(),
                    line: method_line,
                    is_public,
                });

                // Extract full method data
                let func_data =
                    self.extract_impl_method(impl_fn, &type_name, in_test_module, is_trait_impl);
                methods.push(func_data);
            }
        }

        let impl_data = ExtractedImplData {
            type_name,
            trait_name,
            methods: method_infos,
            line,
        };

        (impl_data, methods)
    }

    fn extract_imports(&self, ast: &syn::File) -> Vec<ImportInfo> {
        let mut imports = Vec::new();

        for item in &ast.items {
            if let syn::Item::Use(item_use) = item {
                Self::extract_use_tree(&item_use.tree, String::new(), &mut imports);
            }
        }

        imports
    }

    fn extract_use_tree(tree: &syn::UseTree, prefix: String, imports: &mut Vec<ImportInfo>) {
        match tree {
            syn::UseTree::Path(path) => {
                let new_prefix = if prefix.is_empty() {
                    path.ident.to_string()
                } else {
                    format!("{}::{}", prefix, path.ident)
                };
                Self::extract_use_tree(&path.tree, new_prefix, imports);
            }
            syn::UseTree::Name(name) => {
                let full_path = if prefix.is_empty() {
                    name.ident.to_string()
                } else {
                    format!("{}::{}", prefix, name.ident)
                };
                imports.push(ImportInfo {
                    path: full_path,
                    alias: None,
                    is_glob: false,
                });
            }
            syn::UseTree::Rename(rename) => {
                let full_path = if prefix.is_empty() {
                    rename.ident.to_string()
                } else {
                    format!("{}::{}", prefix, rename.ident)
                };
                imports.push(ImportInfo {
                    path: full_path,
                    alias: Some(rename.rename.to_string()),
                    is_glob: false,
                });
            }
            syn::UseTree::Glob(_) => {
                imports.push(ImportInfo {
                    path: if prefix.is_empty() {
                        "*".to_string()
                    } else {
                        format!("{}::*", prefix)
                    },
                    alias: None,
                    is_glob: true,
                });
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    Self::extract_use_tree(item, prefix.clone(), imports);
                }
            }
        }
    }

    fn extract_type_name(ty: &syn::Type) -> String {
        match ty {
            syn::Type::Path(type_path) => type_path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
            syn::Type::Reference(type_ref) => {
                let inner = Self::extract_type_name(&type_ref.elem);
                if type_ref.mutability.is_some() {
                    format!("&mut {}", inner)
                } else {
                    format!("&{}", inner)
                }
            }
            syn::Type::Slice(type_slice) => {
                format!("[{}]", Self::extract_type_name(&type_slice.elem))
            }
            syn::Type::Array(type_array) => {
                format!("[{}; _]", Self::extract_type_name(&type_array.elem))
            }
            syn::Type::Tuple(type_tuple) => {
                let inner = type_tuple
                    .elems
                    .iter()
                    .map(Self::extract_type_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", inner)
            }
            _ => quote::quote!(#ty).to_string(),
        }
    }

    fn span_to_line(&self, span: &proc_macro2::Span) -> usize {
        span.start().line
    }

    fn estimate_end_line_fn(&self, item_fn: &syn::ItemFn) -> usize {
        // Estimate end line by getting span of closing brace
        item_fn.block.brace_token.span.close().start().line
    }

    fn estimate_end_line_impl_fn(&self, impl_fn: &syn::ImplItemFn) -> usize {
        impl_fn.block.brace_token.span.close().start().line
    }

    fn extract_visibility(&self, vis: &syn::Visibility) -> Option<String> {
        match vis {
            syn::Visibility::Public(_) => Some("pub".to_string()),
            syn::Visibility::Restricted(restricted) => {
                let path_str = restricted
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                Some(format!("pub({})", path_str))
            }
            syn::Visibility::Inherited => None,
        }
    }

    fn extract_impl_visibility(&self, vis: &syn::Visibility) -> Option<String> {
        self.extract_visibility(vis)
    }

    fn is_test_function(&self, attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || attr.path().is_ident("tokio")
                || (attr.path().is_ident("cfg") && {
                    attr.meta
                        .require_list()
                        .ok()
                        .is_some_and(|list| list.tokens.to_string().contains("test"))
                })
        })
    }
}

/// Visitor to extract function calls from a block.
struct CallVisitor {
    calls: Vec<CallSite>,
}

impl CallVisitor {
    fn new() -> Self {
        Self { calls: Vec::new() }
    }
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        let line = expr.func.span().start().line;

        match &*expr.func {
            syn::Expr::Path(path) => {
                let segments: Vec<_> = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                let name = segments.join("::");
                let call_type = if segments.len() > 1 {
                    CallType::StaticMethod
                } else {
                    CallType::Direct
                };

                self.calls.push(CallSite {
                    callee_name: name,
                    call_type,
                    line,
                });
            }
            syn::Expr::Closure(_) => {
                self.calls.push(CallSite {
                    callee_name: "<closure>".to_string(),
                    call_type: CallType::Closure,
                    line,
                });
            }
            syn::Expr::Paren(paren) => {
                // Function pointer call via (expr)(args)
                self.calls.push(CallSite {
                    callee_name: quote::quote!(#paren.expr).to_string(),
                    call_type: CallType::FunctionPointer,
                    line,
                });
            }
            _ => {}
        }

        syn::visit::visit_expr_call(self, expr);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        let line = expr.method.span().start().line;

        self.calls.push(CallSite {
            callee_name: expr.method.to_string(),
            call_type: CallType::Method,
            line,
        });

        syn::visit::visit_expr_method_call(self, expr);
    }
}

/// Visitor to detect transformation patterns (map, filter, fold, etc.)
struct TransformationVisitor {
    patterns: Vec<TransformationPattern>,
}

impl TransformationVisitor {
    fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for TransformationVisitor {
    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        let method_name = expr.method.to_string();
        let line = expr.method.span().start().line;

        let pattern_type = match method_name.as_str() {
            "map" => Some(PatternType::Map),
            "filter" => Some(PatternType::Filter),
            "fold" => Some(PatternType::Fold),
            "flat_map" | "flatten" => Some(PatternType::FlatMap),
            "collect" => Some(PatternType::Collect),
            "for_each" => Some(PatternType::ForEach),
            "find" | "find_map" => Some(PatternType::Find),
            "any" => Some(PatternType::Any),
            "all" => Some(PatternType::All),
            "reduce" => Some(PatternType::Reduce),
            _ => None,
        };

        if let Some(pattern_type) = pattern_type {
            self.patterns
                .push(TransformationPattern { pattern_type, line });
        }

        syn::visit::visit_expr_method_call(self, expr);
    }
}

/// Calculate entropy score for a function block if entropy analysis is enabled.
///
/// This function checks the entropy configuration and calculates entropy-based
/// complexity dampening for functions with repetitive patterns.
///
/// # Returns
///
/// - `Some(EntropyScore)` if entropy is enabled and calculated successfully
/// - `None` if entropy analysis is disabled
fn calculate_entropy_if_enabled(block: &syn::Block) -> Option<EntropyScore> {
    if get_entropy_config().enabled {
        let mut analyzer = EntropyAnalyzer::new();
        let score = analyzer.calculate_entropy(block);

        Some(EntropyScore {
            token_entropy: score.token_entropy,
            pattern_repetition: score.pattern_repetition,
            branch_similarity: score.branch_similarity,
            effective_complexity: score.effective_complexity,
            unique_variables: score.unique_variables,
            max_nesting: score.max_nesting,
            dampening_applied: score.dampening_applied,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn extract_test_code(code: &str) -> ExtractedFileData {
        UnifiedFileExtractor::extract(Path::new("test.rs"), code).expect("Failed to extract")
    }

    #[test]
    fn test_extract_simple_function() {
        let code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);

        let func = &data.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.parameter_names, vec!["a", "b"]);
        assert_eq!(func.cyclomatic, 1);
        assert!(!func.is_test);
        assert!(!func.is_async);
    }

    #[test]
    fn test_extract_function_with_complexity() {
        let code = r#"
fn complex(x: i32) -> i32 {
    if x > 0 {
        if x > 10 {
            x * 2
        } else {
            x + 1
        }
    } else {
        0
    }
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);

        let func = &data.functions[0];
        assert!(func.cyclomatic >= 3);
        assert!(func.nesting >= 2);
    }

    #[test]
    fn test_extract_async_function() {
        let code = r#"
async fn fetch_data() -> String {
    String::new()
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);
        assert!(data.functions[0].is_async);
    }

    #[test]
    fn test_extract_test_function() {
        let code = r#"
#[test]
fn test_something() {
    assert!(true);
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);
        assert!(data.functions[0].is_test);
    }

    #[test]
    fn test_extract_struct() {
        let code = r#"
pub struct MyStruct {
    pub name: String,
    value: i32,
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.structs.len(), 1);

        let s = &data.structs[0];
        assert_eq!(s.name, "MyStruct");
        assert!(s.is_public);
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[0].name, "name");
        assert!(s.fields[0].is_public);
        assert_eq!(s.fields[1].name, "value");
        assert!(!s.fields[1].is_public);
    }

    #[test]
    fn test_extract_impl_block() {
        let code = r#"
struct Foo;

impl Foo {
    pub fn new() -> Self {
        Foo
    }

    fn private_method(&self) {}
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.impls.len(), 1);
        assert_eq!(data.impls[0].type_name, "Foo");
        assert_eq!(data.impls[0].methods.len(), 2);
        assert!(data.impls[0].methods[0].is_public);
        assert!(!data.impls[0].methods[1].is_public);

        // Functions should include methods
        assert_eq!(data.functions.len(), 2);
        assert_eq!(data.functions[0].qualified_name, "Foo::new");
    }

    #[test]
    fn test_extract_trait_impl() {
        let code = r#"
struct Bar;

impl Clone for Bar {
    fn clone(&self) -> Self {
        Bar
    }
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.impls.len(), 1);
        assert_eq!(data.impls[0].trait_name, Some("Clone".to_string()));
        assert_eq!(data.functions.len(), 1);
        assert!(data.functions[0].is_trait_method);
    }

    #[test]
    fn test_extract_imports() {
        let code = r#"
use std::collections::HashMap;
use std::io::{Read, Write};
use crate::module::*;
use something as alias;
"#;
        let data = extract_test_code(code);
        assert!(!data.imports.is_empty());

        // Check specific imports
        let paths: Vec<_> = data.imports.iter().map(|i| &i.path).collect();
        assert!(paths.contains(&&"std::collections::HashMap".to_string()));
        assert!(paths.contains(&&"std::io::Read".to_string()));
        assert!(paths.contains(&&"std::io::Write".to_string()));

        // Check glob import
        let glob_import = data.imports.iter().find(|i| i.is_glob);
        assert!(glob_import.is_some());

        // Check alias
        let alias_import = data.imports.iter().find(|i| i.alias.is_some());
        assert!(alias_import.is_some());
        assert_eq!(alias_import.unwrap().alias.as_ref().unwrap(), "alias");
    }

    #[test]
    fn test_extract_calls() {
        let code = r#"
fn caller() {
    helper();
    obj.method();
    String::from("test");
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);

        let calls = &data.functions[0].calls;
        assert!(calls.iter().any(|c| c.callee_name == "helper"));
        assert!(calls.iter().any(|c| c.callee_name == "method"));
        assert!(calls.iter().any(|c| c.callee_name == "String::from"));
    }

    #[test]
    fn test_extract_transformations() {
        let code = r#"
fn transform(items: Vec<i32>) -> Vec<i32> {
    items
        .iter()
        .map(|x| x * 2)
        .filter(|x| *x > 0)
        .collect()
}
"#;
        let data = extract_test_code(code);
        let patterns = &data.functions[0].transformation_patterns;

        assert!(patterns.iter().any(|p| p.pattern_type == PatternType::Map));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::Filter));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::Collect));
    }

    #[test]
    fn test_extract_io_operations() {
        let code = r#"
fn io_func() {
    println!("Hello");
}
"#;
        let data = extract_test_code(code);
        let io_ops = &data.functions[0].io_operations;
        assert!(!io_ops.is_empty());
        assert!(io_ops.iter().any(|op| op.io_type == IoType::Console));
    }

    #[test]
    fn test_extract_pure_function() {
        let code = r#"
fn pure_add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let data = extract_test_code(code);
        let purity = &data.functions[0].purity_analysis;
        assert!(purity.is_pure);
        assert_eq!(purity.purity_level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_extract_impure_function() {
        let code = r#"
fn impure_func(items: &mut Vec<i32>) {
    items.push(42);
}
"#;
        let data = extract_test_code(code);
        let purity = &data.functions[0].purity_analysis;
        assert!(!purity.is_pure);
    }

    #[test]
    fn test_extract_functions_in_test_module() {
        let code = r#"
fn regular_fn() {}

#[cfg(test)]
mod tests {
    fn test_helper() {}

    #[test]
    fn actual_test() {}
}
"#;
        let data = extract_test_code(code);

        // Should have 3 functions total
        assert_eq!(data.functions.len(), 3);

        // Regular function not in test module
        let regular = data.functions.iter().find(|f| f.name == "regular_fn");
        assert!(regular.is_some());
        assert!(!regular.unwrap().in_test_module);
        assert!(!regular.unwrap().is_test);

        // Helper in test module
        let helper = data.functions.iter().find(|f| f.name == "test_helper");
        assert!(helper.is_some());
        assert!(helper.unwrap().in_test_module);

        // Actual test
        let test = data.functions.iter().find(|f| f.name == "actual_test");
        assert!(test.is_some());
        assert!(test.unwrap().is_test);
        assert!(test.unwrap().in_test_module);
    }

    #[test]
    fn test_extract_line_count() {
        let code = "fn foo() {}\nfn bar() {}\nfn baz() {}\n";
        let data = extract_test_code(code);
        assert_eq!(data.total_lines, 3);
    }

    #[test]
    fn test_batch_extraction() {
        let files: Vec<(PathBuf, String)> = vec![
            (PathBuf::from("a.rs"), "fn a() {}".to_string()),
            (PathBuf::from("b.rs"), "fn b() {}".to_string()),
            (PathBuf::from("c.rs"), "fn c() {}".to_string()),
        ];

        let results = UnifiedFileExtractor::extract_batch(&files, 2);
        assert_eq!(results.len(), 3);

        for (path, result) in results {
            assert!(result.is_ok(), "Failed to extract {:?}", path);
        }
    }

    #[test]
    fn test_extract_visibility() {
        let code = r#"
pub fn public_fn() {}
pub(crate) fn crate_fn() {}
fn private_fn() {}
"#;
        let data = extract_test_code(code);

        let public = data.functions.iter().find(|f| f.name == "public_fn");
        assert_eq!(public.unwrap().visibility, Some("pub".to_string()));

        let crate_vis = data.functions.iter().find(|f| f.name == "crate_fn");
        assert!(crate_vis.unwrap().visibility.is_some());
        assert!(crate_vis
            .unwrap()
            .visibility
            .as_ref()
            .unwrap()
            .contains("crate"));

        let private = data.functions.iter().find(|f| f.name == "private_fn");
        assert!(private.unwrap().visibility.is_none());
    }

    #[test]
    fn test_else_if_chain_nesting() {
        let code = r#"
fn chain(x: i32) {
    if x < 0 {
        println!("negative");
    } else if x == 0 {
        println!("zero");
    } else if x < 10 {
        println!("small");
    } else {
        println!("big");
    }
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);
        assert_eq!(
            data.functions[0].nesting, 1,
            "else-if chain should have nesting 1"
        );
    }

    #[test]
    fn test_nested_if_not_else_if() {
        let code = r#"
fn nested(a: bool, b: bool) {
    if a {
        if b {
            println!("both");
        }
    }
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);
        assert_eq!(
            data.functions[0].nesting, 2,
            "nested if inside then should have nesting 2"
        );
    }

    #[test]
    fn test_match_with_else_if_chain_nesting() {
        let code = r#"
fn matcher(opt: Option<i32>) {
    match opt {
        Some(x) => {
            if x < 0 {
            } else if x == 0 {
            } else {
            }
        }
        None => {}
    }
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);
        assert_eq!(
            data.functions[0].nesting, 2,
            "match + else-if chain should have nesting 2"
        );
    }

    #[test]
    fn test_long_else_if_chain_nesting() {
        let code = r#"
fn long_chain(x: i32) -> &'static str {
    if x == 1 {
        "one"
    } else if x == 2 {
        "two"
    } else if x == 3 {
        "three"
    } else if x == 4 {
        "four"
    } else if x == 5 {
        "five"
    } else if x == 6 {
        "six"
    } else if x == 7 {
        "seven"
    } else {
        "other"
    }
}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.functions.len(), 1);
        assert_eq!(
            data.functions[0].nesting, 1,
            "long else-if chain should still have nesting 1"
        );
    }

    #[test]
    fn test_detect_god_object_pattern() {
        let code = r#"
struct BigStruct {
    field1: i32,
    field2: i32,
    field3: i32,
    field4: i32,
    field5: i32,
    field6: i32, // 6 fields > 5 threshold
}
"#;
        let data = extract_test_code(code);
        assert!(data.detected_patterns.iter().any(|p| matches!(
            p,
            DetectedPattern::GodObject { name, field_count: 6 } if name == "BigStruct"
        )));
    }

    #[test]
    fn test_detect_many_parameters_pattern() {
        let code = r#"
fn many_params(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}
"#;
        let data = extract_test_code(code);
        assert!(data.detected_patterns.iter().any(|p| matches!(
            p,
            DetectedPattern::ManyParameters { name, param_count: 6 } if name == "many_params"
        )));
    }

    #[test]
    fn test_no_patterns_for_test_functions() {
        let code = r#"
#[test]
fn test_with_many_params(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}
"#;
        let data = extract_test_code(code);
        // Test function should not trigger ManyParameters pattern
        assert!(data.detected_patterns.iter().all(|p| !matches!(
            p,
            DetectedPattern::ManyParameters { name, .. } if name == "test_with_many_params"
        )));
    }

    #[test]
    fn test_no_patterns_for_normal_code() {
        let code = r#"
struct SmallStruct { a: i32 }
fn short_fn(x: i32) -> i32 { x }
"#;
        let data = extract_test_code(code);
        assert!(
            data.detected_patterns.is_empty(),
            "Normal code should have no patterns"
        );
    }

    // ===== Spec 214: Test Line Detection =====

    #[test]
    fn test_extract_test_lines_cfg_test_module() {
        let code = r#"
fn production_fn() {}

#[cfg(test)]
mod tests {
    fn test_helper() {}

    #[test]
    fn actual_test() {}
}
"#;
        let data = extract_test_code(code);

        // Test module spans lines 4-11 (8 lines)
        assert!(data.test_lines > 0, "Should have test lines detected");
        assert!(
            data.test_lines >= 6,
            "Test module should have at least 6 lines, got {}",
            data.test_lines
        );
        assert!(
            data.total_lines > data.test_lines,
            "Total should be more than test lines"
        );
    }

    #[test]
    fn test_extract_no_test_lines_without_test_module() {
        let code = r#"
fn regular_fn() {}
fn another_fn() {}
"#;
        let data = extract_test_code(code);
        assert_eq!(data.test_lines, 0, "Should have no test lines");
    }

    #[test]
    fn test_production_lines_calculation() {
        let code = r#"
fn production_fn() {}
fn another_fn() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {}
}
"#;
        let data = extract_test_code(code);

        let production_lines = data.production_lines();
        assert!(
            production_lines < data.total_lines,
            "Production lines should be less than total"
        );
        assert!(production_lines > 0, "Should have production lines");
    }

    #[test]
    fn test_loc_metrics_from_extracted() {
        let code = r#"
fn foo() {}
fn bar() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_foo() {}
}
"#;
        let data = extract_test_code(code);
        let metrics = data.loc_metrics();

        assert_eq!(metrics.total_loc, data.total_lines);
        assert_eq!(metrics.test_loc, data.test_lines);
        assert_eq!(
            metrics.production_loc,
            data.total_lines.saturating_sub(data.test_lines)
        );
    }

    #[test]
    fn test_nested_test_modules_not_double_counted() {
        let code = r#"
fn production() {}

#[cfg(test)]
mod tests {
    fn helper() {}

    mod nested {
        #[test]
        fn test_nested() {}
    }
}
"#;
        let data = extract_test_code(code);

        // The test module should be counted, but nested non-test modules within it
        // should not double-count
        assert!(data.test_lines > 0);
        // The whole #[cfg(test)] block is the test region
        assert!(data.test_lines <= data.total_lines);
    }
}
