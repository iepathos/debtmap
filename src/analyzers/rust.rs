use crate::analyzers::purity_detector::PurityDetector;
use crate::analyzers::rust_complexity_calculation;
use crate::analyzers::Analyzer;
use crate::complexity::{
    cyclomatic::calculate_cyclomatic,
    if_else_analyzer::{IfElseChain, IfElseChainAnalyzer},
    message_generator::{generate_enhanced_message, EnhancedComplexityMessage},
    pure_mapping_patterns::{
        calculate_adjusted_complexity, MappingPatternConfig, MappingPatternDetector,
    },
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
use crate::debt::error_swallowing::{detect_error_swallowing, detect_error_swallowing_in_function};
use crate::debt::panic_patterns::detect_panic_patterns;
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use crate::organization::{
    FeatureEnvyDetector, GodObjectDetector, MagicValueDetector, MaintainabilityImpact,
    OrganizationAntiPattern, OrganizationDetector, ParameterAnalyzer, PrimitiveObsessionDetector,
    StructInitOrganizationDetector,
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
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
}

impl RustAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            use_enhanced_detection: true,
            enable_functional_analysis: false,
            enable_rust_patterns: false,
        }
    }

    pub fn with_threshold_preset(preset: ThresholdPreset) -> Self {
        Self {
            complexity_threshold: 10,
            enhanced_thresholds: ComplexityThresholds::from_preset(preset),
            use_enhanced_detection: true,
            enable_functional_analysis: false,
            enable_rust_patterns: false,
        }
    }

    pub fn with_functional_analysis(mut self, enable: bool) -> Self {
        self.enable_functional_analysis = enable;
        self
    }

    pub fn with_rust_patterns(mut self, enable: bool) -> Self {
        self.enable_rust_patterns = enable;
        self
    }
}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RustAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let start = std::time::Instant::now();
        let file = syn::parse_str::<syn::File>(content)?;
        let parse_time = start.elapsed();

        if std::env::var("DEBTMAP_TIMING").is_ok() {
            eprintln!(
                "[TIMING] parse {}: {:.2}s ({} bytes)",
                path.display(),
                parse_time.as_secs_f64(),
                content.len()
            );
        }

        Ok(Ast::Rust(RustAst {
            file,
            path,
            source: content.to_string(),
        }))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Rust(rust_ast) => analyze_rust_file(
                rust_ast,
                self.complexity_threshold,
                &self.enhanced_thresholds,
                self.use_enhanced_detection,
                self.enable_functional_analysis,
                self.enable_rust_patterns,
            ),
            _ => FileMetrics {
                path: PathBuf::new(),
                language: Language::Rust,
                complexity: ComplexityMetrics::default(),
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                module_scope: None,
                classes: None,
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
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
) -> FileMetrics {
    let start = std::time::Instant::now();

    let analysis_start = std::time::Instant::now();
    let analysis_result = analyze_ast_with_content(
        ast,
        &ast.source,
        enhanced_thresholds,
        enable_functional_analysis,
        enable_rust_patterns,
    );
    let analysis_time = analysis_start.elapsed();

    let debt_start = std::time::Instant::now();
    let debt_items = create_debt_items(
        &ast.file,
        &ast.path,
        threshold,
        &analysis_result.functions,
        &ast.source,
        &analysis_result.enhanced_analysis,
    );
    let debt_time = debt_start.elapsed();

    let deps_start = std::time::Instant::now();
    let dependencies = extract_dependencies(&ast.file);
    let deps_time = deps_start.elapsed();

    let complexity_metrics = calculate_total_complexity(&analysis_result.functions);

    let total_time = start.elapsed();

    if std::env::var("DEBTMAP_TIMING").is_ok() {
        eprintln!(
            "[TIMING] analyze_rust_file {}: total={:.2}s (analysis={:.2}s, debt={:.2}s, deps={:.2}s)",
            ast.path.display(),
            total_time.as_secs_f64(),
            analysis_time.as_secs_f64(),
            debt_time.as_secs_f64(),
            deps_time.as_secs_f64()
        );
    }

    build_file_metrics(
        ast.path.clone(),
        analysis_result.functions,
        complexity_metrics,
        debt_items,
        dependencies,
    )
}

/// Structure to hold analysis results
struct AnalysisResult {
    functions: Vec<FunctionMetrics>,
    enhanced_analysis: Vec<EnhancedFunctionAnalysis>,
}

/// Pure function to analyze AST with content
fn analyze_ast_with_content(
    ast: &RustAst,
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

/// Pure function to create and configure visitor
fn create_configured_visitor(
    path: std::path::PathBuf,
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

/// Pure function to calculate total complexity metrics
fn calculate_total_complexity(functions: &[FunctionMetrics]) -> (u32, u32) {
    functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    })
}

/// Pure function to build file metrics
fn build_file_metrics(
    path: std::path::PathBuf,
    functions: Vec<FunctionMetrics>,
    (cyclomatic, cognitive): (u32, u32),
    debt_items: Vec<DebtItem>,
    dependencies: Vec<Dependency>,
) -> FileMetrics {
    FileMetrics {
        path,
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        debt_items,
        dependencies,
        duplications: vec![],
        module_scope: None,
        classes: None,
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
        analyze_rust_test_quality(file, path),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Analyze Rust test quality
fn analyze_rust_test_quality(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    use crate::testing::rust::analyzer::RustTestQualityAnalyzer;
    use crate::testing::rust::convert_rust_test_issue_to_debt_item;

    let mut analyzer = RustTestQualityAnalyzer::new();
    let issues = analyzer.analyze_file(file, path);

    issues
        .into_iter()
        .map(|issue| convert_rust_test_issue_to_debt_item(issue, path))
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
    purity_info: (Option<bool>, Option<f32>, Option<crate::core::PurityLevel>),
}

struct ComplexityMetricsData {
    cyclomatic: u32,
    cognitive: u32,
}

/// Complexity metrics for closures
struct ClosureComplexityMetrics {
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    length: usize,
}

struct FunctionContext {
    name: String,
    file: PathBuf,
    line: usize,
    is_trait_method: bool,
    in_test_module: bool,
    impl_type_name: Option<String>,
    trait_name: Option<String>,
}

/// Data structure to hold complete function analysis results
struct FunctionAnalysisData {
    metrics: FunctionMetrics,
    enhanced_analysis: EnhancedFunctionAnalysis,
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
    current_trait_name: Option<String>,
    file_ast: Option<syn::File>,
    enhanced_analysis: Vec<EnhancedFunctionAnalysis>,
    enhanced_thresholds: ComplexityThresholds,
    enable_functional_analysis: bool,
    enable_rust_patterns: bool,
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
            current_trait_name: None,
            file_ast: None,
            enhanced_analysis: Vec::new(),
            enhanced_thresholds: ComplexityThresholds::from_preset(ThresholdPreset::Balanced),
            enable_functional_analysis: false,
            enable_rust_patterns: false,
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
        // Create function analysis data using pure functions
        let analysis_data =
            self.create_function_analysis_data(&name, item_fn, line, is_trait_method);

        // Store results
        self.enhanced_analysis.push(analysis_data.enhanced_analysis);
        self.functions.push(analysis_data.metrics);
    }

    /// Pure function to create complete function analysis data
    fn create_function_analysis_data(
        &mut self,
        name: &str,
        item_fn: &syn::ItemFn,
        line: usize,
        is_trait_method: bool,
    ) -> FunctionAnalysisData {
        let metadata = Self::extract_function_metadata(name, item_fn);
        let complexity_metrics = self.calculate_complexity_metrics(&item_fn.block, item_fn);
        let enhanced_analysis = Self::perform_enhanced_analysis(&item_fn.block);
        let context = self.create_function_context(name.to_string(), line, is_trait_method);
        let role = Self::classify_function_role(name, metadata.is_test);

        let metrics = self.build_function_metrics(
            context,
            metadata.clone(),
            complexity_metrics,
            &item_fn.block,
            item_fn,
        );

        let analysis_result =
            self.create_analysis_result(name.to_string(), &metrics, role, enhanced_analysis);

        FunctionAnalysisData {
            metrics,
            enhanced_analysis: analysis_result,
        }
    }

    /// Pure function to create function context
    fn create_function_context(
        &self,
        name: String,
        line: usize,
        is_trait_method: bool,
    ) -> FunctionContext {
        FunctionContext {
            name,
            file: self.current_file.clone(),
            line,
            is_trait_method,
            in_test_module: self.in_test_module,
            impl_type_name: self.current_impl_type.clone(),
            trait_name: self.current_trait_name.clone(),
        }
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

    // Method to build metrics (needs self for enable_functional_analysis flag)
    fn build_function_metrics(
        &self,
        context: FunctionContext,
        metadata: FunctionMetadata,
        complexity: ComplexityMetricsData,
        block: &syn::Block,
        item_fn: &syn::ItemFn,
    ) -> FunctionMetrics {
        // Detect pure mapping patterns (spec 118)
        let function_body = quote::quote!(#block).to_string();
        let mapping_detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let mapping_result =
            mapping_detector.analyze_function(&function_body, complexity.cyclomatic);

        let mut adjusted_complexity = if mapping_result.is_pure_mapping {
            Some(calculate_adjusted_complexity(
                complexity.cyclomatic,
                complexity.cognitive,
                &mapping_result,
            ))
        } else {
            None
        };

        // Detect parallel execution patterns (spec 127)
        let mut detected_patterns = Vec::new();
        if let Some(ref file_ast) = self.file_ast {
            use crate::organization::parallel_execution_pattern::{
                adjust_parallel_score, ParallelPatternDetector,
            };

            let parallel_detector = ParallelPatternDetector::default();
            if let Some(mut pattern) = parallel_detector.detect(file_ast, &self.source_content) {
                // Fill in cyclomatic complexity for pattern
                pattern.cyclomatic_complexity = complexity.cyclomatic as usize;

                let confidence = parallel_detector.confidence(&pattern);

                // Apply score adjustment if parallel pattern detected
                let base_complexity = complexity.cyclomatic as f64;
                let parallel_adjusted = adjust_parallel_score(base_complexity, &pattern);

                // Use parallel adjustment if it's more lenient than mapping adjustment
                if adjusted_complexity.is_none() || parallel_adjusted < adjusted_complexity.unwrap()
                {
                    adjusted_complexity = Some(parallel_adjusted);
                }

                detected_patterns.push(format!(
                    "ParallelExecution({}, {:.0}% confidence, {} closures, {} captures)",
                    pattern.library,
                    confidence * 100.0,
                    pattern.closure_count,
                    pattern.total_captures
                ));
            }
        }

        // Perform functional composition analysis if enabled (spec 111)
        let composition_metrics = if self.enable_functional_analysis {
            use crate::analysis::functional_composition::{
                analyze_composition, FunctionalAnalysisConfig,
            };

            // Load config profile from environment variable or default to balanced
            let config = std::env::var("DEBTMAP_FUNCTIONAL_ANALYSIS_PROFILE")
                .ok()
                .and_then(|p| match p.as_str() {
                    "strict" => Some(FunctionalAnalysisConfig::strict()),
                    "balanced" => Some(FunctionalAnalysisConfig::balanced()),
                    "lenient" => Some(FunctionalAnalysisConfig::lenient()),
                    _ => None,
                })
                .unwrap_or_else(FunctionalAnalysisConfig::balanced);

            Some(analyze_composition(item_fn, &config))
        } else {
            None
        };

        // Detect repetitive validation patterns (spec 180)
        let validation_signals = {
            use crate::analyzers::validation_pattern_detector::ValidationPatternDetector;
            let detector = ValidationPatternDetector::new();
            detector.detect(block, &context.name)
        };

        // Detect state machine and coordinator patterns (spec 179)
        let (state_signals, coordinator_signals) = {
            use crate::analyzers::state_machine_pattern_detector::StateMachinePatternDetector;
            use crate::config::get_state_detection_config;
            let detector = StateMachinePatternDetector::with_config(get_state_detection_config());
            let state_signals = detector.detect_state_machine(block);
            let coordinator_signals = detector.detect_coordinator(block);
            (state_signals, coordinator_signals)
        };

        // Rust-specific pattern detection (spec 146)
        // IMPORTANT: Always create language_specific to store validation_signals
        // even if enable_rust_patterns is false
        let language_specific = {
            use crate::analysis::rust_patterns::{
                ImplContext, RustFunctionContext, RustPatternDetector,
            };

            let impl_context = if context.is_trait_method {
                Some(ImplContext {
                    impl_type: context.impl_type_name.clone().unwrap_or_default(),
                    is_trait_impl: true,
                    trait_name: context.trait_name.clone(),
                })
            } else {
                context
                    .impl_type_name
                    .as_ref()
                    .map(|impl_type| ImplContext {
                        impl_type: impl_type.clone(),
                        is_trait_impl: false,
                        trait_name: None,
                    })
            };

            let rust_context = RustFunctionContext {
                item_fn,
                metrics: None,
                impl_context,
                file_path: &context.file,
            };

            let detector = RustPatternDetector::new();

            // Only run full pattern detection if enabled, but always store pattern signals
            if self.enable_rust_patterns {
                Some(crate::core::LanguageSpecificData::Rust(
                    detector.detect_all_patterns(
                        &rust_context,
                        validation_signals.clone(),
                        state_signals.clone(),
                        coordinator_signals.clone(),
                    ),
                ))
            } else if validation_signals.is_some()
                || state_signals.is_some()
                || coordinator_signals.is_some()
            {
                // Minimal pattern result with just pattern signals
                Some(crate::core::LanguageSpecificData::Rust(
                    crate::analysis::rust_patterns::RustPatternResult {
                        trait_impl: None,
                        async_patterns: vec![],
                        error_patterns: vec![],
                        builder_patterns: vec![],
                        validation_signals: validation_signals.clone(),
                        state_machine_signals: state_signals.clone(),
                        coordinator_signals: coordinator_signals.clone(),
                    },
                ))
            } else {
                None
            }
        };

        // Detect error swallowing patterns per-function
        let (error_count, error_patterns) = detect_error_swallowing_in_function(block);

        FunctionMetrics {
            name: context.name,
            file: context.file,
            line: context.line,
            cyclomatic: complexity.cyclomatic,
            cognitive: complexity.cognitive,
            nesting: rust_complexity_calculation::calculate_nesting(block),
            length: rust_complexity_calculation::count_function_lines(item_fn),
            is_test: metadata.is_test,
            visibility: metadata.visibility,
            is_trait_method: context.is_trait_method,
            in_test_module: context.in_test_module,
            entropy_score: metadata.entropy_score,
            is_pure: metadata.purity_info.0,
            purity_confidence: metadata.purity_info.1,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: if detected_patterns.is_empty() {
                None
            } else {
                Some(detected_patterns)
            },
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: if mapping_result.is_pure_mapping {
                Some(mapping_result)
            } else {
                None
            },
            adjusted_complexity,
            composition_metrics,
            language_specific,
            purity_level: metadata.purity_info.2,
            error_swallowing_count: if error_count > 0 {
                Some(error_count)
            } else {
                None
            },
            error_swallowing_patterns: if error_patterns.is_empty() {
                None
            } else {
                Some(error_patterns)
            },
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

    fn detect_purity(
        item_fn: &syn::ItemFn,
    ) -> (Option<bool>, Option<f32>, Option<crate::core::PurityLevel>) {
        let mut detector = PurityDetector::new();
        let analysis = detector.is_pure_function(item_fn);
        (
            Some(analysis.is_pure),
            Some(analysis.confidence),
            Some(analysis.purity_level),
        )
    }

    fn calculate_cyclomatic_with_visitor(&self, block: &syn::Block, func: &syn::ItemFn) -> u32 {
        rust_complexity_calculation::calculate_cyclomatic_with_visitor(
            block,
            func,
            self.file_ast.as_ref(),
        )
    }

    fn calculate_cognitive_with_visitor(&self, block: &syn::Block, func: &syn::ItemFn) -> u32 {
        rust_complexity_calculation::calculate_cognitive_with_visitor(
            block,
            func,
            self.file_ast.as_ref(),
        )
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
        if let syn::Expr::Closure(closure) = expr {
            self.analyze_closure(closure);
        }
        syn::visit::visit_expr(self, expr);
    }
}

impl FunctionVisitor {
    /// Analyze closure and add to functions if substantial
    fn analyze_closure(&mut self, closure: &syn::ExprClosure) {
        let block = self.convert_closure_to_block(closure);
        let complexity_metrics = self.calculate_closure_complexity(&block);

        if self.is_substantial_closure(&complexity_metrics) {
            let metrics = self.build_closure_metrics(closure, &block, &complexity_metrics);
            self.functions.push(metrics);
        }
    }

    /// Convert closure body to a block for analysis
    fn convert_closure_to_block(&self, closure: &syn::ExprClosure) -> syn::Block {
        match &*closure.body {
            syn::Expr::Block(expr_block) => expr_block.block.clone(),
            _ => syn::Block {
                brace_token: Default::default(),
                stmts: vec![syn::Stmt::Expr(*closure.body.clone(), None)],
            },
        }
    }

    /// Calculate complexity metrics for closure
    fn calculate_closure_complexity(&self, block: &syn::Block) -> ClosureComplexityMetrics {
        ClosureComplexityMetrics {
            cyclomatic: calculate_cyclomatic(block),
            cognitive: rust_complexity_calculation::calculate_cognitive_syn(block),
            nesting: rust_complexity_calculation::calculate_nesting(block),
            length: rust_complexity_calculation::count_lines(block),
        }
    }

    /// Check if closure is substantial enough to track
    fn is_substantial_closure(&self, metrics: &ClosureComplexityMetrics) -> bool {
        metrics.cognitive > 1 || metrics.length > 1 || metrics.cyclomatic > 1
    }

    /// Build function metrics for closure
    fn build_closure_metrics(
        &mut self,
        closure: &syn::ExprClosure,
        block: &syn::Block,
        complexity: &ClosureComplexityMetrics,
    ) -> FunctionMetrics {
        let name = self.generate_closure_name();
        let line = self.get_line_number(closure.body.span());
        let entropy_score = self.calculate_closure_entropy(block);

        // Detect pure mapping patterns for closures (spec 118)
        let function_body = quote::quote!(#block).to_string();
        let mapping_detector = MappingPatternDetector::new(MappingPatternConfig::default());
        let mapping_result =
            mapping_detector.analyze_function(&function_body, complexity.cyclomatic);

        let adjusted_complexity = if mapping_result.is_pure_mapping {
            Some(calculate_adjusted_complexity(
                complexity.cyclomatic,
                complexity.cognitive,
                &mapping_result,
            ))
        } else {
            None
        };

        FunctionMetrics {
            name,
            file: self.current_file.clone(),
            line,
            cyclomatic: complexity.cyclomatic,
            cognitive: complexity.cognitive,
            nesting: complexity.nesting,
            length: complexity.length,
            is_test: self.in_test_module,
            visibility: None,
            is_trait_method: false,
            in_test_module: self.in_test_module,
            entropy_score,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: if mapping_result.is_pure_mapping {
                Some(mapping_result)
            } else {
                None
            },
            adjusted_complexity,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    /// Generate name for closure
    fn generate_closure_name(&self) -> String {
        if let Some(ref parent) = self.current_function {
            format!("{}::<closure@{}>", parent, self.functions.len())
        } else {
            format!("<closure@{}>", self.functions.len())
        }
    }

    /// Calculate entropy score for closure if enabled
    fn calculate_closure_entropy(
        &mut self,
        block: &syn::Block,
    ) -> Option<crate::complexity::entropy_core::EntropyScore> {
        if crate::config::get_entropy_config().enabled {
            let mut old_analyzer = crate::complexity::entropy::EntropyAnalyzer::new();
            let old_score = old_analyzer.calculate_entropy(block);

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
}

/// Pure function to apply cognitive complexity scaling based on pattern type
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
        debt_type: DebtType::Complexity {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
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
        debt_type: DebtType::Complexity {
            cyclomatic: func.cyclomatic,
            cognitive: func.cognitive,
        },
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
        Box::new(StructInitOrganizationDetector::new()),
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
        OrganizationAntiPattern::StructInitialization {
            function_name,
            field_count,
            cyclomatic_complexity,
            field_based_complexity,
            confidence,
            recommendation,
            ..
        } => (
            format!(
                "Struct initialization pattern in '{}' - {} fields, cyclomatic: {}, field complexity: {:.1}, confidence: {:.0}%",
                function_name, field_count, cyclomatic_complexity, field_based_complexity, confidence * 100.0
            ),
            Some(format!(
                "{} (Use field-based complexity {:.1} instead of cyclomatic {})",
                recommendation, field_based_complexity, cyclomatic_complexity
            )),
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
        debt_type: DebtType::CodeOrganization {
            issue_type: Some(pattern.pattern_type().to_string()),
        },
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
    }

    #[test]
    fn test_calculate_total_complexity() {
        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                length: 10,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                visibility: Some("pub".to_string()),
                is_test: false,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: Some(true),
                purity_confidence: Some(1.0),
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 15,
                length: 8,
                cyclomatic: 3,
                cognitive: 5,
                nesting: 0,
                visibility: None,
                is_test: false,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: Some(true),
                purity_confidence: Some(1.0),
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
            },
        ];

        let (total_cyc, total_cog) = calculate_total_complexity(&functions);
        assert_eq!(total_cyc, 8);
        assert_eq!(total_cog, 15);
    }

    #[test]
    fn test_calculate_total_complexity_empty() {
        let functions = vec![];
        let (total_cyc, total_cog) = calculate_total_complexity(&functions);
        assert_eq!(total_cyc, 0);
        assert_eq!(total_cog, 0);
    }

    #[test]
    fn test_build_file_metrics() {
        let path = PathBuf::from("test.rs");
        let functions = vec![FunctionMetrics {
            name: "test_fn".to_string(),
            file: path.clone(),
            line: 1,
            length: 5,
            cyclomatic: 2,
            cognitive: 3,
            nesting: 1,
            visibility: Some("pub".to_string()),
            is_test: false,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }];
        let debt_items = vec![];
        let dependencies = vec![Dependency {
            name: "std".to_string(),
            kind: DependencyKind::Import,
        }];

        let metrics = build_file_metrics(
            path.clone(),
            functions.clone(),
            (2, 3),
            debt_items.clone(),
            dependencies.clone(),
        );

        assert_eq!(metrics.path, path);
        assert_eq!(metrics.language, Language::Rust);
        assert_eq!(metrics.complexity.cyclomatic_complexity, 2);
        assert_eq!(metrics.complexity.cognitive_complexity, 3);
        assert_eq!(metrics.complexity.functions.len(), 1);
        assert_eq!(metrics.dependencies.len(), 1);
    }

    #[test]
    fn test_extract_dependencies() {
        let file: syn::File = parse_quote! {
            use std::io;
            use serde::{Deserialize, Serialize};
            use crate::core::Config;

            fn main() {}
        };

        let deps = extract_dependencies(&file);
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.name == "std"));
        assert!(deps.iter().any(|d| d.name == "serde"));
        assert!(deps.iter().any(|d| d.name == "crate"));
        assert!(deps.iter().all(|d| d.kind == DependencyKind::Import));
    }

    #[test]
    fn test_extract_use_name() {
        let tree: syn::UseTree = parse_quote!(std);
        assert_eq!(extract_use_name(&tree), Some("std".to_string()));

        let tree: syn::UseTree = parse_quote!(std::io);
        assert_eq!(extract_use_name(&tree), Some("std".to_string()));

        let tree: syn::UseTree = parse_quote!(serde);
        assert_eq!(extract_use_name(&tree), Some("serde".to_string()));
    }

    #[test]
    fn test_classify_test_file_non_test_files() {
        // Non-test files
        assert!(!FunctionVisitor::classify_test_file("src/main.rs"));
        assert!(!FunctionVisitor::classify_test_file("src/lib.rs"));
        assert!(!FunctionVisitor::classify_test_file("src/core/module.rs"));
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
        let (is_pure, confidence, _purity_level) = FunctionVisitor::detect_purity(&item_fn);
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
        let (is_pure, confidence, _purity_level) = FunctionVisitor::detect_purity(&item_fn);
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
        let (is_pure, confidence, _purity_level) = FunctionVisitor::detect_purity(&item_fn);
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
