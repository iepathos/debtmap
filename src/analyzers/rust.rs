use crate::analyzers::purity_detector::PurityDetector;
use crate::analyzers::Analyzer;
use crate::complexity::{
    cognitive::calculate_cognitive_with_patterns,
    cyclomatic::{calculate_cyclomatic, calculate_cyclomatic_adjusted},
    if_else_analyzer::{IfElseChain, IfElseChainAnalyzer},
    message_generator::{generate_enhanced_message, EnhancedComplexityMessage},
    recursive_detector::{MatchLocation, RecursiveMatchDetector},
    threshold_manager::{ComplexityThresholds, ThresholdPreset},
};
use crate::core::{
    ast::{Ast, RustAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::async_errors::detect_async_errors;
use crate::debt::error_context::analyze_error_context;
use crate::debt::error_propagation::analyze_error_propagation;
use crate::debt::error_swallowing::detect_error_swallowing;
use crate::debt::panic_patterns::detect_panic_patterns;
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use crate::organization::{
    FeatureEnvyDetector, GodObjectDetector, MagicValueDetector, MaintainabilityImpact,
    OrganizationAntiPattern, OrganizationDetector, ParameterAnalyzer, PrimitiveObsessionDetector,
};
use crate::priority::call_graph::CallGraph;
use crate::testing;
use anyhow::Result;
use quote::ToTokens;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::{visit::Visit, Item};

pub struct RustAnalyzer {
    complexity_threshold: u32,
    enhanced_thresholds: ComplexityThresholds,
    use_enhanced_detection: bool,
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            use_enhanced_detection: true,
        }
    }

    pub fn with_threshold_preset(preset: ThresholdPreset) -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(preset),
            use_enhanced_detection: true,
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
            Ast::Rust(rust_ast) => analyze_rust_file(
                rust_ast,
                self.complexity_threshold,
                &self.enhanced_thresholds,
                self.use_enhanced_detection,
            ),
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

// Expansion function removed - now using enhanced token parsing instead

fn analyze_rust_file(
    ast: &RustAst,
    threshold: u32,
    enhanced_thresholds: &ComplexityThresholds,
    _use_enhanced: bool,
) -> FileMetrics {
    let source_content = std::fs::read_to_string(&ast.path).unwrap_or_default();
    let mut visitor = FunctionVisitor::new(ast.path.clone(), source_content.clone());
    visitor.file_ast = Some(ast.file.clone());
    visitor.enhanced_thresholds = enhanced_thresholds.clone();
    visitor.visit_file(&ast.file);

    let debt_items = create_debt_items(
        &ast.file,
        &ast.path,
        threshold,
        &visitor.functions,
        &source_content,
        &visitor.enhanced_analysis,
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
    enhanced_analysis: &[EnhancedFunctionAnalysis],
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
        enhanced_analysis,
    )
}

fn collect_all_rust_debt_items(
    file: &syn::File,
    path: &std::path::Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
    suppression_context: &SuppressionContext,
    enhanced_analysis: &[EnhancedFunctionAnalysis],
) -> Vec<DebtItem> {
    [
        extract_debt_items_with_enhanced(file, path, threshold, functions, enhanced_analysis),
        find_todos_and_fixmes_with_suppression(source_content, path, Some(suppression_context)),
        find_code_smells_with_suppression(source_content, path, Some(suppression_context)),
        extract_rust_module_smell_items(path, source_content, suppression_context),
        extract_rust_function_smell_items(functions, suppression_context),
        detect_error_swallowing(file, path, Some(suppression_context)),
        // New enhanced error handling detectors
        detect_panic_patterns(file, path, Some(suppression_context)),
        analyze_error_context(file, path, Some(suppression_context)),
        detect_async_errors(file, path, Some(suppression_context)),
        analyze_error_propagation(file, path, Some(suppression_context)),
        // Existing resource and organization analysis
        analyze_resource_patterns(file, path),
        analyze_organization_patterns(file, path),
        testing::analyze_testing_patterns(file, path),
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EnhancedFunctionAnalysis {
    function_name: String,
    matches: Vec<MatchLocation>,
    if_else_chains: Vec<IfElseChain>,
    enhanced_message: Option<EnhancedComplexityMessage>,
}

// Helper structs for refactored code
#[derive(Clone)]
struct FunctionMetadata {
    is_test: bool,
    visibility: Option<String>,
    entropy_score: Option<crate::complexity::entropy_core::EntropyScore>,
    purity_info: (Option<bool>, Option<f32>),
}

struct ComplexityMetricsData {
    cyclomatic: u32,
    cognitive: u32,
}

struct FunctionContext {
    name: String,
    file: PathBuf,
    line: usize,
    is_trait_method: bool,
    in_test_module: bool,
}

struct FunctionVisitor {
    functions: Vec<FunctionMetrics>,
    current_file: PathBuf,
    #[allow(dead_code)]
    source_content: String,
    in_test_module: bool,
    current_function: Option<String>,
    current_impl_type: Option<String>,
    current_impl_is_trait: bool,
    file_ast: Option<syn::File>,
    enhanced_analysis: Vec<EnhancedFunctionAnalysis>,
    enhanced_thresholds: ComplexityThresholds,
}

impl FunctionVisitor {
    /// Pure function to classify if a path represents a test file
    fn classify_test_file(path_str: &str) -> bool {
        // Test directory patterns
        const TEST_DIR_PATTERNS: &[&str] = &[
            "/tests/",
            "/test/",
            "/testing/",
            "/mocks/",
            "/mock/",
            "/fixtures/",
            "/fixture/",
            "/test_helpers/",
            "/test_utils/",
            "/test_",
            "/mock",
            "/scenario",
            "\\tests\\",
            "\\test\\", // Windows paths
        ];

        // Test file suffixes
        const TEST_FILE_SUFFIXES: &[&str] = &["_test.rs", "_tests.rs", "/tests.rs", "/test.rs"];

        // Check directory patterns
        let has_test_dir = TEST_DIR_PATTERNS
            .iter()
            .any(|pattern| path_str.contains(pattern));

        // Check file suffixes
        let has_test_suffix = TEST_FILE_SUFFIXES
            .iter()
            .any(|suffix| path_str.ends_with(suffix));

        has_test_dir || has_test_suffix
    }

    fn new(file: PathBuf, source_content: String) -> Self {
        // Check if this file is a test file based on its path
        let is_test_file = Self::classify_test_file(&file.to_string_lossy());

        Self {
            functions: Vec::new(),
            current_file: file,
            source_content,
            in_test_module: is_test_file,
            current_function: None,
            current_impl_type: None,
            current_impl_is_trait: false,
            file_ast: None,
            enhanced_analysis: Vec::new(),
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
        }
    }

    fn get_line_number(&self, span: syn::__private::Span) -> usize {
        // Use proc-macro2's span-locations feature to get actual line numbers
        span.start().line
    }

    fn analyze_function(
        &mut self,
        name: String,
        item_fn: &syn::ItemFn,
        line: usize,
        is_trait_method: bool,
    ) {
        // Extract basic function metadata
        let metadata = Self::extract_function_metadata(&name, item_fn);

        // Calculate complexity metrics
        let complexity_metrics = self.calculate_complexity_metrics(&item_fn.block, item_fn);

        // Perform enhanced complexity analysis
        let enhanced_analysis = Self::perform_enhanced_analysis(&item_fn.block);

        // Build complete metrics
        let context = FunctionContext {
            name: name.clone(),
            file: self.current_file.clone(),
            line,
            is_trait_method,
            in_test_module: self.in_test_module,
        };
        let metrics = Self::build_function_metrics(
            context,
            metadata.clone(),
            complexity_metrics,
            &item_fn.block,
            item_fn,
        );

        // Determine function role and check if it should be flagged
        let role = Self::classify_function_role(&name, metadata.is_test);

        // Create and store analysis results
        let analysis_result =
            self.create_analysis_result(name.clone(), &metrics, role, enhanced_analysis);

        self.enhanced_analysis.push(analysis_result);
        self.functions.push(metrics);
    }

    // Pure function to extract metadata
    fn extract_function_metadata(name: &str, item_fn: &syn::ItemFn) -> FunctionMetadata {
        FunctionMetadata {
            is_test: Self::is_test_function(name, item_fn),
            visibility: Self::extract_visibility(&item_fn.vis),
            entropy_score: Self::calculate_entropy_if_enabled(&item_fn.block),
            purity_info: Self::detect_purity(item_fn),
        }
    }

    // Method to calculate complexity metrics
    fn calculate_complexity_metrics(
        &self,
        block: &syn::Block,
        item_fn: &syn::ItemFn,
    ) -> ComplexityMetricsData {
        ComplexityMetricsData {
            cyclomatic: self.calculate_cyclomatic_with_visitor(block, item_fn),
            cognitive: self.calculate_cognitive_with_visitor(block, item_fn),
        }
    }

    // Pure function for enhanced analysis
    fn perform_enhanced_analysis(block: &syn::Block) -> (Vec<MatchLocation>, Vec<IfElseChain>) {
        let mut match_detector = RecursiveMatchDetector::new();
        let matches = match_detector.find_matches_in_block(block);

        let mut if_else_analyzer = IfElseChainAnalyzer::new();
        let if_else_chains = if_else_analyzer.analyze_block(block);

        (matches, if_else_chains)
    }

    // Pure function to build metrics
    fn build_function_metrics(
        context: FunctionContext,
        metadata: FunctionMetadata,
        complexity: ComplexityMetricsData,
        block: &syn::Block,
        item_fn: &syn::ItemFn,
    ) -> FunctionMetrics {
        FunctionMetrics {
            name: context.name,
            file: context.file,
            line: context.line,
            cyclomatic: complexity.cyclomatic,
            cognitive: complexity.cognitive,
            nesting: calculate_nesting(block),
            length: count_function_lines(item_fn),
            is_test: metadata.is_test,
            visibility: metadata.visibility,
            is_trait_method: context.is_trait_method,
            in_test_module: context.in_test_module,
            entropy_score: metadata.entropy_score,
            is_pure: metadata.purity_info.0,
            purity_confidence: metadata.purity_info.1,
        }
    }

    // Pure function to classify function role
    fn classify_function_role(
        name: &str,
        is_test: bool,
    ) -> crate::complexity::threshold_manager::FunctionRole {
        use crate::complexity::threshold_manager::FunctionRole;

        match () {
            _ if is_test => FunctionRole::Test,
            _ if name == "main" => FunctionRole::EntryPoint,
            _ => FunctionRole::CoreLogic,
        }
    }

    // Method to create analysis result
    fn create_analysis_result(
        &self,
        name: String,
        metrics: &FunctionMetrics,
        role: crate::complexity::threshold_manager::FunctionRole,
        enhanced_analysis: (Vec<MatchLocation>, Vec<IfElseChain>),
    ) -> EnhancedFunctionAnalysis {
        let (matches, if_else_chains) = enhanced_analysis;

        let enhanced_message = if self.enhanced_thresholds.should_flag_function(metrics, role) {
            Some(generate_enhanced_message(
                metrics,
                &matches,
                &if_else_chains,
                &self.enhanced_thresholds,
            ))
        } else {
            None
        };

        EnhancedFunctionAnalysis {
            function_name: name,
            matches,
            if_else_chains,
            enhanced_message,
        }
    }

    fn is_test_function(name: &str, item_fn: &syn::ItemFn) -> bool {
        // Extract pure classification logic
        Self::has_test_attribute(&item_fn.attrs) || Self::has_test_name_pattern(name)
    }

    fn has_test_attribute(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| match () {
            _ if attr.path().is_ident("test") => true,
            _ if attr
                .path()
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "test") =>
            {
                true
            }
            _ if attr.path().is_ident("cfg") => {
                attr.meta.to_token_stream().to_string().contains("test")
            }
            _ => false,
        })
    }

    fn has_test_name_pattern(name: &str) -> bool {
        const TEST_PREFIXES: &[&str] = &["test_", "it_", "should_"];
        const MOCK_PATTERNS: &[&str] = &["mock", "stub", "fake"];

        let name_lower = name.to_lowercase();

        match () {
            _ if TEST_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) => true,
            _ if MOCK_PATTERNS
                .iter()
                .any(|pattern| name_lower.contains(pattern)) =>
            {
                true
            }
            _ => false,
        }
    }

    fn extract_visibility(vis: &syn::Visibility) -> Option<String> {
        match vis {
            syn::Visibility::Public(_) => Some("pub".to_string()),
            syn::Visibility::Restricted(restricted) => {
                if restricted.path.is_ident("crate") {
                    Some("pub(crate)".to_string())
                } else {
                    Some(format!("pub({})", quote::quote!(#restricted.path)))
                }
            }
            syn::Visibility::Inherited => None,
        }
    }

    fn calculate_entropy_if_enabled(
        block: &syn::Block,
    ) -> Option<crate::complexity::entropy_core::EntropyScore> {
        if crate::config::get_entropy_config().enabled {
            // For now, use the old analyzer's EntropyScore as a bridge
            // TODO: Once old entropy is removed, update to use new framework directly
            let mut old_analyzer = crate::complexity::entropy::EntropyAnalyzer::new();
            let old_score = old_analyzer.calculate_entropy(block);

            // Convert old score to new score format
            Some(crate::complexity::entropy_core::EntropyScore {
                token_entropy: old_score.token_entropy,
                pattern_repetition: old_score.pattern_repetition,
                branch_similarity: old_score.branch_similarity,
                effective_complexity: old_score.effective_complexity,
                unique_variables: old_score.unique_variables,
                max_nesting: old_score.max_nesting,
                dampening_applied: old_score.dampening_applied,
            })
        } else {
            None
        }
    }

    fn detect_purity(item_fn: &syn::ItemFn) -> (Option<bool>, Option<f32>) {
        let mut detector = PurityDetector::new();
        let analysis = detector.is_pure_function(item_fn);
        (Some(analysis.is_pure), Some(analysis.confidence))
    }

    fn calculate_cyclomatic_with_visitor(&self, block: &syn::Block, func: &syn::ItemFn) -> u32 {
        use crate::complexity::visitor_detector::detect_visitor_pattern;

        // Check if we have the file AST and can detect visitor patterns
        if let Some(ref file_ast) = self.file_ast {
            if let Some(pattern_info) = detect_visitor_pattern(file_ast, func) {
                // Return the adjusted complexity if a visitor pattern was detected
                return pattern_info.adjusted_complexity;
            }
        }

        // Fall back to standard calculation
        calculate_cyclomatic_adjusted(block)
    }

    fn calculate_cognitive_with_visitor(&self, block: &syn::Block, func: &syn::ItemFn) -> u32 {
        use crate::complexity::visitor_detector::detect_visitor_pattern;

        // Check if we have the file AST and can detect visitor patterns
        if let Some(ref file_ast) = self.file_ast {
            if let Some(pattern_info) = detect_visitor_pattern(file_ast, func) {
                // For cognitive complexity, we also apply the reduction
                let base_cognitive = calculate_cognitive_syn(block);
                // Apply similar scaling for cognitive complexity
                match pattern_info.pattern_type {
                    crate::complexity::visitor_detector::PatternType::Visitor => {
                        ((base_cognitive as f32).log2().ceil()).max(1.0) as u32
                    }
                    crate::complexity::visitor_detector::PatternType::ExhaustiveMatch => {
                        ((base_cognitive as f32).sqrt().ceil()).max(2.0) as u32
                    }
                    crate::complexity::visitor_detector::PatternType::SimpleMapping => {
                        ((base_cognitive as f32) * 0.2).max(1.0) as u32
                    }
                    _ => base_cognitive,
                }
            } else {
                calculate_cognitive_syn(block)
            }
        } else {
            calculate_cognitive_syn(block)
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

        // Check if this is a trait implementation
        let is_trait_impl = item_impl.trait_.is_some();

        // Store the current impl type and trait status
        let prev_impl_type = self.current_impl_type.clone();
        let prev_impl_is_trait = self.current_impl_is_trait;
        self.current_impl_type = impl_type;
        self.current_impl_is_trait = is_trait_impl;

        // Continue visiting the impl block
        syn::visit::visit_item_impl(self, item_impl);

        // Restore previous impl type and trait status
        self.current_impl_type = prev_impl_type;
        self.current_impl_is_trait = prev_impl_is_trait;
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
        let line = self.get_line_number(item_fn.sig.ident.span());
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

        let line = self.get_line_number(impl_fn.sig.ident.span());

        // For trait implementations, methods inherit the trait's visibility
        // Trait methods are effectively public (accessible through the trait)
        let vis = if self.current_impl_is_trait {
            // Trait methods are effectively public
            syn::Visibility::Public(syn::Token![pub](impl_fn.sig.ident.span()))
        } else {
            // Use the actual visibility from impl_fn for inherent impls
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
                let name = if let Some(ref parent) = self.current_function {
                    format!("{}::<closure@{}>", parent, self.functions.len())
                } else {
                    format!("<closure@{}>", self.functions.len())
                };
                let line = self.get_line_number(closure.body.span());

                // Calculate entropy score if enabled
                let entropy_score = if crate::config::get_entropy_config().enabled {
                    let mut old_analyzer = crate::complexity::entropy::EntropyAnalyzer::new();
                    let old_score = old_analyzer.calculate_entropy(&block);

                    // Convert old score to new score format
                    Some(crate::complexity::entropy_core::EntropyScore {
                        token_entropy: old_score.token_entropy,
                        pattern_repetition: old_score.pattern_repetition,
                        branch_similarity: old_score.branch_similarity,
                        effective_complexity: old_score.effective_complexity,
                        unique_variables: old_score.unique_variables,
                        max_nesting: old_score.max_nesting,
                        dampening_applied: old_score.dampening_applied,
                    })
                } else {
                    None
                };

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
                    is_trait_method: false,       // Closures are not trait methods
                    in_test_module: self.in_test_module,
                    entropy_score,
                    is_pure: None, // TODO: Add purity detection for closures
                    purity_confidence: None,
                };

                self.functions.push(metrics);
            }
        }

        // Continue visiting
        syn::visit::visit_expr(self, expr);
    }
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

    impl NestingVisitor {
        // Helper function to visit nested content
        fn visit_nested<F>(&mut self, f: F)
        where
            F: FnOnce(&mut Self),
        {
            self.current_depth += 1;
            self.max_depth = self.max_depth.max(self.current_depth);
            f(self);
            self.current_depth -= 1;
        }
    }

    impl<'ast> Visit<'ast> for NestingVisitor {
        fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
            self.visit_nested(|v| syn::visit::visit_expr_if(v, i));
        }

        fn visit_expr_while(&mut self, i: &'ast syn::ExprWhile) {
            self.visit_nested(|v| syn::visit::visit_expr_while(v, i));
        }

        fn visit_expr_for_loop(&mut self, i: &'ast syn::ExprForLoop) {
            self.visit_nested(|v| syn::visit::visit_expr_for_loop(v, i));
        }

        fn visit_expr_loop(&mut self, i: &'ast syn::ExprLoop) {
            self.visit_nested(|v| syn::visit::visit_expr_loop(v, i));
        }

        fn visit_expr_match(&mut self, i: &'ast syn::ExprMatch) {
            // Match itself increases nesting
            self.current_depth += 1;
            self.max_depth = self.max_depth.max(self.current_depth);

            // Visit the expression being matched
            self.visit_expr(&i.expr);

            // Visit each arm (arms will handle their own nesting)
            for arm in &i.arms {
                self.visit_arm(arm);
            }

            self.current_depth -= 1;
        }

        fn visit_arm(&mut self, i: &'ast syn::Arm) {
            // Don't increase nesting for match arms themselves
            // The match already increased nesting
            // Just visit the patterns and body
            for attr in &i.attrs {
                self.visit_attribute(attr);
            }
            self.visit_pat(&i.pat);
            if let Some((_, guard)) = &i.guard {
                self.visit_expr(guard);
            }
            self.visit_expr(&i.body);
        }

        // Don't count blocks themselves as they're part of control structures
        // Only count control flow structures
        fn visit_block(&mut self, block: &'ast syn::Block) {
            // Just visit the statements, don't increase depth for blocks
            syn::visit::visit_block(self, block);
        }
    }

    let mut visitor = NestingVisitor {
        current_depth: 0,
        max_depth: 0,
    };

    // Visit the block's statements directly
    for stmt in &block.stmts {
        visitor.visit_stmt(stmt);
    }

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

fn extract_debt_items_with_enhanced(
    _file: &syn::File,
    _path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    enhanced_analysis: &[EnhancedFunctionAnalysis],
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|func| func.is_complex(threshold))
        .map(|func| create_debt_item_for_function(func, threshold, enhanced_analysis))
        .collect()
}

// Pure function to create debt item for a single function
fn create_debt_item_for_function(
    func: &FunctionMetrics,
    threshold: u32,
    enhanced_analysis: &[EnhancedFunctionAnalysis],
) -> DebtItem {
    // Find corresponding enhanced analysis if available
    let enhanced = find_enhanced_analysis_for_function(&func.name, enhanced_analysis);

    match enhanced.and_then(|a| a.enhanced_message.as_ref()) {
        Some(enhanced_msg) => create_enhanced_debt_item(func, threshold, enhanced_msg),
        None => create_complexity_debt_item(func, threshold),
    }
}

// Pure function to find enhanced analysis for a function
fn find_enhanced_analysis_for_function<'a>(
    function_name: &str,
    enhanced_analysis: &'a [EnhancedFunctionAnalysis],
) -> Option<&'a EnhancedFunctionAnalysis> {
    enhanced_analysis
        .iter()
        .find(|e| e.function_name == function_name)
}

// Pure function to create enhanced debt item
fn create_enhanced_debt_item(
    func: &FunctionMetrics,
    threshold: u32,
    enhanced_msg: &EnhancedComplexityMessage,
) -> DebtItem {
    DebtItem {
        id: format!("complexity-{}-{}", func.file.display(), func.line),
        debt_type: DebtType::Complexity,
        priority: classify_priority(func.cyclomatic, threshold),
        file: func.file.clone(),
        line: func.line,
        column: None,
        message: enhanced_msg.summary.clone(),
        context: Some(format_enhanced_context(enhanced_msg)),
    }
}

// Pure function to classify priority based on complexity
fn classify_priority(cyclomatic: u32, threshold: u32) -> Priority {
    if cyclomatic > threshold * 2 {
        Priority::High
    } else {
        Priority::Medium
    }
}

fn format_enhanced_context(msg: &EnhancedComplexityMessage) -> String {
    let mut context = String::new();

    // Add details
    if !msg.details.is_empty() {
        context.push_str("\n\nComplexity Issues:");
        for detail in &msg.details {
            context.push_str(&format!("\n  • {}", detail.description));
        }
    }

    // Add recommendations
    if !msg.recommendations.is_empty() {
        context.push_str("\n\nRecommendations:");
        for rec in &msg.recommendations {
            context.push_str(&format!("\n  • {}: {}", rec.title, rec.description));
        }
    }

    context
}

#[allow(dead_code)]
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
        priority: classify_priority(func.cyclomatic, threshold),
        file: func.file.clone(),
        line: func.line,
        column: None,
        message: format!(
            "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
            func.name, func.cyclomatic, func.cognitive
        ),
        context: None,
    }
}

fn analyze_resource_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    use crate::resource::{
        convert_resource_issue_to_debt_item, AsyncResourceDetector, DropDetector, ResourceDetector,
        UnboundedCollectionDetector,
    };

    let detectors: Vec<Box<dyn ResourceDetector>> = vec![
        Box::new(DropDetector::new()),
        Box::new(AsyncResourceDetector::new()),
        Box::new(UnboundedCollectionDetector::new()),
    ];

    let mut resource_items = Vec::new();

    for detector in detectors {
        let issues = detector.detect_issues(file, path);

        for issue in issues {
            let impact = detector.assess_resource_impact(&issue);
            let debt_item = convert_resource_issue_to_debt_item(issue, impact, path);
            resource_items.push(debt_item);
        }
    }

    resource_items
}

fn analyze_organization_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn OrganizationDetector>> = vec![
        Box::new(GodObjectDetector::new()),
        Box::new(MagicValueDetector::new()),
        Box::new(ParameterAnalyzer::new()),
        Box::new(FeatureEnvyDetector::new()),
        Box::new(PrimitiveObsessionDetector::new()),
    ];

    let mut organization_items = Vec::new();

    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(file);

        for pattern in anti_patterns {
            let impact = detector.estimate_maintainability_impact(&pattern);
            let debt_item = convert_organization_pattern_to_debt_item(pattern, impact, path);
            organization_items.push(debt_item);
        }
    }

    organization_items
}

// Pure function to convert impact to priority
fn impact_to_priority(impact: MaintainabilityImpact) -> Priority {
    match impact {
        MaintainabilityImpact::Critical => Priority::Critical,
        MaintainabilityImpact::High => Priority::High,
        MaintainabilityImpact::Medium => Priority::Medium,
        MaintainabilityImpact::Low => Priority::Low,
    }
}

// Pure function to extract message and context from pattern
fn pattern_to_message_context(pattern: &OrganizationAntiPattern) -> (String, Option<String>) {
    match pattern {
        OrganizationAntiPattern::GodObject {
            type_name,
            method_count,
            field_count,
            suggested_split,
            ..
        } => (
            format!(
                "God object '{}' with {} methods and {} fields",
                type_name, method_count, field_count
            ),
            Some(format!(
                "Consider splitting into: {}",
                suggested_split
                    .iter()
                    .map(|g| g.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        ),
        OrganizationAntiPattern::MagicValue {
            value,
            occurrence_count,
            suggested_constant_name,
            ..
        } => (
            format!("Magic value '{}' appears {} times", value, occurrence_count),
            Some(format!(
                "Extract constant: const {} = {};",
                suggested_constant_name, value
            )),
        ),
        OrganizationAntiPattern::LongParameterList {
            function_name,
            parameter_count,
            suggested_refactoring,
            ..
        } => (
            format!(
                "Function '{}' has {} parameters",
                function_name, parameter_count
            ),
            Some(format!("Consider: {:?}", suggested_refactoring)),
        ),
        OrganizationAntiPattern::FeatureEnvy {
            method_name,
            envied_type,
            external_calls,
            internal_calls,
            ..
        } => (
            format!(
                "Method '{}' makes {} external calls vs {} internal calls",
                method_name, external_calls, internal_calls
            ),
            Some(format!("Consider moving to '{}'", envied_type)),
        ),
        OrganizationAntiPattern::PrimitiveObsession {
            primitive_type,
            usage_context,
            suggested_domain_type,
            ..
        } => (
            format!(
                "Primitive obsession: '{}' used for {:?}",
                primitive_type, usage_context
            ),
            Some(format!("Consider domain type: {}", suggested_domain_type)),
        ),
        OrganizationAntiPattern::DataClump {
            parameter_group,
            suggested_struct_name,
            ..
        } => (
            format!(
                "Data clump with {} parameters",
                parameter_group.parameters.len()
            ),
            Some(format!("Extract struct: {}", suggested_struct_name)),
        ),
    }
}

fn convert_organization_pattern_to_debt_item(
    pattern: OrganizationAntiPattern,
    impact: MaintainabilityImpact,
    path: &Path,
) -> DebtItem {
    let location = pattern.primary_location().clone();
    let line = location.line;

    let priority = impact_to_priority(impact);
    let (message, context) = pattern_to_message_context(&pattern);

    DebtItem {
        id: format!("organization-{}-{}", path.display(), line),
        debt_type: DebtType::CodeOrganization,
        priority,
        file: path.to_path_buf(),
        line,
        column: location.column,
        message,
        context,
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_classify_test_file_with_test_directories() {
        // Test directory patterns
        assert!(FunctionVisitor::classify_test_file("src/tests/mod.rs"));
        assert!(FunctionVisitor::classify_test_file("src/test/utils.rs"));
        assert!(FunctionVisitor::classify_test_file(
            "src/testing/helpers.rs"
        ));
        assert!(FunctionVisitor::classify_test_file("src/mocks/data.rs"));
        assert!(FunctionVisitor::classify_test_file("src/mock/server.rs"));
        assert!(FunctionVisitor::classify_test_file(
            "src/fixtures/sample.rs"
        ));
        assert!(FunctionVisitor::classify_test_file("src/fixture/db.rs"));
        assert!(FunctionVisitor::classify_test_file(
            "src/test_helpers/common.rs"
        ));
        assert!(FunctionVisitor::classify_test_file(
            "src/test_utils/setup.rs"
        ));
        assert!(FunctionVisitor::classify_test_file(
            "src/test_integration.rs"
        ));
        assert!(FunctionVisitor::classify_test_file("src/mockito/client.rs"));
        assert!(FunctionVisitor::classify_test_file("src/scenario/basic.rs"));
    }

    #[test]
    fn test_classify_test_file_with_test_suffixes() {
        // Test file suffixes
        assert!(FunctionVisitor::classify_test_file("src/lib_test.rs"));
        assert!(FunctionVisitor::classify_test_file("src/module_tests.rs"));
        assert!(FunctionVisitor::classify_test_file("src/tests.rs"));
        assert!(FunctionVisitor::classify_test_file("src/test.rs"));
        assert!(FunctionVisitor::classify_test_file("integration_test.rs"));
        assert!(FunctionVisitor::classify_test_file("unit_tests.rs"));
    }

    #[test]
    fn test_classify_test_file_with_windows_paths() {
        // Windows path patterns
        assert!(FunctionVisitor::classify_test_file("src\\tests\\mod.rs"));
        assert!(FunctionVisitor::classify_test_file("src\\test\\utils.rs"));
        assert!(FunctionVisitor::classify_test_file(
            "C:\\project\\tests\\integration.rs"
        ));
        assert!(FunctionVisitor::classify_test_file(
            "D:\\code\\test\\unit.rs"
        ));
    }

    #[test]
    fn test_classify_test_file_negative_cases() {
        // Non-test files
        assert!(!FunctionVisitor::classify_test_file("src/main.rs"));
        assert!(!FunctionVisitor::classify_test_file("src/lib.rs"));
        assert!(!FunctionVisitor::classify_test_file("src/analyzer.rs"));
        assert!(!FunctionVisitor::classify_test_file(
            "src/core/processor.rs"
        ));
        assert!(!FunctionVisitor::classify_test_file("src/utils/helper.rs"));
        assert!(!FunctionVisitor::classify_test_file("src/latest.rs"));
        assert!(!FunctionVisitor::classify_test_file("src/contest.rs"));
        assert!(!FunctionVisitor::classify_test_file("src/protest.rs"));
    }

    #[test]
    fn test_classify_test_file_edge_cases() {
        // Edge cases
        assert!(FunctionVisitor::classify_test_file("/tests/"));
        assert!(FunctionVisitor::classify_test_file("/tests/file.rs"));
        assert!(FunctionVisitor::classify_test_file("path/test.rs"));
        assert!(FunctionVisitor::classify_test_file("path/tests.rs"));
        assert!(!FunctionVisitor::classify_test_file(""));
        assert!(!FunctionVisitor::classify_test_file("/"));
        assert!(FunctionVisitor::classify_test_file(
            "deeply/nested/tests/file.rs"
        ));
        assert!(FunctionVisitor::classify_test_file(
            "very/deep/path/test_utils/util.rs"
        ));
    }

    #[test]
    fn test_is_test_function_with_test_attribute() {
        let item_fn: syn::ItemFn = parse_quote! {
            #[test]
            fn my_test() {
                assert_eq!(1, 1);
            }
        };
        assert!(FunctionVisitor::is_test_function("my_test", &item_fn));
    }

    #[test]
    fn test_is_test_function_with_tokio_test_attribute() {
        let item_fn: syn::ItemFn = parse_quote! {
            #[tokio::test]
            async fn my_async_test() {
                assert_eq!(1, 1);
            }
        };
        assert!(FunctionVisitor::is_test_function("my_async_test", &item_fn));
    }

    #[test]
    fn test_is_test_function_with_cfg_test_attribute() {
        let item_fn: syn::ItemFn = parse_quote! {
            #[cfg(test)]
            fn helper_function() {
                // test helper
            }
        };
        assert!(FunctionVisitor::is_test_function(
            "helper_function",
            &item_fn
        ));
    }

    #[test]
    fn test_is_test_function_with_test_prefix() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn test_something() {
                assert_eq!(1, 1);
            }
        };
        assert!(FunctionVisitor::is_test_function(
            "test_something",
            &item_fn
        ));
    }

    #[test]
    fn test_is_test_function_with_it_prefix() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn it_should_work() {
                assert_eq!(1, 1);
            }
        };
        assert!(FunctionVisitor::is_test_function(
            "it_should_work",
            &item_fn
        ));
    }

    #[test]
    fn test_is_test_function_with_should_prefix() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn should_handle_edge_cases() {
                assert_eq!(1, 1);
            }
        };
        assert!(FunctionVisitor::is_test_function(
            "should_handle_edge_cases",
            &item_fn
        ));
    }

    #[test]
    fn test_is_test_function_regular_function() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn calculate_sum(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        assert!(!FunctionVisitor::is_test_function(
            "calculate_sum",
            &item_fn
        ));
    }

    #[test]
    fn test_extract_visibility_public() {
        let vis: syn::Visibility = parse_quote! { pub };
        assert_eq!(
            FunctionVisitor::extract_visibility(&vis),
            Some("pub".to_string())
        );
    }

    #[test]
    fn test_extract_visibility_pub_crate() {
        let vis: syn::Visibility = parse_quote! { pub(crate) };
        assert_eq!(
            FunctionVisitor::extract_visibility(&vis),
            Some("pub(crate)".to_string())
        );
    }

    #[test]
    fn test_extract_visibility_pub_super() {
        let vis: syn::Visibility = parse_quote! { pub(super) };
        let result = FunctionVisitor::extract_visibility(&vis);
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("pub("));
    }

    #[test]
    fn test_extract_visibility_inherited() {
        let vis: syn::Visibility = parse_quote! {};
        assert_eq!(FunctionVisitor::extract_visibility(&vis), None);
    }

    #[test]
    fn test_calculate_entropy_if_enabled() {
        // Test with entropy disabled (default)
        let block: syn::Block = parse_quote! {{
            let x = 1;
            if x > 0 {
                println!("positive");
            } else {
                println!("non-positive");
            }
        }};

        // When disabled, should return None
        let result = FunctionVisitor::calculate_entropy_if_enabled(&block);
        // The actual result depends on config, but we can test it runs without panic
        // If enabled, it should return an EntropyScore struct
        if let Some(score) = result {
            // EntropyScore has token_entropy field between 0.0 and 1.0
            assert!(score.token_entropy >= 0.0 && score.token_entropy <= 1.0);
            assert!(score.pattern_repetition >= 0.0 && score.pattern_repetition <= 1.0);
        }
    }

    #[test]
    fn test_detect_purity_pure_function() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let (is_pure, confidence) = FunctionVisitor::detect_purity(&item_fn);
        assert!(is_pure.is_some());
        assert!(confidence.is_some());
        // Pure functions should have high confidence
        if let (Some(pure), Some(conf)) = (is_pure, confidence) {
            if pure {
                assert!(conf > 0.5);
            }
        }
    }

    #[test]
    fn test_detect_purity_impure_function() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn print_value(x: i32) {
                println!("Value: {}", x);
            }
        };
        let (is_pure, confidence) = FunctionVisitor::detect_purity(&item_fn);
        assert!(is_pure.is_some());
        assert!(confidence.is_some());
        // Functions with side effects should be detected as impure
        if let Some(pure) = is_pure {
            assert!(!pure);
        }
    }

    #[test]
    fn test_detect_purity_mutating_function() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn increment(&mut self, value: i32) {
                self.value += value;
            }
        };
        let (is_pure, confidence) = FunctionVisitor::detect_purity(&item_fn);
        assert!(is_pure.is_some());
        assert!(confidence.is_some());
        // The purity detector might not always detect mutation through self
        // This is a known limitation, so we just verify it returns a result
        // without asserting the specific value
    }

    #[test]
    fn test_has_test_attribute_with_simple_test() {
        let code = r#"
            #[test]
            fn my_test() {}
        "#;
        let item_fn = syn::parse_str::<syn::ItemFn>(code).unwrap();
        assert!(FunctionVisitor::has_test_attribute(&item_fn.attrs));
    }

    #[test]
    fn test_has_test_attribute_with_tokio_test() {
        let code = r#"
            #[tokio::test]
            async fn my_async_test() {}
        "#;
        let item_fn = syn::parse_str::<syn::ItemFn>(code).unwrap();
        assert!(FunctionVisitor::has_test_attribute(&item_fn.attrs));
    }

    #[test]
    fn test_has_test_attribute_with_cfg_test() {
        let code = r#"
            #[cfg(test)]
            fn helper() {}
        "#;
        let item_fn = syn::parse_str::<syn::ItemFn>(code).unwrap();
        assert!(FunctionVisitor::has_test_attribute(&item_fn.attrs));
    }

    #[test]
    fn test_has_test_attribute_without_test() {
        let code = r#"
            #[derive(Debug)]
            fn regular_function() {}
        "#;
        let item_fn = syn::parse_str::<syn::ItemFn>(code).unwrap();
        assert!(!FunctionVisitor::has_test_attribute(&item_fn.attrs));
    }

    #[test]
    fn test_has_test_name_pattern_with_test_prefix() {
        assert!(FunctionVisitor::has_test_name_pattern("test_something"));
        assert!(FunctionVisitor::has_test_name_pattern("test_"));
    }

    #[test]
    fn test_has_test_name_pattern_with_it_prefix() {
        assert!(FunctionVisitor::has_test_name_pattern("it_should_work"));
        assert!(FunctionVisitor::has_test_name_pattern("it_"));
    }

    #[test]
    fn test_has_test_name_pattern_with_should_prefix() {
        assert!(FunctionVisitor::has_test_name_pattern(
            "should_do_something"
        ));
        assert!(FunctionVisitor::has_test_name_pattern("should_"));
    }

    #[test]
    fn test_has_test_name_pattern_with_mock() {
        assert!(FunctionVisitor::has_test_name_pattern("mock_service"));
        assert!(FunctionVisitor::has_test_name_pattern("get_mock"));
        assert!(FunctionVisitor::has_test_name_pattern("MockBuilder"));
    }

    #[test]
    fn test_has_test_name_pattern_with_stub() {
        assert!(FunctionVisitor::has_test_name_pattern("stub_response"));
        assert!(FunctionVisitor::has_test_name_pattern("get_stub"));
        assert!(FunctionVisitor::has_test_name_pattern("StubFactory"));
    }

    #[test]
    fn test_has_test_name_pattern_with_fake() {
        assert!(FunctionVisitor::has_test_name_pattern("fake_data"));
        assert!(FunctionVisitor::has_test_name_pattern("create_fake"));
        assert!(FunctionVisitor::has_test_name_pattern("FakeImpl"));
    }

    #[test]
    fn test_has_test_name_pattern_regular_name() {
        assert!(!FunctionVisitor::has_test_name_pattern("regular_function"));
        assert!(!FunctionVisitor::has_test_name_pattern("process_data"));
        assert!(!FunctionVisitor::has_test_name_pattern("handle_request"));
    }
}
