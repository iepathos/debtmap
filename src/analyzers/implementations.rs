//! Concrete implementations using dependency injection

use crate::analyzers::traits::{CallGraphAnalyzer, LanguageAnalyzer, PurityAnalyzer, TestDetector};
use crate::core::traits::{ComplexityCalculator, Detector, Scorer};
use anyhow::Result;
use std::path::Path;

/// Unified analyzer that uses injected dependencies
pub struct UnifiedAnalyzer<C, S, D>
where
    C: ComplexityCalculator,
    S: Scorer,
    D: Detector,
{
    #[allow(dead_code)]
    complexity_calculator: C,
    #[allow(dead_code)]
    scorer: S,
    #[allow(dead_code)]
    detector: D,
}

impl<C, S, D> UnifiedAnalyzer<C, S, D>
where
    C: ComplexityCalculator,
    S: Scorer,
    D: Detector,
{
    /// Create a new unified analyzer with injected dependencies
    pub fn new(complexity_calculator: C, scorer: S, detector: D) -> Self {
        Self {
            complexity_calculator,
            scorer,
            detector,
        }
    }
}

/// Rust analyzer implementation using dependency injection
pub struct RustAnalyzerImpl<CG, TD, PA>
where
    CG: CallGraphAnalyzer,
    TD: TestDetector,
    PA: PurityAnalyzer,
{
    call_graph: CG,
    test_detector: TD,
    #[allow(dead_code)]
    purity_analyzer: PA,
}

impl<CG, TD, PA> RustAnalyzerImpl<CG, TD, PA>
where
    CG: CallGraphAnalyzer,
    TD: TestDetector,
    PA: PurityAnalyzer,
{
    /// Create a new Rust analyzer with dependencies
    pub fn new(call_graph: CG, test_detector: TD, purity_analyzer: PA) -> Self {
        Self {
            call_graph,
            test_detector,
            purity_analyzer,
        }
    }
}

impl<CG, TD, PA> LanguageAnalyzer for RustAnalyzerImpl<CG, TD, PA>
where
    CG: CallGraphAnalyzer,
    TD: TestDetector,
    PA: PurityAnalyzer,
{
    fn analyze_file(&self, path: &Path, content: &str) -> Result<crate::core::FileMetrics> {
        // Implementation using injected dependencies
        let _graph = self.call_graph.build_call_graph(content, path)?;
        let _test_meta = self.test_detector.extract_test_metadata(content);

        // Simplified implementation for demonstration
        Ok(crate::core::FileMetrics {
            path: path.to_path_buf(),
            language: crate::core::Language::Rust,
            complexity: Default::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: content.lines().count(),
            module_scope: None,
            classes: None,
        })
    }

    fn language(&self) -> crate::core::Language {
        crate::core::Language::Rust
    }
}

/// Python analyzer implementation using dependency injection
pub struct PythonAnalyzerImpl<CG, TD, PA>
where
    CG: CallGraphAnalyzer,
    TD: TestDetector,
    PA: PurityAnalyzer,
{
    call_graph: CG,
    test_detector: TD,
    #[allow(dead_code)]
    purity_analyzer: PA,
}

impl<CG, TD, PA> PythonAnalyzerImpl<CG, TD, PA>
where
    CG: CallGraphAnalyzer,
    TD: TestDetector,
    PA: PurityAnalyzer,
{
    /// Create a new Python analyzer with dependencies
    pub fn new(call_graph: CG, test_detector: TD, purity_analyzer: PA) -> Self {
        Self {
            call_graph,
            test_detector,
            purity_analyzer,
        }
    }
}

impl<CG, TD, PA> LanguageAnalyzer for PythonAnalyzerImpl<CG, TD, PA>
where
    CG: CallGraphAnalyzer,
    TD: TestDetector,
    PA: PurityAnalyzer,
{
    fn analyze_file(&self, path: &Path, content: &str) -> Result<crate::core::FileMetrics> {
        // Implementation using injected dependencies
        let _graph = self.call_graph.build_call_graph(content, path)?;
        let _test_meta = self.test_detector.extract_test_metadata(content);

        // Simplified implementation for demonstration
        Ok(crate::core::FileMetrics {
            path: path.to_path_buf(),
            language: crate::core::Language::Python,
            complexity: Default::default(),
            debt_items: vec![],
            dependencies: vec![],
            duplications: vec![],
            total_lines: content.lines().count(),
            module_scope: None,
            classes: None,
        })
    }

    fn language(&self) -> crate::core::Language {
        crate::core::Language::Python
    }
}

/// Factory for creating analyzers with proper dependencies
pub struct AnalyzerFactory {
    // Dependencies can be stored here or created on demand
}

impl Default for AnalyzerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyzerFactory {
    /// Create a new factory
    pub fn new() -> Self {
        Self {}
    }

    /// Create a Rust analyzer with all dependencies
    pub fn create_rust_analyzer(&self) -> Box<dyn LanguageAnalyzer> {
        // In real implementation, would create actual dependencies
        // For now, returning a placeholder
        unimplemented!("Would create Rust analyzer with dependencies")
    }

    /// Create analyzer based on language
    pub fn create_analyzer(&self, language: crate::core::Language) -> Box<dyn LanguageAnalyzer> {
        match language {
            crate::core::Language::Rust => self.create_rust_analyzer(),
            crate::core::Language::Python => {
                panic!("Python analysis is not currently supported. Debtmap is focusing exclusively on Rust analysis.")
            }
            crate::core::Language::Unknown => panic!("Cannot create analyzer for unknown language"),
        }
    }
}

// ========= Adapter Implementations =========
// These adapters wrap the existing analyzer implementations
// to implement the LanguageAnalyzer trait

/// Adapter for RustAnalyzer to implement LanguageAnalyzer
#[allow(dead_code)]
struct RustAnalyzerAdapter {
    inner: crate::analyzers::rust::RustAnalyzer,
}

#[allow(dead_code)]
impl RustAnalyzerAdapter {
    fn new() -> Self {
        Self {
            inner: crate::analyzers::rust::RustAnalyzer::new(),
        }
    }
}

impl LanguageAnalyzer for RustAnalyzerAdapter {
    fn analyze_file(&self, path: &Path, content: &str) -> Result<crate::core::FileMetrics> {
        use crate::analyzers::Analyzer;
        let ast = self.inner.parse(content, path.to_path_buf())?;
        Ok(self.inner.analyze(&ast))
    }

    fn language(&self) -> crate::core::Language {
        crate::core::Language::Rust
    }
}
