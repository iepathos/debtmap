pub mod ast;
pub mod errors;
pub mod injection;
pub mod lazy;
pub mod metrics;
pub mod monadic;
pub mod traits;
pub mod types;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub project_path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub complexity: ComplexityReport,
    pub technical_debt: TechnicalDebtReport,
    pub dependencies: DependencyReport,
    pub duplications: Vec<DuplicationBlock>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub file_contexts: HashMap<PathBuf, crate::analysis::FileContext>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplexityReport {
    pub metrics: Vec<FunctionMetrics>,
    pub summary: ComplexitySummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplexitySummary {
    pub total_functions: usize,
    pub average_complexity: f64,
    pub max_complexity: u32,
    pub high_complexity_count: usize,
}

/// Refined purity classification that distinguishes local from external mutations.
///
/// This enum provides more nuanced purity analysis than the simple boolean `is_pure` field.
/// It enables better scoring for functions that use local mutations for efficiency but are
/// functionally pure (referentially transparent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurityLevel {
    /// No mutations whatsoever - pure mathematical functions
    StrictlyPure,

    /// Uses local mutations for efficiency but no external side effects
    /// (builder patterns, accumulators, owned `mut self`)
    LocallyPure,

    /// Reads external state but doesn't modify it (constants, `&self` methods)
    ReadOnly,

    /// Modifies external state or performs I/O (`&mut self`, statics, I/O)
    Impure,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FunctionMetrics {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub length: usize,
    pub is_test: bool,
    pub visibility: Option<String>, // "pub", "pub(crate)", or None for private
    pub is_trait_method: bool,      // Whether this is a trait method implementation
    pub in_test_module: bool,       // Whether this function is inside a #[cfg(test)] module
    pub entropy_score: Option<crate::complexity::entropy_core::EntropyScore>, // Optional entropy-based complexity score
    pub is_pure: Option<bool>, // Whether the function is pure (no side effects)
    pub purity_confidence: Option<f32>, // Confidence level of purity detection (0.0 to 1.0)
    pub purity_reason: Option<String>, // Reason for purity classification (from propagation)
    pub call_dependencies: Option<Vec<String>>, // Function IDs this function calls
    pub detected_patterns: Option<Vec<String>>, // Patterns detected for complexity adjustment
    pub upstream_callers: Option<Vec<String>>, // Functions that call this function
    pub downstream_callees: Option<Vec<String>>, // Functions that this function calls
    pub mapping_pattern_result:
        Option<crate::complexity::pure_mapping_patterns::MappingPatternResult>, // Pure mapping pattern detection result (spec 118)
    pub adjusted_complexity: Option<f64>, // Adjusted complexity score after mapping pattern detection (spec 118)
    pub composition_metrics: Option<crate::analysis::CompositionMetrics>, // AST-based functional composition metrics (spec 111)
    pub language_specific: Option<LanguageSpecificData>, // Language-specific pattern detection results (spec 146)

    // Refined purity classification (replaces is_pure eventually)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purity_level: Option<PurityLevel>,
}

/// Language-specific data to avoid memory overhead for non-applicable files
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LanguageSpecificData {
    Rust(crate::analysis::rust_patterns::RustPatternResult),
    // Future: Python(PythonPatternResult), JavaScript(JSPatternResult)
}

/// Entropy details for explainable output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyDetails {
    pub token_entropy: f64,
    pub pattern_repetition: f64,
    pub branch_similarity: f64,
    pub effective_complexity: f64,
    pub dampening_applied: bool,
    pub dampening_factor: f64,
    pub reasoning: Vec<String>,
}

impl FunctionMetrics {
    pub fn new(name: String, file: PathBuf, line: usize) -> Self {
        Self {
            name,
            file,
            line,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            length: 0,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
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
        }
    }

    pub fn is_complex(&self, threshold: u32) -> bool {
        self.cyclomatic > threshold || self.cognitive > threshold
    }

    /// Get entropy details with explanation for verbose output
    pub fn get_entropy_details(&self) -> Option<EntropyDetails> {
        self.entropy_score.as_ref().map(|score| {
            let mut reasoning = Vec::new();

            // Add reasoning based on metrics
            if score.pattern_repetition > 0.6 {
                reasoning.push(format!(
                    "High pattern repetition detected ({}%)",
                    (score.pattern_repetition * 100.0) as i32
                ));
            }

            if score.token_entropy < 0.4 {
                reasoning.push(format!(
                    "Low token entropy indicates simple patterns ({:.2})",
                    score.token_entropy
                ));
            }

            if score.branch_similarity > 0.7 {
                reasoning.push(format!(
                    "Similar branch structures found ({}% similarity)",
                    (score.branch_similarity * 100.0) as i32
                ));
            }

            let dampening_factor = 1.0 - score.effective_complexity;
            if dampening_factor > 0.3 {
                reasoning.push(format!(
                    "Complexity reduced by {}% due to pattern-based code",
                    (dampening_factor * 100.0) as i32
                ));
            } else {
                reasoning
                    .push("Genuine complexity detected - minimal reduction applied".to_string());
            }

            EntropyDetails {
                token_entropy: score.token_entropy,
                pattern_repetition: score.pattern_repetition,
                branch_similarity: score.branch_similarity,
                effective_complexity: score.effective_complexity,
                dampening_applied: dampening_factor > 0.1,
                dampening_factor,
                reasoning,
            }
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TechnicalDebtReport {
    pub items: Vec<DebtItem>,
    pub by_type: HashMap<DebtType, Vec<DebtItem>>,
    pub priorities: Vec<Priority>,
    pub duplications: Vec<DuplicationBlock>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DebtItem {
    pub id: String,
    pub debt_type: DebtType,
    pub priority: Priority,
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
    pub message: String,
    pub context: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum DebtType {
    Todo,
    Fixme,
    CodeSmell,
    Duplication,
    Complexity,
    Dependency,
    ErrorSwallowing,
    ResourceManagement,
    CodeOrganization,
    // Test-specific debt types
    TestComplexity,
    TestTodo,
    TestDuplication,
    TestQuality,
}

impl std::fmt::Display for DebtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        static DISPLAY_STRINGS: &[(DebtType, &str)] = &[
            (DebtType::Todo, "TODO"),
            (DebtType::Fixme, "FIXME"),
            (DebtType::CodeSmell, "Code Smell"),
            (DebtType::Duplication, "Duplication"),
            (DebtType::Complexity, "Complexity"),
            (DebtType::Dependency, "Dependency"),
            (DebtType::ErrorSwallowing, "Error Swallowing"),
            (DebtType::ResourceManagement, "Resource Management"),
            (DebtType::CodeOrganization, "Code Organization"),
            (DebtType::TestComplexity, "Test Complexity"),
            (DebtType::TestTodo, "Test TODO"),
            (DebtType::TestDuplication, "Test Duplication"),
            (DebtType::TestQuality, "Test Quality"),
        ];

        let display_str = DISPLAY_STRINGS
            .iter()
            .find(|(dt, _)| dt == self)
            .map(|(_, s)| *s)
            .unwrap_or("Unknown");

        write!(f, "{display_str}")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Ord, PartialOrd)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        static DISPLAY_STRINGS: &[(Priority, &str)] = &[
            (Priority::Low, "Low"),
            (Priority::Medium, "Medium"),
            (Priority::High, "High"),
            (Priority::Critical, "Critical"),
        ];

        let display_str = DISPLAY_STRINGS
            .iter()
            .find(|(p, _)| p == self)
            .map(|(_, s)| *s)
            .unwrap_or("Unknown");

        write!(f, "{display_str}")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependencyReport {
    pub modules: Vec<ModuleDependency>,
    pub circular: Vec<CircularDependency>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub module: String,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircularDependency {
    pub cycle: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DuplicationBlock {
    pub hash: String,
    pub lines: usize,
    pub locations: Vec<DuplicationLocation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DuplicationLocation {
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileMetrics {
    pub path: PathBuf,
    pub language: Language,
    pub complexity: ComplexityMetrics,
    pub debt_items: Vec<DebtItem>,
    pub dependencies: Vec<Dependency>,
    pub duplications: Vec<DuplicationBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_scope: Option<ast::ModuleScopeAnalysis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<ast::ClassDef>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ComplexityMetrics {
    pub functions: Vec<FunctionMetrics>,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
}

impl ComplexityMetrics {
    pub fn from_function(func: &FunctionMetrics) -> Self {
        Self {
            functions: vec![func.clone()],
            cyclomatic_complexity: func.cyclomatic,
            cognitive_complexity: func.cognitive,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub kind: DependencyKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DependencyKind {
    Import,
    Module,
    Package,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        static EXTENSION_MAP: &[(&[&str], Language)] = &[
            (&["rs"], Language::Rust),
            (&["js", "jsx", "mjs", "cjs"], Language::JavaScript),
            (&["ts", "tsx", "mts", "cts"], Language::TypeScript),
        ];

        EXTENSION_MAP
            .iter()
            .find(|(exts, _)| exts.contains(&ext))
            .map(|(_, lang)| *lang)
            .unwrap_or(Language::Unknown)
    }

    pub fn from_path(path: &std::path::Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Language::Unknown)
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        static DISPLAY_STRINGS: &[(Language, &str)] = &[
            (Language::Rust, "Rust"),
            (Language::Python, "Python"),
            (Language::JavaScript, "JavaScript"),
            (Language::TypeScript, "TypeScript"),
            (Language::Unknown, "Unknown"),
        ];

        let display_str = DISPLAY_STRINGS
            .iter()
            .find(|(l, _)| l == self)
            .map(|(_, s)| *s)
            .unwrap_or("Unknown");

        write!(f, "{display_str}")
    }
}
