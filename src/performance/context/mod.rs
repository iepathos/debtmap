use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct PatternContext {
    pub module_type: ModuleType,
    pub function_intent: FunctionIntent,
    pub architectural_pattern: Option<ArchitecturalPattern>,
    pub business_criticality: BusinessCriticality,
    pub performance_sensitivity: PerformanceSensitivity,
    pub confidence: f64,
}

impl Default for PatternContext {
    fn default() -> Self {
        Self {
            module_type: ModuleType::Production,
            function_intent: FunctionIntent::Unknown,
            architectural_pattern: None,
            business_criticality: BusinessCriticality::Utility,
            performance_sensitivity: PerformanceSensitivity::Medium,
            confidence: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleType {
    Production,
    Test,
    Benchmark,
    Example,
    Documentation,
    Utility,
    Infrastructure,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionIntent {
    BusinessLogic,
    Setup,
    Teardown,
    Validation,
    DataTransformation,
    IOWrapper,
    ErrorHandling,
    Configuration,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArchitecturalPattern {
    TestFixture,
    Builder,
    Factory,
    Repository,
    ServiceLayer,
    EventHandler,
    Middleware,
    DataAccess,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BusinessCriticality {
    Critical,       // Core business logic, hot paths
    Important,      // Supporting business operations
    Utility,        // Helper functions, utilities
    Infrastructure, // Framework, configuration
    Development,    // Tests, examples, debugging
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PerformanceSensitivity {
    High,       // Real-time, hot paths, user-facing
    Medium,     // Batch processing, background tasks
    Low,        // Setup, configuration, one-time operations
    Irrelevant, // Tests, examples, debugging
}

pub trait ContextAnalyzer {
    fn analyze_module_context(&self, file: &syn::File, file_path: &Path) -> PatternContext;
    fn analyze_function_context(
        &self,
        function: &syn::ItemFn,
        module_context: &PatternContext,
    ) -> PatternContext;
}

pub mod intent_classifier;
pub mod module_classifier;
pub mod severity_adjuster;

pub use intent_classifier::IntentClassifier;
pub use module_classifier::ModuleClassifier;
pub use severity_adjuster::SeverityAdjuster;
