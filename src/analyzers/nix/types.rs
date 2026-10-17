use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct NixFunction {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub length: usize,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub is_test: bool,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NixAnalysis {
    pub functions: Vec<NixFunction>,
}
