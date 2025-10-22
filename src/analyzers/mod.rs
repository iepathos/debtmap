use crate::core::{ast::Ast, FileMetrics, FunctionMetrics};
use crate::priority::file_metrics::FileDebtMetrics;
use anyhow::Result;
use std::path::Path;

pub mod call_graph;
pub mod call_graph_integration;
pub mod context_aware;
pub mod enhanced_analyzer;
pub mod file_analyzer;
pub mod function_registry;
pub mod implementations;
pub mod javascript;
pub mod purity_detector;
pub mod python;
pub mod python_ast_extraction;
pub mod python_asyncio_patterns;
pub mod python_detectors;
pub mod python_exception_flow;
pub mod python_purity;
pub mod rust;
pub mod rust_call_graph;
pub mod rust_constructor_detector;
pub mod rust_data_flow_analyzer;
pub mod rust_enum_converter_detector;
pub mod signature_extractor;
pub mod test_detector;
pub mod trait_implementation_tracker;
pub mod trait_resolver;
pub mod traits;
pub mod type_registry;
pub mod type_tracker;

pub use enhanced_analyzer::{AnalysisResult, EnhancedAnalyzer};

pub trait Analyzer: Send + Sync {
    fn parse(&self, content: &str, path: std::path::PathBuf) -> Result<Ast>;
    fn analyze(&self, ast: &Ast) -> FileMetrics;
    fn language(&self) -> crate::core::Language;
}

pub trait FileAnalyzer {
    fn analyze_file(&self, path: &Path, content: &str) -> Result<FileDebtMetrics>;
    fn aggregate_functions(&self, functions: &[FunctionMetrics]) -> FileDebtMetrics;
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

type Parser = Box<dyn Fn(&str) -> Result<Ast>>;
type Transformer = Box<dyn Fn(Ast) -> Ast>;
type Calculator = Box<dyn Fn(&Ast) -> FileMetrics>;

pub fn compose_analyzers(
    parsers: Vec<Parser>,
    transformers: Vec<Transformer>,
    calculators: Vec<Calculator>,
) -> impl Fn(&str) -> Result<FileMetrics> {
    move |content: &str| {
        let ast = parsers[0](content)?;
        let transformed = transformers.iter().fold(ast, |acc, f| f(acc));
        Ok(calculators[0](&transformed))
    }
}

pub fn get_analyzer(language: crate::core::Language) -> Box<dyn Analyzer> {
    use crate::core::Language;

    type AnalyzerFactory = fn() -> Box<dyn Analyzer>;

    static ANALYZER_MAP: &[(Language, AnalyzerFactory)] = &[
        (Language::Rust, || Box::new(rust::RustAnalyzer::new())),
        (Language::Python, || Box::new(python::PythonAnalyzer::new())),
        (Language::JavaScript, || {
            create_js_analyzer(javascript::JavaScriptAnalyzer::new_javascript, "JavaScript")
        }),
        (Language::TypeScript, || {
            create_js_analyzer(javascript::JavaScriptAnalyzer::new_typescript, "TypeScript")
        }),
    ];

    ANALYZER_MAP
        .iter()
        .find(|(lang, _)| *lang == language)
        .map(|(_, factory)| factory())
        .unwrap_or_else(|| Box::new(NullAnalyzer))
}

pub fn get_analyzer_with_context(
    language: crate::core::Language,
    context_aware: bool,
) -> Box<dyn Analyzer> {
    let base_analyzer = get_analyzer(language);

    if context_aware {
        Box::new(context_aware::ContextAwareAnalyzer::new(base_analyzer))
    } else {
        base_analyzer
    }
}

fn create_js_analyzer<F>(factory: F, lang_name: &str) -> Box<dyn Analyzer>
where
    F: Fn() -> Result<javascript::JavaScriptAnalyzer>,
{
    Box::new(factory().unwrap_or_else(|_| {
        eprintln!("Failed to initialize {lang_name} analyzer");
        factory().unwrap_or_else(|_| panic!("{lang_name} analyzer initialization failed"))
    }))
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
            complexity: crate::core::ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: None,
        }
    }

    fn language(&self) -> crate::core::Language {
        crate::core::Language::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_transform_ast() {
        let ast = Ast::Unknown;
        let result = transform_ast(ast);
        assert!(matches!(result, Ast::Unknown));
    }

    #[test]
    fn test_apply_filters() {
        let metrics = FileMetrics {
            path: PathBuf::from("test.rs"),
            language: crate::core::Language::Rust,
            complexity: crate::core::ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: None,
        };
        let result = apply_filters(metrics.clone());
        assert_eq!(result.path, metrics.path);
        assert_eq!(result.language, metrics.language);
    }

    #[test]
    fn test_get_analyzer_rust() {
        let analyzer = get_analyzer(crate::core::Language::Rust);
        assert_eq!(analyzer.language(), crate::core::Language::Rust);
    }

    #[test]
    fn test_get_analyzer_python() {
        let analyzer = get_analyzer(crate::core::Language::Python);
        assert_eq!(analyzer.language(), crate::core::Language::Python);
    }

    #[test]
    fn test_get_analyzer_unknown() {
        let analyzer = get_analyzer(crate::core::Language::Unknown);
        assert_eq!(analyzer.language(), crate::core::Language::Unknown);
    }

    #[test]
    fn test_null_analyzer_parse() {
        let analyzer = NullAnalyzer;
        let result = analyzer
            .parse("test content", PathBuf::from("test.txt"))
            .unwrap();
        assert!(matches!(result, Ast::Unknown));
    }

    #[test]
    fn test_null_analyzer_analyze() {
        let analyzer = NullAnalyzer;
        let ast = Ast::Unknown;
        let metrics = analyzer.analyze(&ast);
        assert_eq!(metrics.path, PathBuf::new());
        assert_eq!(metrics.language, crate::core::Language::Unknown);
        assert_eq!(metrics.complexity.functions.len(), 0);
        assert_eq!(metrics.debt_items.len(), 0);
        assert_eq!(metrics.dependencies.len(), 0);
        assert_eq!(metrics.duplications.len(), 0);
    }

    #[test]
    fn test_null_analyzer_language() {
        let analyzer = NullAnalyzer;
        assert_eq!(analyzer.language(), crate::core::Language::Unknown);
    }

    #[test]
    fn test_analyze_file() {
        let analyzer = NullAnalyzer;
        let content = String::from("test content");
        let path = PathBuf::from("test.txt");
        let result = analyze_file(content, path.clone(), &analyzer).unwrap();
        assert_eq!(result.language, crate::core::Language::Unknown);
    }

    #[test]
    fn test_compose_analyzers() {
        let parsers: Vec<Parser> = vec![Box::new(|_| Ok(Ast::Unknown))];
        let transformers: Vec<Transformer> = vec![Box::new(|ast| ast)];
        let calculators: Vec<Calculator> = vec![Box::new(|_| FileMetrics {
            path: PathBuf::from("test.rs"),
            language: crate::core::Language::Rust,
            complexity: crate::core::ComplexityMetrics::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            module_scope: None,
            classes: None,
        })];

        let analyzer = compose_analyzers(parsers, transformers, calculators);
        let result = analyzer("test content").unwrap();
        assert_eq!(result.path, PathBuf::from("test.rs"));
        assert_eq!(result.language, crate::core::Language::Rust);
    }
}
