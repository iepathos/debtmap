//! Batch file analysis using stillwater's traverse pattern.
//!
//! This module provides parallel file analysis using stillwater's `traverse_effect`
//! and error accumulation using `Validation` with `traverse`.
//!
//! # Design
//!
//! The module follows the traverse pattern from functional programming:
//!
//! - **`traverse_effect`**: Apply an effectful operation to each element and collect results
//! - **`traverse` with Validation**: Apply a validation and accumulate ALL errors
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::analyzers::batch::{analyze_files_effect, validate_files};
//! use debtmap::effects::run_effect;
//!
//! // Parallel analysis with Effect
//! let results = run_effect(
//!     analyze_files_effect(files),
//!     config,
//! )?;
//!
//! // Validation with error accumulation
//! let validated = validate_files(&paths)?;
//! ```

use crate::analyzers::get_analyzer;
use crate::config::{
    BatchAnalysisConfig, GeneratedCodeMode, ParallelConfig, SolidityLanguageConfig,
};
use crate::core::ast::Ast;
use crate::core::{DebtItem, FileMetrics, Language};
use crate::effects::{
    AnalysisEffect, AnalysisValidation, validation_failure, validation_failures, validation_success,
};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use stillwater::effect::prelude::*;

/// Result of analyzing a single file with full context.
///
/// This struct captures all relevant information from analyzing a file,
/// including metrics, debt items, and optional timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisResult {
    /// Path to the analyzed file
    pub path: PathBuf,

    /// Full file metrics from analysis
    pub metrics: FileMetrics,

    /// Technical debt items found in this file
    pub debt_items: Vec<DebtItem>,

    /// Time taken to analyze this file (if timing was enabled)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis_time: Option<Duration>,

    /// Language-specific package/module name when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
}

impl FileAnalysisResult {
    /// Create a new analysis result without timing information.
    pub fn new(path: PathBuf, metrics: FileMetrics, debt_items: Vec<DebtItem>) -> Self {
        Self {
            path,
            metrics,
            debt_items,
            analysis_time: None,
            package_name: None,
        }
    }

    /// Create a new analysis result with timing information.
    pub fn with_timing(
        path: PathBuf,
        metrics: FileMetrics,
        debt_items: Vec<DebtItem>,
        analysis_time: Duration,
    ) -> Self {
        Self {
            path,
            metrics,
            debt_items,
            analysis_time: Some(analysis_time),
            package_name: None,
        }
    }
}

/// A validated file ready for analysis.
///
/// This struct represents a file that has passed initial validation
/// (existence, readability, syntax check) and is ready for full analysis.
#[derive(Debug, Clone)]
pub struct ValidatedFile {
    /// Path to the file
    pub path: PathBuf,

    /// File content
    pub content: String,

    /// Detected language
    pub language: Language,
}

impl ValidatedFile {
    /// Create a new validated file.
    pub fn new(path: PathBuf, content: String, language: Language) -> Self {
        Self {
            path,
            content,
            language,
        }
    }
}

// ============================================================================
// Effect-based Parallel Analysis
// ============================================================================

/// Analyze multiple files in parallel using stillwater's traverse pattern.
///
/// This function uses `traverse_effect` to apply analysis to each file
/// concurrently, collecting all results. If any file fails to analyze,
/// the entire operation fails with that error.
///
/// For error accumulation (collecting ALL errors), use [`validate_files`] instead.
///
/// # Arguments
///
/// * `paths` - Files to analyze
///
/// # Returns
///
/// An Effect that produces a vector of analysis results when run.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::analyzers::batch::analyze_files_effect;
/// use debtmap::effects::run_effect;
///
/// let files = vec!["src/main.rs".into(), "src/lib.rs".into()];
/// let results = run_effect(analyze_files_effect(files), config)?;
/// for result in results {
///     println!("{}: {} functions", result.path.display(), result.metrics.complexity.functions.len());
/// }
/// ```
pub fn analyze_files_effect(paths: Vec<PathBuf>) -> AnalysisEffect<Vec<FileAnalysisResult>> {
    from_fn(move |env: &RealEnv| {
        let config = env.config();
        let parallel_config = config
            .batch_analysis
            .as_ref()
            .map(|b| &b.parallelism)
            .cloned()
            .unwrap_or_default();
        let collect_timing = config
            .batch_analysis
            .as_ref()
            .map(|b| b.collect_timing)
            .unwrap_or(false);

        let generated_mode = go_generated_code_mode(config);
        let solidity_config = solidity_config_from_debtmap(config);

        analyze_files_parallel_with_mode(
            &paths,
            &parallel_config,
            collect_timing,
            generated_mode,
            solidity_config,
        )
    })
    .boxed()
}

/// Analyze a single file as an Effect.
///
/// This is the fundamental unit of analysis wrapped as an Effect.
/// It reads the file, determines the language, and runs the appropriate analyzer.
pub fn analyze_single_file_effect(path: PathBuf) -> AnalysisEffect<FileAnalysisResult> {
    let path_display = path.display().to_string();
    from_fn(move |env: &RealEnv| {
        let content = env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e.message()), &path)
        })?;

        analyze_file_content(&path, &content, false).map_err(|e| {
            AnalysisError::analysis(format!("Analysis failed for '{}': {}", path_display, e))
        })
    })
    .boxed()
}

/// Analyze files with configuration-based settings.
///
/// This version uses the environment's BatchAnalysisConfig for settings.
pub fn analyze_files_with_config_effect(
    paths: Vec<PathBuf>,
    _config: BatchAnalysisConfig,
) -> AnalysisEffect<Vec<FileAnalysisResult>> {
    analyze_files_effect(paths)
}

// ============================================================================
// Validation-based Error Accumulation
// ============================================================================

/// Validate multiple files, accumulating ALL errors.
///
/// Unlike `analyze_files_effect` which fails at the first error,
/// this function uses `Validation` to collect all validation errors.
/// This is useful for providing comprehensive feedback to users.
///
/// # Arguments
///
/// * `paths` - Files to validate
///
/// # Returns
///
/// A Validation that is either:
/// - `Success(Vec<ValidatedFile>)` if all files are valid
/// - `Failure(NonEmptyVec<AnalysisError>)` with ALL validation errors
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::analyzers::batch::validate_files;
/// use debtmap::effects::run_validation;
///
/// let paths = vec!["src/main.rs".into(), "src/lib.rs".into()];
/// match run_validation(validate_files(&paths)) {
///     Ok(validated) => {
///         // All files validated successfully
///         for file in validated {
///             println!("{}: {} bytes", file.path.display(), file.content.len());
///         }
///     }
///     Err(errors) => {
///         // Some files failed validation - show ALL errors
///         eprintln!("Validation failed with {} errors:", errors);
///     }
/// }
/// ```
pub fn validate_files(paths: &[PathBuf]) -> AnalysisValidation<Vec<ValidatedFile>> {
    let validations: Vec<AnalysisValidation<ValidatedFile>> =
        paths.iter().map(|p| validate_single_file(p)).collect();

    combine_validations_preserve_successes(validations)
}

/// Validate a single file's existence and syntax.
///
/// Performs the following checks:
/// 1. File exists and is readable
/// 2. Content is valid UTF-8
/// 3. Language is supported
///
/// # Returns
///
/// A Validation with either the validated file or an error.
pub fn validate_single_file(path: &Path) -> AnalysisValidation<ValidatedFile> {
    // Check file existence and read content
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return validation_failure(AnalysisError::io_with_path(
                format!("Failed to read file: {}", e),
                path,
            ));
        }
    };

    // Determine language
    let language = Language::from_path(path);
    if language == Language::Unknown {
        return validation_failure(AnalysisError::validation_with_path(
            "Unsupported file type",
            path,
        ));
    }

    // Basic syntax validation for supported languages
    if let Err(e) = validate_syntax(&content, language, path) {
        return validation_failure(e);
    }

    validation_success(ValidatedFile::new(path.to_path_buf(), content, language))
}

/// Validate file syntax based on language.
///
/// Spec 202: Uses UnifiedFileExtractor for Rust files to ensure consistent
/// parsing and SourceMap management.
fn validate_syntax(content: &str, language: Language, path: &Path) -> Result<(), AnalysisError> {
    match language {
        Language::Rust => {
            // Use UnifiedFileExtractor for parsing (spec 202)
            // This ensures SourceMap is reset after parsing
            crate::extraction::UnifiedFileExtractor::extract(path, content).map_err(|e| {
                AnalysisError::parse_with_path(format!("Rust syntax error: {}", e), path)
            })?;
            Ok(())
        }
        Language::Python => {
            // Basic Python syntax check (could use rustpython-parser)
            // For now, just check for basic structure
            if content.contains("def ") || content.contains("class ") || content.trim().is_empty() {
                Ok(())
            } else {
                // Accept any non-empty Python file
                Ok(())
            }
        }
        Language::JavaScript | Language::TypeScript => {
            // Use tree-sitter for JS/TS syntax validation
            use crate::analyzers::typescript::parser::{detect_variant, parse_source};
            let variant = detect_variant(path);
            parse_source(content, path, variant).map_err(|e| {
                AnalysisError::parse_with_path(
                    format!("JavaScript/TypeScript syntax error: {}", e),
                    path,
                )
            })?;
            Ok(())
        }
        Language::Go => {
            crate::analyzers::go::parser::parse_source(content, path).map_err(|e| {
                AnalysisError::parse_with_path(format!("Go syntax error: {}", e), path)
            })?;
            Ok(())
        }
        Language::Solidity => {
            crate::analyzers::solidity::parser::parse_source(content, path).map_err(|e| {
                AnalysisError::parse_with_path(format!("Solidity syntax error: {}", e), path)
            })?;
            Ok(())
        }
        Language::Unknown => Err(AnalysisError::validation_with_path(
            "Cannot validate unknown language",
            path,
        )),
    }
}

/// Validate and then analyze files.
///
/// This combines validation with analysis, providing comprehensive
/// error reporting for validation failures while still performing
/// analysis on valid files.
///
/// # Returns
///
/// A Validation that is either:
/// - `Success(Vec<FileAnalysisResult>)` if all files are valid and analyzed
/// - `Failure(NonEmptyVec<AnalysisError>)` with all validation errors
pub fn validate_and_analyze_files(
    paths: &[PathBuf],
) -> AnalysisValidation<Vec<FileAnalysisResult>> {
    let validated = validate_files(paths);

    match validated {
        stillwater::Validation::Success(files) => {
            // All files validated, now analyze them
            let results: Vec<Result<FileAnalysisResult, AnalysisError>> = files
                .into_iter()
                .map(|f| analyze_validated_file(&f))
                .collect();

            // Collect results or errors
            let mut successes = Vec::new();
            let mut failures = Vec::new();

            for result in results {
                match result {
                    Ok(r) => successes.push(r),
                    Err(e) => failures.push(e),
                }
            }

            if failures.is_empty() {
                validation_success(successes)
            } else {
                validation_failures(failures)
            }
        }
        stillwater::Validation::Failure(errors) => stillwater::Validation::Failure(errors),
    }
}

// ============================================================================
// Internal Implementation
// ============================================================================

/// Analyze files in parallel using rayon.
#[cfg(test)]
fn analyze_files_parallel(
    paths: &[PathBuf],
    config: &ParallelConfig,
    collect_timing: bool,
) -> Result<Vec<FileAnalysisResult>, AnalysisError> {
    analyze_files_parallel_with_mode(
        paths,
        config,
        collect_timing,
        GeneratedCodeMode::SuppressDebt,
        SolidityLanguageConfig::default(),
    )
}

fn analyze_files_parallel_with_mode(
    paths: &[PathBuf],
    config: &ParallelConfig,
    collect_timing: bool,
    generated_mode: GeneratedCodeMode,
    solidity_config: SolidityLanguageConfig,
) -> Result<Vec<FileAnalysisResult>, AnalysisError> {
    let results: Result<Vec<Option<FileAnalysisResult>>, AnalysisError> =
        if !config.enabled || paths.len() <= 1 {
            // Sequential processing
            paths
                .iter()
                .map(|path| {
                    analyze_file_from_path_with_config(
                        path,
                        collect_timing,
                        generated_mode,
                        &solidity_config,
                    )
                })
                .collect()
        } else {
            // Parallel processing with rayon
            let batch_size = config.effective_batch_size();

            if paths.len() <= batch_size {
                // Single batch
                paths
                    .par_iter()
                    .map(|path| {
                        analyze_file_from_path_with_config(
                            path,
                            collect_timing,
                            generated_mode,
                            &solidity_config,
                        )
                    })
                    .collect()
            } else {
                // Chunked processing for large codebases
                paths
                    .chunks(batch_size)
                    .flat_map(|chunk| {
                        chunk
                            .par_iter()
                            .map(|path| {
                                analyze_file_from_path_with_config(
                                    path,
                                    collect_timing,
                                    generated_mode,
                                    &solidity_config,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect()
            }
        };

    results.map(|items| {
        let analyzed = items.into_iter().flatten().collect::<Vec<_>>();
        resolve_solidity_cross_file_calls(resolve_go_cross_file_calls(analyzed))
    })
}

/// Analyze a file from its path.
#[cfg(test)]
fn analyze_file_from_path(
    path: &Path,
    collect_timing: bool,
) -> Result<FileAnalysisResult, AnalysisError> {
    analyze_file_from_path_with_mode(path, collect_timing, GeneratedCodeMode::SuppressDebt)
        .map(|result| result.expect("default generated mode does not exclude files"))
}

#[cfg(test)]
fn analyze_file_from_path_with_mode(
    path: &Path,
    collect_timing: bool,
    generated_mode: GeneratedCodeMode,
) -> Result<Option<FileAnalysisResult>, AnalysisError> {
    analyze_file_from_path_with_config(
        path,
        collect_timing,
        generated_mode,
        &SolidityLanguageConfig::default(),
    )
}

fn analyze_file_from_path_with_config(
    path: &Path,
    collect_timing: bool,
    generated_mode: GeneratedCodeMode,
    solidity_config: &SolidityLanguageConfig,
) -> Result<Option<FileAnalysisResult>, AnalysisError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AnalysisError::io_with_path(format!("Failed to read file: {}", e), path))?;

    if should_exclude_go_file(path, &content, generated_mode)
        || should_exclude_solidity_file(path, &content, solidity_config)
    {
        return Ok(None);
    }

    analyze_file_content_with_config(
        path,
        &content,
        collect_timing,
        generated_mode,
        solidity_config,
    )
    .map(Some)
}

/// Analyze file content with the appropriate analyzer.
fn analyze_file_content(
    path: &Path,
    content: &str,
    collect_timing: bool,
) -> Result<FileAnalysisResult, AnalysisError> {
    analyze_file_content_with_mode(
        path,
        content,
        collect_timing,
        GeneratedCodeMode::SuppressDebt,
    )
}

fn analyze_file_content_with_mode(
    path: &Path,
    content: &str,
    collect_timing: bool,
    generated_mode: GeneratedCodeMode,
) -> Result<FileAnalysisResult, AnalysisError> {
    analyze_file_content_with_config(
        path,
        content,
        collect_timing,
        generated_mode,
        &SolidityLanguageConfig::default(),
    )
}

fn analyze_file_content_with_config(
    path: &Path,
    content: &str,
    collect_timing: bool,
    generated_mode: GeneratedCodeMode,
    solidity_config: &SolidityLanguageConfig,
) -> Result<FileAnalysisResult, AnalysisError> {
    let start = if collect_timing {
        Some(Instant::now())
    } else {
        None
    };

    let language = Language::from_path(path);
    let analyzer = get_analyzer_for_mode(language, generated_mode, solidity_config);

    let ast = analyzer
        .parse(content, path.to_path_buf())
        .map_err(|e| AnalysisError::parse_with_path(format!("Parse failed: {}", e), path))?;

    let package_name = package_name_for_ast(&ast);
    let metrics = analyzer.analyze(&ast);
    let debt_items = metrics.debt_items.clone();

    // Reset SourceMap to prevent overflow when parsing many files.
    // Safe here because we've extracted all metrics (including line numbers) from the AST.
    crate::core::parsing::reset_span_locations();

    let analysis_time = start.map(|s| s.elapsed());

    Ok(FileAnalysisResult {
        path: path.to_path_buf(),
        metrics,
        debt_items,
        analysis_time,
        package_name,
    })
}

fn get_analyzer_for_mode(
    language: Language,
    generated_mode: GeneratedCodeMode,
    solidity_config: &SolidityLanguageConfig,
) -> Box<dyn crate::analyzers::Analyzer> {
    match language {
        Language::Go => Box::new(
            crate::analyzers::go::GoAnalyzer::new().with_generated_code_mode(generated_mode),
        ),
        Language::Solidity => Box::new(
            crate::analyzers::solidity::SolidityAnalyzer::new()
                .with_config(solidity_config.clone()),
        ),
        _ => get_analyzer(language),
    }
}

fn should_exclude_go_file(path: &Path, content: &str, generated_mode: GeneratedCodeMode) -> bool {
    generated_mode == GeneratedCodeMode::Exclude
        && Language::from_path(path) == Language::Go
        && crate::analyzers::go::generated::is_generated_go(path, content)
}

fn should_exclude_solidity_file(
    path: &Path,
    content: &str,
    config: &SolidityLanguageConfig,
) -> bool {
    config.vendor_code == GeneratedCodeMode::Exclude
        && Language::from_path(path) == Language::Solidity
        && crate::analyzers::solidity::generated::is_vendor_or_generated_solidity(path, content)
}

fn go_generated_code_mode(config: &crate::config::DebtmapConfig) -> GeneratedCodeMode {
    config
        .languages
        .as_ref()
        .and_then(|languages| languages.go.as_ref())
        .map(|go| go.generated_code)
        .unwrap_or_default()
}

fn solidity_config_from_debtmap(config: &crate::config::DebtmapConfig) -> SolidityLanguageConfig {
    config
        .languages
        .as_ref()
        .and_then(|languages| languages.solidity.clone())
        .unwrap_or_default()
}

fn package_name_for_ast(ast: &Ast) -> Option<String> {
    match ast {
        Ast::Go(go_ast) => crate::analyzers::go::visitor::package_name_from_ast(go_ast),
        _ => None,
    }
}

/// Analyze a pre-validated file.
fn analyze_validated_file(file: &ValidatedFile) -> Result<FileAnalysisResult, AnalysisError> {
    analyze_file_content(&file.path, &file.content, false)
}

fn resolve_go_cross_file_calls(mut results: Vec<FileAnalysisResult>) -> Vec<FileAnalysisResult> {
    let symbol_index = go_symbol_index(&results);
    let import_maps = go_import_maps(&results);
    let mut edges: Vec<(GoFunctionKey, String, GoFunctionKey)> = Vec::new();

    for result in results.iter_mut().filter(|result| is_go_result(result)) {
        let package = go_package_key(result);
        let imports = import_maps.get(&result.path).cloned().unwrap_or_default();

        for function in &mut result.metrics.complexity.functions {
            let resolved_calls = function
                .call_dependencies
                .clone()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|call| resolve_go_call(&call, &package, &imports, &symbol_index))
                .collect::<Vec<_>>();

            let downstream = resolved_calls
                .iter()
                .map(|call| call.display_name.clone())
                .collect::<Vec<_>>();
            let caller = GoFunctionKey::new(package.clone(), function.name.clone());
            edges.extend(
                resolved_calls
                    .into_iter()
                    .map(|call| (caller.clone(), function.name.clone(), call.key)),
            );

            function.call_dependencies = (!downstream.is_empty()).then_some(downstream.clone());
            function.downstream_callees = (!downstream.is_empty()).then_some(downstream);
        }
    }

    let upstream = go_upstream_callers(edges);
    for result in results.iter_mut().filter(|result| is_go_result(result)) {
        let package = go_package_key(result);
        for function in &mut result.metrics.complexity.functions {
            let key = GoFunctionKey::new(package.clone(), function.name.clone());
            function.upstream_callers = upstream.get(&key).cloned();
        }
    }

    results
}

fn resolve_solidity_cross_file_calls(
    mut results: Vec<FileAnalysisResult>,
) -> Vec<FileAnalysisResult> {
    let index = solidity_function_index(&results);
    let mut edges: Vec<(String, String)> = Vec::new();

    for result in results
        .iter_mut()
        .filter(|result| is_solidity_result(result))
    {
        for function in &mut result.metrics.complexity.functions {
            let downstream = function
                .call_dependencies
                .clone()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|call| resolve_solidity_call(&call, &index))
                .collect::<Vec<_>>();

            if downstream.is_empty() {
                continue;
            }

            edges.extend(
                downstream
                    .iter()
                    .cloned()
                    .map(|callee| (function.name.clone(), callee)),
            );
            function.downstream_callees = Some(downstream.clone());
            function.call_dependencies = Some(downstream);
        }
    }

    let upstream = solidity_upstream_callers(edges);
    for result in results
        .iter_mut()
        .filter(|result| is_solidity_result(result))
    {
        for function in &mut result.metrics.complexity.functions {
            function.upstream_callers = upstream.get(&function.name).cloned();
        }
    }

    results
}

fn solidity_function_index(results: &[FileAnalysisResult]) -> HashMap<String, Vec<String>> {
    results
        .iter()
        .filter(|result| is_solidity_result(result))
        .flat_map(|result| result.metrics.complexity.functions.iter())
        .fold(HashMap::new(), |mut index, function| {
            index
                .entry(solidity_short_name(&function.name))
                .or_insert_with(Vec::new)
                .push(function.name.clone());
            index
        })
}

fn resolve_solidity_call(call: &str, index: &HashMap<String, Vec<String>>) -> Option<String> {
    let short = solidity_short_name(call);
    index.get(&short).and_then(|matches| {
        (matches.len() == 1)
            .then(|| matches.first())
            .flatten()
            .cloned()
    })
}

fn solidity_short_name(name: &str) -> String {
    name.rsplit('.').next().unwrap_or(name).to_string()
}

fn solidity_upstream_callers(edges: Vec<(String, String)>) -> HashMap<String, Vec<String>> {
    edges
        .into_iter()
        .fold(HashMap::new(), |mut upstream, (caller, callee)| {
            let callers = upstream.entry(callee).or_insert_with(Vec::new);
            if !callers.contains(&caller) {
                callers.push(caller);
            }
            upstream
        })
}

fn is_solidity_result(result: &FileAnalysisResult) -> bool {
    result.metrics.language == Language::Solidity
}

#[derive(Debug, Clone)]
struct ResolvedGoCall {
    display_name: String,
    key: GoFunctionKey,
}

fn resolve_go_call(
    call: &str,
    current_package: &GoPackageKey,
    imports: &HashMap<String, String>,
    index: &GoSymbolIndex,
) -> Option<ResolvedGoCall> {
    imported_go_call(call, imports, index)
        .or_else(|| same_package_go_call(call, current_package, index))
}

fn same_package_go_call(
    call: &str,
    package: &GoPackageKey,
    index: &GoSymbolIndex,
) -> Option<ResolvedGoCall> {
    index.by_package.get(package).and_then(|symbols| {
        symbols.contains(call).then(|| ResolvedGoCall {
            display_name: call.to_string(),
            key: GoFunctionKey::new(package.clone(), call.to_string()),
        })
    })
}

fn imported_go_call(
    call: &str,
    imports: &HashMap<String, String>,
    index: &GoSymbolIndex,
) -> Option<ResolvedGoCall> {
    let (alias, function_name) = call.split_once('.')?;
    let import_path = imports.get(alias)?;
    let package = index.by_import_path.get(import_path)?;
    let symbols = index.by_package.get(package)?;

    symbols
        .free_functions
        .contains(function_name)
        .then(|| ResolvedGoCall {
            display_name: function_name.to_string(),
            key: GoFunctionKey::new(package.clone(), function_name.to_string()),
        })
}

#[derive(Debug, Clone, Default)]
struct GoPackageSymbols {
    free_functions: HashSet<String>,
    methods: HashSet<String>,
}

impl GoPackageSymbols {
    fn insert(&mut self, name: &str) {
        if name.contains('.') {
            self.methods.insert(name.to_string());
        } else {
            self.free_functions.insert(name.to_string());
        }
    }

    fn contains(&self, call: &str) -> bool {
        if call.contains('.') {
            self.methods.contains(call)
        } else {
            self.free_functions.contains(call)
        }
    }
}

#[derive(Debug, Clone, Default)]
struct GoSymbolIndex {
    by_package: HashMap<GoPackageKey, GoPackageSymbols>,
    by_import_path: HashMap<String, GoPackageKey>,
}

fn go_symbol_index(results: &[FileAnalysisResult]) -> GoSymbolIndex {
    results.iter().filter(|result| is_go_result(result)).fold(
        GoSymbolIndex::default(),
        |mut index, result| {
            let package = go_package_key(result);
            if is_importable_go_package(&package) {
                let import_path = package.import_path.clone().unwrap_or_default();
                index.by_import_path.insert(import_path, package.clone());
            }
            let package_symbols = index.by_package.entry(package).or_default();
            for function in &result.metrics.complexity.functions {
                package_symbols.insert(&function.name);
            }
            index
        },
    )
}

fn is_importable_go_package(package: &GoPackageKey) -> bool {
    package.import_path.is_some()
        && package
            .package_name
            .as_deref()
            .map(|name| !name.ends_with("_test"))
            .unwrap_or(true)
}

fn go_upstream_callers(
    edges: Vec<(GoFunctionKey, String, GoFunctionKey)>,
) -> HashMap<GoFunctionKey, Vec<String>> {
    edges
        .into_iter()
        .fold(HashMap::new(), |mut upstream, edge| {
            let (_caller_key, caller_name, callee_key) = edge;
            upstream.entry(callee_key).or_default().push(caller_name);
            upstream
        })
}

fn go_import_maps(results: &[FileAnalysisResult]) -> HashMap<PathBuf, HashMap<String, String>> {
    results
        .iter()
        .filter(|result| is_go_result(result))
        .fold(HashMap::new(), |mut maps, result| {
            let imports = std::fs::read_to_string(&result.path)
                .map(|source| go_import_aliases(&source))
                .unwrap_or_default();
            maps.insert(result.path.clone(), imports);
            maps
        })
}

fn is_go_result(result: &FileAnalysisResult) -> bool {
    result.metrics.language == Language::Go
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GoPackageKey {
    directory: PathBuf,
    package_name: Option<String>,
    import_path: Option<String>,
}

fn go_package_key(result: &FileAnalysisResult) -> GoPackageKey {
    let directory = result
        .path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_path_buf();

    GoPackageKey {
        import_path: go_package_import_path(&directory),
        directory,
        package_name: result.package_name.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GoFunctionKey {
    package: GoPackageKey,
    name: String,
}

impl GoFunctionKey {
    fn new(package: GoPackageKey, name: String) -> Self {
        Self { package, name }
    }
}

fn go_package_import_path(directory: &Path) -> Option<String> {
    let module = nearest_go_module(directory)?;
    let relative = directory.strip_prefix(&module.root).ok()?;
    Some(join_go_import_path(&module.path, relative))
}

#[derive(Debug, Clone)]
struct GoModule {
    root: PathBuf,
    path: String,
}

fn nearest_go_module(directory: &Path) -> Option<GoModule> {
    directory.ancestors().filter_map(go_module_at).next()
}

fn go_module_at(directory: &Path) -> Option<GoModule> {
    let go_mod = directory.join("go.mod");
    let source = std::fs::read_to_string(go_mod).ok()?;
    parse_go_module_path(&source).map(|path| GoModule {
        root: directory.to_path_buf(),
        path,
    })
}

fn parse_go_module_path(source: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let line = line.split("//").next().unwrap_or("").trim();
        line.strip_prefix("module ")
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_string)
    })
}

fn join_go_import_path(module_path: &str, relative: &Path) -> String {
    let relative_path = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");

    if relative_path.is_empty() {
        module_path.to_string()
    } else {
        format!("{module_path}/{relative_path}")
    }
}

fn go_import_aliases(source: &str) -> HashMap<String, String> {
    go_import_specs(source)
        .into_iter()
        .filter_map(|spec| {
            let alias = go_import_alias(&spec)?;
            let path = go_import_path(&spec)?;
            Some((alias, path))
        })
        .filter(|(alias, _)| alias != "." && alias != "_")
        .collect()
}

fn go_import_specs(source: &str) -> Vec<String> {
    source
        .lines()
        .fold(ImportScan::default(), |scan, line| scan.next(line))
        .specs
}

#[derive(Debug, Clone, Default)]
struct ImportScan {
    in_block: bool,
    specs: Vec<String>,
}

impl ImportScan {
    fn next(mut self, line: &str) -> Self {
        let trimmed = line.trim();
        if self.in_block {
            if trimmed.starts_with(')') {
                self.in_block = false;
            } else {
                self.specs.push(trimmed.to_string());
            }
            return self;
        }

        if let Some(rest) = trimmed.strip_prefix("import ") {
            if rest.trim_start().starts_with('(') {
                self.in_block = true;
            } else {
                self.specs.push(rest.trim().to_string());
            }
        }

        self
    }
}

fn go_import_alias(spec: &str) -> Option<String> {
    let quote_index = spec.find('"').or_else(|| spec.find('`'))?;
    let prefix = spec[..quote_index].trim();
    prefix
        .split_whitespace()
        .last()
        .map(str::to_string)
        .or_else(|| {
            go_import_path(spec).and_then(|path| path.rsplit('/').next().map(str::to_string))
        })
}

fn go_import_path(spec: &str) -> Option<String> {
    let start = spec.find('"').or_else(|| spec.find('`'))?;
    let quote = spec.as_bytes()[start] as char;
    let rest = &spec[start + 1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

/// Combine validations while preserving successful values.
fn combine_validations_preserve_successes<T>(
    validations: Vec<AnalysisValidation<T>>,
) -> AnalysisValidation<Vec<T>> {
    let mut successes = Vec::new();
    let mut failures: Vec<AnalysisError> = Vec::new();

    for v in validations {
        match v {
            stillwater::Validation::Success(value) => successes.push(value),
            stillwater::Validation::Failure(errors) => {
                for err in errors {
                    failures.push(err);
                }
            }
        }
    }

    if failures.is_empty() {
        validation_success(successes)
    } else {
        validation_failures(failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::run_validation;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_file_analysis_result_new() {
        let path = PathBuf::from("test.rs");
        let metrics = FileMetrics {
            path: path.clone(),
            language: Language::Rust,
            complexity: Default::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: None,
        };

        let result = FileAnalysisResult::new(path.clone(), metrics, vec![]);

        assert_eq!(result.path, path);
        assert!(result.analysis_time.is_none());
    }

    #[test]
    fn test_file_analysis_result_with_timing() {
        let path = PathBuf::from("test.rs");
        let metrics = FileMetrics {
            path: path.clone(),
            language: Language::Rust,
            complexity: Default::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: 0,
            module_scope: None,
            classes: None,
        };

        let duration = Duration::from_millis(100);
        let result = FileAnalysisResult::with_timing(path.clone(), metrics, vec![], duration);

        assert_eq!(result.path, path);
        assert_eq!(result.analysis_time, Some(duration));
    }

    #[test]
    fn test_validated_file_new() {
        let file = ValidatedFile::new(
            PathBuf::from("test.rs"),
            "fn main() {}".to_string(),
            Language::Rust,
        );

        assert_eq!(file.path, PathBuf::from("test.rs"));
        assert_eq!(file.content, "fn main() {}");
        assert_eq!(file.language, Language::Rust);
    }

    #[test]
    fn test_validate_single_file_success() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = validate_single_file(&file_path);

        assert!(result.is_success());
        match result {
            stillwater::Validation::Success(file) => {
                assert_eq!(file.path, file_path);
                assert_eq!(file.language, Language::Rust);
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_validate_single_file_not_found() {
        let result = validate_single_file(Path::new("/nonexistent/file.rs"));

        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_single_file_unknown_language() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.unknown");
        fs::write(&file_path, "some content").unwrap();

        let result = validate_single_file(&file_path);

        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_single_file_syntax_error() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main( { }").unwrap(); // Invalid Rust syntax

        let result = validate_single_file(&file_path);

        assert!(result.is_failure());
    }

    #[test]
    fn test_validate_files_all_success() {
        let temp_dir = create_test_dir();
        let file1 = temp_dir.path().join("a.rs");
        let file2 = temp_dir.path().join("b.rs");
        fs::write(&file1, "fn a() {}").unwrap();
        fs::write(&file2, "fn b() {}").unwrap();

        let paths = vec![file1, file2];
        let result = run_validation(validate_files(&paths));

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_validate_files_accumulates_errors() {
        let temp_dir = create_test_dir();
        let valid_file = temp_dir.path().join("valid.rs");
        fs::write(&valid_file, "fn valid() {}").unwrap();

        let paths = vec![
            valid_file,
            PathBuf::from("/nonexistent/a.rs"),
            PathBuf::from("/nonexistent/b.rs"),
        ];
        let result = validate_files(&paths);

        // Should accumulate BOTH nonexistent file errors
        match result {
            stillwater::Validation::Failure(errors) => {
                let errors_vec: Vec<_> = errors.into_iter().collect();
                assert_eq!(errors_vec.len(), 2);
            }
            _ => panic!("Expected failure with accumulated errors"),
        }
    }

    #[test]
    fn test_analyze_file_from_path() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() { let x = 1; }").unwrap();

        let result = analyze_file_from_path(&file_path, false);

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.path, file_path);
        assert!(analysis.analysis_time.is_none());
    }

    #[test]
    fn test_analyze_file_from_path_with_timing() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = analyze_file_from_path(&file_path, true);

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.analysis_time.is_some());
    }

    #[test]
    fn test_analyze_files_parallel_sequential() {
        let temp_dir = create_test_dir();
        let file1 = temp_dir.path().join("a.rs");
        let file2 = temp_dir.path().join("b.rs");
        fs::write(&file1, "fn a() {}").unwrap();
        fs::write(&file2, "fn b() {}").unwrap();

        let config = ParallelConfig::sequential();
        let result = analyze_files_parallel(&[file1, file2], &config, false);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_analyze_files_parallel_enabled() {
        let temp_dir = create_test_dir();
        let file1 = temp_dir.path().join("a.rs");
        let file2 = temp_dir.path().join("b.rs");
        let file3 = temp_dir.path().join("c.rs");
        fs::write(&file1, "fn a() {}").unwrap();
        fs::write(&file2, "fn b() {}").unwrap();
        fs::write(&file3, "fn c() {}").unwrap();

        let config = ParallelConfig::default();
        let result = analyze_files_parallel(&[file1, file2, file3], &config, false);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_validate_and_analyze_files_success() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let result = run_validation(validate_and_analyze_files(std::slice::from_ref(&file_path)));

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, file_path);
    }

    #[test]
    fn test_validate_and_analyze_files_validation_failure() {
        let paths = vec![PathBuf::from("/nonexistent/file.rs")];
        let result = run_validation(validate_and_analyze_files(&paths));

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_syntax_rust_valid() {
        let content = "fn main() {}";
        let result = validate_syntax(content, Language::Rust, Path::new("test.rs"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_syntax_rust_invalid() {
        let content = "fn main( { }"; // Missing closing paren
        let result = validate_syntax(content, Language::Rust, Path::new("test.rs"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_syntax_python() {
        let content = "def hello():\n    pass";
        let result = validate_syntax(content, Language::Python, Path::new("test.py"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_syntax_go_valid() {
        let content = "package main\n\nfunc main() {}";
        let result = validate_syntax(content, Language::Go, Path::new("main.go"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_go_module_path() {
        let source = "// comment\nmodule example.com/app\n\ngo 1.22\n";

        assert_eq!(
            super::parse_go_module_path(source),
            Some("example.com/app".to_string())
        );
    }

    #[test]
    fn test_join_go_import_path() {
        assert_eq!(
            super::join_go_import_path("example.com/app", Path::new("internal/mathx")),
            "example.com/app/internal/mathx"
        );
        assert_eq!(
            super::join_go_import_path("example.com/app", Path::new("")),
            "example.com/app"
        );
    }

    #[test]
    fn test_go_import_aliases() {
        let source = r#"package main

import (
    "fmt"
    util "example.com/app/internal/mathx"
    _ "example.com/app/internal/sideeffect"
    . "example.com/app/internal/dot"
)
"#;
        let aliases = super::go_import_aliases(source);

        assert_eq!(aliases.get("fmt"), Some(&"fmt".to_string()));
        assert_eq!(
            aliases.get("util"),
            Some(&"example.com/app/internal/mathx".to_string())
        );
        assert!(!aliases.contains_key("_"));
        assert!(!aliases.contains_key("."));
    }

    #[test]
    fn test_validate_syntax_go_invalid() {
        let content = "package main\n\nfunc main( {}";
        let result = validate_syntax(content, Language::Go, Path::new("main.go"));
        assert!(result.is_err());
    }

    #[test]
    fn test_combine_validations_preserve_successes() {
        let validations: Vec<AnalysisValidation<i32>> = vec![
            validation_success(1),
            validation_success(2),
            validation_success(3),
        ];

        let result = combine_validations_preserve_successes(validations);

        match result {
            stillwater::Validation::Success(values) => {
                assert_eq!(values, vec![1, 2, 3]);
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_combine_validations_with_failures() {
        let validations: Vec<AnalysisValidation<i32>> = vec![
            validation_success(1),
            validation_failure(AnalysisError::validation("Error 1")),
            validation_success(3),
            validation_failure(AnalysisError::validation("Error 2")),
        ];

        let result = combine_validations_preserve_successes(validations);

        match result {
            stillwater::Validation::Failure(errors) => {
                let errors_vec: Vec<_> = errors.into_iter().collect();
                assert_eq!(errors_vec.len(), 2);
            }
            _ => panic!("Expected failure"),
        }
    }
}
