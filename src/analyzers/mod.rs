use crate::core::{ast::Ast, FileMetrics};
use anyhow::Result;

pub mod python;
pub mod rust;

pub trait Analyzer: Send + Sync {
    fn parse(&self, content: &str, path: std::path::PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;
    fn language(&self) -> crate::core::Language;
}

pub fn analyze_file(
    content: String,
    path: std::path::PathBuf,
    analyzer: &dyn Analyzer,
) -> Result<FileMetrics> {
    analyzer
        .parse(&content, path.clone())
        .map(transform_ast)
        .map(|ast| analyzer.analyze(&ast))
        .map(apply_filters)
}

fn transform_ast(ast: Ast) -> Ast {
    ast.transform(|a| a)
}

fn apply_filters(metrics: FileMetrics) -> FileMetrics {
    metrics
}

pub fn compose_analyzers(
    parsers: Vec<Box<dyn Fn(&str) -> Result<Ast>>>,
    transformers: Vec<Box<dyn Fn(Ast) -> Ast>>,
    calculators: Vec<Box<dyn Fn(&Ast) -> FileMetrics>>,
) -> impl Fn(&str) -> Result<FileMetrics> {
    move |content: &str| {
        let ast = parsers[0](content)?;
        let transformed = transformers.iter().fold(ast, |acc, f| f(acc));
        Ok(calculators[0](&transformed))
    }
}

pub fn get_analyzer(language: crate::core::Language) -> Box<dyn Analyzer> {
    match language {
        crate::core::Language::Rust => Box::new(rust::RustAnalyzer::new()),
        crate::core::Language::Python => Box::new(python::PythonAnalyzer::new()),
        _ => Box::new(NullAnalyzer),
    }
}

struct NullAnalyzer;

impl Analyzer for NullAnalyzer {
    fn parse(&self, _content: &str, _path: std::path::PathBuf) -> Result<Ast> {
        Ok(Ast::Unknown)
    }

    fn analyze(&self, _ast: &Ast) -> FileMetrics {
        FileMetrics {
            path: std::path::PathBuf::new(),
            language: crate::core::Language::Unknown,
            complexity: crate::core::ComplexityMetrics { functions: vec![] },
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
        }
    }

    fn language(&self) -> crate::core::Language {
        crate::core::Language::Unknown
    }
}
