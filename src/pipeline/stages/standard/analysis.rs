//! Analysis stages for call graphs, traits, and purity.
//!
//! These are thin wrapper stages that delegate to pure functions in sibling modules:
//! - `super::call_graph::build_call_graph`
//! - `super::purity::analyze_purity`

use crate::errors::AnalysisError;
use crate::pipeline::data::PipelineData;
use crate::pipeline::stage::Stage;
use std::path::{Path, PathBuf};

/// Stage 3: Build call graph
///
/// Constructs function call relationships from the parsed metrics.
pub struct CallGraphStage;

impl CallGraphStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CallGraphStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for CallGraphStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let graph = super::super::call_graph::build_call_graph(&data.metrics);
        data.call_graph = Some(graph);
        Ok(data)
    }

    fn name(&self) -> &str {
        "Call Graph Construction"
    }
}

/// Stage 4: Resolve trait calls
///
/// Resolves trait implementations and method calls for better call graph accuracy.
pub struct TraitResolutionStage {
    _project_path: PathBuf,
}

impl TraitResolutionStage {
    pub fn new(project_path: &Path) -> Self {
        Self {
            _project_path: project_path.to_path_buf(),
        }
    }
}

impl Stage for TraitResolutionStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, data: Self::Input) -> Result<Self::Output, Self::Error> {
        // Trait resolution currently integrated into call graph construction
        // This stage is a placeholder for future trait resolution logic
        Ok(data)
    }

    fn name(&self) -> &str {
        "Trait Resolution"
    }
}

/// Stage 6: Analyze function purity
///
/// Determines which functions are pure (no side effects) vs impure (I/O operations).
pub struct PurityAnalysisStage;

impl PurityAnalysisStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PurityAnalysisStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for PurityAnalysisStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        if let Some(ref call_graph) = data.call_graph {
            let purity = super::super::purity::analyze_purity(&data.metrics, call_graph);
            data.purity_scores = Some(purity);
        }
        Ok(data)
    }

    fn name(&self) -> &str {
        "Purity Analysis"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_graph_stage_creation() {
        let stage = CallGraphStage::new();
        assert_eq!(stage.name(), "Call Graph Construction");
    }

    #[test]
    fn test_trait_resolution_stage_creation() {
        let stage = TraitResolutionStage::new(Path::new("."));
        assert_eq!(stage.name(), "Trait Resolution");
    }

    #[test]
    fn test_purity_analysis_stage_creation() {
        let stage = PurityAnalysisStage::new();
        assert_eq!(stage.name(), "Purity Analysis");
    }
}
