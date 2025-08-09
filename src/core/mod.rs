pub mod ast;
pub mod cache;
pub mod lazy;
pub mod metrics;
pub mod monadic;

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FunctionMetrics {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub length: usize,
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
        }
    }

    pub fn is_complex(&self, threshold: u32) -> bool {
        self.cyclomatic > threshold || self.cognitive > threshold
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
}

impl std::fmt::Display for DebtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebtType::Todo => write!(f, "TODO"),
            DebtType::Fixme => write!(f, "FIXME"),
            DebtType::CodeSmell => write!(f, "Code Smell"),
            DebtType::Duplication => write!(f, "Duplication"),
            DebtType::Complexity => write!(f, "Complexity"),
            DebtType::Dependency => write!(f, "Dependency"),
        }
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
        match self {
            Priority::Low => write!(f, "Low"),
            Priority::Medium => write!(f, "Medium"),
            Priority::High => write!(f, "High"),
            Priority::Critical => write!(f, "Critical"),
        }
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub functions: Vec<FunctionMetrics>,
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
        match ext {
            "rs" => Language::Rust,
            "py" => Language::Python,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
            _ => Language::Unknown,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Python => write!(f, "Python"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::Unknown => write!(f, "Unknown"),
        }
    }
}
