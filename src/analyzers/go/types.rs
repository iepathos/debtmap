use crate::core::PurityLevel;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoFunctionKind {
    Function,
    Method,
}

#[derive(Debug, Clone)]
pub struct GoFunction {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub length: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub kind: GoFunctionKind,
    pub is_test: bool,
    pub visibility: Option<String>,
    pub calls: Vec<String>,
    pub purity_level: PurityLevel,
    pub purity_confidence: f32,
    pub purity_patterns: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GoAnalysis {
    pub package_name: Option<String>,
    pub functions: Vec<GoFunction>,
}
