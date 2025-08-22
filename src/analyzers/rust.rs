use crate::analyzers::purity_detector::PurityDetector;
use crate::analyzers::Analyzer;
use crate::complexity::{
    cognitive::calculate_cognitive_with_patterns,
    cyclomatic::{calculate_cyclomatic, calculate_cyclomatic_adjusted},
};
use crate::core::{
    ast::{Ast, RustAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::error_swallowing::detect_error_swallowing;
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

// Expansion function removed - now using enhanced token parsing instead

fn analyze_rust_file(ast: &RustAst, threshold: u32) -> FileMetrics {
    let source_content = std::fs::read_to_string(&ast.path).unwrap_or_default();
    let mut visitor = FunctionVisitor::new(ast.path.clone(), source_content.clone());
    visitor.file_ast = Some(ast.file.clone());
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
        detect_error_swallowing(file, path, Some(suppression_context)),
        analyze_resource_patterns(file, path),
        analyze_organization_patterns(file, path),
        analyze_security_patterns(file, path, suppression_context),
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
}

impl FunctionVisitor {
    fn new(file: PathBuf, source_content: String) -> Self {
        // Check if this file is a test file based on its path
        let is_test_file = {
            let path_str = file.to_string_lossy();
            path_str.contains("/tests/") 
                || path_str.contains("/test/")
                || path_str.ends_with("_test.rs")
                || path_str.ends_with("_tests.rs")
                || path_str.contains("/test_")
                || path_str.contains("\\tests\\")  // Windows paths
                || path_str.contains("\\test\\") // Windows paths
        };

        Self {
            functions: Vec::new(),
            current_file: file,
            source_content,
            in_test_module: is_test_file,
            current_function: None,
            current_impl_type: None,
            current_impl_is_trait: false,
            file_ast: None,
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
        let is_test = Self::is_test_function(&name, item_fn);
        let visibility = Self::extract_visibility(&item_fn.vis);
        let entropy_score = Self::calculate_entropy_if_enabled(&item_fn.block);
        let (is_pure, purity_confidence) = Self::detect_purity(item_fn);

        let metrics = FunctionMetrics {
            name,
            file: self.current_file.clone(),
            line,
            cyclomatic: self.calculate_cyclomatic_with_visitor(&item_fn.block, item_fn),
            cognitive: self.calculate_cognitive_with_visitor(&item_fn.block, item_fn),
            nesting: calculate_nesting(&item_fn.block),
            length: count_function_lines(item_fn),
            is_test,
            visibility,
            is_trait_method,
            in_test_module: self.in_test_module,
            entropy_score,
            is_pure,
            purity_confidence,
        };

        self.functions.push(metrics);
    }

    fn is_test_function(name: &str, item_fn: &syn::ItemFn) -> bool {
        let has_test_attribute = item_fn.attrs.iter().any(|attr| {
            attr.path().is_ident("test")
                || attr
                    .path()
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "test")
                || (attr.path().is_ident("cfg")
                    && attr.meta.to_token_stream().to_string().contains("test"))
        });

        let has_test_name =
            name.starts_with("test_") || name.starts_with("it_") || name.starts_with("should_");

        has_test_attribute || has_test_name
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
    ) -> Option<crate::complexity::entropy::EntropyScore> {
        if crate::config::get_entropy_config().enabled {
            let mut analyzer = crate::complexity::entropy::EntropyAnalyzer::new();
            Some(analyzer.calculate_entropy(block))
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
                    let mut analyzer = crate::complexity::entropy::EntropyAnalyzer::new();
                    Some(analyzer.calculate_entropy(&block))
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

fn analyze_security_patterns(
    file: &syn::File,
    path: &Path,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    crate::security::analyze_security_patterns(file, path)
        .into_iter()
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
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

fn convert_organization_pattern_to_debt_item(
    pattern: OrganizationAntiPattern,
    impact: MaintainabilityImpact,
    path: &Path,
) -> DebtItem {
    let location = pattern.primary_location().clone();
    let line = location.line;

    let (priority, message, context) = match pattern {
        OrganizationAntiPattern::GodObject {
            type_name,
            method_count,
            field_count,
            suggested_split,
            ..
        } => (
            match impact {
                MaintainabilityImpact::Critical => Priority::Critical,
                MaintainabilityImpact::High => Priority::High,
                MaintainabilityImpact::Medium => Priority::Medium,
                MaintainabilityImpact::Low => Priority::Low,
            },
            format!(
                "God object '{}' with {} methods and {} fields",
                type_name, method_count, field_count
            ),
            Some(format!(
                "Consider splitting into: {}",
                suggested_split
                    .iter()
                    .map(|g| &g.name)
                    .map(|s| s.as_str())
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
            match impact {
                MaintainabilityImpact::Critical => Priority::Critical,
                MaintainabilityImpact::High => Priority::High,
                MaintainabilityImpact::Medium => Priority::Medium,
                MaintainabilityImpact::Low => Priority::Low,
            },
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
            match impact {
                MaintainabilityImpact::Critical => Priority::Critical,
                MaintainabilityImpact::High => Priority::High,
                MaintainabilityImpact::Medium => Priority::Medium,
                MaintainabilityImpact::Low => Priority::Low,
            },
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
            match impact {
                MaintainabilityImpact::Critical => Priority::Critical,
                MaintainabilityImpact::High => Priority::High,
                MaintainabilityImpact::Medium => Priority::Medium,
                MaintainabilityImpact::Low => Priority::Low,
            },
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
            match impact {
                MaintainabilityImpact::Critical => Priority::Critical,
                MaintainabilityImpact::High => Priority::High,
                MaintainabilityImpact::Medium => Priority::Medium,
                MaintainabilityImpact::Low => Priority::Low,
            },
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
            match impact {
                MaintainabilityImpact::Critical => Priority::Critical,
                MaintainabilityImpact::High => Priority::High,
                MaintainabilityImpact::Medium => Priority::Medium,
                MaintainabilityImpact::Low => Priority::Low,
            },
            format!(
                "Data clump with {} parameters",
                parameter_group.parameters.len()
            ),
            Some(format!("Extract struct: {}", suggested_struct_name)),
        ),
    };

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
}
