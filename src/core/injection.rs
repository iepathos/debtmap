//! Dependency injection container and builder patterns

use crate::core::traits::{
    Analyzer, Cache, ConfigProvider, Formatter, Parser,
    PriorityCalculator, Scorer,
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
pub struct AnalyzerFactory {
    rust_parser: Arc<dyn Parser<Ast = syn::File>>,
    python_parser: Arc<dyn Parser<Ast = tree_sitter::Tree>>,
    js_parser: Arc<dyn Parser<Ast = tree_sitter::Tree>>,
    ts_parser: Arc<dyn Parser<Ast = tree_sitter::Tree>>,
}

impl AnalyzerFactory {
    /// Create analyzer for a specific language
    pub fn create_analyzer(
        &self,
        language: crate::core::types::Language,
    ) -> Box<dyn Analyzer<Input = String, Output = crate::core::types::ModuleInfo>> {
        match language {
            crate::core::types::Language::Rust => {
                // Create a simple no-op analyzer for demonstration
                Box::new(NoOpAnalyzer::new(crate::core::types::Language::Rust))
            }
            crate::core::types::Language::Python => {
                // Create a simple no-op analyzer for demonstration
                Box::new(NoOpAnalyzer::new(crate::core::types::Language::Python))
            }
            crate::core::types::Language::JavaScript => {
                // Create a simple no-op analyzer for demonstration
                Box::new(NoOpAnalyzer::new(crate::core::types::Language::JavaScript))
            }
            crate::core::types::Language::TypeScript => {
                // Create a simple no-op analyzer for demonstration
                Box::new(NoOpAnalyzer::new(crate::core::types::Language::TypeScript))
            }
        }
    }
}

/// Simple no-op analyzer for demonstration purposes
struct NoOpAnalyzer {
    language: crate::core::types::Language,
}

impl NoOpAnalyzer {
    fn new(language: crate::core::types::Language) -> Self {
        Self { language }
    }
}

impl Analyzer for NoOpAnalyzer {
    type Input = String;
    type Output = crate::core::types::ModuleInfo;

    fn analyze(&self, _input: Self::Input) -> anyhow::Result<Self::Output> {
        // Return a simple module info with minimal data
        Ok(crate::core::types::ModuleInfo {
            name: "module".to_string(),
            language: self.language,
            path: std::path::PathBuf::from("unknown"),
            functions: vec![],
            exports: vec![],
            imports: vec![],
        })
    }

    fn name(&self) -> &str {
        match self.language {
            crate::core::types::Language::Rust => "NoOpRustAnalyzer",
            crate::core::types::Language::Python => "NoOpPythonAnalyzer",
            crate::core::types::Language::JavaScript => "NoOpJavaScriptAnalyzer",
            crate::core::types::Language::TypeScript => "NoOpTypeScriptAnalyzer",
        }
    }
}

/// Service locator pattern for runtime resolution
pub struct ServiceLocator {
    services: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
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
