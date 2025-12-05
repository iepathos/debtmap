//! Data structures flowing through the analysis pipeline.
//!
//! This module defines `PipelineData`, which holds all intermediate and final
//! results as data flows through pipeline stages.

use crate::core::FunctionMetrics;
use crate::priority::call_graph::CallGraph;
use crate::priority::{DebtType, UnifiedDebtItem};
use std::path::PathBuf;

/// Data flowing through the analysis pipeline.
///
/// This structure accumulates results as they flow through stages:
/// 1. File discovery → populates `files`
/// 2. Parsing → populates `metrics`
/// 3. Call graph → populates `call_graph`
/// 4. Coverage → populates `coverage` (optional)
/// 5. Purity → populates `purity_scores`
/// 6. Context → populates `context` (optional)
/// 7. Debt detection → populates `debt_items`
/// 8. Scoring → populates `scored_items`
///
/// Each stage adds its results without modifying previous stages' data.
#[derive(Clone, Debug)]
pub struct PipelineData {
    /// Files discovered in the project
    pub files: Vec<PathBuf>,

    /// Function metrics extracted from files
    pub metrics: Vec<FunctionMetrics>,

    /// Call graph relationships between functions
    pub call_graph: Option<CallGraph>,

    /// Test coverage data (optional)
    pub coverage: Option<CoverageData>,

    /// Purity analysis scores
    pub purity_scores: Option<PurityScores>,

    /// Project context information (optional)
    pub context: Option<ProjectContext>,

    /// Detected technical debt items
    pub debt_items: Vec<UnifiedDebtItem>,

    /// Scored and prioritized debt items
    pub scored_items: Vec<ScoredDebtItem>,
}

impl PipelineData {
    /// Create new pipeline data from discovered files.
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self {
            files,
            metrics: Vec::new(),
            call_graph: None,
            coverage: None,
            purity_scores: None,
            context: None,
            debt_items: Vec::new(),
            scored_items: Vec::new(),
        }
    }

    /// Add function metrics to the pipeline.
    pub fn with_metrics(mut self, metrics: Vec<FunctionMetrics>) -> Self {
        self.metrics = metrics;
        self
    }

    /// Add call graph to the pipeline.
    pub fn with_call_graph(mut self, graph: CallGraph) -> Self {
        self.call_graph = Some(graph);
        self
    }

    /// Add coverage data to the pipeline (optional).
    pub fn with_coverage(mut self, coverage: CoverageData) -> Self {
        self.coverage = Some(coverage);
        self
    }

    /// Add purity scores to the pipeline.
    pub fn with_purity(mut self, purity: PurityScores) -> Self {
        self.purity_scores = Some(purity);
        self
    }

    /// Add project context to the pipeline (optional).
    pub fn with_context(mut self, context: ProjectContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Add debt items to the pipeline.
    pub fn with_debt_items(mut self, items: Vec<UnifiedDebtItem>) -> Self {
        self.debt_items = items;
        self
    }

    /// Add scored items to the pipeline.
    pub fn with_scored_items(mut self, items: Vec<ScoredDebtItem>) -> Self {
        self.scored_items = items;
        self
    }
}

/// Test coverage data loaded from coverage files.
#[derive(Clone, Debug)]
pub struct CoverageData {
    /// Coverage percentage by file
    pub file_coverage: std::collections::HashMap<PathBuf, f64>,

    /// Line coverage information
    pub line_coverage: std::collections::HashMap<PathBuf, Vec<bool>>,
}

impl CoverageData {
    pub fn new() -> Self {
        Self {
            file_coverage: std::collections::HashMap::new(),
            line_coverage: std::collections::HashMap::new(),
        }
    }
}

impl Default for CoverageData {
    fn default() -> Self {
        Self::new()
    }
}

/// Purity analysis scores for functions.
#[derive(Clone, Debug)]
pub struct PurityScores {
    /// Purity score for each function (0.0 = impure, 1.0 = pure)
    pub scores: std::collections::HashMap<String, f64>,

    /// Functions identified as pure
    pub pure_functions: Vec<String>,

    /// Functions with I/O operations
    pub io_functions: Vec<String>,
}

impl PurityScores {
    pub fn new() -> Self {
        Self {
            scores: std::collections::HashMap::new(),
            pure_functions: Vec::new(),
            io_functions: Vec::new(),
        }
    }
}

impl Default for PurityScores {
    fn default() -> Self {
        Self::new()
    }
}

/// Project context information from README, docs, etc.
#[derive(Clone, Debug)]
pub struct ProjectContext {
    /// Project description
    pub description: Option<String>,

    /// Main technologies used
    pub technologies: Vec<String>,

    /// Domain-specific context
    pub domain: Option<String>,
}

impl ProjectContext {
    pub fn new() -> Self {
        Self {
            description: None,
            technologies: Vec::new(),
            domain: None,
        }
    }
}

impl Default for ProjectContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A debt item with its priority score.
#[derive(Clone, Debug)]
pub struct ScoredDebtItem {
    /// The debt item
    pub item: UnifiedDebtItem,

    /// Priority score (0.0 = low, 1.0 = high)
    pub score: f64,

    /// Debt category
    pub category: DebtType,
}

impl ScoredDebtItem {
    pub fn new(item: UnifiedDebtItem, score: f64, category: DebtType) -> Self {
        Self {
            item,
            score,
            category,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_data_creation() {
        let files = vec![PathBuf::from("src/main.rs")];
        let data = PipelineData::new(files.clone());

        assert_eq!(data.files, files);
        assert!(data.metrics.is_empty());
        assert!(data.call_graph.is_none());
    }

    #[test]
    fn test_pipeline_data_builder() {
        let files = vec![PathBuf::from("src/main.rs")];
        let metrics = vec![];

        let data = PipelineData::new(files.clone()).with_metrics(metrics.clone());

        assert_eq!(data.files, files);
        assert_eq!(data.metrics, metrics);
    }

    #[test]
    fn test_coverage_data_creation() {
        let coverage = CoverageData::new();
        assert!(coverage.file_coverage.is_empty());
        assert!(coverage.line_coverage.is_empty());
    }

    #[test]
    fn test_purity_scores_creation() {
        let purity = PurityScores::new();
        assert!(purity.scores.is_empty());
        assert!(purity.pure_functions.is_empty());
    }
}
