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
            "python".to_string(),
            Box::new(language_specific::PythonPatternMatcher::new()),
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
        let language = detect_language(&func.file);

        let context = AnalysisContext {
            function_name: func.name.clone(),
            file_path: func.file.display().to_string(),
            language: language.clone(),
            complexity_before: func.cyclomatic,
            has_side_effects: detect_side_effects(func, data_flow),
            data_dependencies: extract_dependencies(func, data_flow),
        };

        if self.matchers.contains_key(&language) {
            // Parse the function AST based on language
            if let Some(ast) = parse_function_ast(func, &language) {
                // Read source and create context-aware matcher
                if let Ok(source) = crate::io::read_file(&func.file) {
                    use crate::extraction_patterns::language_specific::RustPatternMatcher;
                    let matcher =
                        RustPatternMatcher::with_source_context(&source, func.line);
                    let patterns = matcher.match_patterns(&ast, &context);
                    patterns
                        .into_iter()
                        .map(|pattern| {
                            let confidence = matcher.score_confidence(&pattern, &context);
                            let mut pattern = pattern;
                            pattern.confidence = confidence;
                            matcher.generate_extraction(&pattern)
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
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

fn detect_language(path: &std::path::Path) -> String {
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => "rust".to_string(),
        Some("py") => "python".to_string(),
        Some("js") | Some("jsx") => "javascript".to_string(),
        Some("ts") | Some("tsx") => "typescript".to_string(),
        _ => "unknown".to_string(),
    }
}

fn detect_side_effects(func: &FunctionMetrics, data_flow: Option<&DataFlowGraph>) -> bool {
    if let Some(flow) = data_flow {
        // Create a FunctionId from the function metrics for lookup
        let func_id = crate::priority::call_graph::FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        // Use data flow analysis to detect side effects
        flow.has_side_effects(&func_id)
    } else {
        // Conservative estimate based on function metrics
        // Functions with high complexity are more likely to have side effects
        func.cyclomatic > 10
    }
}

fn extract_dependencies(func: &FunctionMetrics, data_flow: Option<&DataFlowGraph>) -> Vec<String> {
    if let Some(flow) = data_flow {
        // Create a FunctionId from the function metrics for lookup
        let func_id = crate::priority::call_graph::FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };

        // Extract variable dependencies from data flow graph
        flow.get_variable_dependencies(&func_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn parse_function_ast(func: &FunctionMetrics, language: &str) -> Option<syn::File> {
    // Read the source file
    let source = crate::io::read_file(&func.file).ok()?;

    match language {
        "rust" => {
            // For Rust, extract the function and create a minimal syn::File
            extract_rust_function_ast(&source, func)
        }
        "python" => {
            // Python patterns don't use syn::File, they need different handling
            // For now, return None as Python pattern matching uses different AST
            None
        }
        "javascript" | "typescript" => {
            // JavaScript/TypeScript patterns also don't use syn::File
            // They would use tree-sitter AST instead
            None
        }
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

fn format_parameters(params: &[Parameter]) -> String {
    if params.is_empty() {
        "none".to_string()
    } else {
        params
            .iter()
            .map(|p| {
                if p.is_mutable {
                    format!("mut {}: {}", p.name, p.type_hint)
                } else {
                    format!("{}: {}", p.name, p.type_hint)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}
