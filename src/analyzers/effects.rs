//! Effect-based analyzers for project analysis (Spec 207).
//!
//! This module provides effect-based interfaces for analyzing files and projects,
//! enabling configuration access via the Reader pattern and supporting testability
//! with `DebtmapTestEnv`.
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::analyzers::effects::analyze_file_effect;
//!
//! let effect = analyze_file_effect(path.clone(), content);
//! let metrics = run_effect(effect, config)?;
//! ```

use super::{analyze_file, get_analyzer, Analyzer};
use crate::analysis::effects::{analyze_with_env, lift_pure, traverse_effect};
use crate::core::{FileMetrics, Language};
use crate::effects::AnalysisEffect;
use crate::env::RealEnv;
use crate::errors::AnalysisError;
use std::path::PathBuf;
use stillwater::Effect;

/// Analyze a single file as an effect.
///
/// This effect wraps file analysis in the effect system, enabling
/// configuration access and testability.
pub fn analyze_file_effect(
    path: PathBuf,
    content: String,
    language: Language,
) -> AnalysisEffect<FileMetrics> {
    analyze_with_env(move |_env| {
        let analyzer = get_analyzer(language);
        analyze_file(content.clone(), path.clone(), analyzer.as_ref())
            .map_err(|e| AnalysisError::analysis(format!("File analysis failed: {}", e)))
    })
}

/// Analyze multiple files as an effect.
///
/// This effect runs analysis on each file sequentially and collects results.
pub fn analyze_files_effect(
    files: Vec<(PathBuf, String, Language)>,
) -> AnalysisEffect<Vec<FileMetrics>> {
    traverse_effect(files, |(path, content, lang)| {
        analyze_file_effect(path, content, lang)
    })
}

/// Analyze a file with a specific analyzer.
///
/// This is a pure function wrapped in an effect for API consistency.
pub fn analyze_with_analyzer_effect(
    path: PathBuf,
    content: String,
    analyzer: &'static dyn Analyzer,
) -> AnalysisEffect<FileMetrics> {
    let result = analyze_file(content, path, analyzer);
    match result {
        Ok(metrics) => lift_pure(metrics),
        Err(e) => {
            crate::effects::effect_fail(AnalysisError::analysis(format!("Analysis failed: {}", e)))
        }
    }
}

/// Detect language from file path and analyze.
///
/// This effect auto-detects the language from the file extension and
/// runs the appropriate analyzer.
pub fn analyze_file_auto_effect(path: PathBuf, content: String) -> AnalysisEffect<FileMetrics> {
    let language = Language::from_path(&path);
    analyze_file_effect(path, content, language)
}

// =============================================================================
// Project-Level Analysis
// =============================================================================

/// Result of analyzing a project.
#[derive(Debug, Clone)]
pub struct ProjectAnalysisResult {
    /// Metrics for each analyzed file
    pub file_metrics: Vec<FileMetrics>,
    /// Total number of files processed
    pub file_count: usize,
    /// Total complexity across all files
    pub total_complexity: u32,
    /// Average complexity per file
    pub average_complexity: f64,
}

impl ProjectAnalysisResult {
    /// Create a new result from file metrics
    pub fn from_metrics(metrics: Vec<FileMetrics>) -> Self {
        let file_count = metrics.len();
        let total_complexity: u32 = metrics
            .iter()
            .map(|m| m.complexity.cyclomatic_complexity + m.complexity.cognitive_complexity)
            .sum();
        let average_complexity = if file_count > 0 {
            total_complexity as f64 / file_count as f64
        } else {
            0.0
        };

        Self {
            file_metrics: metrics,
            file_count,
            total_complexity,
            average_complexity,
        }
    }
}

/// Analyze a project (multiple files) as an effect.
///
/// This effect analyzes all provided files and aggregates the results
/// into a `ProjectAnalysisResult`.
pub fn analyze_project_effect(
    files: Vec<(PathBuf, String)>,
) -> AnalysisEffect<ProjectAnalysisResult> {
    let files_with_lang: Vec<(PathBuf, String, Language)> = files
        .into_iter()
        .map(|(path, content)| {
            let lang = Language::from_path(&path);
            (path, content, lang)
        })
        .collect();

    let files_effect = analyze_files_effect(files_with_lang);

    crate::effects::effect_from_fn(move |env: &RealEnv| {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| AnalysisError::analysis("Tokio runtime not available"))?;

        let result = runtime.block_on(async { files_effect.run(env).await })?;

        Ok(ProjectAnalysisResult::from_metrics(result))
    })
}

// =============================================================================
// Backwards Compatibility
// =============================================================================

/// Analyze a file (backwards-compatible wrapper).
pub fn analyze_file_result(
    path: &std::path::Path,
    content: &str,
    language: Language,
) -> anyhow::Result<FileMetrics> {
    let analyzer = get_analyzer(language);
    analyze_file(content.to_string(), path.to_path_buf(), analyzer.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::RealEnv;

    #[tokio::test]
    async fn test_analyze_file_effect_rust() {
        let env = RealEnv::default();
        let path = PathBuf::from("test.rs");
        let content = "fn main() { println!(\"Hello\"); }".to_string();

        let effect = analyze_file_effect(path, content, Language::Rust);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.language, Language::Rust);
    }

    #[tokio::test]
    async fn test_analyze_file_auto_effect() {
        let env = RealEnv::default();
        let path = PathBuf::from("test.py");
        let content = "def main(): print('hello')".to_string();

        let effect = analyze_file_auto_effect(path, content);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.language, Language::Python);
    }

    #[tokio::test]
    async fn test_analyze_files_effect() {
        let env = RealEnv::default();
        let files = vec![
            (
                PathBuf::from("a.rs"),
                "fn a() {}".to_string(),
                Language::Rust,
            ),
            (
                PathBuf::from("b.rs"),
                "fn b() {}".to_string(),
                Language::Rust,
            ),
        ];

        let effect = analyze_files_effect(files);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert_eq!(metrics.len(), 2);
    }

    #[test]
    fn test_project_analysis_result_from_metrics() {
        let metrics = vec![
            FileMetrics {
                path: PathBuf::from("a.rs"),
                language: Language::Rust,
                complexity: crate::core::ComplexityMetrics::default(),
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                module_scope: None,
                classes: None,
            },
            FileMetrics {
                path: PathBuf::from("b.rs"),
                language: Language::Rust,
                complexity: crate::core::ComplexityMetrics::default(),
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                module_scope: None,
                classes: None,
            },
        ];

        let result = ProjectAnalysisResult::from_metrics(metrics);
        assert_eq!(result.file_count, 2);
    }

    #[test]
    fn test_backwards_compat_analyze_file() {
        let path = PathBuf::from("test.rs");
        let content = "fn main() {}";

        let result = analyze_file_result(&path, content, Language::Rust);
        assert!(result.is_ok());
    }
}
