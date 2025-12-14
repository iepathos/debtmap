//! File parsing stage.
//!
//! Analyzes discovered files using language-specific parsers to extract
//! function metrics (complexity, LOC, parameters, etc.).

use crate::errors::AnalysisError;
use crate::pipeline::data::PipelineData;
use crate::pipeline::stage::Stage;

/// Stage 2: Parse files to extract metrics
///
/// Analyzes discovered files using language-specific parsers to extract
/// function metrics (complexity, LOC, parameters, etc.).
pub struct ParsingStage;

impl ParsingStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParsingStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for ParsingStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, data: Self::Input) -> Result<Self::Output, Self::Error> {
        // TODO: Integrate with existing analysis code
        // For now, return empty metrics to allow pipeline to compile
        log::warn!("ParsingStage not fully implemented - returning empty metrics");
        Ok(data)
    }

    fn name(&self) -> &str {
        "Parsing"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_stage_creation() {
        let stage = ParsingStage::new();
        assert_eq!(stage.name(), "Parsing");
    }
}
