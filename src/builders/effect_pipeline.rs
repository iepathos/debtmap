//! Effect-based analysis pipelines for debtmap.
//!
//! This module provides Effect-based wrappers around analysis operations,
//! enabling pure functional composition of file reading, parsing, and analysis.
//!
//! # Architecture
//!
//! The analysis pipeline is structured as:
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌───────────────┐
//! │  File Read   │ ──> │    Parse     │ ──> │   Analyze     │
//! │   (Effect)   │     │   (Pure)     │     │    (Pure)     │
//! └──────────────┘     └──────────────┘     └───────────────┘
//!        │                                          │
//!        v                                          v
//! ┌──────────────┐                         ┌───────────────┐
//! │   Coverage   │                         │    Cache      │
//! │   (Effect)   │                         │   (Effect)    │
//! └──────────────┘                         └───────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::builders::effect_pipeline::{analyze_file_effect, analyze_files_parallel_effect};
//! use debtmap::effects::run_effect;
//!
//! // Analyze a single file
//! let effect = analyze_file_effect("src/main.rs".into());
//! let metrics = run_effect(effect, config)?;
//!
//! // Analyze multiple files in parallel
//! let effect = analyze_files_parallel_effect(files);
//! let all_metrics = run_effect(effect, config)?;
//! ```

use crate::analyzers::get_analyzer;
use crate::core::{FunctionMetrics, Language};
use crate::effects::AnalysisEffect;
use crate::env::RealEnv;
use crate::errors::AnalysisError;
use crate::io::effects::{
    cache_get_effect, cache_set_effect, read_file_effect, walk_dir_with_config_effect,
};
use crate::risk::effects::load_coverage_optional_effect;
use std::path::PathBuf;
use stillwater::Effect;

// ============================================================================
// Single File Analysis
// ============================================================================

/// Analyze a single file and return function metrics as an Effect.
///
/// This effect composes:
/// 1. Reading the file content
/// 2. Detecting the language
/// 3. Parsing with the appropriate analyzer
/// 4. Extracting complexity metrics
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::builders::effect_pipeline::analyze_file_effect;
///
/// let effect = analyze_file_effect("src/main.rs".into());
/// let metrics = run_effect(effect, config)?;
///
/// for func in metrics {
///     println!("{}: complexity={}", func.name, func.cyclomatic);
/// }
/// ```
pub fn analyze_file_effect(path: PathBuf) -> AnalysisEffect<Vec<FunctionMetrics>> {
    let path_for_error = path.clone();
    let path_for_analysis = path.clone();

    read_file_effect(path)
        .and_then(move |content| {
            Effect::from_fn(move |_env: &RealEnv| {
                analyze_content_pure(&content, &path_for_analysis)
            })
        })
        .map_err(move |e| {
            AnalysisError::analysis(format!(
                "Failed to analyze '{}': {}",
                path_for_error.display(),
                e.message()
            ))
        })
}

/// Analyze file content directly (pure function, no I/O).
///
/// This is useful when you already have the file content and don't need
/// to read it from disk.
fn analyze_content_pure(
    content: &str,
    path: &std::path::Path,
) -> Result<Vec<FunctionMetrics>, AnalysisError> {
    let language = Language::from_path(path);
    let analyzer = get_analyzer(language);

    let ast = analyzer
        .parse(content, path.to_path_buf())
        .map_err(|e| AnalysisError::parse_with_context(e.to_string(), path.to_path_buf(), 0))?;

    let metrics = analyzer.analyze(&ast);
    Ok(metrics.complexity.functions)
}

/// Analyze a file with caching.
///
/// Checks the cache first for previously analyzed results. If not found,
/// analyzes the file and stores the result in the cache.
///
/// # Cache Key Format
///
/// The cache key is: `analysis:{file_path}`
pub fn analyze_file_cached_effect(path: PathBuf) -> AnalysisEffect<Vec<FunctionMetrics>> {
    let cache_key = format!("analysis:{}", path.display());
    let path_for_analysis = path.clone();
    let cache_key_for_set = cache_key.clone();

    // Try cache first
    cache_get_effect::<Vec<FunctionMetrics>>(cache_key).and_then(move |cached| {
        match cached {
            Some(metrics) => Effect::pure(metrics),
            None => {
                // Not in cache, analyze and cache
                analyze_file_effect(path_for_analysis).and_then(move |metrics| {
                    cache_set_effect(cache_key_for_set, metrics.clone()).map(move |_| metrics)
                })
            }
        }
    })
}

// ============================================================================
// Multi-File Analysis
// ============================================================================

/// Analyze multiple files sequentially.
///
/// For better performance with many files, use `analyze_files_parallel_effect`.
///
/// # Example
///
/// ```rust,ignore
/// let files = vec!["src/main.rs".into(), "src/lib.rs".into()];
/// let effect = analyze_files_effect(files);
/// let all_metrics = run_effect(effect, config)?;
/// ```
pub fn analyze_files_effect(paths: Vec<PathBuf>) -> AnalysisEffect<Vec<Vec<FunctionMetrics>>> {
    if paths.is_empty() {
        return Effect::pure(Vec::new());
    }

    let mut effects: Vec<AnalysisEffect<Vec<FunctionMetrics>>> =
        paths.into_iter().map(analyze_file_effect).collect();

    let first = effects.remove(0);
    effects
        .into_iter()
        .fold(first.map(|m| vec![m]), |acc, eff| {
            acc.and_then(move |mut results| {
                eff.map(move |m| {
                    results.push(m);
                    results
                })
            })
        })
}

/// Analyze multiple files with parallel execution.
///
/// This uses rayon's parallel iterators for CPU-intensive analysis,
/// while keeping the I/O operations sequential within the effect system.
///
/// # Note
///
/// The parallelism is achieved by running the actual analysis in parallel
/// after collecting file contents. This maintains the pure Effect semantics
/// while still benefiting from parallel processing.
pub fn analyze_files_parallel_effect(
    paths: Vec<PathBuf>,
) -> AnalysisEffect<Vec<Vec<FunctionMetrics>>> {
    if paths.is_empty() {
        return Effect::pure(Vec::new());
    }

    // Read all files first (I/O)
    let read_effects: Vec<AnalysisEffect<(PathBuf, String)>> = paths
        .into_iter()
        .map(|p| {
            let path = p.clone();
            read_file_effect(p).map(move |content| (path, content))
        })
        .collect();

    // Sequence all reads
    sequence_effects(read_effects).and_then(|file_contents| {
        Effect::from_fn(move |_env: &RealEnv| {
            use rayon::prelude::*;

            // Parallel analysis
            let results: Vec<Result<Vec<FunctionMetrics>, AnalysisError>> = file_contents
                .par_iter()
                .map(|(path, content)| analyze_content_pure(content, path))
                .collect();

            // Collect results, failing on first error
            results.into_iter().collect()
        })
    })
}

/// Discover and analyze all files in a directory.
///
/// This effect composes:
/// 1. Walking the directory to find source files
/// 2. Filtering by language
/// 3. Analyzing each file in parallel
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::builders::effect_pipeline::analyze_directory_effect;
///
/// let effect = analyze_directory_effect("src".into(), vec![Language::Rust]);
/// let all_metrics = run_effect(effect, config)?;
/// ```
pub fn analyze_directory_effect(
    path: PathBuf,
    languages: Vec<Language>,
) -> AnalysisEffect<Vec<Vec<FunctionMetrics>>> {
    walk_dir_with_config_effect(path, languages).and_then(analyze_files_parallel_effect)
}

// ============================================================================
// Analysis with Coverage
// ============================================================================

/// File analysis result including coverage data.
#[derive(Debug, Clone)]
pub struct FileAnalysisWithCoverage {
    /// Path to the analyzed file
    pub path: PathBuf,
    /// Function metrics from analysis
    pub functions: Vec<FunctionMetrics>,
    /// File-level coverage percentage (if available)
    pub coverage_percent: Option<f64>,
}

/// Analyze a file and include coverage data.
///
/// This composes file analysis with coverage loading for comprehensive
/// risk assessment.
pub fn analyze_file_with_coverage_effect(
    path: PathBuf,
    coverage_path: PathBuf,
) -> AnalysisEffect<FileAnalysisWithCoverage> {
    let path_for_result = path.clone();

    analyze_file_effect(path).and_then(move |functions| {
        load_coverage_optional_effect(coverage_path).map(move |coverage| {
            let coverage_percent =
                coverage.get_file_coverage(std::path::Path::new(&path_for_result));
            FileAnalysisWithCoverage {
                path: path_for_result,
                functions,
                coverage_percent,
            }
        })
    })
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Sequence a vector of effects into an effect of vector.
fn sequence_effects<T>(effects: Vec<AnalysisEffect<T>>) -> AnalysisEffect<Vec<T>>
where
    T: Send + 'static,
{
    if effects.is_empty() {
        return Effect::pure(Vec::new());
    }

    let mut iter = effects.into_iter();
    let first = iter.next().unwrap();

    iter.fold(first.map(|x| vec![x]), |acc, eff| {
        acc.and_then(move |mut results| {
            eff.map(move |x| {
                results.push(x);
                results
            })
        })
    })
}

// ============================================================================
// Backwards-Compatible Wrappers
// ============================================================================

/// Analyze a file synchronously (backwards-compatible wrapper).
///
/// This is provided for compatibility with existing code that doesn't
/// use the effect system. Prefer `analyze_file_effect` for new code.
pub fn analyze_file(path: &std::path::Path) -> anyhow::Result<Vec<FunctionMetrics>> {
    let config = crate::config::DebtmapConfig::default();
    crate::effects::run_effect(analyze_file_effect(path.to_path_buf()), config)
}

/// Analyze multiple files synchronously (backwards-compatible wrapper).
pub fn analyze_files(paths: &[PathBuf]) -> anyhow::Result<Vec<Vec<FunctionMetrics>>> {
    let config = crate::config::DebtmapConfig::default();
    crate::effects::run_effect(analyze_files_parallel_effect(paths.to_vec()), config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::effects::run_effect;
    use tempfile::TempDir;

    fn create_test_env() -> (TempDir, DebtmapConfig) {
        let temp_dir = TempDir::new().unwrap();
        (temp_dir, DebtmapConfig::default())
    }

    fn create_rust_file_content() -> &'static str {
        r#"
fn simple_function() {
    println!("hello");
}

fn complex_function(x: i32) -> i32 {
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
"#
    }

    #[test]
    fn test_analyze_file_effect() {
        let (temp_dir, config) = create_test_env();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, create_rust_file_content()).unwrap();

        let effect = analyze_file_effect(file_path);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.len(), 2);

        // Find complex_function
        let complex_func = metrics.iter().find(|m| m.name == "complex_function");
        assert!(complex_func.is_some());
        let complex_func = complex_func.unwrap();
        assert!(complex_func.cyclomatic > 1);
    }

    #[test]
    fn test_analyze_file_effect_not_found() {
        let config = DebtmapConfig::default();
        let effect = analyze_file_effect("/nonexistent/test.rs".into());
        let result = run_effect(effect, config);

        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_files_effect() {
        let (temp_dir, config) = create_test_env();

        let file1 = temp_dir.path().join("a.rs");
        let file2 = temp_dir.path().join("b.rs");

        std::fs::write(&file1, "fn a() {}").unwrap();
        std::fs::write(&file2, "fn b() {}").unwrap();

        let effect = analyze_files_effect(vec![file1, file2]);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].len(), 1);
        assert_eq!(metrics[1].len(), 1);
    }

    #[test]
    fn test_analyze_files_parallel_effect() {
        let (temp_dir, config) = create_test_env();

        let files: Vec<PathBuf> = (0..5)
            .map(|i| {
                let path = temp_dir.path().join(format!("file{}.rs", i));
                std::fs::write(&path, format!("fn func{}() {{}}", i)).unwrap();
                path
            })
            .collect();

        let effect = analyze_files_parallel_effect(files);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.len(), 5);
    }

    #[test]
    fn test_analyze_directory_effect() {
        let (temp_dir, config) = create_test_env();

        // Create a src directory with some files
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(src_dir.join("lib.rs"), "pub fn hello() {}").unwrap();

        let effect = analyze_directory_effect(src_dir, vec![Language::Rust]);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.len(), 2);
    }

    #[test]
    fn test_analyze_file_cached_effect() {
        let (temp_dir, config) = create_test_env();
        let file_path = temp_dir.path().join("cached.rs");
        std::fs::write(&file_path, "fn cached() {}").unwrap();

        // First call - should analyze and cache
        let effect1 = analyze_file_cached_effect(file_path.clone());
        let result1 = run_effect(effect1, config.clone());
        assert!(result1.is_ok());

        // Second call - should hit cache
        let effect2 = analyze_file_cached_effect(file_path);
        let result2 = run_effect(effect2, config);
        assert!(result2.is_ok());

        // Results should be the same
        assert_eq!(result1.unwrap().len(), result2.unwrap().len());
    }

    #[test]
    fn test_analyze_content_pure() {
        let content = "fn test() { if true { println!(\"hi\"); } }";
        let path = PathBuf::from("test.rs");

        let result = analyze_content_pure(content, &path);
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "test");
    }

    #[test]
    fn test_backwards_compatible_analyze_file() {
        let (temp_dir, _) = create_test_env();
        let file_path = temp_dir.path().join("compat.rs");
        std::fs::write(&file_path, "fn compat() {}").unwrap();

        let result = analyze_file(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_sequence_effects_empty() {
        let config = DebtmapConfig::default();
        let effect: AnalysisEffect<Vec<i32>> = sequence_effects(vec![]);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
