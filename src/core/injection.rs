//! Dependency injection container and builder patterns

use crate::core::traits::{
    Analyzer, Cache, CacheStats, ConfigProvider, Formatter, PriorityCalculator, Scorer,
};
use anyhow::Result;
use std::sync::Arc;

/// Main application container using dependency injection
pub struct AppContainer {
    /// Rust language analyzer
    pub rust_analyzer: Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>,
    /// Python language analyzer
    pub python_analyzer: Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>,
    /// JavaScript language analyzer
    pub js_analyzer: Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>,
    /// TypeScript language analyzer
    pub ts_analyzer: Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>,
    /// Debt scorer
    pub debt_scorer: Arc<dyn Scorer<Item = crate::core::types::DebtItem>>,
    /// Analysis cache
    pub cache: Arc<dyn Cache<Key = String, Value = Vec<u8>>>,
    /// Configuration provider
    pub config: Arc<dyn ConfigProvider>,
    /// Priority calculator
    pub priority_calculator: Arc<dyn PriorityCalculator<Item = crate::core::types::DebtItem>>,
    /// Formatters
    pub json_formatter: Arc<dyn Formatter<Report = crate::core::types::AnalysisResult>>,
    pub markdown_formatter: Arc<dyn Formatter<Report = crate::core::types::AnalysisResult>>,
    pub terminal_formatter: Arc<dyn Formatter<Report = crate::core::types::AnalysisResult>>,
}

/// Builder for the application container
pub struct AppContainerBuilder {
    rust_analyzer:
        Option<Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>>,
    python_analyzer:
        Option<Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>>,
    js_analyzer: Option<Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>>,
    ts_analyzer: Option<Arc<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>>>,
    debt_scorer: Option<Arc<dyn Scorer<Item = crate::core::types::DebtItem>>>,
    cache: Option<Arc<dyn Cache<Key = String, Value = Vec<u8>>>>,
    config: Option<Arc<dyn ConfigProvider>>,
    priority_calculator: Option<Arc<dyn PriorityCalculator<Item = crate::core::types::DebtItem>>>,
    json_formatter: Option<Arc<dyn Formatter<Report = crate::core::types::AnalysisResult>>>,
    markdown_formatter: Option<Arc<dyn Formatter<Report = crate::core::types::AnalysisResult>>>,
    terminal_formatter: Option<Arc<dyn Formatter<Report = crate::core::types::AnalysisResult>>>,
}

impl Default for AppContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppContainerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            rust_analyzer: None,
            python_analyzer: None,
            js_analyzer: None,
            ts_analyzer: None,
            debt_scorer: None,
            cache: None,
            config: None,
            priority_calculator: None,
            json_formatter: None,
            markdown_formatter: None,
            terminal_formatter: None,
        }
    }

    /// Set the Rust analyzer
    pub fn with_rust_analyzer(
        mut self,
        analyzer: impl Analyzer<Input = String, Output = crate::core::types::ModuleInfo> + 'static,
    ) -> Self {
        self.rust_analyzer = Some(Arc::new(analyzer));
        self
    }

    /// Set the Python analyzer
    pub fn with_python_analyzer(
        mut self,
        analyzer: impl Analyzer<Input = String, Output = crate::core::types::ModuleInfo> + 'static,
    ) -> Self {
        self.python_analyzer = Some(Arc::new(analyzer));
        self
    }

    /// Set the JavaScript analyzer
    pub fn with_js_analyzer(
        mut self,
        analyzer: impl Analyzer<Input = String, Output = crate::core::types::ModuleInfo> + 'static,
    ) -> Self {
        self.js_analyzer = Some(Arc::new(analyzer));
        self
    }

    /// Set the TypeScript analyzer
    pub fn with_ts_analyzer(
        mut self,
        analyzer: impl Analyzer<Input = String, Output = crate::core::types::ModuleInfo> + 'static,
    ) -> Self {
        self.ts_analyzer = Some(Arc::new(analyzer));
        self
    }

    /// Set the debt scorer
    pub fn with_debt_scorer(
        mut self,
        scorer: impl Scorer<Item = crate::core::types::DebtItem> + 'static,
    ) -> Self {
        self.debt_scorer = Some(Arc::new(scorer));
        self
    }

    /// Set the cache implementation
    pub fn with_cache(
        mut self,
        cache: impl Cache<Key = String, Value = Vec<u8>> + 'static,
    ) -> Self {
        self.cache = Some(Arc::new(cache));
        self
    }

    /// Set the configuration provider
    pub fn with_config(mut self, config: impl ConfigProvider + 'static) -> Self {
        self.config = Some(Arc::new(config));
        self
    }

    /// Set the priority calculator
    pub fn with_priority_calculator(
        mut self,
        calculator: impl PriorityCalculator<Item = crate::core::types::DebtItem> + 'static,
    ) -> Self {
        self.priority_calculator = Some(Arc::new(calculator));
        self
    }

    /// Set the JSON formatter
    pub fn with_json_formatter(
        mut self,
        formatter: impl Formatter<Report = crate::core::types::AnalysisResult> + 'static,
    ) -> Self {
        self.json_formatter = Some(Arc::new(formatter));
        self
    }

    /// Set the Markdown formatter
    pub fn with_markdown_formatter(
        mut self,
        formatter: impl Formatter<Report = crate::core::types::AnalysisResult> + 'static,
    ) -> Self {
        self.markdown_formatter = Some(Arc::new(formatter));
        self
    }

    /// Set the terminal formatter
    pub fn with_terminal_formatter(
        mut self,
        formatter: impl Formatter<Report = crate::core::types::AnalysisResult> + 'static,
    ) -> Self {
        self.terminal_formatter = Some(Arc::new(formatter));
        self
    }

    /// Build the container
    pub fn build(self) -> Result<AppContainer, String> {
        Ok(AppContainer {
            rust_analyzer: self.rust_analyzer.ok_or("Rust analyzer is required")?,
            python_analyzer: self.python_analyzer.ok_or("Python analyzer is required")?,
            js_analyzer: self.js_analyzer.ok_or("JavaScript analyzer is required")?,
            ts_analyzer: self.ts_analyzer.ok_or("TypeScript analyzer is required")?,
            debt_scorer: self.debt_scorer.ok_or("Debt scorer is required")?,
            cache: self.cache.ok_or("Cache is required")?,
            config: self.config.ok_or("Config provider is required")?,
            priority_calculator: self
                .priority_calculator
                .ok_or("Priority calculator is required")?,
            json_formatter: self.json_formatter.ok_or("JSON formatter is required")?,
            markdown_formatter: self
                .markdown_formatter
                .ok_or("Markdown formatter is required")?,
            terminal_formatter: self
                .terminal_formatter
                .ok_or("Terminal formatter is required")?,
        })
    }
}

/// Factory trait for creating instances
pub trait Factory<T> {
    /// Create a new instance
    fn create(&self) -> T;
}

/// Analyzer factory for creating language-specific analyzers
pub struct AnalyzerFactory;

impl AnalyzerFactory {
    /// Create analyzer for a specific language
    pub fn create_analyzer(
        &self,
        language: crate::core::types::Language,
    ) -> Box<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>> {
        match language {
            crate::core::types::Language::Rust => Box::new(RustAnalyzerAdapter::new()),
            crate::core::types::Language::Python => Box::new(PythonAnalyzerAdapter::new()),
            crate::core::types::Language::JavaScript => Box::new(JavaScriptAnalyzerAdapter::new()),
            crate::core::types::Language::TypeScript => Box::new(TypeScriptAnalyzerAdapter::new()),
        }
    }
}

/// Adapter for Rust analyzer to implement Analyzer trait
pub struct RustAnalyzerAdapter {
    inner: crate::analyzers::rust::RustAnalyzer,
}

impl Default for RustAnalyzerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyzerAdapter {
    pub fn new() -> Self {
        Self {
            inner: crate::analyzers::rust::RustAnalyzer::new(),
        }
    }
}

impl Analyzer for RustAnalyzerAdapter {
    type Input = String;
    type Output = crate::core::types::ModuleInfo;

    fn analyze(&self, input: Self::Input) -> anyhow::Result<Self::Output> {
        // Parse the input string and analyze it using the existing Analyzer trait
        use crate::analyzers::Analyzer as AnalyzerImpl;
        let path = std::path::PathBuf::from("temp.rs");
        let ast = self.inner.parse(&input, path.clone())?;
        let file_metrics = self.inner.analyze(&ast);

        // Convert FileMetrics to ModuleInfo
        Ok(crate::core::types::ModuleInfo {
            name: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module")
                .to_string(),
            language: crate::core::types::Language::Rust,
            path: path.clone(),
            functions: file_metrics
                .complexity
                .functions
                .into_iter()
                .map(|f| crate::core::types::FunctionInfo {
                    name: f.name,
                    location: crate::core::types::SourceLocation {
                        file: path.clone(),
                        line: f.line,
                        column: 0,
                        end_line: Some(f.line + f.length),
                        end_column: Some(0),
                    },
                    parameters: vec![],
                    return_type: None,
                    is_public: true,
                    is_async: false,
                    is_generic: false,
                    doc_comment: None,
                })
                .collect(),
            exports: vec![],
            imports: file_metrics
                .dependencies
                .iter()
                .map(|d| d.name.clone())
                .collect(),
        })
    }

    fn name(&self) -> &str {
        "RustAnalyzer"
    }
}

/// Adapter for Python analyzer to implement Analyzer trait
pub struct PythonAnalyzerAdapter {
    inner: crate::analyzers::python::PythonAnalyzer,
}

impl Default for PythonAnalyzerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonAnalyzerAdapter {
    pub fn new() -> Self {
        Self {
            inner: crate::analyzers::python::PythonAnalyzer::new(),
        }
    }
}

impl Analyzer for PythonAnalyzerAdapter {
    type Input = String;
    type Output = crate::core::types::ModuleInfo;

    fn analyze(&self, input: Self::Input) -> anyhow::Result<Self::Output> {
        // Parse the input string and analyze it using the existing Analyzer trait
        use crate::analyzers::Analyzer as AnalyzerImpl;
        let path = std::path::PathBuf::from("temp.py");
        let ast = self.inner.parse(&input, path.clone())?;
        let file_metrics = self.inner.analyze(&ast);

        // Convert FileMetrics to ModuleInfo
        Ok(crate::core::types::ModuleInfo {
            name: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module")
                .to_string(),
            language: crate::core::types::Language::Python,
            path: path.clone(),
            functions: file_metrics
                .complexity
                .functions
                .into_iter()
                .map(|f| crate::core::types::FunctionInfo {
                    name: f.name,
                    location: crate::core::types::SourceLocation {
                        file: path.clone(),
                        line: f.line,
                        column: 0,
                        end_line: Some(f.line + f.length),
                        end_column: Some(0),
                    },
                    parameters: vec![],
                    return_type: None,
                    is_public: true,
                    is_async: false,
                    is_generic: false,
                    doc_comment: None,
                })
                .collect(),
            exports: vec![],
            imports: file_metrics
                .dependencies
                .iter()
                .map(|d| d.name.clone())
                .collect(),
        })
    }

    fn name(&self) -> &str {
        "PythonAnalyzer"
    }
}

/// Adapter for JavaScript analyzer to implement Analyzer trait
pub struct JavaScriptAnalyzerAdapter;

impl Default for JavaScriptAnalyzerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptAnalyzerAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JavaScriptAnalyzerAdapter {
    type Input = String;
    type Output = crate::core::types::ModuleInfo;

    fn analyze(&self, _input: Self::Input) -> anyhow::Result<Self::Output> {
        // JavaScript analysis implementation
        // For now, use a basic implementation
        let path = std::path::PathBuf::from("temp.js");

        Ok(crate::core::types::ModuleInfo {
            name: "module".to_string(),
            language: crate::core::types::Language::JavaScript,
            path,
            functions: vec![],
            exports: vec![],
            imports: vec![],
        })
    }

    fn name(&self) -> &str {
        "JavaScriptAnalyzer"
    }
}

/// Adapter for TypeScript analyzer to implement Analyzer trait
pub struct TypeScriptAnalyzerAdapter;

impl Default for TypeScriptAnalyzerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptAnalyzerAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TypeScriptAnalyzerAdapter {
    type Input = String;
    type Output = crate::core::types::ModuleInfo;

    fn analyze(&self, _input: Self::Input) -> anyhow::Result<Self::Output> {
        // TypeScript analysis implementation
        // For now, use a basic implementation
        let path = std::path::PathBuf::from("temp.ts");

        Ok(crate::core::types::ModuleInfo {
            name: "module".to_string(),
            language: crate::core::types::Language::TypeScript,
            path,
            functions: vec![],
            exports: vec![],
            imports: vec![],
        })
    }

    fn name(&self) -> &str {
        "TypeScriptAnalyzer"
    }
}

/// Service locator pattern for runtime resolution
pub struct ServiceLocator {
    services: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl Default for ServiceLocator {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceLocator {
    /// Create a new service locator
    pub fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
        }
    }

    /// Register a service
    pub fn register<T: 'static + Send + Sync>(&mut self, service: T) {
        let type_id = std::any::TypeId::of::<T>();
        self.services.insert(type_id, Box::new(service));
    }

    /// Resolve a service
    pub fn resolve<T: 'static>(&self) -> Option<&T> {
        let type_id = std::any::TypeId::of::<T>();
        self.services
            .get(&type_id)
            .and_then(|service| service.downcast_ref::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::traits::PriorityFactor;
    use crate::core::types::{DebtCategory, DebtItem, Language, ModuleInfo};

    // Mock implementations for testing
    struct MockAnalyzer {
        language: Language,
    }

    impl Analyzer for MockAnalyzer {
        type Input = String;
        type Output = ModuleInfo;

        fn analyze(&self, _input: Self::Input) -> anyhow::Result<Self::Output> {
            Ok(ModuleInfo {
                name: "test_module".to_string(),
                language: self.language,
                path: std::path::PathBuf::from("test.rs"),
                functions: vec![],
                exports: vec![],
                imports: vec![],
            })
        }

        fn name(&self) -> &str {
            "MockAnalyzer"
        }
    }

    struct MockScorer;

    impl Scorer for MockScorer {
        type Item = DebtItem;

        fn score(&self, item: &Self::Item) -> f64 {
            match item.category {
                DebtCategory::Complexity => 5.0,
                DebtCategory::Testing => 3.0,
                _ => 1.0,
            }
        }

        fn methodology(&self) -> &str {
            "Mock scoring based on debt type"
        }
    }

    struct MockCache {
        data: std::collections::HashMap<String, Vec<u8>>,
    }

    impl Cache for MockCache {
        type Key = String;
        type Value = Vec<u8>;

        fn get(&self, key: &Self::Key) -> Option<Self::Value> {
            self.data.get(key).cloned()
        }

        fn set(&mut self, key: Self::Key, value: Self::Value) {
            self.data.insert(key, value);
        }

        fn clear(&mut self) {
            self.data.clear();
        }

        fn stats(&self) -> CacheStats {
            CacheStats {
                hits: 0,
                misses: 0,
                entries: self.data.len(),
                memory_usage: 0,
            }
        }
    }

    struct MockConfigProvider;

    impl ConfigProvider for MockConfigProvider {
        fn get(&self, key: &str) -> Option<String> {
            match key {
                "complexity_threshold" => Some("10".to_string()),
                "max_file_size" => Some("1000000".to_string()),
                _ => None,
            }
        }

        fn set(&mut self, _key: String, _value: String) {
            // Mock implementation
        }

        fn load_from_file(&self, _path: &std::path::Path) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct MockPriorityCalculator;

    impl PriorityCalculator for MockPriorityCalculator {
        type Item = DebtItem;

        fn calculate_priority(&self, item: &Self::Item) -> f64 {
            match item.category {
                DebtCategory::Complexity => 0.8,
                DebtCategory::Testing => 0.5,
                _ => 0.2,
            }
        }

        fn get_factors(&self, _item: &Self::Item) -> Vec<PriorityFactor> {
            vec![PriorityFactor {
                name: "debt_type".to_string(),
                weight: 1.0,
                value: 0.5,
                description: "Mock factor".to_string(),
            }]
        }
    }

    struct MockFormatter;

    impl Formatter for MockFormatter {
        type Report = crate::core::types::AnalysisResult;

        fn format(&self, _report: &Self::Report) -> anyhow::Result<String> {
            Ok("Mock formatted report".to_string())
        }

        fn format_name(&self) -> &str {
            "mock"
        }
    }

    #[test]
    fn test_app_container_builder() {
        let builder = AppContainerBuilder::new()
            .with_rust_analyzer(MockAnalyzer {
                language: Language::Rust,
            })
            .with_python_analyzer(MockAnalyzer {
                language: Language::Python,
            })
            .with_js_analyzer(MockAnalyzer {
                language: Language::JavaScript,
            })
            .with_ts_analyzer(MockAnalyzer {
                language: Language::TypeScript,
            })
            .with_debt_scorer(MockScorer)
            .with_cache(MockCache {
                data: std::collections::HashMap::new(),
            })
            .with_config(MockConfigProvider)
            .with_priority_calculator(MockPriorityCalculator)
            .with_json_formatter(MockFormatter)
            .with_markdown_formatter(MockFormatter)
            .with_terminal_formatter(MockFormatter);

        let container = builder.build();
        assert!(container.is_ok());
    }

    #[test]
    fn test_builder_missing_analyzer() {
        let builder = AppContainerBuilder::new()
            .with_python_analyzer(MockAnalyzer {
                language: Language::Python,
            })
            .with_js_analyzer(MockAnalyzer {
                language: Language::JavaScript,
            })
            .with_ts_analyzer(MockAnalyzer {
                language: Language::TypeScript,
            })
            .with_debt_scorer(MockScorer)
            .with_cache(MockCache {
                data: std::collections::HashMap::new(),
            })
            .with_config(MockConfigProvider)
            .with_priority_calculator(MockPriorityCalculator)
            .with_json_formatter(MockFormatter)
            .with_markdown_formatter(MockFormatter)
            .with_terminal_formatter(MockFormatter);

        let container = builder.build();
        assert!(container.is_err());
        if let Err(msg) = container {
            assert!(msg.contains("Rust analyzer is required"));
        }
    }

    #[test]
    fn test_service_locator() {
        let mut locator = ServiceLocator::new();

        // Register a service
        locator.register(MockScorer);

        // Resolve the service
        let scorer = locator.resolve::<MockScorer>();
        assert!(scorer.is_some());

        // Try to resolve non-existent service
        let missing = locator.resolve::<MockCache>();
        assert!(missing.is_none());
    }

    #[test]
    fn test_analyzer_factory() {
        let factory = AnalyzerFactory;

        let rust_analyzer = factory.create_analyzer(Language::Rust);
        assert_eq!(rust_analyzer.name(), "RustAnalyzer");

        let python_analyzer = factory.create_analyzer(Language::Python);
        assert_eq!(python_analyzer.name(), "PythonAnalyzer");

        let js_analyzer = factory.create_analyzer(Language::JavaScript);
        assert_eq!(js_analyzer.name(), "JavaScriptAnalyzer");

        let ts_analyzer = factory.create_analyzer(Language::TypeScript);
        assert_eq!(ts_analyzer.name(), "TypeScriptAnalyzer");
    }
}
