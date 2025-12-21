//! Common type definitions used across the codebase

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Language enumeration for all supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
}

impl Language {
    /// Get file extensions for this language
    pub fn extensions(&self) -> &[&str] {
        match self {
            Language::Rust => &["rs"],
            Language::Python => &["py", "pyw"],
        }
    }

    /// Get the display name for this language
    pub fn display_name(&self) -> &str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
        }
    }
}

/// Severity levels for issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Major,
    Critical,
}

/// Location in source code
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: PathBuf, line: usize, column: usize) -> Self {
        Self {
            file,
            line,
            column,
            end_line: None,
            end_column: None,
        }
    }

    /// Set the end position
    pub fn with_end(mut self, end_line: usize, end_column: usize) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }
}

/// Function metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub location: SourceLocation,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub is_public: bool,
    pub is_async: bool,
    pub is_generic: bool,
    pub doc_comment: Option<String>,
}

/// Module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub language: Language,
    pub functions: Vec<FunctionInfo>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

/// Technical debt item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtItem {
    pub id: String,
    pub category: DebtCategory,
    pub severity: Severity,
    pub location: SourceLocation,
    pub description: String,
    pub impact: f64,
    pub effort: f64,
    pub priority: f64,
    pub suggestions: Vec<String>,
}

/// Categories of technical debt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebtCategory {
    Complexity,
    Duplication,
    Organization,
    Testing,
    Documentation,
    Performance,
    Security,
    Maintainability,
    ErrorHandling,
    CodeSmell,
}

impl DebtCategory {
    /// Get display name for this category
    pub fn display_name(&self) -> &str {
        match self {
            DebtCategory::Complexity => "Complexity",
            DebtCategory::Duplication => "Duplication",
            DebtCategory::Organization => "Organization",
            DebtCategory::Testing => "Testing",
            DebtCategory::Documentation => "Documentation",
            DebtCategory::Performance => "Performance",
            DebtCategory::Security => "Security",
            DebtCategory::Maintainability => "Maintainability",
            DebtCategory::ErrorHandling => "Error Handling",
            DebtCategory::CodeSmell => "Code Smell",
        }
    }
}

/// Analysis result container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub project_path: PathBuf,
    pub modules: Vec<ModuleInfo>,
    pub debt_items: Vec<DebtItem>,
    pub total_score: f64,
    pub metrics: ProjectMetrics,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Project-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetrics {
    pub total_files: usize,
    pub total_lines: usize,
    pub total_functions: usize,
    pub average_complexity: f64,
    pub test_coverage: Option<f64>,
    pub debt_score: f64,
    pub language_breakdown: HashMap<Language, usize>,
}

/// Configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project_path: PathBuf,
    pub output_format: OutputFormat,
    pub ignore_patterns: Vec<String>,
    pub thresholds: Thresholds,
    pub enable_cache: bool,
    pub parallel: bool,
    pub verbose: bool,
    pub enable_functional_analysis: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project_path: PathBuf::from("."),
            output_format: OutputFormat::Terminal,
            ignore_patterns: vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/venv/**".to_string(),
                "**/__pycache__/**".to_string(),
            ],
            thresholds: Thresholds::default(),
            enable_cache: true,
            parallel: true,
            verbose: false,
            enable_functional_analysis: false,
        }
    }
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
    Html,
}

/// Configurable thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub max_complexity: u32,
    pub max_function_length: usize,
    pub max_parameters: usize,
    pub max_nesting_depth: usize,
    pub min_test_coverage: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            max_complexity: 10,
            max_function_length: 50,
            max_parameters: 5,
            max_nesting_depth: 4,
            min_test_coverage: 80.0,
        }
    }
}

/// Error types for the application
#[derive(Debug, thiserror::Error)]
pub enum DebtmapError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}

/// Result type alias
pub type DebtmapResult<T> = Result<T, DebtmapError>;
