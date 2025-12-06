pub mod confidence;
pub mod language_specific;
pub mod naming;

use crate::core::{FileMetrics, FunctionMetrics};
use crate::data_flow::DataFlowGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractablePattern {
    AccumulationLoop {
        iterator_binding: String,
        accumulator: String,
        operation: AccumulationOp,
        filter: Option<Box<Expression>>,
        transform: Option<Box<Expression>>,
        start_line: usize,
        end_line: usize,
    },
    GuardChainSequence {
        checks: Vec<GuardCheck>,
        early_return: ReturnType,
        start_line: usize,
        end_line: usize,
    },
    TransformationPipeline {
        stages: Vec<TransformStage>,
        input_binding: String,
        output_type: String,
        start_line: usize,
        end_line: usize,
    },
    SimilarBranches {
        condition_var: String,
        common_operations: Vec<Statement>,
        branch_specific: Vec<Vec<Statement>>,
        start_line: usize,
        end_line: usize,
    },
    NestedExtraction {
        outer_scope: String,
        inner_patterns: Vec<Box<ExtractablePattern>>,
        start_line: usize,
        end_line: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccumulationOp {
    Sum,
    Product,
    Concatenation,
    Collection,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expression {
    pub code: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardCheck {
    pub condition: String,
    pub return_value: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnType {
    pub type_name: String,
    pub is_early_return: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformStage {
    pub operation: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub code: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedPattern {
    pub pattern: ExtractablePattern,
    pub confidence: f32,
    pub context: AnalysisContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    pub function_name: String,
    pub file_path: String,
    pub language: String,
    pub complexity_before: u32,
    pub has_side_effects: bool,
    pub data_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSuggestion {
    pub pattern_type: ExtractablePattern,
    pub start_line: usize,
    pub end_line: usize,
    pub suggested_name: String,
    pub confidence: f32,
    pub parameters: Vec<Parameter>,
    pub return_type: String,
    pub complexity_reduction: ComplexityImpact,
    pub example_transformation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_hint: String,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityImpact {
    pub current_cyclomatic: u32,
    pub predicted_cyclomatic: u32,
    pub current_cognitive: u32,
    pub predicted_cognitive: u32,
    pub extracted_function_complexity: u32,
}

pub trait PatternMatcher: Send + Sync {
    fn match_patterns(&self, ast: &syn::File, context: &AnalysisContext) -> Vec<MatchedPattern>;
    fn score_confidence(&self, pattern: &MatchedPattern, context: &AnalysisContext) -> f32;
    fn generate_extraction(&self, pattern: &MatchedPattern) -> ExtractionSuggestion;
}

pub trait ExtractionAnalyzer {
    fn analyze_function(
        &self,
        func: &FunctionMetrics,
        file: &FileMetrics,
        data_flow: Option<&DataFlowGraph>,
    ) -> Vec<ExtractionSuggestion>;

    fn generate_recommendation(
        &self,
        suggestion: &ExtractionSuggestion,
        verbosity: VerbosityLevel,
    ) -> String;
}

#[derive(Debug, Clone, Copy)]
pub enum VerbosityLevel {
    Summary,
    Normal,
    Detailed,
}

pub struct UnifiedExtractionAnalyzer {
    #[allow(dead_code)]
    matchers: HashMap<String, Box<dyn PatternMatcher>>,
}

impl Default for UnifiedExtractionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedExtractionAnalyzer {
    pub fn new() -> Self {
        let mut matchers: HashMap<String, Box<dyn PatternMatcher>> = HashMap::new();

        // Register language-specific matchers
        matchers.insert(
            "rust".to_string(),
            Box::new(language_specific::RustPatternMatcher::new()),
        );
        matchers.insert(
            "javascript".to_string(),
            Box::new(language_specific::JavaScriptPatternMatcher::new()),
        );

        Self { matchers }
    }
}

impl ExtractionAnalyzer for UnifiedExtractionAnalyzer {
    fn analyze_function(
        &self,
        func: &FunctionMetrics,
        _file: &FileMetrics,
        data_flow: Option<&DataFlowGraph>,
    ) -> Vec<ExtractionSuggestion> {
        create_analysis_context(func, data_flow)
            .and_then(|context| build_analysis_pipeline(&context, func))
            .and_then(|(context, ast, source)| {
                execute_pattern_matching(&context, &ast, &source, func.line)
            })
            .unwrap_or_default()
    }

    fn generate_recommendation(
        &self,
        suggestion: &ExtractionSuggestion,
        verbosity: VerbosityLevel,
    ) -> String {
        match verbosity {
            VerbosityLevel::Summary => {
                format!(
                    "Extract '{}' (lines {}-{}) - confidence: {:.0}%, complexity reduction: {} -> {}",
                    suggestion.suggested_name,
                    suggestion.start_line,
                    suggestion.end_line,
                    suggestion.confidence * 100.0,
                    suggestion.complexity_reduction.current_cyclomatic,
                    suggestion.complexity_reduction.predicted_cyclomatic
                )
            }
            VerbosityLevel::Normal => {
                format!(
                    "Extract '{}' (lines {}-{})\n  \
                     Confidence: {:.0}%\n  \
                     Parameters: {}\n  \
                     Returns: {}\n  \
                     Complexity reduction: {} -> {} (cyclomatic), {} -> {} (cognitive)",
                    suggestion.suggested_name,
                    suggestion.start_line,
                    suggestion.end_line,
                    suggestion.confidence * 100.0,
                    format_parameters(&suggestion.parameters),
                    suggestion.return_type,
                    suggestion.complexity_reduction.current_cyclomatic,
                    suggestion.complexity_reduction.predicted_cyclomatic,
                    suggestion.complexity_reduction.current_cognitive,
                    suggestion.complexity_reduction.predicted_cognitive
                )
            }
            VerbosityLevel::Detailed => {
                format!(
                    "Extract '{}' (lines {}-{})\n\
                     Confidence: {:.0}%\n\
                     Parameters: {}\n\
                     Returns: {}\n\
                     Complexity Impact:\n\
                     - Current cyclomatic: {}\n\
                     - Predicted cyclomatic: {}\n\
                     - Current cognitive: {}\n\
                     - Predicted cognitive: {}\n\
                     - Extracted function complexity: {}\n\n\
                     Example transformation:\n{}\n",
                    suggestion.suggested_name,
                    suggestion.start_line,
                    suggestion.end_line,
                    suggestion.confidence * 100.0,
                    format_parameters(&suggestion.parameters),
                    suggestion.return_type,
                    suggestion.complexity_reduction.current_cyclomatic,
                    suggestion.complexity_reduction.predicted_cyclomatic,
                    suggestion.complexity_reduction.current_cognitive,
                    suggestion.complexity_reduction.predicted_cognitive,
                    suggestion
                        .complexity_reduction
                        .extracted_function_complexity,
                    suggestion.example_transformation
                )
            }
        }
    }
}

// Pure function for language detection
fn detect_language(path: &std::path::Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(map_extension_to_language)
        .unwrap_or_else(|| "unknown".to_string())
}

// Pure function for extension mapping
fn map_extension_to_language(ext: &str) -> String {
    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        _ => "unknown",
    }
    .to_string()
}

// Pure function for side effect detection
fn detect_side_effects(func: &FunctionMetrics, data_flow: Option<&DataFlowGraph>) -> bool {
    data_flow
        .map(|flow| analyze_side_effects_from_dataflow(func, flow))
        .unwrap_or_else(|| estimate_side_effects_from_complexity(func))
}

// Pure function for dataflow-based side effect analysis
fn analyze_side_effects_from_dataflow(func: &FunctionMetrics, flow: &DataFlowGraph) -> bool {
    let func_id = create_function_id(func);
    flow.has_side_effects(&func_id)
}

// Pure function for complexity-based side effect estimation
fn estimate_side_effects_from_complexity(func: &FunctionMetrics) -> bool {
    func.cyclomatic > 10
}

// Pure function for creating FunctionId
fn create_function_id(func: &FunctionMetrics) -> crate::priority::call_graph::FunctionId {
    crate::priority::call_graph::FunctionId::new(func.file.clone(), func.name.clone(), func.line)
}

// Pure function for dependency extraction
fn extract_dependencies(func: &FunctionMetrics, data_flow: Option<&DataFlowGraph>) -> Vec<String> {
    data_flow
        .and_then(|flow| extract_variable_dependencies(func, flow))
        .unwrap_or_default()
}

// Pure function for variable dependency extraction
fn extract_variable_dependencies(
    func: &FunctionMetrics,
    flow: &DataFlowGraph,
) -> Option<Vec<String>> {
    let func_id = create_function_id(func);
    flow.get_variable_dependencies(&func_id)
        .map(|deps| deps.iter().cloned().collect())
}

// Pure function for AST parsing with language dispatch
fn parse_function_ast(func: &FunctionMetrics, language: &str) -> Option<syn::File> {
    crate::io::read_file(&func.file)
        .map_err(|e| {
            eprintln!(
                "Warning: Failed to read file {}: {}",
                func.file.display(),
                e
            );
            e
        })
        .ok()
        .and_then(|source| parse_ast_by_language(&source, func, language))
}

// Pure function for language-specific AST parsing
fn parse_ast_by_language(
    source: &str,
    func: &FunctionMetrics,
    language: &str,
) -> Option<syn::File> {
    match language {
        "rust" => extract_rust_function_ast(source, func),
        "python" | "javascript" | "typescript" => None, // Different AST types
        _ => None,
    }
}

fn extract_rust_function_ast(source: &str, func: &FunctionMetrics) -> Option<syn::File> {
    use syn::{parse_str, Item, ItemFn};

    // Parse the entire file
    let file = parse_str::<syn::File>(source).ok()?;

    // Find the function by name and approximate line number
    for item in &file.items {
        if let Item::Fn(item_fn) = item {
            if item_fn.sig.ident == func.name {
                // Create a new File with just this function
                let single_fn_file = syn::File {
                    shebang: None,
                    attrs: vec![],
                    items: vec![Item::Fn(item_fn.clone())],
                };
                return Some(single_fn_file);
            }
        }
        // Also check inside impl blocks
        if let Item::Impl(item_impl) = item {
            for impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(method) = impl_item {
                    if method.sig.ident == func.name {
                        // Convert method to standalone function for analysis
                        let item_fn = ItemFn {
                            attrs: method.attrs.clone(),
                            vis: method.vis.clone(),
                            sig: method.sig.clone(),
                            block: Box::new(method.block.clone()),
                        };
                        let single_fn_file = syn::File {
                            shebang: None,
                            attrs: vec![],
                            items: vec![Item::Fn(item_fn)],
                        };
                        return Some(single_fn_file);
                    }
                }
            }
        }
    }

    None
}

// Pure function for parameter formatting using functional style
fn format_parameters(params: &[Parameter]) -> String {
    match params.is_empty() {
        true => "none".to_string(),
        false => params
            .iter()
            .map(format_single_parameter)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

// Pure function for formatting a single parameter
fn format_single_parameter(param: &Parameter) -> String {
    let mutability = if param.is_mutable { "mut " } else { "" };
    format!("{}{}: {}", mutability, param.name, param.type_hint)
}

// Pure functional pipeline components for analysis

// Creates analysis context from function metrics and data flow
fn create_analysis_context(
    func: &FunctionMetrics,
    data_flow: Option<&DataFlowGraph>,
) -> Result<AnalysisContext, AnalysisError> {
    let language = detect_language(&func.file);

    Ok(AnalysisContext {
        function_name: func.name.clone(),
        file_path: func.file.display().to_string(),
        language,
        complexity_before: func.cyclomatic,
        has_side_effects: detect_side_effects(func, data_flow),
        data_dependencies: extract_dependencies(func, data_flow),
    })
}

// Builds the analysis pipeline by preparing AST and source
fn build_analysis_pipeline(
    context: &AnalysisContext,
    func: &FunctionMetrics,
) -> Result<(AnalysisContext, syn::File, String), AnalysisError> {
    let ast = parse_function_ast(func, &context.language).ok_or(AnalysisError::ParseError)?;

    let source = crate::io::read_file(&func.file).map_err(|_| AnalysisError::IoError)?;

    Ok((context.clone(), ast, source))
}

// Executes pattern matching pipeline using Result chaining
fn execute_pattern_matching(
    context: &AnalysisContext,
    ast: &syn::File,
    source: &str,
    function_line: usize,
) -> Result<Vec<ExtractionSuggestion>, AnalysisError> {
    use crate::extraction_patterns::language_specific::RustPatternMatcher;

    let matcher = RustPatternMatcher::with_source_context(source, function_line);
    let patterns = matcher.match_patterns(ast, context);

    Ok(patterns
        .into_iter()
        .map(|pattern| apply_confidence_scoring(pattern, &matcher, context))
        .map(|pattern| matcher.generate_extraction(&pattern))
        .collect())
}

// Pure function for applying confidence scoring
fn apply_confidence_scoring(
    mut pattern: MatchedPattern,
    matcher: &crate::extraction_patterns::language_specific::RustPatternMatcher,
    context: &AnalysisContext,
) -> MatchedPattern {
    pattern.confidence = matcher.score_confidence(&pattern, context);
    pattern
}

// Error type for analysis pipeline
#[derive(Debug)]
#[allow(dead_code)]
enum AnalysisError {
    ParseError,
    IoError,
    MatcherNotFound,
}

// Extension trait for Result chaining in analysis pipeline
#[allow(dead_code)]
trait AnalysisResult<T> {
    fn and_then_analysis<U, F>(self, f: F) -> Result<U, AnalysisError>
    where
        F: FnOnce(T) -> Result<U, AnalysisError>;
}

impl<T> AnalysisResult<T> for Result<T, AnalysisError> {
    fn and_then_analysis<U, F>(self, f: F) -> Result<U, AnalysisError>
    where
        F: FnOnce(T) -> Result<U, AnalysisError>,
    {
        self.and_then(f)
    }
}
